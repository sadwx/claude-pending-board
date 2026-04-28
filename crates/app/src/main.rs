#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod hud_show;
mod plugin_install;
mod services;
mod state;
mod tray;
#[cfg(target_os = "windows")]
mod wsl_env_setup;

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
            // Run as a menu-bar agent on macOS — no Dock icon, no app-switcher
            // entry. The HUD and Settings windows still appear when shown;
            // closing the last window does not exit the app (tray keeps it
            // alive). Must come before any window is shown.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Best-effort: keep the installed plugin.json clean of hook
            // entries for other OSes. Claude Code 2.1.x ignores the
            // `platform` field on hook entries, so without this they show
            // up in `/hooks` and ENOENT on every fire. Non-fatal on error.
            tauri::async_runtime::spawn_blocking(|| {
                let _ = plugin_install::sanitize_installed_plugin_json();
            });

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
            .transparent(true)
            .always_on_top(true)
            .visible(false)
            .skip_taskbar(true)
            .build()?;

            // Pre-create the Settings window hidden. Creating it here during
            // setup (main thread) avoids race conditions and webview hangs we
            // saw when creating it on-demand from a command handler.
            let settings_window = tauri::WebviewWindowBuilder::new(
                app,
                "settings",
                tauri::WebviewUrl::App("settings/index.html".into()),
            )
            .title("Settings - Claude Pending Board")
            .inner_size(480.0, 500.0)
            .resizable(true)
            .visible(false)
            .skip_taskbar(true)
            .build()?;

            // Intercept the close button: hide the window instead of destroying
            // it, so we can keep reopening the same window.
            let settings_handle = settings_window.clone();
            settings_window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = settings_handle.hide();
                }
            });

            services::boot(app.handle(), shared_state.clone());
            tray::setup(app)?;

            // Auto-configure WSLENV so click-to-focus works for WSL-origin
            // entries without manual `setx` from the user. Runs in a
            // blocking task — touches the registry and may shell out to
            // PowerShell for the broadcast on first run, neither of which
            // should hold up app boot.
            #[cfg(target_os = "windows")]
            tauri::async_runtime::spawn_blocking(wsl_env_setup::ensure_wezterm_pane_in_wslenv);

            tracing::info!("Claude Pending Board started");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_entries,
            commands::focus_entry,
            commands::dismiss_hud,
            commands::dismiss_panel_opened,
            commands::manual_open,
            commands::open_settings,
            commands::hide_settings,
            commands::reset_hud_position,
            commands::get_config,
            commands::apply_config,
            commands::check_hooks_installed,
            commands::install_plugin,
            commands::dismiss_entry,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
