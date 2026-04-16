use crate::types::TerminalMatch;
use std::path::Path;

/// Known terminal binary names that we can focus programmatically.
const KNOWN_TERMINALS: &[&str] = &["wezterm-gui", "wezterm", "iTerm2"];

/// Trait for terminal-specific operations.
/// Implementations live in the `adapters` crate.
pub trait TerminalAdapter: Send + Sync {
    /// Name of this adapter (e.g. "WezTerm", "iTerm2").
    fn name(&self) -> &str;

    /// Check if this adapter's terminal is available on the system.
    fn is_available(&self) -> bool;

    /// Try to match the given `claude_pid` to a pane in this terminal.
    /// Returns `None` if the terminal doesn't own this session.
    fn detect(&self, claude_pid: u32) -> Option<TerminalMatch>;

    /// Focus the terminal pane identified by `terminal_match`.
    fn focus_pane(&self, terminal_match: &TerminalMatch) -> Result<(), AdapterError>;

    /// Spawn a new terminal tab running `claude --resume <session_id>` in `cwd`.
    fn spawn_resume(&self, cwd: &Path, session_id: &str) -> Result<(), AdapterError>;
}

#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("terminal binary not found in PATH")]
    BinaryNotFound,
    #[error("terminal command failed: {0}")]
    CommandFailed(String),
    #[error("no matching pane found")]
    NoPaneFound,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Walk the process tree from `start_pid` upward, looking for a process whose
/// name matches a known terminal. Returns the first match found.
///
/// `depth_cap` limits how far up we walk (default 20) to prevent infinite loops.
pub fn ancestor_walk(start_pid: u32, depth_cap: usize) -> Option<(String, u32)> {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

    let mut sys = System::new();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::everything(),
    );

    let mut current_pid = Pid::from_u32(start_pid);
    let mut visited = std::collections::HashSet::new();

    for _ in 0..depth_cap {
        if !visited.insert(current_pid) {
            tracing::warn!(pid = ?current_pid, "cycle detected in process tree walk");
            return None;
        }

        let process = sys.process(current_pid)?;
        let name = process.name().to_string_lossy().to_string();

        // Strip .exe suffix on Windows for matching
        let name_normalized = name.strip_suffix(".exe").unwrap_or(&name);

        if KNOWN_TERMINALS
            .iter()
            .any(|t| t.eq_ignore_ascii_case(name_normalized))
        {
            return Some((name, current_pid.as_u32()));
        }

        current_pid = process.parent()?;
    }

    tracing::debug!(start_pid, depth_cap, "ancestor walk exhausted depth cap");
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ancestor_walk_from_nonexistent_pid() {
        let result = ancestor_walk(0xFFFFFF, 20);
        assert!(result.is_none());
    }

    #[test]
    fn test_ancestor_walk_depth_cap() {
        let result = ancestor_walk(1, 1);
        let _ = result; // Just verify no panic or infinite loop
    }

    #[test]
    fn test_known_terminals_list() {
        assert!(KNOWN_TERMINALS.contains(&"wezterm-gui"));
        assert!(KNOWN_TERMINALS.contains(&"iTerm2"));
    }
}
