use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::launcher::CommandEntry;
use crate::providers::CommandProvider;

pub struct RulerProvider;

impl RulerProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl CommandProvider for RulerProvider {
    fn name(&self) -> &str {
        "Ruler"
    }

    async fn commands(&self) -> Vec<CommandEntry> {
        vec![CommandEntry {
            id: "ruler.open".into(),
            name: "Pixel Ruler".into(),
            description: "Measure distances on screen with a pixel ruler overlay".into(),
            category: "Extensions".into(),
            icon: None,
            match_indices: vec![],
            score: 0,
        }]
    }

    fn execute(&self, id: &str) -> Option<Result<String, String>> {
        if id == "ruler.open" {
            Some(Ok("action:ruler".into()))
        } else {
            None
        }
    }
}

// --- Types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgePoint {
    pub x: f64,
    pub y: f64,
    pub direction: String,
}

// --- Tauri Commands ---

/// Open the ruler overlay window spanning all screens.
#[tauri::command]
pub fn ruler_open(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;

    // If overlay already exists, focus it
    if let Some(win) = app.get_webview_window("ruler-overlay") {
        win.set_focus().ok();
        return Ok(());
    }

    let screens = get_all_screens();
    if screens.is_empty() {
        return Err("No screens found".into());
    }

    // Compute bounding rect across all screens
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    for s in &screens {
        min_x = min_x.min(s.x);
        min_y = min_y.min(s.y);
        max_x = max_x.max(s.x + s.width);
        max_y = max_y.max(s.y + s.height);
    }
    let total_w = max_x - min_x;
    let total_h = max_y - min_y;

    // Hide main launcher
    crate::window::hide(&app);

    // Create overlay window
    let url = tauri::WebviewUrl::App("ruler.html".into());
    let win = tauri::WebviewWindowBuilder::new(&app, "ruler-overlay", url)
        .title("Pixel Ruler")
        .position(min_x, min_y)
        .inner_size(total_w, total_h)
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(true)
        .build()
        .map_err(|e| format!("Failed to create overlay window: {e}"))?;

    // Set NSWindow level to overlay (above floating panels)
    #[cfg(target_os = "macos")]
    {
        let _ = win.with_webview(move |webview| {
            let ns_window = webview.ns_window();
            if !ns_window.is_null() {
                unsafe {
                    let ns_window = ns_window as *mut objc2::runtime::AnyObject;
                    // kCGOverlayWindowLevel = 102
                    let _: () = objc2::msg_send![&*ns_window, setLevel: 102i64];
                }
            }
        });
    }

    // Close overlay when it loses focus, and re-show main
    let handle = app.clone();
    win.on_window_event(move |event| {
        if let tauri::WindowEvent::Destroyed = event {
            crate::window::show_main(&handle);
        }
    });

    Ok(())
}

/// Close the ruler overlay window.
#[tauri::command]
pub fn ruler_close(app: tauri::AppHandle) {
    use tauri::Manager;
    if let Some(win) = app.get_webview_window("ruler-overlay") {
        win.close().ok();
    }
}

/// Get info for all connected screens.
#[tauri::command]
pub fn ruler_get_all_screens() -> Vec<crate::extensions::window_management::ScreenInfo> {
    get_all_screens()
}

