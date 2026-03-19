//! Window management utilities.
//!
//! Centralizes all show/hide/toggle logic so it isn't duplicated across
//! the global shortcut handler, tray icon handler, and IPC commands.

use tauri::{AppHandle, Manager};

/// Toggle the main window's visibility. If hidden, shows + centers + focuses.
pub fn toggle(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
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
