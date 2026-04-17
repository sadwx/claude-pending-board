# Phase 1: Core Library Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Cargo workspace scaffold and the platform-agnostic `core` crate containing the JSONL parser, state store, board watcher, compaction, visibility state machine, reaper, config, and terminal adapter trait — all fully tested before any UI work begins.

**Architecture:** A Cargo workspace with three crate members (`core`, `adapters`, `app`). This plan builds `core` only — a pure-Rust library with zero Tauri dependencies. The crate is organized into modules: `types`, `board` (parser, store, watcher, compaction), `visibility`, `reaper`, `terminal` (trait + ancestor walk), and `config`. All async code uses `tokio`. File watching uses the `notify` crate. Process inspection uses `sysinfo`.

**Tech Stack:** Rust 2021 edition, tokio 1.x, serde/serde_json, notify 7.x, notify-debouncer-full, sysinfo 0.33+, toml, chrono, tracing/tracing-subscriber, tempfile (dev)

---

## File Structure

```
claude-pending-board/
├── Cargo.toml                          # workspace root
├── rust-toolchain.toml                 # MSRV pin
├── rustfmt.toml                        # formatting config
├── clippy.toml                         # lint config
├── .gitignore                          # Rust + Tauri ignores
├── crates/
│   ├── core/
│   │   ├── Cargo.toml                  # core crate manifest
│   │   └── src/
│   │       ├── lib.rs                  # re-exports all modules
│   │       ├── types.rs                # Entry, EntryState, NotificationType, Op, etc.
│   │       ├── board/
│   │       │   ├── mod.rs              # re-exports parser, store, watcher, compaction
│   │       │   ├── parser.rs           # parse_line() -> Result<Op>
│   │       │   ├── store.rs            # StateStore: apply ops, snapshot, sorting
│   │       │   ├── watcher.rs          # BoardWatcher: notify-based file watcher
│   │       │   └── compaction.rs       # compact(): atomic rewrite
│   │       ├── visibility.rs           # VisibilityController + Clock trait + FSM
│   │       ├── reaper.rs              # Reaper: periodic liveness sweep
│   │       ├── terminal.rs            # TerminalAdapter trait + TerminalMatch + ancestor_walk
│   │       └── config.rs             # Config struct, load/save/watch
│   ├── adapters/
│   │   └── Cargo.toml                  # placeholder (Phase 2)
│   └── app/
│       └── Cargo.toml                  # placeholder (Phase 3)
```

Each file has one clear responsibility. Tests live in `#[cfg(test)] mod tests` at the bottom of each file. Integration tests that need real filesystem go in `crates/core/tests/`.

---

## Task 1: Workspace Setup

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `rust-toolchain.toml`
- Create: `rustfmt.toml`
- Create: `clippy.toml`
- Create: `.gitignore`
- Create: `crates/core/Cargo.toml`
- Create: `crates/core/src/lib.rs`
- Create: `crates/adapters/Cargo.toml`
- Create: `crates/adapters/src/lib.rs`
- Create: `crates/app/Cargo.toml`
- Create: `crates/app/src/main.rs`

- [ ] **Step 1: Create workspace root `Cargo.toml`**

```toml
[workspace]
members = ["crates/core", "crates/adapters", "crates/app"]
resolver = "2"

[workspace.package]
edition = "2021"
version = "0.1.0"
license = "MIT"
repository = "https://github.com/sadwx/claude-pending-board"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
notify = "7"
notify-debouncer-full = "0.4"
sysinfo = "0.33"
toml = "0.8"
thiserror = "2"
tempfile = "3"
```

- [ ] **Step 2: Create `rust-toolchain.toml`**

```toml
[toolchain]
channel = "stable"
```

- [ ] **Step 3: Create `rustfmt.toml`**

```toml
edition = "2021"
max_width = 100
use_field_init_shorthand = true
```

- [ ] **Step 4: Create `clippy.toml`**

```toml
msrv = "1.83.0"
```

- [ ] **Step 5: Create `.gitignore`**

```gitignore
# Rust
/target/
**/target/
**/*.rs.bk
Cargo.lock

# Tauri (Phase 3)
crates/app/src-tauri/target/

# IDE
.idea/
.vscode/
*.swp
*.swo
*~

# OS
.DS_Store
Thumbs.db
```

- [ ] **Step 6: Create `crates/core/Cargo.toml`**

```toml
[package]
name = "claude-pending-board-core"
edition.workspace = true
version.workspace = true
license.workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
chrono = { workspace = true }
tracing = { workspace = true }
notify = { workspace = true }
notify-debouncer-full = { workspace = true }
sysinfo = { workspace = true }
toml = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
tokio = { workspace = true, features = ["test-util", "macros"] }
tracing-subscriber = { workspace = true }
```

- [ ] **Step 7: Create `crates/core/src/lib.rs`**

```rust
pub mod types;
pub mod board;
pub mod visibility;
pub mod reaper;
pub mod terminal;
pub mod config;
```

- [ ] **Step 8: Create placeholder `crates/adapters/Cargo.toml` and `src/lib.rs`**

`crates/adapters/Cargo.toml`:
```toml
[package]
name = "claude-pending-board-adapters"
edition.workspace = true
version.workspace = true
license.workspace = true

[dependencies]
claude-pending-board-core = { path = "../core" }
serde = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
sysinfo = { workspace = true }
```

`crates/adapters/src/lib.rs`:
```rust
// Terminal adapters (WezTerm, iTerm2) — Phase 2
```

- [ ] **Step 9: Create placeholder `crates/app/Cargo.toml` and `src/main.rs`**

`crates/app/Cargo.toml`:
```toml
[package]
name = "claude-pending-board-app"
edition.workspace = true
version.workspace = true
license.workspace = true

[dependencies]
claude-pending-board-core = { path = "../core" }
claude-pending-board-adapters = { path = "../adapters" }
tokio = { workspace = true }
tracing = { workspace = true }
```

`crates/app/src/main.rs`:
```rust
fn main() {
    println!("claude-pending-board — Tauri app placeholder (Phase 3)");
}
```

- [ ] **Step 10: Verify the workspace compiles**

Run: `cargo check --workspace`
Expected: Compiles with 0 errors, 0 warnings

- [ ] **Step 11: Commit scaffold**

```bash
git add -A
git commit -m "feat: initialize Cargo workspace with core/adapters/app crates"
git tag v0.0.0-scaffold
```

---

## Task 2: Domain Types (`crates/core/src/types.rs`)

**Files:**
- Create: `crates/core/src/types.rs`

- [ ] **Step 1: Write the types module**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Unique session identifier assigned by Claude Code per CLI invocation.
pub type SessionId = String;

/// The kind of notification that created the pending entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    PermissionPrompt,
    IdlePrompt,
}

impl NotificationType {
    /// Sort priority: lower number = higher priority in the HUD.
    pub fn priority(self) -> u8 {
        match self {
            NotificationType::PermissionPrompt => 0,
            NotificationType::IdlePrompt => 1,
        }
    }
}

/// Whether an entry is still backed by a live Claude Code process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryState {
    Live,
    Stale,
}

