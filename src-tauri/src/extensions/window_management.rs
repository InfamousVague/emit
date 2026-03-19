//! Window management extension — snap windows to screen zones with dock/menu bar awareness.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::launcher::CommandEntry;
use crate::providers::CommandProvider;

// ── Types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub window_id: u32,
    pub app_name: String,
    pub title: String,
    pub bundle_id: Option<String>,
    pub bounds: WindowBounds,
    pub is_on_screen: bool,
    pub pid: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenInfo {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub visible_x: f64,
    pub visible_y: f64,
    pub visible_width: f64,
    pub visible_height: f64,
    pub dock_position: Option<String>,
    pub is_primary: bool,
    pub menu_bar_height: f64,
}

pub fn detect_dock_position(screen: &ScreenInfo) -> Option<String> {
    // Coordinates are in top-left origin (screen coordinates).
    // Dock steals space from one edge, shrinking the visible area.
    let threshold = 5.0;
    let left_diff = screen.visible_x - screen.x;
    let right_diff = (screen.x + screen.width) - (screen.visible_x + screen.visible_width);
    // Bottom gap = total height minus top inset (menu_bar) minus visible height
    let bottom_diff = screen.height - screen.menu_bar_height - screen.visible_height;

    if bottom_diff > threshold {
        Some("bottom".into())
    } else if left_diff > threshold {
        Some("left".into())
    } else if right_diff > threshold {
        Some("right".into())
    } else {
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SnapPosition {
    LeftHalf,
    RightHalf,
    TopHalf,
    BottomHalf,
    TopLeftQuarter,
    TopRightQuarter,
    BottomLeftQuarter,
    BottomRightQuarter,
    LeftThird,
    CenterThird,
    RightThird,
    LeftTwoThirds,
    RightTwoThirds,
    Maximize,
    Center,
}

/// Tracks the window that was focused before Emit appeared.
#[derive(Debug, Default)]
pub struct WmState {
    pub last_focused_pid: Option<i32>,
    pub last_focused_window_id: Option<u32>,
}

pub type SharedWmState = Arc<RwLock<WmState>>;

// ── Snap position calculation ────────────────────────────────────────────

pub fn snap_position_to_rect(position: &SnapPosition, screen: &ScreenInfo) -> WindowBounds {
    let x = screen.visible_x;
    let y = screen.visible_y;
    let w = screen.visible_width;
    let h = screen.visible_height;

    match position {
        SnapPosition::LeftHalf => WindowBounds { x, y, width: w / 2.0, height: h },
        SnapPosition::RightHalf => WindowBounds { x: x + w / 2.0, y, width: w / 2.0, height: h },
        SnapPosition::TopHalf => WindowBounds { x, y, width: w, height: h / 2.0 },
        SnapPosition::BottomHalf => WindowBounds { x, y: y + h / 2.0, width: w, height: h / 2.0 },

        SnapPosition::TopLeftQuarter => WindowBounds { x, y, width: w / 2.0, height: h / 2.0 },
        SnapPosition::TopRightQuarter => WindowBounds { x: x + w / 2.0, y, width: w / 2.0, height: h / 2.0 },
        SnapPosition::BottomLeftQuarter => WindowBounds { x, y: y + h / 2.0, width: w / 2.0, height: h / 2.0 },
        SnapPosition::BottomRightQuarter => WindowBounds { x: x + w / 2.0, y: y + h / 2.0, width: w / 2.0, height: h / 2.0 },

        SnapPosition::LeftThird => WindowBounds { x, y, width: w / 3.0, height: h },
        SnapPosition::CenterThird => WindowBounds { x: x + w / 3.0, y, width: w / 3.0, height: h },
        SnapPosition::RightThird => WindowBounds { x: x + 2.0 * w / 3.0, y, width: w / 3.0, height: h },

        SnapPosition::LeftTwoThirds => WindowBounds { x, y, width: 2.0 * w / 3.0, height: h },
        SnapPosition::RightTwoThirds => WindowBounds { x: x + w / 3.0, y, width: 2.0 * w / 3.0, height: h },

        SnapPosition::Maximize => WindowBounds { x, y, width: w, height: h },
        SnapPosition::Center => {
            let cw = w * 0.7;
            let ch = h * 0.7;
            WindowBounds {
                x: x + (w - cw) / 2.0,
                y: y + (h - ch) / 2.0,
                width: cw,
                height: ch,
            }
        }
    }
}


// ── macOS implementation ─────────────────────────────────────────────────

#[cfg(target_os = "macos")]
pub(crate) mod macos {
    use super::*;
    use core_foundation::base::{CFRelease, CFTypeRef, TCFType};
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;
    use core_graphics::display::{
        CGWindowListCopyWindowInfo, kCGNullWindowID, kCGWindowListExcludeDesktopElements,
        kCGWindowListOptionOnScreenOnly,
    };
    use std::ffi::c_void;
    use std::os::raw::c_int;

    type AXUIElementRef = CFTypeRef;
    type pid_t = i32;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
        fn AXIsProcessTrustedWithOptions(options: CFTypeRef) -> bool;
        fn AXUIElementCreateApplication(pid: pid_t) -> AXUIElementRef;
        fn AXUIElementCopyAttributeValue(
            element: AXUIElementRef,
            attribute: CFTypeRef,
            value: *mut CFTypeRef,
        ) -> c_int;
        fn AXUIElementSetAttributeValue(
            element: AXUIElementRef,
            attribute: CFTypeRef,
            value: CFTypeRef,
        ) -> c_int;
        fn AXUIElementPerformAction(element: AXUIElementRef, action: CFTypeRef) -> c_int;
        fn AXValueCreate(value_type: u32, value: *const c_void) -> CFTypeRef;
    }

    const K_AX_VALUE_TYPE_CG_POINT: u32 = 1;
    const K_AX_VALUE_TYPE_CG_SIZE: u32 = 2;

    static K_AX_TRUSTED_CHECK_OPTION_PROMPT: &str =
        "AXTrustedCheckOptionPrompt";

    pub fn check_accessibility() -> bool {
        unsafe { AXIsProcessTrusted() }
    }

    pub fn request_accessibility() -> bool {
        unsafe {
            let key = CFString::new(K_AX_TRUSTED_CHECK_OPTION_PROMPT);
            let value = CFBoolean::true_value();
            let dict = CFDictionary::from_CFType_pairs(&[(
                key.as_CFType(),
                value.as_CFType(),
            )]);
            AXIsProcessTrustedWithOptions(dict.as_concrete_TypeRef() as CFTypeRef)
        }
    }

    pub fn list_windows() -> Vec<WindowInfo> {
        let options = kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements;
        let window_list = unsafe { CGWindowListCopyWindowInfo(options, kCGNullWindowID) };
        if window_list.is_null() {
            return vec![];
        }

        let count = unsafe { core_foundation::array::CFArrayGetCount(window_list as _) };
        let mut windows = Vec::new();
        let own_pid = std::process::id() as i32;

        for i in 0..count {
            let dict = unsafe {
                core_foundation::array::CFArrayGetValueAtIndex(window_list as _, i) as CFTypeRef
            };
            if dict.is_null() {
                continue;
            }

            let layer = get_dict_number(dict, "kCGWindowLayer").unwrap_or(-1);
            if layer != 0 {
                continue;
            }

            let pid = get_dict_number(dict, "kCGWindowOwnerPID").unwrap_or(0) as i32;
            if pid == own_pid {
                continue;
            }

            let window_id = get_dict_number(dict, "kCGWindowNumber").unwrap_or(0) as u32;
            let app_name = get_dict_string(dict, "kCGWindowOwnerName").unwrap_or_default();
            let title = get_dict_string(dict, "kCGWindowName").unwrap_or_default();

            // Skip windows with no title (usually background windows)
            if title.is_empty() && app_name.is_empty() {
                continue;
            }

            let bounds = get_dict_bounds(dict);

            // Skip tiny windows (toolbars, status items, etc.)
            if bounds.width < 50.0 || bounds.height < 50.0 {
                continue;
            }

            windows.push(WindowInfo {
                window_id,
                app_name,
                title,
                bundle_id: None,
                bounds,
                is_on_screen: true,
                pid,
            });
        }

        unsafe { CFRelease(window_list as _) };
        windows
    }

    fn get_dict_number(dict: CFTypeRef, key: &str) -> Option<i64> {
        unsafe {
            let cf_key = CFString::new(key);
            let mut value: CFTypeRef = std::ptr::null();
            let result = core_foundation::dictionary::CFDictionaryGetValueIfPresent(
                dict as _,
                cf_key.as_concrete_TypeRef() as *const c_void,
                &mut value as *mut _ as *mut *const c_void,
            );
            if result == 0 || value.is_null() {
                return None;
            }
            let num = CFNumber::wrap_under_get_rule(value as _);
            num.to_i64()
        }
    }

    fn get_dict_string(dict: CFTypeRef, key: &str) -> Option<String> {
        unsafe {
            let cf_key = CFString::new(key);
            let mut value: CFTypeRef = std::ptr::null();
            let result = core_foundation::dictionary::CFDictionaryGetValueIfPresent(
                dict as _,
                cf_key.as_concrete_TypeRef() as *const c_void,
                &mut value as *mut _ as *mut *const c_void,
            );
            if result == 0 || value.is_null() {
                return None;
            }
            let cf_str = CFString::wrap_under_get_rule(value as _);
            Some(cf_str.to_string())
        }
    }

    fn get_dict_bounds(dict: CFTypeRef) -> WindowBounds {
        unsafe {
            let cf_key = CFString::new("kCGWindowBounds");
            let mut value: CFTypeRef = std::ptr::null();
            let result = core_foundation::dictionary::CFDictionaryGetValueIfPresent(
                dict as _,
                cf_key.as_concrete_TypeRef() as *const c_void,
                &mut value as *mut _ as *mut *const c_void,
            );
            if result == 0 || value.is_null() {
                return WindowBounds { x: 0.0, y: 0.0, width: 0.0, height: 0.0 };
            }

            // kCGWindowBounds is a CFDictionary with X, Y, Width, Height
            let bounds_dict = value;
            let x = get_dict_number(bounds_dict, "X").unwrap_or(0) as f64;
            let y = get_dict_number(bounds_dict, "Y").unwrap_or(0) as f64;
            let width = get_dict_number(bounds_dict, "Width").unwrap_or(0) as f64;
            let height = get_dict_number(bounds_dict, "Height").unwrap_or(0) as f64;
            WindowBounds { x, y, width, height }
        }
    }

    pub fn get_screen_info() -> ScreenInfo {
        use objc2::rc::Retained;
        use objc2::runtime::{AnyClass, AnyObject};
        use objc2_foundation::NSRect;

        // NSScreen.mainScreen and frame/visibleFrame are thread-safe for reading.
        // MainThreadMarker::new() returns None on background threads (where Tauri
        // commands run), so we use raw objc messaging instead.
        unsafe {
            let cls = AnyClass::get(c"NSScreen").expect("NSScreen class not found");
            let screen: Option<Retained<AnyObject>> = objc2::msg_send![cls, mainScreen];

            if let Some(screen) = screen {
                let frame: NSRect = objc2::msg_send![&screen, frame];
                let visible: NSRect = objc2::msg_send![&screen, visibleFrame];

                let screen_height = frame.size.height;
                let is_primary = frame.origin.x == 0.0 && frame.origin.y == 0.0;
                let menu_bar_height = if is_primary {
                    screen_height - (visible.origin.y + visible.size.height)
                } else {
                    0.0
                };

                // Convert from Cocoa bottom-left to screen top-left coordinates
                let visible_y_top = screen_height - (visible.origin.y + visible.size.height);

                let mut info = ScreenInfo {
                    x: frame.origin.x,
                    y: 0.0,
                    width: frame.size.width,
                    height: screen_height,
                    visible_x: visible.origin.x,
                    visible_y: visible_y_top,
                    visible_width: visible.size.width,
                    visible_height: visible.size.height,
                    dock_position: None,
                    is_primary,
                    menu_bar_height,
                };
                info.dock_position = detect_dock_position(&info);
                return info;
            }
        }
        // Fallback (shouldn't happen on macOS)
        ScreenInfo {
            x: 0.0, y: 0.0, width: 1440.0, height: 900.0,
            visible_x: 0.0, visible_y: 25.0, visible_width: 1440.0, visible_height: 875.0,
            dock_position: None, is_primary: true, menu_bar_height: 25.0,
        }
    }

    pub fn snap_window(pid: i32, bounds: &WindowBounds) -> Result<(), String> {
        if !check_accessibility() {
            return Err("Accessibility permission not granted".into());
        }

        unsafe {
            let app_ref = AXUIElementCreateApplication(pid);
            if app_ref.is_null() {
                return Err("Failed to create AXUIElement for application".into());
            }

            // Get focused window of the app
            let ax_focused = CFString::new("AXFocusedWindow");
            let mut window_ref: CFTypeRef = std::ptr::null();
            let result = AXUIElementCopyAttributeValue(
                app_ref,
                ax_focused.as_concrete_TypeRef() as CFTypeRef,
                &mut window_ref,
            );
            if result != 0 || window_ref.is_null() {
                CFRelease(app_ref);
                return Err("Failed to get focused window".into());
            }

            // Set position
            let point = core_graphics_types::geometry::CGPoint::new(bounds.x, bounds.y);
            let ax_position = CFString::new("AXPosition");
            let position_value =
                AXValueCreate(K_AX_VALUE_TYPE_CG_POINT, &point as *const _ as *const c_void);
            if !position_value.is_null() {
                AXUIElementSetAttributeValue(
                    window_ref,
                    ax_position.as_concrete_TypeRef() as CFTypeRef,
                    position_value,
                );
                CFRelease(position_value);
            }

            // Set size
            let size = core_graphics_types::geometry::CGSize::new(bounds.width, bounds.height);
            let ax_size = CFString::new("AXSize");
            let size_value =
                AXValueCreate(K_AX_VALUE_TYPE_CG_SIZE, &size as *const _ as *const c_void);
            if !size_value.is_null() {
                AXUIElementSetAttributeValue(
                    window_ref,
                    ax_size.as_concrete_TypeRef() as CFTypeRef,
                    size_value,
                );
                CFRelease(size_value);
            }

            CFRelease(window_ref);
            CFRelease(app_ref);
        }

        Ok(())
    }

    pub fn focus_window(pid: i32) -> Result<(), String> {
        unsafe {
            let app_ref = AXUIElementCreateApplication(pid);
            if app_ref.is_null() {
                return Err("Failed to create AXUIElement for application".into());
            }

            // Raise the window
            let ax_focused = CFString::new("AXFocusedWindow");
            let mut window_ref: CFTypeRef = std::ptr::null();
            let result = AXUIElementCopyAttributeValue(
                app_ref,
                ax_focused.as_concrete_TypeRef() as CFTypeRef,
                &mut window_ref,
            );
            if result == 0 && !window_ref.is_null() {
                let ax_raise = CFString::new("AXRaise");
                AXUIElementPerformAction(
                    window_ref,
                    ax_raise.as_concrete_TypeRef() as CFTypeRef,
                );
                CFRelease(window_ref);
            }

            CFRelease(app_ref);
        }

        // Activate the app via NSRunningApplication
        let workspace = objc2_app_kit::NSWorkspace::sharedWorkspace();
        let running_apps = workspace.runningApplications();
        for app in running_apps.iter() {
            let app_pid = app.processIdentifier();
            if app_pid == pid {
                unsafe {
                    // Use raw objc2 msg_send to call activate
                    let _: bool = objc2::msg_send![&*app, activateWithOptions: 2_usize];
                }
                break;
            }
        }

        Ok(())
    }

    pub fn capture_last_focused() -> Option<(i32, u32)> {
        let workspace = objc2_app_kit::NSWorkspace::sharedWorkspace();
        let front_app = workspace.frontmostApplication();
        let front_app = front_app?;
        let pid = front_app.processIdentifier();

        if pid <= 0 || pid == std::process::id() as i32 {
            return None;
        }

        // Find matching window from CG window list
        let options = kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements;
        let window_list = unsafe { CGWindowListCopyWindowInfo(options, kCGNullWindowID) };
        if window_list.is_null() {
            return None;
        }

        let count = unsafe { core_foundation::array::CFArrayGetCount(window_list as _) };
        let mut found_window_id = None;

        for i in 0..count {
            let dict = unsafe {
                core_foundation::array::CFArrayGetValueAtIndex(window_list as _, i) as CFTypeRef
            };
            if dict.is_null() {
                continue;
            }
            let layer = get_dict_number(dict, "kCGWindowLayer").unwrap_or(-1);
            if layer != 0 {
                continue;
            }
            let w_pid = get_dict_number(dict, "kCGWindowOwnerPID").unwrap_or(0) as i32;
            if w_pid == pid {
                let wid = get_dict_number(dict, "kCGWindowNumber").unwrap_or(0) as u32;
                if wid > 0 {
                    found_window_id = Some(wid);
                    break;
                }
            }
        }

        unsafe { CFRelease(window_list as _) };
        found_window_id.map(|wid| (pid, wid))
    }

}

// ── CommandProvider ──────────────────────────────────────────────────────

pub struct WindowManagementProvider;

impl WindowManagementProvider {
    pub fn new() -> Self {
        Self
    }
}

struct SnapCommand {
    id: &'static str,
    name: &'static str,
    description: &'static str,
}

const SNAP_COMMANDS: &[SnapCommand] = &[
    SnapCommand { id: "wm.snap.left_half", name: "Snap Left Half", description: "Snap focused window to left half of screen" },
    SnapCommand { id: "wm.snap.right_half", name: "Snap Right Half", description: "Snap focused window to right half of screen" },
    SnapCommand { id: "wm.snap.top_half", name: "Snap Top Half", description: "Snap focused window to top half of screen" },
    SnapCommand { id: "wm.snap.bottom_half", name: "Snap Bottom Half", description: "Snap focused window to bottom half of screen" },
    SnapCommand { id: "wm.snap.top_left", name: "Snap Top Left Quarter", description: "Snap focused window to top-left quarter" },
    SnapCommand { id: "wm.snap.top_right", name: "Snap Top Right Quarter", description: "Snap focused window to top-right quarter" },
    SnapCommand { id: "wm.snap.bottom_left", name: "Snap Bottom Left Quarter", description: "Snap focused window to bottom-left quarter" },
    SnapCommand { id: "wm.snap.bottom_right", name: "Snap Bottom Right Quarter", description: "Snap focused window to bottom-right quarter" },
    SnapCommand { id: "wm.snap.left_third", name: "Snap Left Third", description: "Snap focused window to left third of screen" },
    SnapCommand { id: "wm.snap.center_third", name: "Snap Center Third", description: "Snap focused window to center third of screen" },
    SnapCommand { id: "wm.snap.right_third", name: "Snap Right Third", description: "Snap focused window to right third of screen" },
    SnapCommand { id: "wm.snap.left_two_thirds", name: "Snap Left Two Thirds", description: "Snap focused window to left two-thirds of screen" },
    SnapCommand { id: "wm.snap.right_two_thirds", name: "Snap Right Two Thirds", description: "Snap focused window to right two-thirds of screen" },
    SnapCommand { id: "wm.snap.maximize", name: "Maximize Window", description: "Maximize focused window to fill screen" },
    SnapCommand { id: "wm.snap.center", name: "Center Window", description: "Center focused window at 70% of screen" },
];

pub fn snap_id_to_position(id: &str) -> Option<SnapPosition> {
    match id {
        "wm.snap.left_half" => Some(SnapPosition::LeftHalf),
        "wm.snap.right_half" => Some(SnapPosition::RightHalf),
        "wm.snap.top_half" => Some(SnapPosition::TopHalf),
        "wm.snap.bottom_half" => Some(SnapPosition::BottomHalf),
        "wm.snap.top_left" => Some(SnapPosition::TopLeftQuarter),
        "wm.snap.top_right" => Some(SnapPosition::TopRightQuarter),
        "wm.snap.bottom_left" => Some(SnapPosition::BottomLeftQuarter),
        "wm.snap.bottom_right" => Some(SnapPosition::BottomRightQuarter),
        "wm.snap.left_third" => Some(SnapPosition::LeftThird),
        "wm.snap.center_third" => Some(SnapPosition::CenterThird),
        "wm.snap.right_third" => Some(SnapPosition::RightThird),
        "wm.snap.left_two_thirds" => Some(SnapPosition::LeftTwoThirds),
        "wm.snap.right_two_thirds" => Some(SnapPosition::RightTwoThirds),
        "wm.snap.maximize" => Some(SnapPosition::Maximize),
        "wm.snap.center" => Some(SnapPosition::Center),
        _ => None,
    }
}

#[async_trait]
impl CommandProvider for WindowManagementProvider {
    fn name(&self) -> &str {
        "WindowManagement"
    }

    async fn commands(&self) -> Vec<CommandEntry> {
        let mut cmds = vec![CommandEntry {
            id: "wm.open".into(),
            name: "Window Manager".into(),
            description: "Manage window positions and layouts".into(),
            category: "Extensions".into(),
            icon: None,
            match_indices: vec![],
            score: 0,
        }];

        for sc in SNAP_COMMANDS {
            cmds.push(CommandEntry {
                id: sc.id.into(),
                name: sc.name.into(),
                description: sc.description.into(),
                category: "Window Management".into(),
                icon: None,
                match_indices: vec![],
                score: 0,
            });
        }

        cmds
    }

    fn execute(&self, id: &str) -> Option<Result<String, String>> {
        if id == "wm.open" {
            Some(Ok("view:window-manager".into()))
        } else if id.starts_with("wm.snap.") {
            Some(Ok(format!("action:{}", id)))
        } else {
            None
        }
    }
}

// ── Tauri commands ───────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
#[tauri::command]
pub fn wm_check_accessibility() -> bool {
    macos::check_accessibility()
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn wm_check_accessibility() -> bool {
    false
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub fn wm_request_accessibility() -> bool {
    macos::request_accessibility()
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn wm_request_accessibility() -> bool {
    false
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub fn wm_list_windows() -> Result<Vec<WindowInfo>, String> {
    Ok(macos::list_windows())
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn wm_list_windows() -> Result<Vec<WindowInfo>, String> {
    Err("Window management is only supported on macOS".into())
}

#[tauri::command]
pub fn wm_get_app_icon(app_name: String) -> Result<Option<String>, String> {
    Ok(crate::icons::get_cached_icon(&app_name))
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub fn wm_get_screen_info() -> Result<ScreenInfo, String> {
    Ok(macos::get_screen_info())
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn wm_get_screen_info() -> Result<ScreenInfo, String> {
    Err("Window management is only supported on macOS".into())
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn wm_snap_window(
    window_id: u32,
    position: SnapPosition,
) -> Result<(), String> {
    let screen = macos::get_screen_info();
    let rect = snap_position_to_rect(&position, &screen);

    // Find the PID for this window
    let windows = macos::list_windows();
    let win = windows.iter().find(|w| w.window_id == window_id)
        .ok_or_else(|| "Window not found".to_string())?;
    let pid = win.pid;

    macos::snap_window(pid, &rect)?;
    macos::focus_window(pid)?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub async fn wm_snap_window(
    _window_id: u32,
    _position: SnapPosition,
) -> Result<(), String> {
    Err("Window management is only supported on macOS".into())
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn wm_snap_focused(
    position: SnapPosition,
    state: tauri::State<'_, SharedWmState>,
) -> Result<(), String> {
    let wm = state.read().await;
    let pid = wm.last_focused_pid
        .ok_or_else(|| "No previously focused window".to_string())?;
    drop(wm);

    let screen = macos::get_screen_info();
    let rect = snap_position_to_rect(&position, &screen);

    macos::snap_window(pid, &rect)?;
    macos::focus_window(pid)?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub async fn wm_snap_focused(
    _position: SnapPosition,
    _state: tauri::State<'_, SharedWmState>,
) -> Result<(), String> {
    Err("Window management is only supported on macOS".into())
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_screen() -> ScreenInfo {
        ScreenInfo {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
            visible_x: 0.0,
            visible_y: 25.0,
            visible_width: 1920.0,
            visible_height: 1055.0,
            dock_position: None,
            is_primary: true,
            menu_bar_height: 25.0,
        }
    }

    #[test]
    fn test_left_half() {
        let r = snap_position_to_rect(&SnapPosition::LeftHalf, &test_screen());
        assert_eq!(r.x, 0.0);
        assert_eq!(r.y, 25.0);
        assert_eq!(r.width, 960.0);
        assert_eq!(r.height, 1055.0);
    }

    #[test]
    fn test_right_half() {
        let r = snap_position_to_rect(&SnapPosition::RightHalf, &test_screen());
        assert_eq!(r.x, 960.0);
        assert_eq!(r.y, 25.0);
        assert_eq!(r.width, 960.0);
        assert_eq!(r.height, 1055.0);
    }

    #[test]
    fn test_top_half() {
        let r = snap_position_to_rect(&SnapPosition::TopHalf, &test_screen());
        assert_eq!(r.x, 0.0);
        assert_eq!(r.y, 25.0);
        assert_eq!(r.width, 1920.0);
        assert_eq!(r.height, 527.5);
    }

    #[test]
    fn test_bottom_half() {
        let r = snap_position_to_rect(&SnapPosition::BottomHalf, &test_screen());
        assert_eq!(r.x, 0.0);
        assert_eq!(r.y, 552.5);
        assert_eq!(r.width, 1920.0);
        assert_eq!(r.height, 527.5);
    }

    #[test]
    fn test_quarters() {
        let s = test_screen();
        let tl = snap_position_to_rect(&SnapPosition::TopLeftQuarter, &s);
        assert_eq!(tl.x, 0.0);
        assert_eq!(tl.width, 960.0);
        assert_eq!(tl.height, 527.5);

        let br = snap_position_to_rect(&SnapPosition::BottomRightQuarter, &s);
        assert_eq!(br.x, 960.0);
        assert_eq!(br.y, 552.5);
    }

    #[test]
    fn test_thirds() {
        let s = test_screen();
        let l = snap_position_to_rect(&SnapPosition::LeftThird, &s);
        assert_eq!(l.width, 640.0);

        let c = snap_position_to_rect(&SnapPosition::CenterThird, &s);
        assert_eq!(c.x, 640.0);
        assert_eq!(c.width, 640.0);

        let r = snap_position_to_rect(&SnapPosition::RightThird, &s);
        assert_eq!(r.x, 1280.0);
        assert_eq!(r.width, 640.0);
    }

    #[test]
    fn test_two_thirds() {
        let s = test_screen();
        let lt = snap_position_to_rect(&SnapPosition::LeftTwoThirds, &s);
        assert_eq!(lt.width, 1280.0);

        let rt = snap_position_to_rect(&SnapPosition::RightTwoThirds, &s);
        assert_eq!(rt.x, 640.0);
        assert_eq!(rt.width, 1280.0);
    }

    #[test]
    fn test_maximize() {
        let r = snap_position_to_rect(&SnapPosition::Maximize, &test_screen());
        assert_eq!(r.x, 0.0);
        assert_eq!(r.y, 25.0);
        assert_eq!(r.width, 1920.0);
        assert_eq!(r.height, 1055.0);
    }

    #[test]
    fn test_center() {
        let r = snap_position_to_rect(&SnapPosition::Center, &test_screen());
        let expected_w = 1920.0 * 0.7;
        let expected_h = 1055.0 * 0.7;
        assert!((r.width - expected_w).abs() < 0.01);
        assert!((r.height - expected_h).abs() < 0.01);
        // Should be centered
        assert!((r.x - (1920.0 - expected_w) / 2.0).abs() < 0.01);
    }

    #[test]
    fn test_snap_id_to_position() {
        assert!(matches!(snap_id_to_position("wm.snap.left_half"), Some(SnapPosition::LeftHalf)));
        assert!(matches!(snap_id_to_position("wm.snap.maximize"), Some(SnapPosition::Maximize)));
        assert!(snap_id_to_position("invalid").is_none());
    }

    #[test]
    fn test_detect_dock_bottom() {
        // Top-left coords: menu=25, dock=70 at bottom → visible_height = 1080 - 25 - 70 = 985
        let screen = ScreenInfo {
            x: 0.0, y: 0.0, width: 1920.0, height: 1080.0,
            visible_x: 0.0, visible_y: 25.0, visible_width: 1920.0, visible_height: 985.0,
            dock_position: None, is_primary: true, menu_bar_height: 25.0,
        };
        assert_eq!(detect_dock_position(&screen), Some("bottom".into()));
    }

    #[test]
    fn test_detect_dock_left() {
        // Dock on left: visible_x offset by 70, visible_width shrunk
        let screen = ScreenInfo {
            x: 0.0, y: 0.0, width: 1920.0, height: 1080.0,
            visible_x: 70.0, visible_y: 25.0, visible_width: 1850.0, visible_height: 1055.0,
            dock_position: None, is_primary: true, menu_bar_height: 25.0,
        };
        assert_eq!(detect_dock_position(&screen), Some("left".into()));
    }

    #[test]
    fn test_detect_dock_right() {
        // Dock on right: visible_width shrunk, visible_x stays 0
        let screen = ScreenInfo {
            x: 0.0, y: 0.0, width: 1920.0, height: 1080.0,
            visible_x: 0.0, visible_y: 25.0, visible_width: 1850.0, visible_height: 1055.0,
            dock_position: None, is_primary: true, menu_bar_height: 25.0,
        };
        assert_eq!(detect_dock_position(&screen), Some("right".into()));
    }

    #[test]
    fn test_detect_dock_hidden() {
        // No dock: visible area = full screen minus menu bar
        let screen = ScreenInfo {
            x: 0.0, y: 0.0, width: 1920.0, height: 1080.0,
            visible_x: 0.0, visible_y: 25.0, visible_width: 1920.0, visible_height: 1055.0,
            dock_position: None, is_primary: true, menu_bar_height: 25.0,
        };
        assert_eq!(detect_dock_position(&screen), None);
    }

}
