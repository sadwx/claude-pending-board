use crate::state::SharedState;
use claude_pending_board_core::board::compaction;
use claude_pending_board_core::board::watcher::BoardWatcher;
use claude_pending_board_core::reaper::{self, RealProcessTable, RealSessionFiles};
use claude_pending_board_core::visibility::{VisibilityAction, VisibilityEvent};
use chrono::Duration;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;

fn board_path() -> PathBuf {
    let home = dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".claude").join("pending").join("board.jsonl")
}

pub fn boot(app: &AppHandle, state: SharedState) {
    let app_handle = app.clone();

    let (op_tx, op_rx) = mpsc::unbounded_channel();
    let board_file = board_path();

    if board_file.exists() {
        if let Err(e) = compaction::compact(&board_file, Duration::hours(24)) {
            tracing::warn!(error = %e, "startup compaction failed");
        }
    }

    match BoardWatcher::start(board_file, op_tx) {
        Ok(watcher) => {
            tracing::info!("board watcher started");
            std::mem::forget(watcher);
        }
        Err(e) => {
            tracing::error!(error = %e, "failed to start board watcher");
        }
    }

    let app_for_ops = app_handle.clone();
    let state_for_ops = state.clone();
    tauri::async_runtime::spawn(async move {
        process_ops(op_rx, app_for_ops, state_for_ops).await;
    });

    let app_for_reaper = app_handle.clone();
    let state_for_reaper = state.clone();
    tauri::async_runtime::spawn(async move {
        reaper_loop(app_for_reaper, state_for_reaper).await;
    });

    let app_for_tick = app_handle;
    let state_for_tick = state;
    tauri::async_runtime::spawn(async move {
        visibility_tick_loop(app_for_tick, state_for_tick).await;
    });
}

async fn process_ops(
    mut op_rx: mpsc::UnboundedReceiver<Vec<claude_pending_board_core::types::Op>>,
    app: AppHandle,
    state: SharedState,
) {
    while let Some(ops) = op_rx.recv().await {
        let mut s = state.lock().unwrap();
        let count_before = s.store.len();

        for op in ops {
            s.store.apply(op);
        }

        let count_after = s.store.len();
        let entries = s.entries();

        let action = if count_after > count_before {
            s.visibility.handle(VisibilityEvent::EntryAdded { board_count: count_after })
        } else if count_after < count_before {
            s.visibility.handle(VisibilityEvent::EntryRemoved { board_count: count_after })
        } else {
            VisibilityAction::None
        };

        drop(s);

        let _ = app.emit("entries-updated", &entries);
        let _ = app.emit("badge-count", count_after);
        apply_visibility_action(&app, &action);
    }
}

async fn reaper_loop(_app: AppHandle, state: SharedState) {
    let proc_table = RealProcessTable;
    let session_files = RealSessionFiles::new();

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;

        let entries = {
            let s = state.lock().unwrap();
            s.entries()
        };

        let stale_ops = reaper::sweep(&entries, &proc_table, &session_files);

        if !stale_ops.is_empty() {
            let board_file = board_path();
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&board_file)
            {
                use std::io::Write;
                for op in &stale_ops {
                    if let Ok(line) = serde_json::to_string(op) {
                        let _ = writeln!(file, "{}", line);
                    }
                }
            }
        }
    }
}

async fn visibility_tick_loop(app: AppHandle, state: SharedState) {
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        let action = {
            let mut s = state.lock().unwrap();
            s.visibility.handle(VisibilityEvent::Tick)
        };

        apply_visibility_action(&app, &action);
    }
}

fn apply_visibility_action(app: &AppHandle, action: &VisibilityAction) {
    match action {
        VisibilityAction::ShowHud => {
            if let Some(window) = app.get_webview_window("hud") {
                let _ = window.show();
            }
        }
        VisibilityAction::HideHud => {
            if let Some(window) = app.get_webview_window("hud") {
                let _ = window.hide();
            }
        }
        VisibilityAction::UpdateBadge { count } => {
            let _ = app.emit("badge-count", count);
        }
        VisibilityAction::None => {}
    }
}