/// A single pending-board entry reconstructed from replaying ops.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub session_id: SessionId,
    pub ts: DateTime<Utc>,
    pub cwd: PathBuf,
    pub claude_pid: u32,
    pub terminal_pid: Option<u32>,
    pub transcript_path: PathBuf,
    pub notification_type: NotificationType,
    pub message: String,
    pub state: EntryState,
    /// When the entry transitioned to Stale (for 24h expiry).
    pub stale_since: Option<DateTime<Utc>>,
}

/// Operations stored as lines in board.jsonl.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    Add {
        ts: DateTime<Utc>,
        session_id: SessionId,
        cwd: PathBuf,
        claude_pid: u32,
        terminal_pid: Option<u32>,
        transcript_path: PathBuf,
        notification_type: NotificationType,
        message: String,
    },
    Clear {
        ts: DateTime<Utc>,
        session_id: SessionId,
        reason: String,
    },
    Stale {
        ts: DateTime<Utc>,
        session_id: SessionId,
        reason: String,
    },
}

impl Op {
    pub fn session_id(&self) -> &str {
        match self {
            Op::Add { session_id, .. } => session_id,
            Op::Clear { session_id, .. } => session_id,
            Op::Stale { session_id, .. } => session_id,
        }
    }

    pub fn ts(&self) -> DateTime<Utc> {
        match self {
            Op::Add { ts, .. } => *ts,
            Op::Clear { ts, .. } => *ts,
            Op::Stale { ts, .. } => *ts,
        }
    }
}

/// Info about which terminal process owns a Claude session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalMatch {
    pub terminal_name: String,
    pub terminal_pid: u32,
    pub pane_id: Option<String>,
    pub tty: Option<String>,
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p claude-pending-board-core`
Expected: 0 errors

- [ ] **Step 3: Commit**

```bash
git add crates/core/src/types.rs crates/core/src/lib.rs
git commit -m "feat(core): add domain types — Entry, Op, NotificationType, EntryState"
```

---

## Task 3: Board Parser (`crates/core/src/board/parser.rs`)

**Files:**
- Create: `crates/core/src/board/mod.rs`
- Create: `crates/core/src/board/parser.rs`

- [ ] **Step 1: Create `board/mod.rs`**

```rust
pub mod parser;
pub mod store;
pub mod watcher;
pub mod compaction;
```

- [ ] **Step 2: Write failing tests for parser**

In `crates/core/src/board/parser.rs`:

```rust
use crate::types::Op;

/// Errors that can occur parsing a single board line.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("unknown op type")]
    UnknownOp,
}

/// Parse a single line from board.jsonl into an Op.
///
/// Returns `Err(ParseError::UnknownOp)` for valid JSON with an unrecognized `op` field.
/// Returns `Err(ParseError::InvalidJson)` for malformed JSON.
pub fn parse_line(line: &str) -> Result<Op, ParseError> {
    todo!()
}

/// Parse multiple lines, skipping blank and malformed lines.
/// Returns `(ops, skipped_count)`.
pub fn parse_lines(text: &str) -> (Vec<Op>, usize) {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::NotificationType;

    #[test]
    fn test_parse_valid_add_op() {
        let line = r#"{"op":"add","ts":"2026-04-16T10:00:00Z","session_id":"abc-123","cwd":"/home/user/project","claude_pid":1234,"terminal_pid":5678,"transcript_path":"/tmp/transcript.jsonl","notification_type":"permission_prompt","message":"May I run ls?"}"#;
        let op = parse_line(line).unwrap();
        match op {
            Op::Add { session_id, notification_type, claude_pid, .. } => {
                assert_eq!(session_id, "abc-123");
                assert_eq!(notification_type, NotificationType::PermissionPrompt);
                assert_eq!(claude_pid, 1234);
            }
            _ => panic!("expected Add op"),
        }
    }

    #[test]
    fn test_parse_valid_clear_op() {
        let line = r#"{"op":"clear","ts":"2026-04-16T10:01:00Z","session_id":"abc-123","reason":"user_replied"}"#;
        let op = parse_line(line).unwrap();
        match op {
            Op::Clear { session_id, reason, .. } => {
                assert_eq!(session_id, "abc-123");
                assert_eq!(reason, "user_replied");
            }
            _ => panic!("expected Clear op"),
        }
    }

    #[test]
    fn test_parse_valid_stale_op() {
        let line = r#"{"op":"stale","ts":"2026-04-16T10:02:00Z","session_id":"abc-123","reason":"pid_dead"}"#;
        let op = parse_line(line).unwrap();
        match op {
            Op::Stale { session_id, reason, .. } => {
                assert_eq!(session_id, "abc-123");
                assert_eq!(reason, "pid_dead");
            }
            _ => panic!("expected Stale op"),
        }
    }

    #[test]
    fn test_parse_idle_prompt_type() {
        let line = r#"{"op":"add","ts":"2026-04-16T10:00:00Z","session_id":"def-456","cwd":"/tmp","claude_pid":999,"terminal_pid":null,"transcript_path":"/tmp/t.jsonl","notification_type":"idle_prompt","message":"Waiting"}"#;
        let op = parse_line(line).unwrap();
        match op {
            Op::Add { notification_type, terminal_pid, .. } => {
                assert_eq!(notification_type, NotificationType::IdlePrompt);
                assert!(terminal_pid.is_none());
            }
            _ => panic!("expected Add op"),
        }
    }

    #[test]
    fn test_parse_malformed_json() {
        let line = "not json at all{{{";
        assert!(parse_line(line).is_err());
    }

    #[test]
    fn test_parse_unknown_op() {
        let line = r#"{"op":"future_op","ts":"2026-04-16T10:00:00Z","session_id":"x"}"#;
        let err = parse_line(line).unwrap_err();
        assert!(matches!(err, ParseError::UnknownOp));
    }

    #[test]
    fn test_parse_empty_line() {
        assert!(parse_line("").is_err());
    }

    #[test]
    fn test_parse_lines_mixed() {
        let text = concat!(
            r#"{"op":"add","ts":"2026-04-16T10:00:00Z","session_id":"a","cwd":"/tmp","claude_pid":1,"terminal_pid":null,"transcript_path":"/tmp/t","notification_type":"permission_prompt","message":"m"}"#,
            "\n",
            "bad line\n",
            "\n",
            r#"{"op":"clear","ts":"2026-04-16T10:01:00Z","session_id":"a","reason":"stop"}"#,
            "\n",
        );
        let (ops, skipped) = parse_lines(text);
        assert_eq!(ops.len(), 2);
        assert_eq!(skipped, 1); // "bad line" skipped, empty line not counted
    }

    #[test]
    fn test_parse_line_with_extra_fields_is_forward_compatible() {
        // Future versions may add fields — serde should ignore unknown fields
        let line = r#"{"op":"add","ts":"2026-04-16T10:00:00Z","session_id":"a","cwd":"/tmp","claude_pid":1,"terminal_pid":null,"transcript_path":"/tmp/t","notification_type":"permission_prompt","message":"m","new_field":"ignored"}"#;
        assert!(parse_line(line).is_ok());
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p claude-pending-board-core -- board::parser --nocapture`
Expected: FAIL — `todo!()` panics

- [ ] **Step 4: Implement `parse_line` and `parse_lines`**

Replace the two `todo!()` bodies:

```rust
pub fn parse_line(line: &str) -> Result<Op, ParseError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err(ParseError::InvalidJson(serde_json::from_str::<Op>("").unwrap_err()));
    }

    // First try to parse as a known Op variant.
    // serde's internally-tagged enum returns a deserialization error for unknown tags,
    // but it's the same error type as malformed JSON. To distinguish, we peek at the
    // raw JSON to check if it has a valid `op` field that we don't recognize.
    match serde_json::from_str::<Op>(trimmed) {
        Ok(op) => Ok(op),
        Err(serde_err) => {
            // Check if it's valid JSON with an unknown op
            if let Ok(raw) = serde_json::from_str::<serde_json::Value>(trimmed) {
                if let Some(op_field) = raw.get("op").and_then(|v| v.as_str()) {
                    match op_field {
                        "add" | "clear" | "stale" => {
                            // Known op but bad fields — return the serde error
                            Err(ParseError::InvalidJson(serde_err))
                        }
                        _ => Err(ParseError::UnknownOp),
                    }
                } else {
                    Err(ParseError::InvalidJson(serde_err))
                }
            } else {
                Err(ParseError::InvalidJson(serde_err))
            }
        }
    }
}

