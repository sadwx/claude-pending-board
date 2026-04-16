use claude_pending_board_core::terminal::{AdapterError, TerminalAdapter};
use claude_pending_board_core::types::TerminalMatch;
use serde::Deserialize;
use std::path::Path;
use std::process::Command;

/// WezTerm pane info from `wezterm cli list --format json`.
#[derive(Debug, Deserialize)]
struct WezTermPane {
    #[allow(dead_code)]
    window_id: u64,
    #[allow(dead_code)]
    tab_id: u64,
    pane_id: u64,
    #[serde(default)]
    #[allow(dead_code)]
    title: String,
    #[serde(default)]
    #[allow(dead_code)]
    cwd: String,
}

pub struct WezTermAdapter;

impl Default for WezTermAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl WezTermAdapter {
    pub fn new() -> Self {
        Self
    }

    fn find_binary() -> Option<String> {
        if Command::new("wezterm")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some("wezterm".to_string());
        }

        #[cfg(target_os = "windows")]
        {
            let program_files = std::env::var("ProgramFiles").unwrap_or_default();
            let path = format!("{}\\WezTerm\\wezterm.exe", program_files);
            if std::path::Path::new(&path).exists() {
                return Some(path);
            }
        }

        None
    }

    fn list_panes() -> Result<Vec<WezTermPane>, AdapterError> {
        let binary = Self::find_binary().ok_or(AdapterError::BinaryNotFound)?;

        let output = Command::new(&binary)
            .args(["cli", "list", "--format", "json"])
            .output()
            .map_err(|e| {
                AdapterError::CommandFailed(format!("failed to run wezterm cli list: {e}"))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AdapterError::CommandFailed(format!(
                "wezterm cli list failed: {stderr}"
            )));
        }

        let panes: Vec<WezTermPane> = serde_json::from_slice(&output.stdout)
            .map_err(|e| AdapterError::CommandFailed(format!("failed to parse pane list: {e}")))?;

        Ok(panes)
    }

    fn activate_pane(pane_id: u64) -> Result<(), AdapterError> {
        let binary = Self::find_binary().ok_or(AdapterError::BinaryNotFound)?;

        let output = Command::new(&binary)
            .args(["cli", "activate-pane", "--pane-id", &pane_id.to_string()])
            .output()
            .map_err(|e| {
                AdapterError::CommandFailed(format!("failed to run activate-pane: {e}"))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AdapterError::CommandFailed(format!(
                "activate-pane failed: {stderr}"
            )));
        }

        Ok(())
    }

    fn find_pane_for_pid(claude_pid: u32, panes: &[WezTermPane]) -> Option<(u64, TerminalMatch)> {
        let (terminal_name, terminal_pid) =
            claude_pending_board_core::terminal::ancestor_walk(claude_pid, 20)?;

        if let Some(pane) = panes.first() {
            return Some((
                pane.pane_id,
                TerminalMatch {
                    terminal_name,
                    terminal_pid,
                    pane_id: Some(pane.pane_id.to_string()),
                    tty: None,
                },
            ));
        }

        None
    }
}

impl TerminalAdapter for WezTermAdapter {
    fn name(&self) -> &str {
        "WezTerm"
    }

    fn is_available(&self) -> bool {
        Self::find_binary().is_some()
    }

    fn detect(&self, claude_pid: u32) -> Option<TerminalMatch> {
        let panes = Self::list_panes().ok()?;
        Self::find_pane_for_pid(claude_pid, &panes).map(|(_, m)| m)
    }

    fn focus_pane(&self, terminal_match: &TerminalMatch) -> Result<(), AdapterError> {
        let pane_id: u64 = terminal_match
            .pane_id
            .as_ref()
            .ok_or(AdapterError::NoPaneFound)?
            .parse()
            .map_err(|e| AdapterError::CommandFailed(format!("invalid pane_id: {e}")))?;

        Self::activate_pane(pane_id)
    }

    fn spawn_resume(&self, cwd: &Path, session_id: &str) -> Result<(), AdapterError> {
        let binary = Self::find_binary().ok_or(AdapterError::BinaryNotFound)?;

        let output = Command::new(&binary)
            .args([
                "cli",
                "spawn",
                "--cwd",
                &cwd.to_string_lossy(),
                "--",
                "claude",
                "--resume",
                session_id,
            ])
            .output()
            .map_err(|e| AdapterError::CommandFailed(format!("failed to spawn: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AdapterError::CommandFailed(format!(
                "wezterm cli spawn failed: {stderr}"
            )));
        }

        tracing::info!(session_id, cwd = %cwd.display(), "spawned resume in new WezTerm tab");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wezterm_adapter_name() {
        let adapter = WezTermAdapter::new();
        assert_eq!(adapter.name(), "WezTerm");
    }

    #[test]
    fn test_find_pane_single_pane() {
        let panes = vec![WezTermPane {
            window_id: 0,
            tab_id: 0,
            pane_id: 42,
            title: "test".to_string(),
            cwd: "file:///tmp".to_string(),
        }];
        // Returns None because ancestor_walk won't find WezTerm from fake PID
        let result = WezTermAdapter::find_pane_for_pid(99999, &panes);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_wezterm_pane_json() {
        let json = r#"[{"window_id":0,"tab_id":0,"pane_id":0,"workspace":"default","size":{"rows":24,"cols":80},"title":"test","cwd":"file:///home/user"}]"#;
        let panes: Vec<WezTermPane> = serde_json::from_str(json).unwrap();
        assert_eq!(panes.len(), 1);
        assert_eq!(panes[0].pane_id, 0);
        assert_eq!(panes[0].cwd, "file:///home/user");
    }

    #[test]
    fn test_parse_wezterm_pane_json_with_extra_fields() {
        let json = r#"[{"window_id":0,"tab_id":0,"pane_id":5,"title":"x","cwd":"file:///tmp","future_field":true}]"#;
        let panes: Vec<WezTermPane> = serde_json::from_str(json).unwrap();
        assert_eq!(panes[0].pane_id, 5);
    }

    #[test]
    #[ignore]
    fn test_wezterm_is_available() {
        let adapter = WezTermAdapter::new();
        assert!(adapter.is_available(), "WezTerm binary not found in PATH");
    }

    #[test]
    #[ignore]
    fn test_wezterm_list_panes() {
        let panes = WezTermAdapter::list_panes().expect("failed to list panes");
        assert!(!panes.is_empty(), "no panes found — is WezTerm running?");
        for pane in &panes {
            println!(
                "  pane_id={} tab_id={} window_id={} title={:?} cwd={:?}",
                pane.pane_id, pane.tab_id, pane.window_id, pane.title, pane.cwd
            );
        }
    }
}
