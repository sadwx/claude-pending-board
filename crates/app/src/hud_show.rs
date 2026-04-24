//! Cross-platform HUD show helper that does not steal focus.
//!
//! `tauri::WebviewWindow::show()` maps to `ShowWindow(SW_SHOW)` on Windows and
//! `[NSWindow makeKeyAndOrderFront:]` on macOS, both of which activate the
//! window and steal focus from whatever the user was working on. The HUD is
//! an always-on-top floating overlay and should never take focus — the user
//! reaches it with the mouse, not the keyboard.
//!
//! Implementation note: we initially tried `SetWindowPos(… SWP_NOACTIVATE …)`
//! and `ShowWindow(SW_SHOWNOACTIVATE)` on Windows to "show without
//! activating", but WebView2's IPC pipe does not initialize on a show that
//! skips activation — invoke() calls from the HUD silently fail to reach
//! Rust, breaking dismiss, focus, and settings navigation. The same caveat
//! applies to `orderFrontRegardless` on macOS. The working pattern is: show
//! normally (so the webview activates), then immediately restore focus to
//! whatever the user was on before. The flicker is imperceptible.

use tauri::WebviewWindow;

pub fn show_without_activation(window: &WebviewWindow) {
    #[cfg(target_os = "windows")]
    windows_impl::show(window);

    #[cfg(target_os = "macos")]
    macos_impl::show(window);

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = window.show();
    }
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use tauri::WebviewWindow;
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, SetForegroundWindow};

    pub fn show(window: &WebviewWindow) {
        // Remember whoever had the foreground right now, show the HUD normally
        // (so WebView2 activates and its IPC pipe initializes cleanly — any
        // SW_SHOWNOACTIVATE / SWP_NOACTIVATE path breaks invoke() on Windows),
        // then restore the previous window's foreground state. The user sees
        // the HUD appear, but keeps their focus.
        unsafe {
            let prev = GetForegroundWindow();
            let _ = window.show();
            if !prev.is_invalid() {
                let _ = SetForegroundWindow(prev);
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod macos_impl {
    use objc2::class;
    use objc2::msg_send;
    use objc2::runtime::AnyObject;
    use tauri::WebviewWindow;

    // NSApplicationActivationOptions.ActivateIgnoringOtherApps
    const NS_APPLICATION_ACTIVATE_IGNORING_OTHER_APPS: usize = 2;

    pub fn show(window: &WebviewWindow) {
        // Snapshot the frontmost app, show the HUD (which activates our app
        // and finalizes the webview's IPC pipe), then reactivate the previous
        // app. See the module-level comment for why non-activating show
        // breaks invoke().
        unsafe {
            let workspace: *mut AnyObject = msg_send![class!(NSWorkspace), sharedWorkspace];
            let prev_app: *mut AnyObject = if workspace.is_null() {
                std::ptr::null_mut()
            } else {
                msg_send![workspace, frontmostApplication]
            };

            let _ = window.show();

            if !prev_app.is_null() {
                let _: bool = msg_send![
                    prev_app,
                    activateWithOptions: NS_APPLICATION_ACTIVATE_IGNORING_OTHER_APPS
                ];
            }
        }
    }
}