fn get_all_screens() -> Vec<crate::extensions::window_management::ScreenInfo> {
    #[cfg(target_os = "macos")]
    {
        use objc2::rc::Retained;
        use objc2::runtime::{AnyClass, AnyObject};
        use objc2_foundation::NSRect;

        let mut screens = Vec::new();

        unsafe {
            let cls = AnyClass::get(c"NSScreen").expect("NSScreen class not found");
            let ns_screens: Option<Retained<AnyObject>> = objc2::msg_send![cls, screens];

            if let Some(ns_screens) = ns_screens {
                let count: usize = objc2::msg_send![&ns_screens, count];

                // Get primary screen height for coordinate conversion
                let primary: Option<Retained<AnyObject>> = objc2::msg_send![cls, mainScreen];
                let primary_height = if let Some(ref p) = primary {
                    let frame: NSRect = objc2::msg_send![p, frame];
                    frame.size.height
                } else {
                    900.0
                };

                for i in 0..count {
                    let screen: Retained<AnyObject> =
                        objc2::msg_send![&ns_screens, objectAtIndex: i];
                    let frame: NSRect = objc2::msg_send![&screen, frame];
                    let visible: NSRect = objc2::msg_send![&screen, visibleFrame];

                    let is_primary = frame.origin.x == 0.0 && frame.origin.y == 0.0;

                    // Convert from Cocoa bottom-left to top-left coordinates
                    let y_top = primary_height - (frame.origin.y + frame.size.height);
                    let visible_y_top =
                        primary_height - (visible.origin.y + visible.size.height);

                    let menu_bar_height = if is_primary {
                        frame.size.height - (visible.origin.y + visible.size.height)
                    } else {
                        0.0
                    };

                    let mut info = crate::extensions::window_management::ScreenInfo {
                        x: frame.origin.x,
                        y: y_top,
                        width: frame.size.width,
                        height: frame.size.height,
                        visible_x: visible.origin.x,
                        visible_y: visible_y_top,
                        visible_width: visible.size.width,
                        visible_height: visible.size.height,
                        dock_position: None,
                        is_primary,
                        menu_bar_height,
                    };
                    info.dock_position =
                        crate::extensions::window_management::detect_dock_position(&info);
                    screens.push(info);
                }
            }
        }

        screens
    }

    #[cfg(not(target_os = "macos"))]
    {
        vec![crate::extensions::window_management::ScreenInfo {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
            visible_x: 0.0,
            visible_y: 0.0,
            visible_width: 1920.0,
            visible_height: 1080.0,
            dock_position: None,
            is_primary: true,
            menu_bar_height: 0.0,
        }]
    }
}

/// Capture a screen region and return as base64 PNG for the zoom panel.
#[tauri::command]
pub fn ruler_capture_region(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        use core_graphics::display::{CGDisplay, kCGWindowListOptionOnScreenOnly, kCGNullWindowID, kCGWindowImageDefault};
        use core_graphics::geometry::{CGPoint, CGRect, CGSize};
        use image::RgbaImage;

        let rect = CGRect::new(
            &CGPoint::new(x, y),
            &CGSize::new(width.max(1.0), height.max(1.0)),
        );

        let cg_image = CGDisplay::screenshot(
            rect,
            kCGWindowListOptionOnScreenOnly,
            kCGNullWindowID,
            kCGWindowImageDefault,
        )
        .ok_or_else(|| "Failed to capture screen region".to_string())?;

        let w = cg_image.width();
        let h = cg_image.height();
        let bytes_per_row = cg_image.bytes_per_row();
        let data = cg_image.data();
        let raw = data.bytes();

        // CGImage data is BGRA; convert to RGBA
        let mut rgba = Vec::with_capacity(w * h * 4);
        for row in 0..h {
            for col in 0..w {
                let offset = row * bytes_per_row + col * 4;
                if offset + 3 < raw.len() {
                    rgba.push(raw[offset + 2]); // R
                    rgba.push(raw[offset + 1]); // G
                    rgba.push(raw[offset]);     // B
                    rgba.push(raw[offset + 3]); // A
                } else {
                    rgba.extend_from_slice(&[0, 0, 0, 255]);
                }
            }
        }

        let img = RgbaImage::from_raw(w as u32, h as u32, rgba)
            .ok_or_else(|| "Failed to create image from capture data".to_string())?;

        let mut png_buf = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut png_buf);
        img.write_to(&mut cursor, image::ImageFormat::Png)
            .map_err(|e| format!("Failed to encode PNG: {e}"))?;

        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&png_buf);
        Ok(format!("data:image/png;base64,{b64}"))
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (x, y, width, height);
        Err("Screen capture not supported on this platform".into())
    }
}