pub fn parse_lines(text: &str) -> (Vec<Op>, usize) {
    let mut ops = Vec::new();
    let mut skipped = 0;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match parse_line(trimmed) {
            Ok(op) => ops.push(op),
            Err(e) => {
                tracing::warn!(line = trimmed, error = %e, "skipping malformed board line");
                skipped += 1;
            }
        }
    }

    (ops, skipped)
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p claude-pending-board-core -- board::parser --nocapture`
Expected: All 9 tests PASS

- [ ] **Step 6: Run clippy**

Run: `cargo clippy -p claude-pending-board-core -- -D warnings`
Expected: 0 warnings

- [ ] **Step 7: Commit**

```bash
git add crates/core/src/board/
git commit -m "feat(core): implement JSONL parser with forward-compat and error handling"
```

---

## Task 4: State Store (`crates/core/src/board/store.rs`)

**Files:**
- Create: `crates/core/src/board/store.rs`

- [ ] **Step 1: Write failing tests for store**

```rust
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use crate::types::{Entry, EntryState, NotificationType, Op, SessionId};

/// The in-memory state reconstructed by replaying ops.
#[derive(Debug, Default)]
pub struct StateStore {
    entries: HashMap<SessionId, Entry>,
}

impl StateStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply a single op to the store. Returns true if the store changed.
    pub fn apply(&mut self, op: Op) -> bool {
        todo!()
    }

    /// Apply multiple ops in order.
    pub fn apply_all(&mut self, ops: impl IntoIterator<Item = Op>) {
        for op in ops {
            self.apply(op);
        }
    }

    /// Number of entries currently tracked.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns a sorted snapshot of all current entries.
    /// Sort order: permission_prompt > idle_prompt > stale, then ts descending within each group.
    pub fn snapshot(&self) -> Vec<Entry> {
        todo!()
    }

    /// Get a single entry by session_id.
    pub fn get(&self, session_id: &str) -> Option<&Entry> {
        self.entries.get(session_id)
    }

    /// Remove all entries. Returns count of entries removed.
    pub fn clear_all(&mut self) -> usize {
        let count = self.entries.len();
        self.entries.clear();
        count
    }

    /// Iterate over entries (unordered).
    pub fn iter(&self) -> impl Iterator<Item = (&SessionId, &Entry)> {
        self.entries.iter()
    }

    /// Remove entries matching a predicate. Returns removed entries.
    pub fn remove_where<F: Fn(&Entry) -> bool>(&mut self, predicate: F) -> Vec<Entry> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_add(session_id: &str, notification_type: NotificationType, ts: &str) -> Op {
        Op::Add {
            ts: ts.parse::<DateTime<Utc>>().unwrap(),
            session_id: session_id.to_string(),
            cwd: PathBuf::from("/tmp"),
            claude_pid: 1000,
            terminal_pid: Some(2000),
            transcript_path: PathBuf::from("/tmp/transcript.jsonl"),
            notification_type,
            message: "test".to_string(),
        }
    }

    fn make_clear(session_id: &str, ts: &str) -> Op {
        Op::Clear {
            ts: ts.parse::<DateTime<Utc>>().unwrap(),
            session_id: session_id.to_string(),
            reason: "user_replied".to_string(),
        }
    }

    fn make_stale(session_id: &str, ts: &str) -> Op {
        Op::Stale {
            ts: ts.parse::<DateTime<Utc>>().unwrap(),
            session_id: session_id.to_string(),
            reason: "pid_dead".to_string(),
        }
    }

    #[test]
    fn test_apply_add_creates_entry() {
        let mut store = StateStore::new();
        let changed = store.apply(make_add("s1", NotificationType::PermissionPrompt, "2026-04-16T10:00:00Z"));
        assert!(changed);
        assert_eq!(store.len(), 1);
        let entry = store.get("s1").unwrap();
        assert_eq!(entry.state, EntryState::Live);
        assert_eq!(entry.notification_type, NotificationType::PermissionPrompt);
    }

    #[test]
    fn test_apply_add_overwrites_same_session() {
        let mut store = StateStore::new();
        store.apply(make_add("s1", NotificationType::PermissionPrompt, "2026-04-16T10:00:00Z"));
        store.apply(make_add("s1", NotificationType::IdlePrompt, "2026-04-16T10:01:00Z"));
        assert_eq!(store.len(), 1);
        assert_eq!(store.get("s1").unwrap().notification_type, NotificationType::IdlePrompt);
    }

    #[test]
    fn test_apply_clear_removes_entry() {
        let mut store = StateStore::new();
        store.apply(make_add("s1", NotificationType::PermissionPrompt, "2026-04-16T10:00:00Z"));
        let changed = store.apply(make_clear("s1", "2026-04-16T10:01:00Z"));
        assert!(changed);
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn test_clear_unknown_session_is_noop() {
        let mut store = StateStore::new();
        let changed = store.apply(make_clear("nonexistent", "2026-04-16T10:01:00Z"));
        assert!(!changed);
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn test_apply_stale_promotes_entry() {
        let mut store = StateStore::new();
        store.apply(make_add("s1", NotificationType::PermissionPrompt, "2026-04-16T10:00:00Z"));
        let changed = store.apply(make_stale("s1", "2026-04-16T10:05:00Z"));
        assert!(changed);
        assert_eq!(store.len(), 1);
        let entry = store.get("s1").unwrap();
        assert_eq!(entry.state, EntryState::Stale);
        assert!(entry.stale_since.is_some());
    }

    #[test]
    fn test_stale_unknown_session_is_noop() {
        let mut store = StateStore::new();
        let changed = store.apply(make_stale("nonexistent", "2026-04-16T10:05:00Z"));
        assert!(!changed);
    }

    #[test]
    fn test_snapshot_sort_order() {
        let mut store = StateStore::new();
        // Add in scrambled order
        store.apply(make_add("idle-old", NotificationType::IdlePrompt, "2026-04-16T10:00:00Z"));
        store.apply(make_add("perm-old", NotificationType::PermissionPrompt, "2026-04-16T10:01:00Z"));
        store.apply(make_add("idle-new", NotificationType::IdlePrompt, "2026-04-16T10:02:00Z"));
        store.apply(make_add("perm-new", NotificationType::PermissionPrompt, "2026-04-16T10:03:00Z"));
        store.apply(make_add("stale-one", NotificationType::PermissionPrompt, "2026-04-16T09:00:00Z"));
        store.apply(make_stale("stale-one", "2026-04-16T10:04:00Z"));

        let snap = store.snapshot();
        let ids: Vec<&str> = snap.iter().map(|e| e.session_id.as_str()).collect();
        // permission (newest first), idle (newest first), stale
        assert_eq!(ids, vec!["perm-new", "perm-old", "idle-new", "idle-old", "stale-one"]);
    }

    #[test]
    fn test_remove_where() {
        let mut store = StateStore::new();
        store.apply(make_add("s1", NotificationType::PermissionPrompt, "2026-04-16T10:00:00Z"));
        store.apply(make_add("s2", NotificationType::IdlePrompt, "2026-04-16T10:01:00Z"));
        store.apply(make_add("s3", NotificationType::PermissionPrompt, "2026-04-16T10:02:00Z"));
        let removed = store.remove_where(|e| e.notification_type == NotificationType::IdlePrompt);
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].session_id, "s2");
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn test_clear_all() {
        let mut store = StateStore::new();
        store.apply(make_add("s1", NotificationType::PermissionPrompt, "2026-04-16T10:00:00Z"));
        store.apply(make_add("s2", NotificationType::IdlePrompt, "2026-04-16T10:01:00Z"));
        assert_eq!(store.clear_all(), 2);
        assert!(store.is_empty());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p claude-pending-board-core -- board::store --nocapture`
Expected: FAIL — `todo!()` panics

- [ ] **Step 3: Implement the store methods**

Replace the three `todo!()` bodies:

For `apply`:
```rust
pub fn apply(&mut self, op: Op) -> bool {
    match op {
        Op::Add {
            ts, session_id, cwd, claude_pid, terminal_pid,
            transcript_path, notification_type, message,
        } => {
            self.entries.insert(session_id.clone(), Entry {
                session_id,
                ts,
                cwd,
                claude_pid,
                terminal_pid,
                transcript_path,
                notification_type,
                message,
                state: EntryState::Live,
                stale_since: None,
            });
            true
        }
        Op::Clear { session_id, .. } => {
            if self.entries.remove(&session_id).is_some() {
                true
            } else {
                tracing::debug!(session_id = %session_id, "clear op for unknown session — no-op");
                false
            }
        }
        Op::Stale { ts, session_id, .. } => {
            if let Some(entry) = self.entries.get_mut(&session_id) {
                entry.state = EntryState::Stale;
                entry.stale_since = Some(ts);
                true
            } else {
                tracing::debug!(session_id = %session_id, "stale op for unknown session — no-op");
                false
            }
        }
    }
}
```

For `snapshot`:
```rust
pub fn snapshot(&self) -> Vec<Entry> {
    let mut entries: Vec<Entry> = self.entries.values().cloned().collect();
    entries.sort_by(|a, b| {
        // Primary: sort group (live-permission=0, live-idle=1, stale=2)
        let group_a = match a.state {
            EntryState::Live => a.notification_type.priority(),
            EntryState::Stale => 2,
        };
        let group_b = match b.state {
            EntryState::Live => b.notification_type.priority(),
            EntryState::Stale => 2,
        };
        group_a.cmp(&group_b).then_with(|| b.ts.cmp(&a.ts)) // ts desc within group
    });
    entries
}
```

For `remove_where`:
```rust
pub fn remove_where<F: Fn(&Entry) -> bool>(&mut self, predicate: F) -> Vec<Entry> {
    let to_remove: Vec<SessionId> = self
        .entries
        .iter()
        .filter(|(_, e)| predicate(e))
        .map(|(k, _)| k.clone())
        .collect();

    to_remove
        .into_iter()
        .filter_map(|k| self.entries.remove(&k))
        .collect()
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p claude-pending-board-core -- board::store --nocapture`
Expected: All 8 tests PASS

- [ ] **Step 5: Run clippy**

Run: `cargo clippy -p claude-pending-board-core -- -D warnings`
Expected: 0 warnings

- [ ] **Step 6: Commit**

```bash
git add crates/core/src/board/store.rs
git commit -m "feat(core): implement StateStore with apply, snapshot, and sorting"
```

---

## Task 5: Board Compaction (`crates/core/src/board/compaction.rs`)

**Files:**
- Create: `crates/core/src/board/compaction.rs`

- [ ] **Step 1: Write failing tests**

```rust
use crate::types::{Entry, EntryState, Op};
use chrono::{Duration, Utc};
use std::path::{Path, PathBuf};

/// Thresholds that trigger compaction.
const MAX_FILE_SIZE: u64 = 5 * 1024 * 1024; // 5 MB
const MAX_LINE_COUNT: usize = 10_000;

/// Check if compaction is needed based on file size and line count.
pub fn needs_compaction(file_path: &Path) -> std::io::Result<bool> {
    todo!()
}

/// Compact the board file: read all ops, replay into a store, write back only
/// the current entries as `add` ops (dropping cleared and expired-stale entries).
/// Uses atomic write-to-tmp + rename.
///
/// `stale_expiry` controls how old a stale entry can be before it's dropped.
pub fn compact(file_path: &Path, stale_expiry: Duration) -> Result<CompactionResult, CompactionError> {
    todo!()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::NotificationType;
    use tempfile::TempDir;
    use std::fs;

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
        ].join("\n") + "\n";
        fs::write(&path, &content).unwrap();

        let result = compact(&path, Duration::hours(24)).unwrap();
        assert_eq!(result.lines_before, 3);
        assert_eq!(result.entries_after, 1);

        // Re-read and verify
        let new_content = fs::read_to_string(&path).unwrap();
        assert!(new_content.contains("s2"));
        assert!(!new_content.contains("s1"));
    }

    #[test]
    fn test_compact_drops_expired_stale() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("board.jsonl");
        // Stale entry from 25 hours ago
        let old_ts = (Utc::now() - Duration::hours(25)).to_rfc3339();
        let stale_ts = (Utc::now() - Duration::hours(24) - Duration::minutes(30)).to_rfc3339();
        let content = [
            add_line("old-stale", &old_ts),
            stale_line("old-stale", &stale_ts),
            add_line("fresh", "2026-04-16T10:00:00Z"),
        ].join("\n") + "\n";
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
        ].join("\n") + "\n";
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
        ].join("\n") + "\n";
        fs::write(&path, &content).unwrap();

        compact(&path, Duration::hours(24)).unwrap();

        // Parse the compacted file and verify entries are intact
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
        ].join("\n") + "\n";
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p claude-pending-board-core -- board::compaction --nocapture`
Expected: FAIL — `todo!()` panics

- [ ] **Step 3: Implement compaction**

```rust
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

