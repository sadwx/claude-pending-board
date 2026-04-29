//! Auto-configure `WSLENV` so `WEZTERM_PANE` crosses the Windows→WSL boundary,
//! letting click-to-focus address an existing WSL tab via `wezterm cli
//! activate-pane`. Without this, the bash hook inside WSL never sees
//! `$WEZTERM_PANE` and the click falls through to spawn-a-new-tab.
//!
//! Runs at every app launch in a `spawn_blocking` task; idempotent — once the
//! token is in `WSLENV`, subsequent runs are a single registry read and a
//! debug log.

#![cfg(target_os = "windows")]

use std::os::windows::process::CommandExt;
use std::process::Command;

const TOKEN: &str = "WEZTERM_PANE/u";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Outcome of a single `ensure_wezterm_pane_in_wslenv` run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// HKCU already contained the token; nothing was written.
    Unchanged,
    /// HKCU was rewritten with a new value — any process launched before
    /// this point (notably wezterm-gui) is now running with stale env.
    Updated,
    /// WSL not detected, or the registry write failed; no env change took
    /// effect this run.
    NoOp,
}

pub fn ensure_wezterm_pane_in_wslenv() -> Status {
    if !wsl_detected() {
        tracing::debug!("WSL not detected; skipping WSLENV setup");
        return Status::NoOp;
    }

    // Windows merges WSLENV from HKLM (machine) and HKCU (user) when launching
    // new processes — but for non-PATH vars, USER overrides MACHINE entirely.
    // So if we only wrote HKCU we'd silently clobber any system-level tokens
    // (e.g. `JRE_HOME/p`). Read both, merge, and write the union to HKCU.
    let user_wslenv = read_user_wslenv();
    let machine_wslenv = read_machine_wslenv();
    match merge_wslenv(user_wslenv.as_deref(), machine_wslenv.as_deref(), TOKEN) {
        Some(new_value) => {
            tracing::info!(
                user_old = %user_wslenv.as_deref().unwrap_or("(unset)"),
                machine = %machine_wslenv.as_deref().unwrap_or("(unset)"),
                user_new = %new_value,
                "appending WEZTERM_PANE/u to user WSLENV"
            );
            match write_user_wslenv(&new_value) {
                Ok(()) => Status::Updated,
                Err(e) => {
                    tracing::warn!(error = %e, "failed to update WSLENV — click-to-focus inside WSL will fall back to spawn-a-new-tab");
                    Status::NoOp
                }
            }
        }
        None => {
            tracing::debug!("WSLENV already includes {}; nothing to do", TOKEN);
            Status::Unchanged
        }
    }
}

/// True if at least one `wezterm-gui.exe` process is currently running.
///
/// Used after a successful `Updated` to decide whether to surface a "restart
/// WezTerm" warning: WezTerm captures its WSLENV at launch and never re-reads
/// it, so a running instance has stale env after we updated HKCU.
pub fn wezterm_gui_running() -> bool {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::everything(),
    );
    sys.processes()
        .values()
        .any(|p| p.name().eq_ignore_ascii_case("wezterm-gui.exe"))
}

fn wsl_detected() -> bool {
    let Ok(output) = Command::new("wsl.exe")
        .args(["-l", "-q"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    // Treat any non-whitespace byte in stdout as "at least one distro". `wsl
    // -l -q` emits UTF-16 LE with a BOM, so a string parse would need
    // decoding — but we only care about presence, not content.
    output.stdout.iter().any(|b| !b.is_ascii_whitespace())
}

fn read_user_wslenv() -> Option<String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu.open_subkey("Environment").ok()?;
    env.get_value::<String, _>("WSLENV").ok()
}

fn read_machine_wslenv() -> Option<String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let env = hklm
        .open_subkey(r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment")
        .ok()?;
    env.get_value::<String, _>("WSLENV").ok()
}

fn write_user_wslenv(new_value: &str) -> Result<(), String> {
    // Shell out to PowerShell rather than writing the registry directly so
    // the .NET SetEnvironmentVariable wrapper handles the WM_SETTINGCHANGE
    // broadcast for us. This only runs the once when we actually need to
    // change the value.
    let escaped = new_value.replace('\'', "''");
    let script = format!("[Environment]::SetEnvironmentVariable('WSLENV', '{escaped}', 'User')");
    let status = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|e| format!("failed to spawn powershell: {e}"))?;
    if !status.success() {
        return Err(format!("powershell exited {status}"));
    }
    Ok(())
}

