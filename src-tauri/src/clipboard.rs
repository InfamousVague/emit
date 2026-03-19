use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

const MAX_HISTORY: usize = 100;
const POLL_INTERVAL_MS: u64 = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardMetadata {
    pub width: u32,
    pub height: u32,
    pub size_bytes: u64,
    pub source_app: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardItem {
    pub id: String,
    pub content: String,
    pub content_type: String,
    pub timestamp: u64,
    pub preview: String,
    #[serde(default)]
    pub image_path: Option<String>,
    #[serde(default)]
    pub metadata: Option<ClipboardMetadata>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ClipboardState {
    pub items: Vec<ClipboardItem>,
    #[serde(skip)]
    last_content: String,
    #[serde(skip)]
    last_image_hash: Option<u64>,
}

pub type SharedClipboardState = Arc<RwLock<ClipboardState>>;

fn storage_path() -> PathBuf {
    let config = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("com.infamousvague.emit");
    std::fs::create_dir_all(&config).ok();
    config.join("clipboard_history.json")
}

fn images_dir() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("com.infamousvague.emit")
        .join("clipboard_images");
    std::fs::create_dir_all(&dir).ok();
    dir
}

fn truncate_preview(content: &str, max_len: usize) -> String {
    let single_line = content.lines().next().unwrap_or("");
    if single_line.len() > max_len {
        format!("{}…", &single_line[..max_len])
    } else {
        single_line.to_string()
    }
}

fn hash_bytes(data: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    let sample = &data[..data.len().min(2048)];
    sample.hash(&mut hasher);
    hasher.finish()
}

impl ClipboardState {
    pub fn load() -> Self {
        let path = storage_path();
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
            std::fs::write(storage_path(), json).ok();
        }
    }

    pub fn add(&mut self, content: String) {
        if content.trim().is_empty() {
            return;
        }

        // Remove duplicate if already in history
        self.items.retain(|item| item.content != content);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let content_type = if content.starts_with("http://") || content.starts_with("https://") {
            "url".to_string()
        } else {
            "text".to_string()
        };

        let preview = truncate_preview(&content, 80);

        self.items.insert(
            0,
            ClipboardItem {
                id: format!("clipboard.item_{now}"),
                content,
                content_type,
                timestamp: now,
                preview,
                image_path: None,
                metadata: None,
            },
        );

        if self.items.len() > MAX_HISTORY {
            // Clean up image files for items being dropped
            for item in self.items.drain(MAX_HISTORY..) {
                if let Some(path) = &item.image_path {
                    std::fs::remove_file(path).ok();
                }
            }
        }

        self.save();
    }

    pub fn add_image(&mut self, png_data: &[u8], metadata: ClipboardMetadata) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let id = format!("clipboard.item_{now}");
        let image_path = images_dir().join(format!("{now}.png"));

        if std::fs::write(&image_path, png_data).is_err() {
            log::error!("Failed to write clipboard image to {:?}", image_path);
            return;
        }

        let dims = format!("{}×{}", metadata.width, metadata.height);
        let size = format_bytes(metadata.size_bytes);
        let preview = match &metadata.source_app {
            Some(app) => format!("Image ({dims}, {size}) from {app}"),
            None => format!("Image ({dims}, {size})"),
        };

        self.items.insert(
            0,
            ClipboardItem {
                id,
                content: format!("Image ({dims})"),
                content_type: "image".to_string(),
                timestamp: now,
                preview,
                image_path: Some(image_path.to_string_lossy().to_string()),
                metadata: Some(metadata),
            },
        );

        if self.items.len() > MAX_HISTORY {
            for item in self.items.drain(MAX_HISTORY..) {
                if let Some(path) = &item.image_path {
                    std::fs::remove_file(path).ok();
                }
            }
        }

        self.save();
    }

    pub fn delete(&mut self, id: &str) {
        if let Some(item) = self.items.iter().find(|i| i.id == id) {
            if let Some(path) = &item.image_path {
                std::fs::remove_file(path).ok();
            }
        }
        self.items.retain(|item| item.id != id);
        self.save();
    }

    pub fn clear(&mut self) {
        for item in &self.items {
            if let Some(path) = &item.image_path {
                std::fs::remove_file(path).ok();
            }
        }
        self.items.clear();
        self.save();
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// Read the system clipboard text via `pbpaste`.
async fn read_clipboard_text() -> Option<String> {
    let output = tokio::process::Command::new("pbpaste")
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    if text.is_empty() { None } else { Some(text) }
}

/// Read image data from macOS clipboard via NSPasteboard.
/// Returns PNG bytes and dimensions if an image is present.
#[cfg(target_os = "macos")]
fn read_clipboard_image() -> Option<(Vec<u8>, u32, u32)> {
    use objc2_app_kit::{NSBitmapImageRep, NSBitmapImageFileType, NSPasteboard};
    use objc2_foundation::NSString;

    unsafe {
        let pasteboard = NSPasteboard::generalPasteboard();

        // Check for image types
        let png_type = NSString::from_str("public.png");
        let tiff_type = NSString::from_str("public.tiff");

        // Try PNG first, then TIFF
        let image_data = pasteboard
            .dataForType(&png_type)
            .or_else(|| pasteboard.dataForType(&tiff_type))?;

        let raw_bytes: &[u8] = image_data.as_bytes_unchecked();

        // Create bitmap rep to get dimensions
        let bitmap_rep = NSBitmapImageRep::imageRepWithData(&image_data)?;
        let width = bitmap_rep.pixelsWide() as u32;
        let height = bitmap_rep.pixelsHigh() as u32;

        // Check if data is already PNG
        let is_png = raw_bytes.len() >= 4 && raw_bytes[0..4] == [0x89, 0x50, 0x4E, 0x47];
        if is_png {
            Some((raw_bytes.to_vec(), width, height))
        } else {
            // Convert TIFF to PNG
            let png_data = bitmap_rep.representationUsingType_properties(
                NSBitmapImageFileType::PNG,
                &objc2_foundation::NSDictionary::new(),
            )?;
            let png_bytes: &[u8] = png_data.as_bytes_unchecked();
            Some((png_bytes.to_vec(), width, height))
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn read_clipboard_image() -> Option<(Vec<u8>, u32, u32)> {
    None
}

/// Get the frontmost application name.
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

/// Get base64-encoded PNG data for a clipboard image item.
pub fn get_image_base64(items: &[ClipboardItem], id: &str) -> Result<String, String> {
    let item = items
        .iter()
        .find(|i| i.id == id)
        .ok_or_else(|| "Item not found".to_string())?;

    let path = item
        .image_path
        .as_ref()
        .ok_or_else(|| "Not an image item".to_string())?;

    let data = std::fs::read(path).map_err(|e| format!("Failed to read image: {e}"))?;

    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
    Ok(format!("data:image/png;base64,{b64}"))
}

/// Start the background clipboard monitor that polls for changes.
pub fn start_monitor(state: SharedClipboardState) {
    tauri::async_runtime::spawn(async move {
        // Initialize last_content from current clipboard
        if let Some(text) = read_clipboard_text().await {
            state.write().await.last_content = text;
        }

        // Initialize image hash
        if let Some((data, _, _)) = read_clipboard_image() {
            state.write().await.last_image_hash = Some(hash_bytes(&data));
        }

        log::info!("Clipboard monitor started (text + image)");

        loop {
            tokio::time::sleep(std::time::Duration::from_millis(POLL_INTERVAL_MS)).await;

            // Check for image changes first
            if let Some((png_data, width, height)) = read_clipboard_image() {
                let img_hash = hash_bytes(&png_data);
                let mut s = state.write().await;
                if s.last_image_hash != Some(img_hash) {
                    log::info!(
                        "Clipboard image detected ({}×{}, {} bytes)",
                        width,
                        height,
                        png_data.len()
                    );
                    s.last_image_hash = Some(img_hash);
                    let source_app = frontmost_app_name();
                    let metadata = ClipboardMetadata {
                        width,
                        height,
                        size_bytes: png_data.len() as u64,
                        source_app,
                    };
                    s.add_image(&png_data, metadata);
                    // Update text tracker too to avoid double-fire
                    if let Some(text) = read_clipboard_text().await {
                        s.last_content = text;
                    }
                    continue;
                }
            }

            // Check for text changes
            let current = match read_clipboard_text().await {
                Some(text) => text,
                None => continue,
            };

            let mut s = state.write().await;
            if current != s.last_content {
                log::info!("Clipboard change detected ({} chars)", current.len());
                s.last_content = current.clone();
                s.add(current);
                // Reset image hash since this is a text copy
                s.last_image_hash = None;
            }
        }
    });
}
