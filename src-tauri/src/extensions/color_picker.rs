use async_trait::async_trait;

use crate::launcher::CommandEntry;
use crate::providers::CommandProvider;

pub struct ColorPickerProvider;

impl ColorPickerProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl CommandProvider for ColorPickerProvider {
    fn name(&self) -> &str {
        "ColorPicker"
    }

    async fn commands(&self) -> Vec<CommandEntry> {
        vec![CommandEntry {
            id: "color_picker.open".into(),
            name: "Color Picker".into(),
            description: "Pick colors from your screen and save palettes".into(),
            category: "Design".into(),
            icon: None,
            match_indices: vec![],
            score: 0,
        }]
    }

    fn execute(&self, id: &str) -> Option<Result<String, String>> {
        if id == "color_picker.open" {
            Some(Ok("view:color-picker".into()))
        } else {
            None
        }
    }
}

/// Launch the native macOS color sampler (eyedropper).
/// Picks colors in a loop — each pick emits a `color-picker-pick` event,
/// then re-shows the sampler. Escape emits `color-picker-done`.
#[cfg(target_os = "macos")]
#[tauri::command]
pub fn color_picker_sample_screen(app: tauri::AppHandle) {
    // Hide main window before starting
    crate::window::hide(&app);

    // Small delay to let the window hide animation complete
    let app_clone = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(200));
        show_sampler(app_clone);
    });
}

#[cfg(target_os = "macos")]
fn show_sampler(app: tauri::AppHandle) {
    use tauri::Emitter;

    // NSColorSampler must be called on the main thread
    let app_clone = app.clone();
    let _ = app.run_on_main_thread(move || {
        use block2::RcBlock;
        use objc2_app_kit::{NSColor, NSColorSampler, NSColorSpace};

        let sampler = NSColorSampler::new();
        let app_inner = app_clone.clone();

        let handler = RcBlock::new(move |color: *mut NSColor| {
            if color.is_null() {
                // User pressed Escape — done picking
                let _ = app_inner.emit("color-picker-done", ());
                crate::window::show_main(&app_inner);
            } else {
                // Convert to sRGB and extract components
                let color_ref = unsafe { &*color };
                let srgb_space = NSColorSpace::sRGBColorSpace();
                let rgb = if let Some(rgb_color) = color_ref.colorUsingColorSpace(&srgb_space) {
                    let r = rgb_color.redComponent();
                    let g = rgb_color.greenComponent();
                    let b = rgb_color.blueComponent();
                    Some((
                        (r * 255.0).round() as u8,
                        (g * 255.0).round() as u8,
                        (b * 255.0).round() as u8,
                    ))
                } else {
                    None
                };

                if let Some((r, g, b)) = rgb {
                    let hex = format!("#{:02x}{:02x}{:02x}", r, g, b);
                    let payload = serde_json::json!({
                        "hex": hex,
                        "rgb": { "r": r, "g": g, "b": b }
                    });
                    let _ = app_inner.emit("color-picker-pick", payload);

                    // Audible feedback: play system sound
                    let _ = std::process::Command::new("afplay")
                        .arg("/System/Library/Sounds/Tink.aiff")
                        .spawn();
                }

                // Re-show sampler for next pick
                let app_next = app_inner.clone();
                show_sampler(app_next);
            }
        });

        unsafe {
            sampler.showSamplerWithSelectionHandler(&handler);
        }
    });
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn color_picker_sample_screen(_app: tauri::AppHandle) -> Result<(), String> {
    Err("Color sampling is only supported on macOS".into())
}

/// Save palettes to ~/.config/com.infamousvague.emit/palettes.json
#[tauri::command]
pub fn color_picker_save_palettes(palettes: serde_json::Value) -> Result<(), String> {
    let path = palettes_path()?;
    let json = serde_json::to_string_pretty(&palettes).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("Failed to write palettes: {e}"))
}

/// Load palettes from disk. Returns empty array if file doesn't exist.
#[tauri::command]
pub fn color_picker_load_palettes() -> Result<serde_json::Value, String> {
    let path = palettes_path()?;
    if !path.exists() {
        return Ok(serde_json::json!([]));
    }
    let data = std::fs::read_to_string(&path).map_err(|e| format!("Failed to read palettes: {e}"))?;
    serde_json::from_str(&data).map_err(|e| format!("Failed to parse palettes: {e}"))
}

fn palettes_path() -> Result<std::path::PathBuf, String> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| "Cannot determine config directory".to_string())?
        .join("com.infamousvague.emit");
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config dir: {e}"))?;
    Ok(config_dir.join("palettes.json"))
}
