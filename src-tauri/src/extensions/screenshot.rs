use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::launcher::CommandEntry;
use crate::providers::CommandProvider;

const MAX_SCREENSHOTS: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotItem {
    pub id: String,
    pub path: String,
    pub thumbnail_path: String,
    pub timestamp: u64,
    pub width: u32,
    pub height: u32,
    pub source_app: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ScreenshotIndex {
    pub items: Vec<ScreenshotItem>,
}

pub struct ScreenshotProvider;

impl ScreenshotProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl CommandProvider for ScreenshotProvider {
    fn name(&self) -> &str {
        "Screenshot"
    }

    async fn commands(&self) -> Vec<CommandEntry> {
        vec![CommandEntry {
            id: "screenshot.open".into(),
            name: "Screenshot".into(),
            description: "Capture windows and manage screenshots".into(),
            category: "Extensions".into(),
            icon: None,
            match_indices: vec![],
            score: 0,
        }]
    }

    fn execute(&self, id: &str) -> Option<Result<String, String>> {
        if id == "screenshot.open" {
            Some(Ok("view:screenshot".into()))
        } else {
            None
        }
    }
}

fn screenshots_dir() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("com.infamousvague.emit")
        .join("screenshots");
    std::fs::create_dir_all(&dir).ok();
    dir
}

fn thumbs_dir() -> PathBuf {
    let dir = screenshots_dir().join("thumbs");
    std::fs::create_dir_all(&dir).ok();
    dir
}

fn index_path() -> PathBuf {
    screenshots_dir().join("index.json")
}

impl ScreenshotIndex {
    pub fn load() -> Self {
        let path = index_path();
        if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            std::fs::write(index_path(), json).ok();
        }
    }

    pub fn add(&mut self, item: ScreenshotItem) {
        self.items.insert(0, item);
        if self.items.len() > MAX_SCREENSHOTS {
            for item in self.items.drain(MAX_SCREENSHOTS..) {
                std::fs::remove_file(&item.path).ok();
                std::fs::remove_file(&item.thumbnail_path).ok();
            }
        }
        self.save();
    }

    pub fn delete(&mut self, id: &str) {
        if let Some(item) = self.items.iter().find(|i| i.id == id) {
            std::fs::remove_file(&item.path).ok();
            std::fs::remove_file(&item.thumbnail_path).ok();
        }
        self.items.retain(|i| i.id != id);
        self.save();
    }
}

fn generate_thumbnail(input_path: &std::path::Path, thumb_path: &std::path::Path) -> Result<(), String> {
    let img = image::open(input_path).map_err(|e| format!("Failed to open image: {e}"))?;
    let thumb = img.thumbnail(200, 200);
    thumb.save(thumb_path).map_err(|e| format!("Failed to save thumbnail: {e}"))?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn frontmost_app_name() -> Option<String> {
    use objc2_app_kit::NSWorkspace;
    let workspace = NSWorkspace::sharedWorkspace();
    let app = workspace.frontmostApplication()?;
    let name = app.localizedName()?;
    Some(name.to_string())
}

#[cfg(not(target_os = "macos"))]
fn frontmost_app_name() -> Option<String> {
    None
}

#[derive(Debug, Clone, Copy)]
enum CaptureMode {
    Region,
    Window,
    Screen,
}

/// Capture a region interactively using macOS screencapture.
#[tauri::command]
pub fn screenshot_capture_region(app: tauri::AppHandle) {
    crate::window::hide(&app);
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(300));
        do_capture(&app, CaptureMode::Region);
    });
}

/// Capture a window interactively using macOS screencapture.
#[tauri::command]
pub fn screenshot_capture_window(app: tauri::AppHandle) {
    crate::window::hide(&app);
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(300));
        do_capture(&app, CaptureMode::Window);
    });
}

/// Capture the entire screen using macOS screencapture.
#[tauri::command]
pub fn screenshot_capture_screen(app: tauri::AppHandle) {
    crate::window::hide(&app);
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(300));
        do_capture(&app, CaptureMode::Screen);
    });
}

