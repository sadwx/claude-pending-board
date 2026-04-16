use crate::types::Op;
use notify::{RecommendedWatcher, RecursiveMode, Watcher, Event, EventKind};
use std::io::{BufRead, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

#[derive(Debug, thiserror::Error)]
pub enum WatcherError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("notify error: {0}")]
    Notify(#[from] notify::Error),
}

/// Watches `board.jsonl` for new appended lines and sends parsed Ops.
pub struct BoardWatcher {
    board_path: PathBuf,
    _watcher: RecommendedWatcher,
}

impl BoardWatcher {
    /// Start watching the board file. New ops are sent to `op_tx`.
    /// If the file doesn't exist yet, the watcher watches the parent directory
    /// and starts reading once the file is created.
    pub fn start(
        board_path: PathBuf,
        op_tx: mpsc::UnboundedSender<Vec<Op>>,
    ) -> Result<Self, WatcherError> {
        // Ensure parent directory exists
        if let Some(parent) = board_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Read existing content and establish cursor
        let cursor = Arc::new(Mutex::new(0u64));

        // Read any existing content
        if board_path.exists() {
            let content = std::fs::read_to_string(&board_path)?;
            if !content.is_empty() {
                let (ops, _) = crate::board::parser::parse_lines(&content);
                if !ops.is_empty() {
                    let _ = op_tx.send(ops);
                }
                *cursor.lock().unwrap() = content.len() as u64;
            }
        }

        let path_clone = board_path.clone();
        let cursor_clone = cursor.clone();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            let event = match res {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!(error = %e, "board watcher error");
                    return;
                }
            };

            match event.kind {
                EventKind::Modify(_) | EventKind::Create(_) => {
                    if let Err(e) = Self::read_new_lines(&path_clone, &cursor_clone, &op_tx) {
                        tracing::warn!(error = %e, "failed to read new board lines");
                    }
                }
                EventKind::Remove(_) => {
                    tracing::warn!("board file deleted — clearing cursor");
                    *cursor_clone.lock().unwrap() = 0;
                }
                _ => {}
            }
        })?;

        // Watch the parent directory so we catch file creation
        let watch_path = board_path.parent().unwrap_or(Path::new("."));
        watcher.watch(watch_path, RecursiveMode::NonRecursive)?;

        Ok(Self {
            board_path,
            _watcher: watcher,
        })
    }

    fn read_new_lines(
        path: &Path,
        cursor: &Arc<Mutex<u64>>,
        op_tx: &mpsc::UnboundedSender<Vec<Op>>,
    ) -> std::io::Result<()> {
        let mut file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };

        let mut pos = cursor.lock().unwrap();
        let metadata = file.metadata()?;

        // If file is smaller than cursor, it was truncated/replaced (compaction)
        if metadata.len() < *pos {
            *pos = 0;
        }

        if metadata.len() == *pos {
            return Ok(()); // no new data
        }

        file.seek(SeekFrom::Start(*pos))?;
        let reader = std::io::BufReader::new(&file);
        let mut new_text = String::new();

        for line in reader.lines() {
            let line = line?;
            new_text.push_str(&line);
            new_text.push('\n');
        }

        *pos = metadata.len();
        drop(pos);

        if !new_text.is_empty() {
            let (ops, skipped) = crate::board::parser::parse_lines(&new_text);
            if skipped > 0 {
                tracing::warn!(skipped, "skipped malformed lines during incremental read");
            }
            if !ops.is_empty() {
                let _ = op_tx.send(ops);
            }
        }

        Ok(())
    }

    pub fn board_path(&self) -> &Path {
        &self.board_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    use std::time::Duration as StdDuration;

    #[tokio::test]
    async fn test_watcher_reads_existing_content() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("board.jsonl");
        let line = r#"{"op":"add","ts":"2026-04-16T10:00:00Z","session_id":"s1","cwd":"/tmp","claude_pid":1,"terminal_pid":null,"transcript_path":"/tmp/t","notification_type":"permission_prompt","message":"m"}"#;
        fs::write(&path, format!("{}\n", line)).unwrap();

        let (tx, mut rx) = mpsc::unbounded_channel();
        let _watcher = BoardWatcher::start(path, tx).unwrap();

        let ops = tokio::time::timeout(StdDuration::from_secs(1), rx.recv())
            .await
            .expect("timeout waiting for initial ops")
            .expect("channel closed");

        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].session_id(), "s1");
    }

    #[tokio::test]
    async fn test_watcher_detects_append() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("board.jsonl");
        fs::write(&path, "").unwrap();

        let (tx, mut rx) = mpsc::unbounded_channel();
        let _watcher = BoardWatcher::start(path.clone(), tx).unwrap();

        // Give watcher time to start
        tokio::time::sleep(StdDuration::from_millis(500)).await;

        // Append a line
        let line = r#"{"op":"add","ts":"2026-04-16T10:00:00Z","session_id":"s2","cwd":"/tmp","claude_pid":2,"terminal_pid":null,"transcript_path":"/tmp/t","notification_type":"idle_prompt","message":"m"}"#;
        use std::io::Write;
        let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(file, "{}", line).unwrap();
        drop(file); // ensure flush/close before watcher fires

        let ops = tokio::time::timeout(StdDuration::from_secs(10), rx.recv())
            .await
            .expect("timeout waiting for appended ops")
            .expect("channel closed");

        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].session_id(), "s2");
    }

    #[tokio::test]
    async fn test_watcher_handles_missing_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("pending").join("board.jsonl");

        let (tx, _rx) = mpsc::unbounded_channel();
        let result = BoardWatcher::start(path, tx);
        assert!(result.is_ok());
    }
}
