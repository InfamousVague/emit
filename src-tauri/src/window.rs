//! Window management utilities.
//!
//! Centralizes all show/hide/toggle logic so it isn't duplicated across
//! the global shortcut handler, tray icon handler, and IPC commands.

use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager};

/// Guard to prevent the focus-loss handler from hiding the window
/// during the show sequence (activation policy switch triggers a
/// spurious Focused(false) event).
static SHOWING: AtomicBool = AtomicBool::new(false);

/// Toggle the main window's visibility. If hidden, shows + centers + focuses.
/// Before showing, captures the currently focused window for window management.
pub fn toggle(app: &AppHandle) {

    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            fade_hide(&window);
            // Restore Accessory policy so dock icon disappears
            #[cfg(target_os = "macos")]
            restore_accessory_policy();
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
    if SHOWING.load(Ordering::SeqCst) {
        return;
    }
    if let Some(window) = app.get_webview_window("main") {
        fade_hide(&window);
    }
    #[cfg(target_os = "macos")]
    restore_accessory_policy();
}

/// Show, center, and focus a window with a fade-in animation.
fn show(window: &tauri::WebviewWindow) {
    SHOWING.store(true, Ordering::SeqCst);

    #[cfg(target_os = "macos")]
    {
        // Activate the application so it can come to the foreground.
        // Required when running as Accessory (no dock icon) — without this
        // the window is "visible" according to Tauri but not rendered on screen.
        activate_app();

        let _ = window.show();
        center_on_active_display(window);
        let _ = window.set_focus();
        // Allow hide after a short delay to avoid spurious focus-loss events
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(500));
            SHOWING.store(false, Ordering::SeqCst);
        });
        return;
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = window.show();
        let _ = window.center();
        let _ = window.set_focus();
        SHOWING.store(false, Ordering::SeqCst);
    }
}

/// Hide the window.
fn fade_hide(window: &tauri::WebviewWindow) {
    let _ = window.hide();
}

/// Temporarily switch to Regular activation policy so the window can appear,
/// then activate the application.
#[cfg(target_os = "macos")]
fn activate_app() {
    if let Some(mtm) = objc2::MainThreadMarker::new() {
        use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
        let ns_app = NSApplication::sharedApplication(mtm);
        ns_app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
        unsafe {
            let _: () = objc2::msg_send![&ns_app, activateIgnoringOtherApps: true];
        }
    }
}

/// Restore Accessory activation policy so the dock icon disappears.
#[cfg(target_os = "macos")]
fn restore_accessory_policy() {
    if let Some(mtm) = objc2::MainThreadMarker::new() {
        use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
        let ns_app = NSApplication::sharedApplication(mtm);
        ns_app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
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

/// Center the window on the display that currently has the cursor.
#[cfg(target_os = "macos")]
fn center_on_active_display(window: &tauri::WebviewWindow) {
    let _ = window.with_webview(|webview| {
        let ns_window = webview.ns_window();
        if ns_window.is_null() {
            return;
        }
        unsafe {
            use objc2_app_kit::NSScreen;
            use objc2_foundation::NSPoint;

            let mtm = objc2::MainThreadMarker::new()
                .expect("with_webview runs on main thread");

            // Get the current mouse location
            let mouse_location: NSPoint =
                objc2::msg_send![objc2::class!(NSEvent), mouseLocation];

            // Find which screen contains the mouse
            let screens = NSScreen::screens(mtm);
            let mut target_frame = None;
            for screen in screens.iter() {
                let frame = screen.frame();
                if mouse_location.x >= frame.origin.x
                    && mouse_location.x <= frame.origin.x + frame.size.width
                    && mouse_location.y >= frame.origin.y
                    && mouse_location.y <= frame.origin.y + frame.size.height
                {
                    target_frame = Some(frame);
                    break;
                }
            }

            // Fall back to main screen
            let screen_frame = target_frame.unwrap_or_else(|| {
                NSScreen::mainScreen(mtm)
                    .map(|s| s.frame())
                    .unwrap_or(objc2_foundation::NSRect::new(
                        NSPoint::new(0.0, 0.0),
                        objc2_foundation::NSSize::new(1920.0, 1080.0),
                    ))
            });

            let ns_window = ns_window as *mut objc2::runtime::AnyObject;
            let win_frame: objc2_foundation::NSRect = objc2::msg_send![&*ns_window, frame];

            let x = screen_frame.origin.x
                + (screen_frame.size.width - win_frame.size.width) / 2.0;
            let y = screen_frame.origin.y
                + (screen_frame.size.height - win_frame.size.height) / 2.0
                + (screen_frame.size.height * 0.1); // Slightly above center

            let origin = NSPoint::new(x, y);
            let _: () = objc2::msg_send![&*ns_window, setFrameOrigin: origin];
        }
    });
}

/// Show the main window (used on first launch).
pub fn show_main(app: &AppHandle) {

    if let Some(window) = app.get_webview_window("main") {
        show(&window);
    }
}

/// Make the window background transparent manually via NSWindow APIs.
/// This replaces Tauri's `transparent: true` config which causes rendering
/// issues on recent macOS versions.
#[cfg(target_os = "macos")]
pub fn set_transparent_background(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.with_webview(|webview| {
            let ns_window = webview.ns_window();
            if ns_window.is_null() {
                return;
            }
            unsafe {
                let ns_window = ns_window as *mut objc2::runtime::AnyObject;
                // Make NSWindow background transparent
                let _: () = objc2::msg_send![&*ns_window, setOpaque: false];
                let clear_color: *mut objc2::runtime::AnyObject =
                    objc2::msg_send![objc2::class!(NSColor), clearColor];
                let _: () = objc2::msg_send![&*ns_window, setBackgroundColor: clear_color];
            }
            // Also make the WKWebView background transparent
            let wk_webview = webview.inner() as *mut objc2::runtime::AnyObject;
            unsafe {
                let no = objc2_foundation::NSNumber::new_bool(false);
                let key = objc2_foundation::NSString::from_str("drawsBackground");
                let _: () = objc2::msg_send![&*wk_webview, setValue: &*no forKey: &*key];
            }
        });
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