/// Detect color edges around a point for snap mode.
#[tauri::command]
pub fn ruler_detect_edges(x: f64, y: f64, radius: f64) -> Result<Vec<EdgePoint>, String> {
    #[cfg(target_os = "macos")]
    {
        use core_graphics::display::{CGDisplay, kCGWindowListOptionOnScreenOnly, kCGNullWindowID, kCGWindowImageDefault};
        use core_graphics::geometry::{CGPoint, CGRect, CGSize};

        let r = radius.max(5.0);
        let rect = CGRect::new(
            &CGPoint::new(x - r, y - r),
            &CGSize::new(r * 2.0, r * 2.0),
        );

        let cg_image = CGDisplay::screenshot(
            rect,
            kCGWindowListOptionOnScreenOnly,
            kCGNullWindowID,
            kCGWindowImageDefault,
        )
        .ok_or_else(|| "Failed to capture for edge detection".to_string())?;

        let w = cg_image.width();
        let h = cg_image.height();
        let bpr = cg_image.bytes_per_row();
        let data = cg_image.data();
        let raw = data.bytes();

        let pixel = |col: usize, row: usize| -> (u8, u8, u8) {
            let off = row * bpr + col * 4;
            if off + 2 < raw.len() {
                (raw[off + 2], raw[off + 1], raw[off])
            } else {
                (0, 0, 0)
            }
        };

        let color_diff = |a: (u8, u8, u8), b: (u8, u8, u8)| -> f64 {
            let dr = a.0 as f64 - b.0 as f64;
            let dg = a.1 as f64 - b.1 as f64;
            let db = a.2 as f64 - b.2 as f64;
            (dr * dr + dg * dg + db * db).sqrt()
        };

        let cx = w / 2;
        let cy = h / 2;
        let threshold = 20.0;
        let mut edges = Vec::new();

        let center_color = pixel(cx, cy);

        // Scan right
        for dx in 1..cx.min(r as usize) {
            if color_diff(center_color, pixel(cx + dx, cy)) > threshold {
                edges.push(EdgePoint { x: x + dx as f64, y, direction: "right".into() });
                break;
            }
        }
        // Scan left
        for dx in 1..cx.min(r as usize) {
            if cx >= dx && color_diff(center_color, pixel(cx - dx, cy)) > threshold {
                edges.push(EdgePoint { x: x - dx as f64, y, direction: "left".into() });
                break;
            }
        }
        // Scan down
        for dy in 1..cy.min(r as usize) {
            if color_diff(center_color, pixel(cx, cy + dy)) > threshold {
                edges.push(EdgePoint { x, y: y + dy as f64, direction: "down".into() });
                break;
            }
        }
        // Scan up
        for dy in 1..cy.min(r as usize) {
            if cy >= dy && color_diff(center_color, pixel(cx, cy - dy)) > threshold {
                edges.push(EdgePoint { x, y: y - dy as f64, direction: "up".into() });
                break;
            }
        }

        Ok(edges)
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (x, y, radius);
        Ok(vec![])
    }
}

/// Copy measurement text to clipboard.
#[tauri::command]
pub fn ruler_copy_measurements(data: String) -> Result<(), String> {
    let mut child = std::process::Command::new("pbcopy")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to launch pbcopy: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(data.as_bytes())
            .map_err(|e| format!("Failed to write to pbcopy: {e}"))?;
    }
    child
        .wait()
        .map_err(|e| format!("pbcopy failed: {e}"))?;
    Ok(())
}

/// Capture the full screen with overlay visible and save to screenshots dir.
#[tauri::command]
pub fn ruler_screenshot_overlay() -> Result<String, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join("com.infamousvague.emit")
        .join("screenshots");
    std::fs::create_dir_all(&dir).ok();

    let path = dir.join(format!("ruler_{now}.png"));
    let path_str = path.to_string_lossy().to_string();

    let status = std::process::Command::new("screencapture")
        .args(["-x", &path_str])
        .status()
        .map_err(|e| format!("screencapture failed: {e}"))?;

    if !status.success() {
        return Err("screencapture returned non-zero".into());
    }

    // Copy to clipboard
    std::process::Command::new("osascript")
        .args([
            "-e",
            &format!(
                "set the clipboard to (read (POSIX file \"{path_str}\") as «class PNGf»)"
            ),
        ])
        .status()
        .ok();

    Ok(path_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ruler_provider() {
        let provider = RulerProvider::new();
        assert_eq!(provider.name(), "Ruler");
        assert_eq!(
            provider.execute("ruler.open"),
            Some(Ok("action:ruler".into()))
        );
        assert_eq!(provider.execute("unknown"), None);
    }
}
