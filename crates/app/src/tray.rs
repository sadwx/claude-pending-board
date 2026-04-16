use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, Manager,
};

pub fn setup(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let open_item = MenuItemBuilder::with_id("open", "Open").build(app)?;
    let settings_item = MenuItemBuilder::with_id("settings", "Settings...").build(app)?;
    let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .items(&[&open_item, &settings_item, &quit_item])
        .build()?;

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "open" => {
                let state: tauri::State<crate::state::SharedState> = app.state();
                let mut s = state.lock().unwrap();
                let action = s
                    .visibility
                    .handle(claude_pending_board_core::visibility::VisibilityEvent::ManualOpen);
                let entries = s.entries();
                drop(s);

                if action == claude_pending_board_core::visibility::VisibilityAction::ShowHud {
                    if let Some(window) = app.get_webview_window("hud") {
                        let _ = window.show();
                        let _ = tauri::Emitter::emit(app, "entries-updated", &entries);
                    }
                }
            }
            "settings" => {
                if let Some(window) = app.get_webview_window("settings") {
                    let _ = window.show();
                    let _ = window.set_focus();
                } else {
                    let _ = tauri::WebviewWindowBuilder::new(
                        app,
                        "settings",
                        tauri::WebviewUrl::App("settings/index.html".into()),
                    )
                    .title("Settings - Claude Pending Board")
                    .inner_size(480.0, 500.0)
                    .resizable(true)
                    .build();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                let state: tauri::State<crate::state::SharedState> = app.state();
                let mut s = state.lock().unwrap();
                let action = s
                    .visibility
                    .handle(claude_pending_board_core::visibility::VisibilityEvent::ManualOpen);
                let entries = s.entries();
                drop(s);

                if action == claude_pending_board_core::visibility::VisibilityAction::ShowHud {
                    if let Some(window) = app.get_webview_window("hud") {
                        let _ = window.show();
                        let _ = tauri::Emitter::emit(app, "entries-updated", &entries);
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}
