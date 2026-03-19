use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use super::crypto::{decrypt_secrets, encrypt_secrets};
use super::manifest::all_manifests;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionState {
    pub enabled: bool,
    pub settings: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionRegistry {
    pub states: HashMap<String, ExtensionState>,
}

impl ExtensionRegistry {
    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("com.infamousvague.emit")
            .join("extensions.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        let mut registry = if path.exists() {
            fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_else(|| ExtensionRegistry {
                    states: HashMap::new(),
                })
        } else {
            ExtensionRegistry {
                states: HashMap::new(),
            }
        };

        // Initialize any missing extensions from manifests
        for manifest in all_manifests() {
            registry
                .states
                .entry(manifest.id)
                .or_insert_with(|| ExtensionState {
                    enabled: manifest.default_enabled,
                    settings: serde_json::Value::Object(serde_json::Map::new()),
                });
        }

        registry
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        // Encrypt secret fields before writing to disk
        let mut to_save = self.clone();
        for state in to_save.states.values_mut() {
            state.settings = encrypt_secrets(&state.settings);
        }
        if let Ok(json) = serde_json::to_string_pretty(&to_save) {
            let _ = fs::write(&path, json);
        }
    }

    pub fn is_enabled(&self, id: &str) -> bool {
        self.states
            .get(id)
            .map(|s| s.enabled)
            .unwrap_or(false)
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) {
        if let Some(state) = self.states.get_mut(id) {
            state.enabled = enabled;
        } else {
            self.states.insert(
                id.to_string(),
                ExtensionState {
                    enabled,
                    settings: serde_json::Value::Object(serde_json::Map::new()),
                },
            );
        }
        self.save();
    }

    pub fn get_settings(&self, id: &str) -> serde_json::Value {
        self.states
            .get(id)
            .map(|s| decrypt_secrets(&s.settings))
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()))
    }

    pub fn set_settings(&mut self, id: &str, settings: serde_json::Value) {
        if let Some(state) = self.states.get_mut(id) {
            state.settings = settings;
        }
        self.save();
    }
}