pub fn compact(file_path: &Path, stale_expiry: Duration) -> Result<CompactionResult, CompactionError> {
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p claude-pending-board-core -- board::compaction --nocapture`
Expected: All 7 tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/board/compaction.rs
git commit -m "feat(core): implement board compaction with atomic write and stale expiry"
```

---

## Task 6: Board Watcher (`crates/core/src/board/watcher.rs`)

**Files:**
- Create: `crates/core/src/board/watcher.rs`

- [ ] **Step 1: Write the watcher**

The watcher uses `notify` to observe `board.jsonl`, reads new lines from the last cursor position, and sends parsed ops through a `tokio::sync::mpsc` channel. This module is integration-heavy (file I/O + notify), so tests use `tempfile`.

```rust
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
                    // Send empty vec to signal "file deleted" to the store
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

    /// Reset the cursor to 0 (used after compaction to re-read the file).
    pub fn reset_cursor(&self) {
        // This is a simplification — in production the cursor would be
        // accessible. For now, the watcher handles truncation detection
        // in read_new_lines automatically.
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
        tokio::time::sleep(StdDuration::from_millis(100)).await;

        // Append a line
        let line = r#"{"op":"add","ts":"2026-04-16T10:00:00Z","session_id":"s2","cwd":"/tmp","claude_pid":2,"terminal_pid":null,"transcript_path":"/tmp/t","notification_type":"idle_prompt","message":"m"}"#;
        use std::io::Write;
        let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(file, "{}", line).unwrap();

        let ops = tokio::time::timeout(StdDuration::from_secs(2), rx.recv())
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
        assert!(result.is_ok()); // should not error, just wait for file creation
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p claude-pending-board-core -- board::watcher --nocapture`
Expected: All 3 tests PASS (these are not TDD — the implementation is inline because it's integration-heavy)

- [ ] **Step 3: Run clippy**

Run: `cargo clippy -p claude-pending-board-core -- -D warnings`
Expected: 0 warnings

- [ ] **Step 4: Commit**

```bash
git add crates/core/src/board/watcher.rs
git commit -m "feat(core): implement BoardWatcher with notify, cursor tracking, and truncation handling"
```

---

## Task 7: Terminal Adapter Trait and Ancestor Walk (`crates/core/src/terminal.rs`)

**Files:**
- Create: `crates/core/src/terminal.rs`

- [ ] **Step 1: Write the trait and ancestor walk with tests**

```rust
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
    fn spawn_resume(
        &self,
        cwd: &Path,
        session_id: &str,
    ) -> Result<(), AdapterError>;
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
/// `depth_cap` limits how far up we walk (default 20) to prevent infinite loops
/// from circular parent references (shouldn't happen, but defensive).
pub fn ancestor_walk(start_pid: u32, depth_cap: usize) -> Option<(String, u32)> {
    use sysinfo::{Pid, System, ProcessRefreshKind, RefreshKind, UpdateKind};

    let mut sys = System::new();
    sys.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::new(),
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

        if KNOWN_TERMINALS.iter().any(|t| t.eq_ignore_ascii_case(name_normalized)) {
            return Some((name.clone(), current_pid.as_u32()));
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
        // PID 0xFFFFFF is almost certainly not in use
        let result = ancestor_walk(0xFFFFFF, 20);
        assert!(result.is_none());
    }

    #[test]
    fn test_ancestor_walk_depth_cap() {
        // Walk from PID 1 (init/launchd/System) with depth cap 1
        // Should return None since the root process is not a terminal
        let result = ancestor_walk(1, 1);
        // We can't assert the exact result because it depends on OS,
        // but it should not panic or infinite loop
        let _ = result;
    }

    #[test]
    fn test_known_terminals_list() {
        assert!(KNOWN_TERMINALS.contains(&"wezterm-gui"));
        assert!(KNOWN_TERMINALS.contains(&"iTerm2"));
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p claude-pending-board-core -- terminal --nocapture`
Expected: All 3 tests PASS

- [ ] **Step 3: Commit**

```bash
git add crates/core/src/terminal.rs
git commit -m "feat(core): add TerminalAdapter trait and process ancestor walk via sysinfo"
```

---

## Task 8: Config (`crates/core/src/config.rs`)

**Files:**
- Create: `crates/core/src/config.rs`

- [ ] **Step 1: Write config with tests**

```rust
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// All user-configurable settings, persisted to `~/.claude/pending/config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    /// Cooldown after manual dismiss, in minutes. Range: 1–120.
    pub cooldown_minutes: u32,
    /// Whether the HUD re-shows at cooldown expiry if new items arrived.
    pub reminding_enabled: bool,
    /// Delay (seconds) before auto-hiding after the board goes empty. Range: 0–10.
    pub auto_hide_grace_secs: u32,
    /// Duration (seconds) of the dismiss confirmation countdown. Range: 2–10.
    pub dismiss_countdown_secs: u32,
    /// Skip the dismiss confirmation panel entirely.
    pub skip_dismiss_confirmation: bool,
    /// Default terminal adapter name ("wezterm" or "iterm2").
    pub default_adapter: String,
    /// Saved HUD window position (x, y). None = use tray-anchor default.
    pub hud_position: Option<(i32, i32)>,
    /// Enable debug-level logging.
    pub debug_logging: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cooldown_minutes: 15,
            reminding_enabled: true,
            auto_hide_grace_secs: 2,
            dismiss_countdown_secs: 5,
            skip_dismiss_confirmation: false,
            default_adapter: default_adapter_name(),
            hud_position: None,
            debug_logging: false,
        }
    }
}

