use crate::types::{Entry, EntryState, Op};
use chrono::Utc;
use serde::Deserialize;
use std::path::PathBuf;

/// Abstraction over OS process queries for testability.
pub trait ProcessTable: Send + Sync {
    fn is_alive(&self, pid: u32) -> bool;
}

/// Real process table backed by sysinfo.
pub struct RealProcessTable;

impl ProcessTable for RealProcessTable {
    fn is_alive(&self, pid: u32) -> bool {
        use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
        let mut sys = System::new();
        sys.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[Pid::from_u32(pid)]),
            true,
            ProcessRefreshKind::everything(),
        );
        sys.process(Pid::from_u32(pid)).is_some()
    }
}

/// Abstraction over session file reads for testability.
pub trait SessionFiles: Send + Sync {
    fn read_session_id(&self, claude_pid: u32) -> Option<String>;
}

/// Real session file reader from `~/.claude/sessions/<pid>.json`.
pub struct RealSessionFiles {
    sessions_dir: PathBuf,
}

impl Default for RealSessionFiles {
    fn default() -> Self {
        Self::new()
    }
}

impl RealSessionFiles {
    pub fn new() -> Self {
        let home = dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Self {
            sessions_dir: home.join(".claude").join("sessions"),
        }
    }
}

#[derive(Deserialize)]
struct SessionFileContent {
    #[serde(rename = "sessionId")]
    session_id: String,
}

impl SessionFiles for RealSessionFiles {
    fn read_session_id(&self, claude_pid: u32) -> Option<String> {
        let path = self.sessions_dir.join(format!("{}.json", claude_pid));
        let content = std::fs::read_to_string(&path).ok()?;
        let parsed: SessionFileContent = serde_json::from_str(&content).ok()?;
        Some(parsed.session_id)
    }
}

/// Check result for a single entry.
#[derive(Debug, PartialEq, Eq)]
pub enum LivenessResult {
    Alive,
    Dead,
    Mismatched { reason: String },
}

/// Check liveness of a single entry.
pub fn check_liveness(
    entry: &Entry,
    proc_table: &dyn ProcessTable,
    session_files: &dyn SessionFiles,
) -> LivenessResult {
    if !proc_table.is_alive(entry.claude_pid) {
        return LivenessResult::Dead;
    }

    match session_files.read_session_id(entry.claude_pid) {
        None => LivenessResult::Mismatched {
            reason: "session_file_missing".to_string(),
        },
        Some(file_session_id) if file_session_id != entry.session_id => {
            LivenessResult::Mismatched {
                reason: "mismatch".to_string(),
            }
        }
        Some(_) => LivenessResult::Alive,
    }
}

/// Run a full reaper sweep over all entries. Returns the stale ops to write.
pub fn sweep(
    entries: &[Entry],
    proc_table: &dyn ProcessTable,
    session_files: &dyn SessionFiles,
) -> Vec<Op> {
    let now = Utc::now();
    let mut stale_ops = Vec::new();

    for entry in entries {
        if entry.state != EntryState::Live {
            continue;
        }

        let result = check_liveness(entry, proc_table, session_files);
        let reason = match result {
            LivenessResult::Alive => continue,
            LivenessResult::Dead => "pid_dead".to_string(),
            LivenessResult::Mismatched { reason } => reason,
        };

        tracing::info!(
            session_id = %entry.session_id,
            claude_pid = entry.claude_pid,
            reason = %reason,
            "promoting entry to stale"
        );

        stale_ops.push(Op::Stale {
            ts: now,
            session_id: entry.session_id.clone(),
            reason,
        });
    }

    stale_ops
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::NotificationType;
    use std::collections::HashMap;

    struct MockProcessTable {
        alive_pids: Vec<u32>,
    }

    impl ProcessTable for MockProcessTable {
        fn is_alive(&self, pid: u32) -> bool {
            self.alive_pids.contains(&pid)
        }
    }

    struct MockSessionFiles {
        sessions: HashMap<u32, String>,
    }

    impl SessionFiles for MockSessionFiles {
        fn read_session_id(&self, claude_pid: u32) -> Option<String> {
            self.sessions.get(&claude_pid).cloned()
        }
    }

    fn make_entry(session_id: &str, claude_pid: u32) -> Entry {
        Entry {
            session_id: session_id.to_string(),
            ts: "2026-04-16T10:00:00Z".parse().unwrap(),
            cwd: PathBuf::from("/tmp"),
            claude_pid,
            terminal_pid: Some(5000),
            transcript_path: PathBuf::from("/tmp/t.jsonl"),
            notification_type: NotificationType::PermissionPrompt,
            message: "test".to_string(),
            state: EntryState::Live,
            stale_since: None,
        }
    }

    #[test]
    fn test_alive_process_with_matching_session() {
        let entry = make_entry("session-abc", 1000);
        let proc_table = MockProcessTable { alive_pids: vec![1000] };
        let session_files = MockSessionFiles {
            sessions: HashMap::from([(1000, "session-abc".to_string())]),
        };
        assert_eq!(check_liveness(&entry, &proc_table, &session_files), LivenessResult::Alive);
    }

    #[test]
    fn test_dead_process() {
        let entry = make_entry("session-abc", 1000);
        let proc_table = MockProcessTable { alive_pids: vec![] };
        let session_files = MockSessionFiles { sessions: HashMap::new() };
        assert_eq!(check_liveness(&entry, &proc_table, &session_files), LivenessResult::Dead);
    }

    #[test]
    fn test_pid_recycled_session_file_missing() {
        let entry = make_entry("session-abc", 1000);
        let proc_table = MockProcessTable { alive_pids: vec![1000] };
        let session_files = MockSessionFiles { sessions: HashMap::new() };
        assert_eq!(
            check_liveness(&entry, &proc_table, &session_files),
            LivenessResult::Mismatched { reason: "session_file_missing".to_string() }
        );
    }

    #[test]
    fn test_pid_recycled_session_mismatch() {
        let entry = make_entry("session-abc", 1000);
        let proc_table = MockProcessTable { alive_pids: vec![1000] };
        let session_files = MockSessionFiles {
            sessions: HashMap::from([(1000, "different-session".to_string())]),
        };
        assert_eq!(
            check_liveness(&entry, &proc_table, &session_files),
            LivenessResult::Mismatched { reason: "mismatch".to_string() }
        );
    }

    #[test]
    fn test_sweep_generates_stale_ops_for_dead_entries() {
        let entries = vec![
            make_entry("alive", 1000),
            make_entry("dead", 2000),
            make_entry("recycled", 3000),
        ];
        let proc_table = MockProcessTable { alive_pids: vec![1000, 3000] };
        let session_files = MockSessionFiles {
            sessions: HashMap::from([
                (1000, "alive".to_string()),
                (3000, "wrong-session".to_string()),
            ]),
        };

        let ops = sweep(&entries, &proc_table, &session_files);
        assert_eq!(ops.len(), 2);

        let session_ids: Vec<&str> = ops.iter().map(|o| o.session_id()).collect();
        assert!(session_ids.contains(&"dead"));
        assert!(session_ids.contains(&"recycled"));
    }

    #[test]
    fn test_sweep_skips_already_stale_entries() {
        let mut entry = make_entry("already-stale", 9999);
        entry.state = EntryState::Stale;
        entry.stale_since = Some(Utc::now());

        let proc_table = MockProcessTable { alive_pids: vec![] };
        let session_files = MockSessionFiles { sessions: HashMap::new() };

        let ops = sweep(&[entry], &proc_table, &session_files);
        assert!(ops.is_empty());
    }
}
