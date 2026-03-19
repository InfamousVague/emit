//! Persistent application settings with JSON file storage.
//!
//! Settings are stored at `~/.config/com.infamousvague.emit/settings.json`
//! and fall back to sensible defaults when the file doesn't exist.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub shortcut: String,
    pub launch_at_login: bool,
    pub show_in_dock: bool,
    pub max_results: usize,
    pub check_for_updates: bool,
    pub replace_spotlight: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            shortcut: "Alt+Space".to_string(),
            launch_at_login: false,
            show_in_dock: false,
            max_results: 20,
            check_for_updates: true,
            replace_spotlight: false,
        }
    }
}

#[cfg(target_os = "macos")]
pub fn set_spotlight_enabled(enabled: bool) -> Result<(), String> {
    let enabled_str = if enabled { "true" } else { "false" };
    let plist_value = format!(
        "<dict><key>enabled</key><{enabled_str}/>\
         <key>value</key><dict><key>parameters</key>\
         <array><integer>65535</integer><integer>49</integer>\
         <integer>1048576</integer></array>\
         <key>type</key><string>standard</string></dict></dict>"
    );

    std::process::Command::new("defaults")
        .args([
            "write",
            "com.apple.symbolichotkeys",
            "AppleSymbolicHotKeys",
            "-dict-add",
            "64",
            &plist_value,
        ])
        .output()
        .map_err(|e| format!("Failed to write Spotlight preference: {e}"))?;

    std::process::Command::new("/System/Library/PrivateFrameworks/SystemAdministration.framework/Resources/activateSettings")
        .arg("-u")
        .output()
        .map_err(|e| format!("Failed to activate settings: {e}"))?;

    Ok(())
}

impl Settings {
    fn config_path() -> PathBuf {
        let dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("com.infamousvague.emit");
        fs::create_dir_all(&dir).ok();
        dir.join("settings.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        match fs::read_to_string(&path) {
            Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
            Err(_) => {
                let settings = Self::default();
                settings.save().ok();
                settings
            }
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(Self::config_path(), json).map_err(|e| e.to_string())
    }
}