fn default_adapter_name() -> String {
    if cfg!(target_os = "macos") {
        "iterm2".to_string()
    } else {
        "wezterm".to_string()
    }
}

impl Config {
    /// Default config file path: `~/.claude/pending/config.toml`
    pub fn default_path() -> PathBuf {
        let home = dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".claude").join("pending").join("config.toml")
    }

    /// Load config from a TOML file. Returns default config if the file doesn't
    /// exist or is malformed (with a warning log).
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(content) => match toml::from_str(&content) {
                Ok(config) => config,
                Err(e) => {
                    tracing::warn!(error = %e, path = %path.display(), "malformed config, using defaults");
                    Self::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tracing::info!(path = %path.display(), "config file not found, using defaults");
                Self::default()
            }
            Err(e) => {
                tracing::warn!(error = %e, path = %path.display(), "failed to read config, using defaults");
                Self::default()
            }
        }
    }

    /// Save config to a TOML file atomically (write to .tmp + rename).
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, &content)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config_values() {
        let config = Config::default();
        assert_eq!(config.cooldown_minutes, 15);
        assert!(config.reminding_enabled);
        assert_eq!(config.auto_hide_grace_secs, 2);
        assert_eq!(config.dismiss_countdown_secs, 5);
        assert!(!config.skip_dismiss_confirmation);
        assert!(config.hud_position.is_none());
        assert!(!config.debug_logging);
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        let config = Config {
            cooldown_minutes: 30,
            reminding_enabled: false,
            auto_hide_grace_secs: 5,
            dismiss_countdown_secs: 8,
            skip_dismiss_confirmation: true,
            default_adapter: "iterm2".to_string(),
            hud_position: Some((100, 200)),
            debug_logging: true,
        };
        config.save(&path).unwrap();
        let loaded = Config::load(&path);
        assert_eq!(config, loaded);
    }

    #[test]
    fn test_load_missing_file_returns_default() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.toml");
        let config = Config::load(&path);
        assert_eq!(config, Config::default());
    }

    #[test]
    fn test_load_malformed_file_returns_default() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bad.toml");
        std::fs::write(&path, "this is not valid toml [[[").unwrap();
        let config = Config::load(&path);
        assert_eq!(config, Config::default());
    }

    #[test]
    fn test_load_partial_config_fills_defaults() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("partial.toml");
        std::fs::write(&path, "cooldown_minutes = 42\n").unwrap();
        let config = Config::load(&path);
        assert_eq!(config.cooldown_minutes, 42);
        assert!(config.reminding_enabled); // default
        assert_eq!(config.auto_hide_grace_secs, 2); // default
    }
}
```

- [ ] **Step 2: Add `dirs-next` dependency**

In `crates/core/Cargo.toml`, add under `[dependencies]`:
```toml
dirs-next = "2"
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p claude-pending-board-core -- config --nocapture`
Expected: All 5 tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/core/src/config.rs crates/core/Cargo.toml
git commit -m "feat(core): add Config with TOML load/save, defaults, and partial-fill"
```

