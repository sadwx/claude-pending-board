#![cfg(target_os = "macos")]

use claude_pending_board_core::terminal::{AdapterError, TerminalAdapter};
use claude_pending_board_core::types::TerminalMatch;
use std::path::Path;
use std::process::Command;

pub struct ITerm2Adapter;

impl ITerm2Adapter {
    pub fn new() -> Self {
        Self
    }

    fn is_iterm2_installed() -> bool {
        Path::new("/Applications/iTerm.app").exists()
    }

    fn run_osascript(script: &str) -> Result<String, AdapterError> {
        let output = Command::new("osascript")
            .args(["-e", script])
            .output()
            .map_err(|e| AdapterError::CommandFailed(format!("osascript failed: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AdapterError::CommandFailed(format!(
                "osascript error: {stderr}"
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn get_tty(pid: u32) -> Option<String> {
        let output = Command::new("ps")
            .args(["-o", "tty=", "-p", &pid.to_string()])
            .output()
            .ok()?;

        if output.status.success() {
            let tty = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !tty.is_empty() && tty != "??" {
                return Some(tty);
            }
        }
        None
    }
}

impl Default for ITerm2Adapter {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalAdapter for ITerm2Adapter {
    fn name(&self) -> &str {
        "iTerm2"
    }

    fn is_available(&self) -> bool {
        Self::is_iterm2_installed()
    }

    fn detect(&self, claude_pid: u32) -> Option<TerminalMatch> {
        let (terminal_name, terminal_pid) =
            claude_pending_board_core::terminal::ancestor_walk(claude_pid, 20)?;

        if !terminal_name.contains("iTerm") {
            return None;
        }

        let tty = Self::get_tty(claude_pid);

        Some(TerminalMatch {
            terminal_name,
            terminal_pid,
            pane_id: None,
            tty,
        })
    }

    fn focus_pane(&self, terminal_match: &TerminalMatch) -> Result<(), AdapterError> {
        let script = r#"tell application "iTerm2" to activate"#;
        Self::run_osascript(script)?;

        if let Some(tty) = &terminal_match.tty {
            let script = format!(
                r#"tell application "iTerm2"
    repeat with w in windows
        repeat with t in tabs of w
            repeat with s in sessions of t
                if tty of s contains "{tty}" then
                    select t
                    select s
                    return "found"
                end if
            end repeat
        end repeat
    end repeat
    return "not_found"
end tell"#,
                tty = tty
            );
            let result = Self::run_osascript(&script)?;
            if result == "not_found" {
                tracing::warn!(tty, "iTerm2 session with matching tty not found");
            }
        }

        Ok(())
    }

    fn spawn_resume(
        &self,
        cwd: &Path,
        session_id: &str,
        // iTerm2 only ships on macOS where WSL doesn't apply; the field is
        // accepted for trait compatibility and ignored.
        _wsl_distro: Option<&str>,
    ) -> Result<(), AdapterError> {
        let cwd_str = cwd.to_string_lossy();
        let script = format!(
            r#"tell application "iTerm2"
    activate
    tell current window
        create tab with default profile command "cd {cwd} && claude --resume {session_id}"
    end tell
end tell"#,
            cwd = cwd_str,
            session_id = session_id
        );
        Self::run_osascript(&script)?;

        tracing::info!(session_id, cwd = %cwd.display(), "spawned resume in new iTerm2 tab");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iterm2_adapter_name() {
        let adapter = ITerm2Adapter::new();
        assert_eq!(adapter.name(), "iTerm2");
    }

    #[test]
    fn test_detect_returns_none_for_fake_pid() {
        let adapter = ITerm2Adapter::new();
        assert!(adapter.detect(0xFFFFFF).is_none());
    }

    #[test]
    #[ignore]
    fn test_iterm2_is_available() {
        let adapter = ITerm2Adapter::new();
        assert!(
            adapter.is_available(),
            "iTerm2 not found at /Applications/iTerm.app"
        );
    }
}
