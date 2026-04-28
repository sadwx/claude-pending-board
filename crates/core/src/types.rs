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
    /// When the entry transitioned to Stale (for the 1h expiry — see
    /// the periodic stale cleanup loop in `crates/app/src/services.rs`).
    pub stale_since: Option<DateTime<Utc>>,
    /// Name of the WSL distro the entry originated in (e.g. `"Ubuntu-24.04"`),
    /// or `None` for entries created on Windows or macOS directly.
    /// Drives reaper short-circuit and click-to-resume routing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wsl_distro: Option<String>,
    /// WezTerm pane id captured from `$WEZTERM_PANE` at hook time. When
    /// present, click-to-focus calls `wezterm cli activate-pane` directly
    /// instead of walking the process tree — this is what makes WSL
    /// click-to-focus land on the existing tab and also fixes Windows
    /// multi-pane targeting. `None` for non-WezTerm shells (iTerm2, plain
    /// Terminal.app, etc.) or older hook versions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wezterm_pane_id: Option<String>,
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
        /// See [`Entry::wsl_distro`].
        #[serde(default, skip_serializing_if = "Option::is_none")]
        wsl_distro: Option<String>,
        /// See [`Entry::wezterm_pane_id`].
        #[serde(default, skip_serializing_if = "Option::is_none")]
        wezterm_pane_id: Option<String>,
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