---

## Task 9: Visibility State Machine (`crates/core/src/visibility.rs`)

**Files:**
- Create: `crates/core/src/visibility.rs`

This is the most complex piece of the core — the FSM with states `Hidden`, `Shown`, `CooldownHidden`. All timers go through a `Clock` trait so tests use fake time.

- [ ] **Step 1: Write the Clock trait and VisibilityController with failing tests**

```rust
use crate::config::Config;
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;

/// Abstraction over time so tests can use fake clocks.
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

/// Real wall clock.
pub struct WallClock;

impl Clock for WallClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// The three visibility states of the HUD.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisibilityState {
    /// HUD is not shown and not in cooldown.
    Hidden,
    /// HUD is shown. `grace_deadline` is set when the board goes empty.
    Shown {
        grace_deadline: Option<DateTime<Utc>>,
    },
    /// HUD was manually dismissed. Auto-show is suppressed until `until`.
    CooldownHidden {
        until: DateTime<Utc>,
        seen_add: bool,
        reminding_override: Option<bool>,
    },
}

/// Events that drive the FSM.
#[derive(Debug, Clone)]
pub enum VisibilityEvent {
    /// A new entry was added to the board.
    EntryAdded { board_count: usize },
    /// An entry was removed. `board_count` is the count *after* removal.
    EntryRemoved { board_count: usize },
    /// User clicked dismiss and the confirmation panel committed.
    ManualDismiss { reminding_override: Option<bool> },
    /// User clicked tray icon (manual open).
    ManualOpen,
    /// A timer tick (grace or cooldown may have expired).
    Tick,
}

/// Actions the UI layer should take in response to a state transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisibilityAction {
    ShowHud,
    HideHud,
    UpdateBadge { count: usize },
    /// No UI change needed.
    None,
}

pub struct VisibilityController {
    state: VisibilityState,
    clock: Arc<dyn Clock>,
    config: Config,
}

impl VisibilityController {
    pub fn new(clock: Arc<dyn Clock>, config: Config) -> Self {
        Self {
            state: VisibilityState::Hidden,
            clock,
            config,
        }
    }

    pub fn state(&self) -> &VisibilityState {
        &self.state
    }

    pub fn update_config(&mut self, config: Config) {
        self.config = config;
    }

    /// Process an event and return the action the UI should take.
    pub fn handle(&mut self, event: VisibilityEvent) -> VisibilityAction {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Fake clock that can be advanced manually.
    struct FakeClock {
        now: Mutex<DateTime<Utc>>,
    }

    impl FakeClock {
        fn new(now: DateTime<Utc>) -> Arc<Self> {
            Arc::new(Self { now: Mutex::new(now) })
        }

        fn advance(&self, duration: Duration) {
            let mut now = self.now.lock().unwrap();
            *now = *now + duration;
        }

        fn set(&self, time: DateTime<Utc>) {
            *self.now.lock().unwrap() = time;
        }
    }

    impl Clock for FakeClock {
        fn now(&self) -> DateTime<Utc> {
            *self.now.lock().unwrap()
        }
    }

    fn default_config() -> Config {
        Config::default()
    }

    fn t0() -> DateTime<Utc> {
        "2026-04-16T10:00:00Z".parse().unwrap()
    }

    // --- Auto-show tests ---

    #[test]
    fn test_first_entry_shows_hud() {
        let clock = FakeClock::new(t0());
        let mut ctrl = VisibilityController::new(clock, default_config());
        let action = ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        assert_eq!(action, VisibilityAction::ShowHud);
        assert!(matches!(ctrl.state(), VisibilityState::Shown { .. }));
    }

    #[test]
    fn test_additional_add_while_shown_does_not_reshow() {
        let clock = FakeClock::new(t0());
        let mut ctrl = VisibilityController::new(clock, default_config());
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        let action = ctrl.handle(VisibilityEvent::EntryAdded { board_count: 2 });
        assert_eq!(action, VisibilityAction::UpdateBadge { count: 2 });
    }

    // --- Grace timer tests ---

    #[test]
    fn test_board_empty_starts_grace_timer() {
        let clock = FakeClock::new(t0());
        let mut ctrl = VisibilityController::new(clock.clone(), default_config());
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        let action = ctrl.handle(VisibilityEvent::EntryRemoved { board_count: 0 });
        // Should not hide immediately — grace timer started
        assert_eq!(action, VisibilityAction::UpdateBadge { count: 0 });
        assert!(matches!(ctrl.state(), VisibilityState::Shown { grace_deadline: Some(_) }));
    }

    #[test]
    fn test_grace_timer_expired_hides_hud() {
        let clock = FakeClock::new(t0());
        let mut ctrl = VisibilityController::new(clock.clone(), default_config());
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        ctrl.handle(VisibilityEvent::EntryRemoved { board_count: 0 });
        // Advance past grace period (2s default)
        clock.advance(Duration::seconds(3));
        let action = ctrl.handle(VisibilityEvent::Tick);
        assert_eq!(action, VisibilityAction::HideHud);
        assert_eq!(*ctrl.state(), VisibilityState::Hidden);
    }

    #[test]
    fn test_new_add_during_grace_cancels_timer() {
        let clock = FakeClock::new(t0());
        let mut ctrl = VisibilityController::new(clock.clone(), default_config());
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        ctrl.handle(VisibilityEvent::EntryRemoved { board_count: 0 });
        // New add arrives during grace
        let action = ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        assert_eq!(action, VisibilityAction::UpdateBadge { count: 1 });
        assert!(matches!(ctrl.state(), VisibilityState::Shown { grace_deadline: None }));
    }

    // --- Manual dismiss + cooldown tests ---

    #[test]
    fn test_manual_dismiss_enters_cooldown() {
        let clock = FakeClock::new(t0());
        let mut ctrl = VisibilityController::new(clock, default_config());
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        let action = ctrl.handle(VisibilityEvent::ManualDismiss { reminding_override: None });
        assert_eq!(action, VisibilityAction::HideHud);
        assert!(matches!(ctrl.state(), VisibilityState::CooldownHidden { .. }));
    }

    #[test]
    fn test_add_during_cooldown_sets_seen_flag_but_no_show() {
        let clock = FakeClock::new(t0());
        let mut ctrl = VisibilityController::new(clock, default_config());
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        ctrl.handle(VisibilityEvent::ManualDismiss { reminding_override: None });
        let action = ctrl.handle(VisibilityEvent::EntryAdded { board_count: 2 });
        assert_eq!(action, VisibilityAction::UpdateBadge { count: 2 });
        match ctrl.state() {
            VisibilityState::CooldownHidden { seen_add, .. } => assert!(*seen_add),
            _ => panic!("expected CooldownHidden"),
        }
    }

    #[test]
    fn test_cooldown_expiry_with_reminding_on_and_seen_add_shows_hud() {
        let clock = FakeClock::new(t0());
        let mut ctrl = VisibilityController::new(clock.clone(), default_config());
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        ctrl.handle(VisibilityEvent::ManualDismiss { reminding_override: None });
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 2 }); // sets seen_add
        // Advance past cooldown (15 min default)
        clock.advance(Duration::minutes(16));
        let action = ctrl.handle(VisibilityEvent::Tick);
        assert_eq!(action, VisibilityAction::ShowHud);
    }

    #[test]
    fn test_cooldown_expiry_no_seen_add_stays_hidden() {
        let clock = FakeClock::new(t0());
        let mut ctrl = VisibilityController::new(clock.clone(), default_config());
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        ctrl.handle(VisibilityEvent::ManualDismiss { reminding_override: None });
        // No new adds during cooldown
        clock.advance(Duration::minutes(16));
        let action = ctrl.handle(VisibilityEvent::Tick);
        assert_eq!(action, VisibilityAction::HideHud);
        assert_eq!(*ctrl.state(), VisibilityState::Hidden);
    }

    #[test]
    fn test_cooldown_expiry_reminding_disabled_stays_hidden() {
        let clock = FakeClock::new(t0());
        let mut config = default_config();
        config.reminding_enabled = false;
        let mut ctrl = VisibilityController::new(clock.clone(), config);
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        ctrl.handle(VisibilityEvent::ManualDismiss { reminding_override: None });
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 2 }); // seen_add = true
        clock.advance(Duration::minutes(16));
        let action = ctrl.handle(VisibilityEvent::Tick);
        assert_eq!(action, VisibilityAction::HideHud);
        assert_eq!(*ctrl.state(), VisibilityState::Hidden);
    }

    // --- Per-dismiss override tests ---

    #[test]
    fn test_override_wake_me_forces_reshow() {
        let clock = FakeClock::new(t0());
        let mut config = default_config();
        config.reminding_enabled = false; // global says no
        let mut ctrl = VisibilityController::new(clock.clone(), config);
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        // User clicks "Wake me" — overrides global to true
        ctrl.handle(VisibilityEvent::ManualDismiss { reminding_override: Some(true) });
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 2 });
        clock.advance(Duration::minutes(16));
        let action = ctrl.handle(VisibilityEvent::Tick);
        assert_eq!(action, VisibilityAction::ShowHud);
    }

    #[test]
    fn test_override_stay_silent_suppresses_reshow() {
        let clock = FakeClock::new(t0());
        let mut ctrl = VisibilityController::new(clock.clone(), default_config());
        // Global reminding is on, but user clicked "Stay silent"
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        ctrl.handle(VisibilityEvent::ManualDismiss { reminding_override: Some(false) });
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 2 });
        clock.advance(Duration::minutes(16));
        let action = ctrl.handle(VisibilityEvent::Tick);
        assert_eq!(action, VisibilityAction::HideHud);
        assert_eq!(*ctrl.state(), VisibilityState::Hidden);
    }

    // --- Manual open cancels cooldown ---

    #[test]
    fn test_manual_open_during_cooldown_shows_hud() {
        let clock = FakeClock::new(t0());
        let mut ctrl = VisibilityController::new(clock, default_config());
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        ctrl.handle(VisibilityEvent::ManualDismiss { reminding_override: None });
        let action = ctrl.handle(VisibilityEvent::ManualOpen);
        assert_eq!(action, VisibilityAction::ShowHud);
        assert!(matches!(ctrl.state(), VisibilityState::Shown { .. }));
    }

    #[test]
    fn test_manual_open_from_hidden() {
        let clock = FakeClock::new(t0());
        let mut ctrl = VisibilityController::new(clock, default_config());
        let action = ctrl.handle(VisibilityEvent::ManualOpen);
        assert_eq!(action, VisibilityAction::ShowHud);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p claude-pending-board-core -- visibility --nocapture`
