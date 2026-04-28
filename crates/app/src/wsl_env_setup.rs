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

pub fn ensure_wezterm_pane_in_wslenv() {
    if !wsl_detected() {
        tracing::debug!("WSL not detected; skipping WSLENV setup");
        return;
    }

    let current = read_user_wslenv().unwrap_or_default();
    match merge_wslenv(&current, TOKEN) {
        Some(new_value) => {
            tracing::info!(
                old = %current,
                new = %new_value,
                "appending WEZTERM_PANE/u to user WSLENV"
            );
            if let Err(e) = write_user_wslenv(&new_value) {
                tracing::warn!(error = %e, "failed to update WSLENV — click-to-focus inside WSL will fall back to spawn-a-new-tab");
            }
        }
        None => {
            tracing::debug!("WSLENV already includes {}; nothing to do", TOKEN);
        }
    }
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

/// Pure-string merge: returns `Some(new_value)` if `token` was missing and
/// needs appending, `None` if it's already present (idempotent re-run).
/// Tokens are colon-separated, matching `WSLENV` syntax (the `/u` suffix on
/// our token signals "Windows → Unix path translation, treat as plain string"
/// — see Microsoft's WSLENV docs).
fn merge_wslenv(current: &str, token: &str) -> Option<String> {
    let already_present = current.split(':').any(|t| !t.is_empty() && t == token);
    if already_present {
        return None;
    }
    if current.is_empty() {
        return Some(token.to_string());
    }
    let trimmed = current.trim_end_matches(':');
    Some(format!("{trimmed}:{token}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_into_empty() {
        assert_eq!(merge_wslenv("", TOKEN), Some(TOKEN.to_string()));
    }

    #[test]
    fn merge_when_token_already_present_is_noop() {
        assert_eq!(merge_wslenv("WEZTERM_PANE/u", TOKEN), None);
        assert_eq!(merge_wslenv("FOO/p:WEZTERM_PANE/u", TOKEN), None);
        assert_eq!(merge_wslenv("WEZTERM_PANE/u:BAR", TOKEN), None);
        assert_eq!(merge_wslenv("FOO:WEZTERM_PANE/u:BAR", TOKEN), None);
    }

    #[test]
    fn merge_appends_to_existing_tokens() {
        assert_eq!(
            merge_wslenv("USERPROFILE/p", TOKEN),
            Some("USERPROFILE/p:WEZTERM_PANE/u".to_string())
        );
        assert_eq!(
            merge_wslenv("FOO:BAR/p", TOKEN),
            Some("FOO:BAR/p:WEZTERM_PANE/u".to_string())
        );
    }

    #[test]
    fn merge_strips_trailing_colon_to_avoid_empty_token() {
        assert_eq!(
            merge_wslenv("FOO:", TOKEN),
            Some("FOO:WEZTERM_PANE/u".to_string())
        );
    }

    #[test]
    fn merge_does_not_match_partial_tokens() {
        // "WEZTERM_PANE" without the /u suffix is a different token and
        // should not satisfy the check.
        assert_eq!(
            merge_wslenv("WEZTERM_PANE", TOKEN),
            Some("WEZTERM_PANE:WEZTERM_PANE/u".to_string())
        );
        // Substring shouldn't match either.
        assert_eq!(
            merge_wslenv("OTHER_WEZTERM_PANE/u_THING", TOKEN),
            Some("OTHER_WEZTERM_PANE/u_THING:WEZTERM_PANE/u".to_string())
        );
    }

    #[test]
    fn merge_handles_leading_colon() {
        // `:FOO` parses as ["", "FOO"]; the empty token is filtered out.
        assert_eq!(
            merge_wslenv(":FOO", TOKEN),
            Some(":FOO:WEZTERM_PANE/u".to_string())
        );
    }
}
