use base64::Engine;
use std::path::{Path, PathBuf};
use tokio::process::Command;

/// Returns the icon cache directory, creating it if needed.
fn cache_dir() -> PathBuf {
    let dir = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("com.infamousvague.emit")
        .join("icons");
    std::fs::create_dir_all(&dir).ok();
    dir
}

/// Sanitize an app name into a safe filename slug.
fn slug(name: &str) -> String {
    name.to_lowercase()
        .replace(' ', "-")
        .replace('.', "-")
}

/// Returns the cached PNG path for an app.
fn cached_png_path(app_name: &str) -> PathBuf {
    cache_dir().join(format!("{}.png", slug(app_name)))
}

/// Locate the .icns file inside an app bundle.
///
/// Uses PlistBuddy to read CFBundleIconFile from Info.plist (handles both
/// binary and XML plists). Falls back to common default names.
async fn find_icns_file(app_path: &Path) -> Option<PathBuf> {
    let plist = app_path.join("Contents/Info.plist");
    if !plist.exists() {
        return None;
    }

    // Try PlistBuddy to read CFBundleIconFile
    let output = Command::new("/usr/libexec/PlistBuddy")
        .arg("-c")
        .arg("Print :CFBundleIconFile")
        .arg(&plist)
        .output()
        .await
        .ok()?;

    if output.status.success() {
        let mut icon_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !icon_name.ends_with(".icns") {
            icon_name.push_str(".icns");
        }
        let icns_path = app_path.join("Contents/Resources").join(&icon_name);
        if icns_path.exists() {
            return Some(icns_path);
        }
    }

    // Fallback: try common icon names
    let resources = app_path.join("Contents/Resources");
    for name in &["AppIcon.icns", "app.icns", "icon.icns"] {
        let path = resources.join(name);
        if path.exists() {
            return Some(path);
        }
    }

    None
}

/// Extract an app icon to a base64 data URI.
///
/// Returns a cached result if available, otherwise converts the .icns to a
/// 64px PNG via `sips` and encodes as a data URI.
pub async fn extract_icon(app_path: &Path, app_name: &str) -> Option<String> {
    let png_path = cached_png_path(app_name);

    // Use cached PNG if it exists
    if png_path.exists() {
        return read_as_data_uri(&png_path);
    }

    // Find the .icns source
    let icns_path = find_icns_file(app_path).await?;

    // Convert to 64px PNG via sips
    let status = Command::new("sips")
        .arg("-s")
        .arg("format")
        .arg("png")
        .arg("--resampleWidth")
        .arg("64")
        .arg(&icns_path)
        .arg("--out")
        .arg(&png_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .ok()?;

    if !status.success() {
        log::warn!("sips failed for {app_name}");
        return None;
    }

    read_as_data_uri(&png_path)
}

/// Read a PNG file and return it as a base64 data URI.
fn read_as_data_uri(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Some(format!("data:image/png;base64,{encoded}"))
}

/// Look up a previously cached icon by app name.
pub fn get_cached_icon(app_name: &str) -> Option<String> {
    let path = cached_png_path(app_name);
    if path.exists() {
        read_as_data_uri(&path)
    } else {
        None
    }
}