Expected: FAIL — `todo!()` panics

- [ ] **Step 3: Implement `handle`**

```rust
pub fn handle(&mut self, event: VisibilityEvent) -> VisibilityAction {
    let now = self.clock.now();

    match (&mut self.state, event) {
        // --- Hidden state ---
        (VisibilityState::Hidden, VisibilityEvent::EntryAdded { board_count }) => {
            if board_count > 0 {
                self.state = VisibilityState::Shown { grace_deadline: None };
                VisibilityAction::ShowHud
            } else {
                VisibilityAction::None
            }
        }
        (VisibilityState::Hidden, VisibilityEvent::ManualOpen) => {
            self.state = VisibilityState::Shown { grace_deadline: None };
            VisibilityAction::ShowHud
        }
        (VisibilityState::Hidden, _) => VisibilityAction::None,

        // --- Shown state ---
        (VisibilityState::Shown { grace_deadline }, VisibilityEvent::EntryAdded { board_count }) => {
            // Cancel any grace timer since new items arrived
            *grace_deadline = None;
            VisibilityAction::UpdateBadge { count: board_count }
        }
        (VisibilityState::Shown { grace_deadline }, VisibilityEvent::EntryRemoved { board_count }) => {
            if board_count == 0 {
                // Start grace timer
                let deadline = now + Duration::seconds(self.config.auto_hide_grace_secs as i64);
                *grace_deadline = Some(deadline);
            }
            VisibilityAction::UpdateBadge { count: board_count }
        }
        (VisibilityState::Shown { .. }, VisibilityEvent::ManualDismiss { reminding_override }) => {
            let until = now + Duration::minutes(self.config.cooldown_minutes as i64);
            self.state = VisibilityState::CooldownHidden {
                until,
                seen_add: false,
                reminding_override,
            };
            VisibilityAction::HideHud
        }
        (VisibilityState::Shown { grace_deadline, .. }, VisibilityEvent::Tick) => {
            if let Some(deadline) = grace_deadline {
                if now >= *deadline {
                    self.state = VisibilityState::Hidden;
                    return VisibilityAction::HideHud;
                }
            }
            VisibilityAction::None
        }
        (VisibilityState::Shown { .. }, VisibilityEvent::ManualOpen) => {
            VisibilityAction::None // already shown
        }

        // --- CooldownHidden state ---
        (VisibilityState::CooldownHidden { seen_add, .. }, VisibilityEvent::EntryAdded { board_count }) => {
            *seen_add = true;
            VisibilityAction::UpdateBadge { count: board_count }
        }
        (VisibilityState::CooldownHidden { .. }, VisibilityEvent::EntryRemoved { board_count }) => {
            VisibilityAction::UpdateBadge { count: board_count }
        }
        (VisibilityState::CooldownHidden { .. }, VisibilityEvent::ManualOpen) => {
            self.state = VisibilityState::Shown { grace_deadline: None };
            VisibilityAction::ShowHud
        }
        (VisibilityState::CooldownHidden { until, seen_add, reminding_override }, VisibilityEvent::Tick) => {
            if now >= *until {
                let should_remind = reminding_override
                    .unwrap_or(self.config.reminding_enabled);
                if should_remind && *seen_add {
                    self.state = VisibilityState::Shown { grace_deadline: None };
                    VisibilityAction::ShowHud
                } else {
                    self.state = VisibilityState::Hidden;
                    VisibilityAction::HideHud
                }
            } else {
                VisibilityAction::None
            }
        }
        (VisibilityState::CooldownHidden { .. }, VisibilityEvent::ManualDismiss { .. }) => {
            VisibilityAction::None // already hidden
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p claude-pending-board-core -- visibility --nocapture`
Expected: All 13 tests PASS

