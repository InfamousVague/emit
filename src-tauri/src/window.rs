//! Window management utilities.
//!
//! Centralizes all show/hide/toggle logic so it isn't duplicated across
//! the global shortcut handler, tray icon handler, and IPC commands.

use tauri::{AppHandle, Manager};

/// Toggle the main window's visibility. If hidden, shows + centers + focuses.
/// Before showing, captures the currently focused window for window management.
pub fn toggle(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            // Capture the frontmost window before Emit takes focus
            #[cfg(target_os = "macos")]
            {
                use crate::extensions::window_management::SharedWmState;
                if let Some(wm_state) = app.try_state::<SharedWmState>() {
                    if let Some((pid, wid)) =
                        crate::extensions::window_management::macos::capture_last_focused()
                    {
                        let wm_state = wm_state.inner().clone();
                        tauri::async_runtime::spawn(async move {
                            let mut state = wm_state.write().await;
                            state.last_focused_pid = Some(pid);
                            state.last_focused_window_id = Some(wid);
                        });
                    }
                }
            }
            show(&window);
        }
    }
}

/// Hide the main window.
pub fn hide(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

/// Show, center, and focus a window.
fn show(window: &tauri::WebviewWindow) {
    let _ = window.show();
    let _ = window.center();
    let _ = window.set_focus();
}

/// Show the main window (used on first launch).
pub fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        show(&window);
    }
}

/// Set the NSWindow level to floating panel so backdrop-filter stays active
/// even when the window loses key/focus status.
#[cfg(target_os = "macos")]
pub fn set_floating_panel(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.with_webview(|webview| {
            use objc2_app_kit::NSFloatingWindowLevel;
            let ns_window = webview.ns_window();
            if !ns_window.is_null() {
                // NSWindow.setLevel expects NSWindowLevel (i64)
                let level = NSFloatingWindowLevel;
                unsafe {
                    let ns_window = ns_window as *mut objc2::runtime::AnyObject;
                    let _: () = objc2::msg_send![&*ns_window, setLevel: level];
                }
            }
        });
    }
}