fn do_capture(app: &tauri::AppHandle, mode: CaptureMode) {
    use tauri::Emitter;

    let source_app = match mode {
        CaptureMode::Window => frontmost_app_name(),
        _ => None,
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let tmp_path = std::env::temp_dir().join(format!("emit_capture_{now}.png"));
    let tmp_str = tmp_path.to_string_lossy().to_string();

    let mut cmd = std::process::Command::new("screencapture");
    match mode {
        CaptureMode::Region => { cmd.args(["-i", "-x", &tmp_str]); }
        CaptureMode::Window => { cmd.args(["-iW", "-x", &tmp_str]); }
        CaptureMode::Screen => { cmd.args(["-x", &tmp_str]); }
    }

    let status = cmd.status();
    let captured = matches!(status, Ok(s) if s.success()) && tmp_path.exists();

    if !captured {
        std::fs::remove_file(&tmp_path).ok();
        crate::window::show_main(app);
        return;
    }

    let final_path = screenshots_dir().join(format!("{now}.png"));
    let thumb_path = thumbs_dir().join(format!("{now}_thumb.png"));

    let (width, height) = match image::image_dimensions(&tmp_path) {
        Ok(dims) => dims,
        Err(_) => {
            std::fs::remove_file(&tmp_path).ok();
            crate::window::show_main(app);
            return;
        }
    };

    // Move capture to screenshots directory
    if let Err(e) = std::fs::rename(&tmp_path, &final_path) {
        log::error!("Failed to move capture: {e}");
        std::fs::remove_file(&tmp_path).ok();
        crate::window::show_main(app);
        return;
    }

    if let Err(e) = generate_thumbnail(&final_path, &thumb_path) {
        log::error!("Thumbnail failed: {e}");
    }

    let item = ScreenshotItem {
        id: format!("screenshot_{now}"),
        path: final_path.to_string_lossy().to_string(),
        thumbnail_path: thumb_path.to_string_lossy().to_string(),
        timestamp: now,
        width,
        height,
        source_app,
    };

    let mut index = ScreenshotIndex::load();
    index.add(item.clone());

    let _ = app.emit("screenshot-captured", &item);
    crate::window::show_main(app);
}

#[tauri::command]
pub fn screenshot_list() -> Result<Vec<ScreenshotItem>, String> {
    Ok(ScreenshotIndex::load().items)
}

#[tauri::command]
pub fn screenshot_delete(id: String) -> Result<(), String> {
    let mut index = ScreenshotIndex::load();
    index.delete(&id);
    Ok(())
}

#[tauri::command]
pub fn screenshot_copy(id: String) -> Result<(), String> {
    let index = ScreenshotIndex::load();
    let item = index
        .items
        .iter()
        .find(|i| i.id == id)
        .ok_or_else(|| "Screenshot not found".to_string())?;

    // Copy image to clipboard via macOS pbcopy
    let status = std::process::Command::new("osascript")
        .args([
            "-e",
            &format!(
                "set the clipboard to (read (POSIX file \"{}\") as «class PNGf»)",
                item.path
            ),
        ])
        .status()
        .map_err(|e| format!("Failed to copy: {e}"))?;

    if !status.success() {
        return Err("Copy to clipboard failed".into());
    }
    Ok(())
}

#[tauri::command]
pub fn screenshot_get_image(path: String) -> Result<String, String> {
    let data = std::fs::read(&path).map_err(|e| format!("Failed to read image: {e}"))?;
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
    Ok(format!("data:image/png;base64,{b64}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_thumbnail() {
        use image::{GenericImageView, RgbaImage, Rgba};

        let tmp = std::env::temp_dir();
        let input = tmp.join("test_thumb_input.png");
        let output = tmp.join("test_thumb_output.png");

        // Create a 400x300 test image
        let img = RgbaImage::from_pixel(400, 300, Rgba([0, 128, 255, 255]));
        img.save(&input).unwrap();

        generate_thumbnail(&input, &output).unwrap();

        let thumb = image::open(&output).unwrap();
        let (w, _h) = thumb.dimensions();
        assert!(w <= 200);

        std::fs::remove_file(&input).ok();
        std::fs::remove_file(&output).ok();
    }

    #[test]
    fn test_screenshot_index_crud() {
        let mut index = ScreenshotIndex::default();
        assert_eq!(index.items.len(), 0);

        let item = ScreenshotItem {
            id: "test_1".into(),
            path: "/tmp/test_1.png".into(),
            thumbnail_path: "/tmp/test_1_thumb.png".into(),
            timestamp: 1000,
            width: 800,
            height: 600,
            source_app: Some("TestApp".into()),
        };

        index.items.push(item);
        assert_eq!(index.items.len(), 1);

        index.items.retain(|i| i.id != "test_1");
        assert_eq!(index.items.len(), 0);
    }
}