/// Pure-string merge: figure out what to write to `HKCU\Environment\WSLENV`.
///
/// Returns `Some(new_value)` if a write is needed, `None` if the user-level
/// value already contains `token` (idempotent re-run).
///
/// The trick: on first run the user-level value may be empty while the
/// machine-level value carries existing tokens (e.g. `JRE_HOME/p` set by an
/// installer). For non-PATH env vars Windows resolves USER OVER MACHINE at
/// process launch — so writing only `WEZTERM_PANE/u` to HKCU would clobber
/// the machine tokens for new processes. To preserve them, when HKCU is
/// empty we seed the new value with the machine value before appending our
/// token. Subsequent runs see HKCU is non-empty and respect that as-is.
fn merge_wslenv(user: Option<&str>, machine: Option<&str>, token: &str) -> Option<String> {
    // Idempotency: if HKCU already has the token we're done. We deliberately
    // don't inspect HKLM here — if the machine value carries the token the
    // user has effectively configured WSLENV manually and our HKCU write
    // would only overwrite their preference.
    if user.is_some_and(|u| contains_token(u, token)) {
        return None;
    }

    // Pick the seed: HKCU if non-empty, otherwise HKLM, otherwise empty.
    let seed = user
        .filter(|u| !u.is_empty())
        .or_else(|| machine.filter(|m| !m.is_empty()))
        .unwrap_or("");

    if seed.is_empty() {
        return Some(token.to_string());
    }

    // Don't double-up if the seed already has the token (only reachable when
    // HKCU was empty and HKLM has it — rare but possible).
    if contains_token(seed, token) {
        return Some(seed.to_string());
    }

    let trimmed = seed.trim_end_matches(':');
    Some(format!("{trimmed}:{token}"))
}

fn contains_token(value: &str, token: &str) -> bool {
    value.split(':').any(|t| !t.is_empty() && t == token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_into_empty_when_neither_set() {
        assert_eq!(merge_wslenv(None, None, TOKEN), Some(TOKEN.to_string()));
        assert_eq!(
            merge_wslenv(Some(""), Some(""), TOKEN),
            Some(TOKEN.to_string())
        );
    }

    #[test]
    fn merge_when_user_already_has_token_is_noop() {
        assert_eq!(merge_wslenv(Some("WEZTERM_PANE/u"), None, TOKEN), None);
        assert_eq!(
            merge_wslenv(Some("FOO/p:WEZTERM_PANE/u"), None, TOKEN),
            None
        );
        assert_eq!(
            merge_wslenv(Some("WEZTERM_PANE/u:BAR"), Some("anything"), TOKEN),
            None
        );
        assert_eq!(
            merge_wslenv(Some("FOO:WEZTERM_PANE/u:BAR"), None, TOKEN),
            None
        );
    }

    #[test]
    fn merge_appends_to_user_tokens_when_user_set() {
        assert_eq!(
            merge_wslenv(Some("USERPROFILE/p"), None, TOKEN),
            Some("USERPROFILE/p:WEZTERM_PANE/u".to_string())
        );
        assert_eq!(
            merge_wslenv(Some("FOO:BAR/p"), Some("ignored"), TOKEN),
            Some("FOO:BAR/p:WEZTERM_PANE/u".to_string())
        );
    }

    #[test]
    fn merge_seeds_from_machine_when_user_unset() {
        // The original bug: writing just `WEZTERM_PANE/u` to HKCU would
        // clobber the machine-level `JRE_HOME/p` for new processes (USER
        // wins over MACHINE). Seed with the machine value first.
        assert_eq!(
            merge_wslenv(None, Some("JRE_HOME/p"), TOKEN),
            Some("JRE_HOME/p:WEZTERM_PANE/u".to_string())
        );
        assert_eq!(
            merge_wslenv(Some(""), Some("JRE_HOME/p:OTHER/p"), TOKEN),
            Some("JRE_HOME/p:OTHER/p:WEZTERM_PANE/u".to_string())
        );
    }

    #[test]
    fn merge_machine_value_already_has_token() {
        // The token is in machine WSLENV but user is empty. We still need
        // to write the merged value to HKCU because USER-empty + MACHINE-set
        // means USER will win as empty otherwise. Result is the machine
        // value verbatim.
        assert_eq!(
            merge_wslenv(None, Some("WEZTERM_PANE/u"), TOKEN),
            Some("WEZTERM_PANE/u".to_string())
        );
        assert_eq!(
            merge_wslenv(None, Some("FOO:WEZTERM_PANE/u"), TOKEN),
            Some("FOO:WEZTERM_PANE/u".to_string())
        );
    }

    #[test]
    fn merge_strips_trailing_colon_to_avoid_empty_token() {
        assert_eq!(
            merge_wslenv(Some("FOO:"), None, TOKEN),
            Some("FOO:WEZTERM_PANE/u".to_string())
        );
    }

    #[test]
    fn merge_does_not_match_partial_tokens() {
        // "WEZTERM_PANE" without the /u suffix is a different token.
        assert_eq!(
            merge_wslenv(Some("WEZTERM_PANE"), None, TOKEN),
            Some("WEZTERM_PANE:WEZTERM_PANE/u".to_string())
        );
        // Substring shouldn't match either.
        assert_eq!(
            merge_wslenv(Some("OTHER_WEZTERM_PANE/u_THING"), None, TOKEN),
            Some("OTHER_WEZTERM_PANE/u_THING:WEZTERM_PANE/u".to_string())
        );
    }

    #[test]
    fn merge_handles_leading_colon() {
        assert_eq!(
            merge_wslenv(Some(":FOO"), None, TOKEN),
            Some(":FOO:WEZTERM_PANE/u".to_string())
        );
    }
}
