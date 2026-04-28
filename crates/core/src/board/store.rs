use crate::types::{Entry, EntryState, Op, SessionId};
use std::collections::HashMap;

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
        match op {
            Op::Add {
                ts,
                session_id,
                cwd,
                claude_pid,
                terminal_pid,
                transcript_path,
                notification_type,
                message,
                wsl_distro,
                wezterm_pane_id,
            } => {
                self.entries.insert(
                    session_id.clone(),
                    Entry {
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
                        wsl_distro,
                        wezterm_pane_id,
                    },
                );
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
        let mut entries: Vec<Entry> = self.entries.values().cloned().collect();
        entries.sort_by(|a, b| {
            let group_a = match a.state {
                EntryState::Live => a.notification_type.priority(),
                EntryState::Stale => 2,
            };
            let group_b = match b.state {
                EntryState::Live => b.notification_type.priority(),
                EntryState::Stale => 2,
            };
            group_a.cmp(&group_b).then_with(|| b.ts.cmp(&a.ts))
        });
        entries
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::NotificationType;
    use chrono::{DateTime, Utc};
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
            wsl_distro: None,
            wezterm_pane_id: None,
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
        let changed = store.apply(make_add(
            "s1",
            NotificationType::PermissionPrompt,
            "2026-04-16T10:00:00Z",
        ));
        assert!(changed);
        assert_eq!(store.len(), 1);
        let entry = store.get("s1").unwrap();
        assert_eq!(entry.state, EntryState::Live);
        assert_eq!(entry.notification_type, NotificationType::PermissionPrompt);
    }

    #[test]
    fn test_apply_add_overwrites_same_session() {
        let mut store = StateStore::new();
        store.apply(make_add(
            "s1",
            NotificationType::PermissionPrompt,
            "2026-04-16T10:00:00Z",
        ));
        store.apply(make_add(
            "s1",
            NotificationType::IdlePrompt,
            "2026-04-16T10:01:00Z",
        ));
        assert_eq!(store.len(), 1);
        assert_eq!(
            store.get("s1").unwrap().notification_type,
            NotificationType::IdlePrompt
        );
    }

    #[test]
    fn test_apply_clear_removes_entry() {
        let mut store = StateStore::new();
        store.apply(make_add(
            "s1",
            NotificationType::PermissionPrompt,
            "2026-04-16T10:00:00Z",
        ));
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
        store.apply(make_add(
            "s1",
            NotificationType::PermissionPrompt,
            "2026-04-16T10:00:00Z",
        ));
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
        store.apply(make_add(
            "idle-old",
            NotificationType::IdlePrompt,
            "2026-04-16T10:00:00Z",
        ));
        store.apply(make_add(
            "perm-old",
            NotificationType::PermissionPrompt,
            "2026-04-16T10:01:00Z",
        ));
        store.apply(make_add(
            "idle-new",
            NotificationType::IdlePrompt,
            "2026-04-16T10:02:00Z",
        ));
        store.apply(make_add(
            "perm-new",
            NotificationType::PermissionPrompt,
            "2026-04-16T10:03:00Z",
        ));
        store.apply(make_add(
            "stale-one",
            NotificationType::PermissionPrompt,
            "2026-04-16T09:00:00Z",
        ));
        store.apply(make_stale("stale-one", "2026-04-16T10:04:00Z"));

        let snap = store.snapshot();
        let ids: Vec<&str> = snap.iter().map(|e| e.session_id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["perm-new", "perm-old", "idle-new", "idle-old", "stale-one"]
        );
    }

    #[test]
    fn test_remove_where() {
        let mut store = StateStore::new();
        store.apply(make_add(
            "s1",
            NotificationType::PermissionPrompt,
            "2026-04-16T10:00:00Z",
        ));
        store.apply(make_add(
            "s2",
            NotificationType::IdlePrompt,
            "2026-04-16T10:01:00Z",
        ));
        store.apply(make_add(
            "s3",
            NotificationType::PermissionPrompt,
            "2026-04-16T10:02:00Z",
        ));
        let removed = store.remove_where(|e| e.notification_type == NotificationType::IdlePrompt);
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].session_id, "s2");
        assert_eq!(store.len(), 2);
    }
}
