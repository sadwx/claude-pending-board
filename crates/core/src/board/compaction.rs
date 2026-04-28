use crate::types::{EntryState, Op};
use chrono::{Duration, Utc};
use std::path::Path;

/// Thresholds that trigger compaction.
const MAX_FILE_SIZE: u64 = 5 * 1024 * 1024; // 5 MB
const MAX_LINE_COUNT: usize = 10_000;

/// Check if compaction is needed based on file size and line count.
pub fn needs_compaction(file_path: &Path) -> std::io::Result<bool> {
    match std::fs::metadata(file_path) {
        Ok(meta) => {
            if meta.len() > MAX_FILE_SIZE {
                return Ok(true);
            }
            let content = std::fs::read_to_string(file_path)?;
            let line_count = content.lines().filter(|l| !l.trim().is_empty()).count();
            Ok(line_count > MAX_LINE_COUNT)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

#[derive(Debug)]
pub struct CompactionResult {
    pub entries_before: usize,
    pub entries_after: usize,
    pub lines_before: usize,
    pub lines_after: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum CompactionError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to serialize entry: {0}")]
    Serialize(#[from] serde_json::Error),
}

/// Compact the board file: read all ops, replay into a store, write back only
/// the current entries as `add` ops (dropping cleared and expired-stale entries).
/// Uses atomic write-to-tmp + rename.
pub fn compact(
    file_path: &Path,
    stale_expiry: Duration,
) -> Result<CompactionResult, CompactionError> {
    let content = match std::fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(CompactionError::Io(e)),
    };

    let lines_before = content.lines().filter(|l| !l.trim().is_empty()).count();
    let (ops, _skipped) = crate::board::parser::parse_lines(&content);

    // Replay ops into a store
    let mut store = crate::board::store::StateStore::new();
    let entries_before = ops.len();
    store.apply_all(ops);

    // Drop expired stale entries
    let now = Utc::now();
    store.remove_where(|entry| {
        if entry.state == EntryState::Stale {
            if let Some(stale_since) = entry.stale_since {
                return now.signed_duration_since(stale_since) > stale_expiry;
            }
        }
        false
    });

    // Write surviving entries as add ops (plus stale ops for stale entries)
    let snapshot = store.snapshot();
    let entries_after = snapshot.len();

    let mut lines = Vec::new();
    for entry in &snapshot {
        let add_op = Op::Add {
            ts: entry.ts,
            session_id: entry.session_id.clone(),
            cwd: entry.cwd.clone(),
            claude_pid: entry.claude_pid,
            terminal_pid: entry.terminal_pid,
            transcript_path: entry.transcript_path.clone(),
            notification_type: entry.notification_type,
            message: entry.message.clone(),
            wsl_distro: entry.wsl_distro.clone(),
            wezterm_pane_id: entry.wezterm_pane_id.clone(),
        };
        lines.push(serde_json::to_string(&add_op)?);

        if entry.state == EntryState::Stale {
            if let Some(stale_since) = entry.stale_since {
                let stale_op = Op::Stale {
                    ts: stale_since,
                    session_id: entry.session_id.clone(),
                    reason: "compaction".to_string(),
                };
                lines.push(serde_json::to_string(&stale_op)?);
            }
        }
    }

    let lines_after = lines.len();

    // Atomic write: write to .tmp, then rename
    let tmp_path = file_path.with_extension("jsonl.tmp");
    let output = if lines.is_empty() {
        String::new()
    } else {
        lines.join("\n") + "\n"
    };
    std::fs::write(&tmp_path, &output)?;
    std::fs::rename(&tmp_path, file_path)?;

    tracing::info!(
        lines_before,
        lines_after,
        entries_before,
        entries_after,
        "board compaction complete"
    );

    Ok(CompactionResult {
        entries_before,
        entries_after,
        lines_before,
        lines_after,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn add_line(session_id: &str, ts: &str) -> String {
        format!(
            r#"{{"op":"add","ts":"{}","session_id":"{}","cwd":"/tmp","claude_pid":1000,"terminal_pid":null,"transcript_path":"/tmp/t","notification_type":"permission_prompt","message":"m"}}"#,
            ts, session_id
        )
    }

    fn clear_line(session_id: &str, ts: &str) -> String {
        format!(
            r#"{{"op":"clear","ts":"{}","session_id":"{}","reason":"user_replied"}}"#,
            ts, session_id
        )
    }

    fn stale_line(session_id: &str, ts: &str) -> String {
        format!(
            r#"{{"op":"stale","ts":"{}","session_id":"{}","reason":"pid_dead"}}"#,
            ts, session_id
        )
    }

    #[test]
    fn test_compact_removes_cleared_entries() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("board.jsonl");
        let content = [
            add_line("s1", "2026-04-16T10:00:00Z"),
            add_line("s2", "2026-04-16T10:01:00Z"),
            clear_line("s1", "2026-04-16T10:02:00Z"),
        ]
        .join("\n")
            + "\n";
        fs::write(&path, &content).unwrap();

        let result = compact(&path, Duration::hours(24)).unwrap();
        assert_eq!(result.lines_before, 3);
        assert_eq!(result.entries_after, 1);

        let new_content = fs::read_to_string(&path).unwrap();
        assert!(new_content.contains("s2"));
        assert!(!new_content.contains("s1"));
    }

    #[test]
    fn test_compact_drops_expired_stale() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("board.jsonl");
        let old_ts = (Utc::now() - Duration::hours(25)).to_rfc3339();
        let stale_ts = (Utc::now() - Duration::hours(24) - Duration::minutes(30)).to_rfc3339();
        let content = [
            add_line("old-stale", &old_ts),
            stale_line("old-stale", &stale_ts),
            add_line("fresh", "2026-04-16T10:00:00Z"),
        ]
        .join("\n")
            + "\n";
        fs::write(&path, &content).unwrap();

        let result = compact(&path, Duration::hours(24)).unwrap();
        assert_eq!(result.entries_after, 1);

        let new_content = fs::read_to_string(&path).unwrap();
        assert!(new_content.contains("fresh"));
        assert!(!new_content.contains("old-stale"));
    }

    #[test]
    fn test_compact_keeps_recent_stale() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("board.jsonl");
        let recent_ts = (Utc::now() - Duration::hours(1)).to_rfc3339();
        let stale_ts = Utc::now().to_rfc3339();
        let content = [
            add_line("recent-stale", &recent_ts),
            stale_line("recent-stale", &stale_ts),
        ]
        .join("\n")
            + "\n";
        fs::write(&path, &content).unwrap();

        let result = compact(&path, Duration::hours(24)).unwrap();
        assert_eq!(result.entries_after, 1);
    }

    #[test]
    fn test_compact_roundtrip_preserves_data() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("board.jsonl");
        let content = [
            add_line("s1", "2026-04-16T10:00:00Z"),
            add_line("s2", "2026-04-16T10:01:00Z"),
        ]
        .join("\n")
            + "\n";
        fs::write(&path, &content).unwrap();

        compact(&path, Duration::hours(24)).unwrap();

        let new_content = fs::read_to_string(&path).unwrap();
        let (ops, skipped) = crate::board::parser::parse_lines(&new_content);
        assert_eq!(ops.len(), 2);
        assert_eq!(skipped, 0);
    }

    #[test]
    fn test_compact_handles_empty_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("board.jsonl");
        fs::write(&path, "").unwrap();

        let result = compact(&path, Duration::hours(24)).unwrap();
        assert_eq!(result.entries_after, 0);
        assert_eq!(result.lines_after, 0);
    }

    #[test]
    fn test_compact_skips_malformed_lines() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("board.jsonl");
        let content = [
            add_line("s1", "2026-04-16T10:00:00Z"),
            "garbage line".to_string(),
            add_line("s2", "2026-04-16T10:01:00Z"),
        ]
        .join("\n")
            + "\n";
        fs::write(&path, &content).unwrap();

        let result = compact(&path, Duration::hours(24)).unwrap();
        assert_eq!(result.entries_after, 2);
    }

    #[test]
    fn test_needs_compaction_small_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("board.jsonl");
        fs::write(&path, "small content\n").unwrap();
        assert!(!needs_compaction(&path).unwrap());
    }

    #[test]
    fn test_needs_compaction_missing_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.jsonl");
        assert!(!needs_compaction(&path).unwrap());
    }
}
