#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod services;
mod state;
mod tray;

use state::{AppState, SharedState};
use std::sync::{Arc, Mutex};

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("claude_pending_board=info")
        .init();

    let shared_state: SharedState = Arc::new(Mutex::new(AppState::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .manage(shared_state.clone())
        .setup(move |app| {
            // Build the HUD window before booting services so the async op pipeline
            // can always find it via get_webview_window("hud"). Without this, ops
            // loaded from a non-empty board.jsonl at startup can race the window
            // creation and silently drop the ShowHud action.
            let _hud_window = tauri::WebviewWindowBuilder::new(
                app,
                "hud",
                tauri::WebviewUrl::App("hud/index.html".into()),
            )
            .title("Claude Pending Board")
            .inner_size(380.0, 440.0)
            .resizable(false)
            .decorations(false)
            .always_on_top(true)
            .visible(false)
            .skip_taskbar(true)
            .build()?;

            services::boot(app.handle(), shared_state.clone());
            tray::setup(app)?;

            tracing::info!("Claude Pending Board started");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_entries,
            commands::focus_entry,
            commands::dismiss_hud,
            commands::manual_open,
            commands::open_settings,
            commands::get_config,
            commands::apply_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
