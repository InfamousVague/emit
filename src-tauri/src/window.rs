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
            fade_hide(&window);
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

/// Hide the main window with a fade-out animation.
pub fn hide(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        fade_hide(&window);
    }
}

/// Show, center, and focus a window with a fade-in animation.
fn show(window: &tauri::WebviewWindow) {
    // Start transparent, then fade in
    #[cfg(target_os = "macos")]
    {
        let w = window.clone();
        set_alpha(&w, 0.0);
        let _ = window.show();
        let _ = window.center();
        let _ = window.set_focus();
        std::thread::spawn(move || {
            animate_alpha(&w, 0.0, 1.0, 150);
        });
        return;
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = window.show();
        let _ = window.center();
        let _ = window.set_focus();
    }
}

/// Fade out then hide the window.
fn fade_hide(window: &tauri::WebviewWindow) {
    #[cfg(target_os = "macos")]
    {
        let w = window.clone();
        std::thread::spawn(move || {
            animate_alpha(&w, 1.0, 0.0, 120);
            let _ = w.hide();
            // Reset alpha for next show
            set_alpha(&w, 1.0);
        });
        return;
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = window.hide();
    }
}

/// Set NSWindow alpha value directly.
#[cfg(target_os = "macos")]
fn set_alpha(window: &tauri::WebviewWindow, alpha: f64) {
    let _ = window.with_webview(move |webview| {
        let ns_window = webview.ns_window();
        if !ns_window.is_null() {
            unsafe {
                let ns_window = ns_window as *mut objc2::runtime::AnyObject;
                let _: () = objc2::msg_send![&*ns_window, setAlphaValue: alpha];
            }
        }
    });
}

/// Animate NSWindow alpha from `from` to `to` over `duration_ms` milliseconds.
#[cfg(target_os = "macos")]
fn animate_alpha(window: &tauri::WebviewWindow, from: f64, to: f64, duration_ms: u64) {
    let steps = 12u64;
    let step_duration = std::time::Duration::from_millis(duration_ms / steps);

    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        // Ease out cubic
        let eased = 1.0 - (1.0 - t).powi(3);
        let alpha = from + (to - from) * eased;
        set_alpha(window, alpha);
        if i < steps {
            std::thread::sleep(step_duration);
        }
    }
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
                let level = NSFloatingWindowLevel;
                unsafe {
                    let ns_window = ns_window as *mut objc2::runtime::AnyObject;
                    let _: () = objc2::msg_send![&*ns_window, setLevel: level];
                }
            }
        });
    }
}
