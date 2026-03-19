//! Persistent application settings with JSON file storage.
//!
//! Settings are stored at `~/.config/com.infamousvague.emit/settings.json`
//! and fall back to sensible defaults when the file doesn't exist.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub shortcut: String,
    pub launch_at_login: bool,
    pub show_in_dock: bool,
    pub max_results: usize,
    pub check_for_updates: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            shortcut: "Alt+Space".to_string(),
            launch_at_login: false,
            show_in_dock: false,
            max_results: 20,
            check_for_updates: true,
        }
    }
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
