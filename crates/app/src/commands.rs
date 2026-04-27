use crate::state::SharedState;
use claude_pending_board_core::config::Config;
use claude_pending_board_core::types::Entry;
use claude_pending_board_core::visibility::{VisibilityAction, VisibilityEvent};
use tauri::{AppHandle, Emitter, Manager, State};

#[tauri::command]
pub fn list_entries(state: State<SharedState>) -> Vec<Entry> {
    let s = state.lock().unwrap();
    s.entries()
}

#[tauri::command]
pub fn focus_entry(state: State<SharedState>, session_id: String) -> Result<String, String> {
    // Collect all data we need while holding the lock, then release it before
    // calling into adapter methods (which may block or call external processes).
    let (entry, terminal_match_opt, adapter_name) = {
        let s = state.lock().unwrap();
        let entry = s
            .store
            .get(&session_id)
            .ok_or_else(|| "entry not found".to_string())?
            .clone();

        // Skip the host-side ancestor walk for WSL-origin entries — the
        // claude_pid lives in WSL's pid namespace, so the walk can never
        // find a Windows wezterm parent. WSL clicks always route through
        // spawn_resume.
        let terminal_match_opt = if entry.state
            == claude_pending_board_core::types::EntryState::Live
            && entry.wsl_distro.is_none()
        {
            s.adapter_registry.detect(entry.claude_pid).map(|(_, m)| m)
        } else {
            None
        };

        let adapter_name = s.config.default_adapter.clone();
        (entry, terminal_match_opt, adapter_name)
    };

    // Now lock again only to get the adapter reference and call it outside
    if let Some(terminal_match) = terminal_match_opt {
        let s = state.lock().unwrap();
        // We need to detect again to get the adapter reference (can't store refs across lock)
        if let Some((adapter, _)) = s.adapter_registry.detect(entry.claude_pid) {
            // Clone the adapter name to call focus without holding lock reference
            let adapter_name_inner = adapter.name().to_string();
            drop(s);
            // Re-acquire to call via name lookup (avoids lifetime issues)
            let s2 = state.lock().unwrap();
            if let Some(a) = s2.adapter_registry.get_by_name(&adapter_name_inner) {
                a.focus_pane(&terminal_match)
                    .map_err(|e| format!("focus failed: {e}"))?;
                return Ok("focused".to_string());
            }
        }
    }

    {
        let s = state.lock().unwrap();
        if let Some(adapter) = s.adapter_registry.get_by_name(&adapter_name) {
            adapter
                .spawn_resume(&entry.cwd, &entry.session_id, entry.wsl_distro.as_deref())
                .map_err(|e| format!("spawn failed: {e}"))?;
            return Ok("resumed".to_string());
        }
    }

    Err("no adapter available".to_string())
}

#[tauri::command]
pub fn dismiss_hud(
    app: AppHandle,
    state: State<SharedState>,
    reminding_override: Option<bool>,
) -> Result<(), String> {
    let mut s = state.lock().unwrap();
    let action = s
        .visibility
        .handle(VisibilityEvent::ManualDismiss { reminding_override });
    drop(s);

    if action == VisibilityAction::HideHud {
        if let Some(window) = app.get_webview_window("hud") {
            let _ = window.hide();
        }
    }
    Ok(())
}

#[tauri::command]
pub fn manual_open(app: AppHandle, state: State<SharedState>) -> Result<(), String> {
    let mut s = state.lock().unwrap();
    let action = s.visibility.handle(VisibilityEvent::ManualOpen);
    let entries = s.entries();
    drop(s);

    if action == VisibilityAction::ShowHud {
        if let Some(window) = app.get_webview_window("hud") {
            crate::hud_show::show_without_activation(&window);
            let _ = app.emit("entries-updated", &entries);
        }
    }
    Ok(())
}

