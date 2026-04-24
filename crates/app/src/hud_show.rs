//! Cross-platform HUD show helper that does not steal focus.
//!
//! `tauri::WebviewWindow::show()` maps to `ShowWindow(SW_SHOW)` on Windows and
//! `[NSWindow makeKeyAndOrderFront:]` on macOS, both of which activate the
//! window and steal focus from whatever the user was working on. The HUD is an
//! always-on-top floating overlay and should never take focus — the user
//! reaches it with the mouse, not the keyboard.

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
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
    };

    pub fn show(window: &WebviewWindow) {
        match window.hwnd() {
            Ok(hwnd) => unsafe {
                if let Err(e) = SetWindowPos(
                    hwnd,
                    Some(HWND_TOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOACTIVATE | SWP_SHOWWINDOW | SWP_NOMOVE | SWP_NOSIZE,
                ) {
                    tracing::warn!(error = %e, "SetWindowPos failed; falling back to show()");
                    let _ = window.show();
                }
            },
            Err(e) => {
                tracing::warn!(error = %e, "could not retrieve HWND; falling back to show()");
                let _ = window.show();
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod macos_impl {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;
    use tauri::WebviewWindow;

    pub fn show(window: &WebviewWindow) {
        match window.ns_window() {
            Ok(ptr) if !ptr.is_null() => unsafe {
                let ns_window: *mut AnyObject = ptr as *mut AnyObject;
                let _: () = msg_send![ns_window, orderFrontRegardless];
            },
            Ok(_) => {
                tracing::warn!("ns_window pointer was null; falling back to show()");
                let _ = window.show();
            }
            Err(e) => {
                tracing::warn!(error = %e, "could not retrieve NSWindow; falling back to show()");
                let _ = window.show();
            }
        }
    }
}