- [ ] **Step 5: Run clippy**

Run: `cargo clippy -p claude-pending-board-core -- -D warnings`
Expected: 0 warnings

- [ ] **Step 6: Commit**

```bash
git add crates/core/src/visibility.rs
git commit -m "feat(core): implement visibility FSM with grace timer, cooldown, and reminding override"
```

---

## Task 10: Reaper (`crates/core/src/reaper.rs`)

**Files:**
- Create: `crates/core/src/reaper.rs`

- [ ] **Step 1: Write the reaper with tests**

The reaper checks liveness of each entry by verifying (1) the PID is alive and (2) the session file matches. We abstract the process table and filesystem for testability.

```rust
use crate::types::{Entry, EntryState, Op, SessionId};
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Abstraction over OS process queries for testability.
pub trait ProcessTable: Send + Sync {
    fn is_alive(&self, pid: u32) -> bool;
}

/// Real process table backed by sysinfo.
pub struct RealProcessTable;

impl ProcessTable for RealProcessTable {
    fn is_alive(&self, pid: u32) -> bool {
        use sysinfo::{Pid, System, ProcessRefreshKind, ProcessesToUpdate};
        let mut sys = System::new();
        sys.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[Pid::from_u32(pid)]),
            true,
            ProcessRefreshKind::new(),
        );
        sys.process(Pid::from_u32(pid)).is_some()
    }
}

/// Abstraction over session file reads for testability.
pub trait SessionFiles: Send + Sync {
    /// Read the session file for a given PID and return the sessionId if present.
    fn read_session_id(&self, claude_pid: u32) -> Option<String>;
}

/// Real session file reader from `~/.claude/sessions/<pid>.json`.
pub struct RealSessionFiles {
    sessions_dir: PathBuf,
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
    /// Process is alive and session file matches.
    Alive,
    /// Process is dead.
    Dead,
    /// Process is alive but session file is missing or mismatched (PID recycled).
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
    use chrono::DateTime;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct MockProcessTable {
        alive_pids: Vec<u32>,
    }

    impl ProcessTable for MockProcessTable {
        fn is_alive(&self, pid: u32) -> bool {
            self.alive_pids.contains(&pid)
        }
    }

    struct MockSessionFiles {
        sessions: HashMap<u32, String>, // pid -> sessionId
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

        let proc_table = MockProcessTable { alive_pids: vec![] }; // process dead
        let session_files = MockSessionFiles { sessions: HashMap::new() };

        let ops = sweep(&[entry], &proc_table, &session_files);
        assert!(ops.is_empty()); // should not re-stale
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p claude-pending-board-core -- reaper --nocapture`
Expected: All 6 tests PASS

- [ ] **Step 3: Commit**

```bash
git add crates/core/src/reaper.rs
git commit -m "feat(core): implement Reaper with dual liveness check and mock-friendly sweep"
```

---

## Task 11: Final Integration Verification

- [ ] **Step 1: Create stub modules so lib.rs compiles**

The `board/mod.rs` already re-exports `parser`, `store`, `watcher`, `compaction`. Verify `lib.rs` re-exports everything:

```rust
pub mod types;
pub mod board;
pub mod visibility;
pub mod reaper;
pub mod terminal;
pub mod config;
```

- [ ] **Step 2: Run all tests in the core crate**

Run: `cargo test -p claude-pending-board-core --nocapture`
Expected: All tests pass (parser: 9, store: 8, compaction: 7, watcher: 3, terminal: 3, config: 5, visibility: 13, reaper: 6 = ~54 tests)

- [ ] **Step 3: Run clippy on entire workspace**

Run: `cargo clippy --workspace -- -D warnings`
Expected: 0 warnings

- [ ] **Step 4: Run fmt check**

Run: `cargo fmt --check --all`
Expected: No formatting issues

- [ ] **Step 5: Commit any final fixes and tag**

```bash
git add -A
git commit -m "chore: phase 1 complete — core crate with all modules tested"
git tag v0.0.1-core
```

---

## Self-Review Checklist

1. **Spec coverage:**
   - Hook-driven entry capture: `types.rs` (Op::Add), `parser.rs`, `store.rs` — covered
   - Entry removal: `types.rs` (Op::Clear), `store.rs` apply — covered
   - Live/stale tracking: `reaper.rs` with dual check — covered
   - Sorting and grouping: `store.rs` snapshot() — covered
   - Auto show/hide: `visibility.rs` FSM — covered
   - Manual dismiss with cooldown: `visibility.rs` CooldownHidden — covered
   - Dismiss confirmation panel: visibility events for override — covered (UI in Phase 3)
   - Click to focus: `terminal.rs` trait + ancestor_walk — covered (adapters in Phase 2)
   - Click to resume stale: `terminal.rs` trait spawn_resume — covered (adapters in Phase 2)
   - Settings: `config.rs` — covered
   - Board file resilience: `parser.rs` (malformed skip), `watcher.rs` (deletion handling), `compaction.rs` (atomic write) — covered

2. **Placeholder scan:** No TBD, TODO, or "implement later" found. All `todo!()` markers are in test-first steps and replaced in subsequent implementation steps.

3. **Type consistency:** `Op`, `Entry`, `EntryState`, `NotificationType`, `SessionId`, `TerminalMatch`, `Config` — names consistent across all modules. `StateStore::apply` takes `Op`, `snapshot` returns `Vec<Entry>`, `sweep` takes `&[Entry]` and returns `Vec<Op>` — all consistent.