/// Shared helper to show (and focus) the pre-created Settings window.
///
/// The settings window is built up-front during app setup — we just show it here.
/// This avoids the race condition and hang that happened when creating the window
/// on-demand from a Tauri command or tray menu handler.
pub fn open_settings_window(app: &AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("settings")
        .ok_or_else(|| "settings window not found".to_string())?;
    window
        .show()
        .map_err(|e| format!("failed to show settings: {e}"))?;
    window
        .set_focus()
        .map_err(|e| format!("failed to focus settings: {e}"))?;
    tracing::info!("settings window shown");
    Ok(())
}

#[tauri::command]
pub fn open_settings(app: AppHandle) -> Result<(), String> {
    open_settings_window(&app)
}

#[tauri::command]
pub fn hide_settings(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("settings") {
        window
            .hide()
            .map_err(|e| format!("failed to hide settings: {e}"))?;
    }
    Ok(())
}

/// Move the HUD window to its default position (bottom-right of the primary
/// monitor, near the tray) and clear any saved position in config.
#[tauri::command]
pub fn reset_hud_position(app: AppHandle, state: State<SharedState>) -> Result<(), String> {
    let window = app
        .get_webview_window("hud")
        .ok_or_else(|| "hud window not found".to_string())?;

    let monitor = window
        .primary_monitor()
        .map_err(|e| format!("failed to get monitor: {e}"))?
        .ok_or_else(|| "no primary monitor".to_string())?;

    let size = monitor.size();
    let scale = monitor.scale_factor();

    // HUD is 380x440 logical pixels. Margin + taskbar allowance at the bottom.
    let hud_w = (380.0 * scale) as i32;
    let hud_h = (440.0 * scale) as i32;
    let margin_right = (16.0 * scale) as i32;
    let margin_bottom = (64.0 * scale) as i32;

    let x = size.width as i32 - hud_w - margin_right;
    let y = size.height as i32 - hud_h - margin_bottom;

    let position = tauri::PhysicalPosition::new(x, y);
    window
        .set_position(position)
        .map_err(|e| format!("failed to set position: {e}"))?;

    let mut s = state.lock().unwrap();
    s.config.hud_position = None;
    s.config
        .save(&Config::default_path())
        .map_err(|e| format!("failed to save config: {e}"))?;

    tracing::info!(x, y, "HUD position reset to tray-anchor default");
    Ok(())
}

#[tauri::command]
pub fn get_config(state: State<SharedState>) -> Config {
    let s = state.lock().unwrap();
    s.config.clone()
}

#[tauri::command]
pub fn apply_config(state: State<SharedState>, config: Config) -> Result<(), String> {
    let mut s = state.lock().unwrap();
    s.visibility.update_config(config.clone());
    config
        .save(&Config::default_path())
        .map_err(|e| format!("failed to save config: {e}"))?;
    s.config = config;
    Ok(())
}

#[tauri::command]
pub fn check_hooks_installed() -> crate::plugin_install::HookStatus {
    crate::plugin_install::detect()
}

#[tauri::command]
pub async fn install_plugin() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(crate::plugin_install::install)
        .await
        .map_err(|e| format!("install task failed: {e}"))?
}

/// Manually dismiss a single entry from the HUD.
///
/// Appends a `clear` op with reason `user_dismissed` to the board file. The
/// watcher picks it up, the store drops the entry, and the HUD re-renders
/// through the normal pipeline — same shape as hook-driven clears or the
/// periodic stale-cleanup loop.
#[tauri::command]
pub fn dismiss_entry(session_id: String) -> Result<(), String> {
    use claude_pending_board_core::types::Op;
    use std::io::Write;

    let home = dirs_next::home_dir().ok_or_else(|| "no home dir".to_string())?;
    let board_file = home.join(".claude").join("pending").join("board.jsonl");

    let op = Op::Clear {
        ts: chrono::Utc::now(),
        session_id,
        reason: "user_dismissed".to_string(),
    };
    let line = serde_json::to_string(&op).map_err(|e| format!("serialize: {e}"))?;

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&board_file)
        .map_err(|e| format!("open board: {e}"))?;
    writeln!(file, "{}", line).map_err(|e| format!("write: {e}"))?;
    Ok(())
}
