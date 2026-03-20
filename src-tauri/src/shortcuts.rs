//! Centralized keyboard shortcut registry.
//!
//! Extensions register their default shortcuts at startup. Users can rebind
//! shortcuts via the settings UI. All shortcuts are dispatched through a single
//! global shortcut handler.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};
use tokio::sync::RwLock;

pub type SharedShortcutRegistry = Arc<RwLock<ShortcutRegistry>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutBinding {
    pub id: String,
    pub label: String,
    pub default_keys: String,
    pub keys: String,
    pub extension_id: String,
}

#[derive(Debug, Clone, Default)]
pub struct ShortcutRegistry {
    pub bindings: HashMap<String, ShortcutBinding>,
}

impl ShortcutRegistry {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    /// Register a shortcut binding. If the user has a saved override, use that instead.
    pub fn register(&mut self, binding: ShortcutBinding, user_overrides: &HashMap<String, String>) {
        let mut b = binding;
        if let Some(user_keys) = user_overrides.get(&b.id) {
            b.keys = user_keys.clone();
        }
        self.bindings.insert(b.id.clone(), b);
    }

    /// Rebind a shortcut to new keys.
    pub fn rebind(&mut self, id: &str, new_keys: &str) -> Result<(), String> {
        let binding = self
            .bindings
            .get_mut(id)
            .ok_or_else(|| format!("Shortcut '{id}' not found"))?;
        binding.keys = new_keys.to_string();
        Ok(())
    }

    /// Find which shortcut ID matches a given Tauri shortcut.
    pub fn resolve_shortcut(&self, shortcut: &Shortcut) -> Option<String> {
        for binding in self.bindings.values() {
            if let Ok(parsed) = parse_shortcut(&binding.keys) {
                let shortcut_mods = if shortcut.mods.is_empty() { None } else { Some(shortcut.mods) };
                if parsed.mods == shortcut_mods && parsed.key == shortcut.key {
                    return Some(binding.id.clone());
                }
            }
        }
        None
    }

    /// Get all bindings as a sorted list.
    pub fn all_bindings(&self) -> Vec<ShortcutBinding> {
        let mut list: Vec<_> = self.bindings.values().cloned().collect();
        list.sort_by(|a, b| a.extension_id.cmp(&b.extension_id).then(a.id.cmp(&b.id)));
        list
    }

    /// Export user overrides (only non-default bindings) for persistence.
    pub fn user_overrides(&self) -> HashMap<String, String> {
        self.bindings
            .iter()
            .filter(|(_, b)| b.keys != b.default_keys)
            .map(|(id, b)| (id.clone(), b.keys.clone()))
            .collect()
    }

    /// Get all Tauri shortcuts to register.
    pub fn tauri_shortcuts(&self) -> Vec<Shortcut> {
        self.bindings
            .values()
            .filter_map(|b| {
                parse_shortcut(&b.keys)
                    .ok()
                    .map(|p| Shortcut::new(p.mods, p.key))
            })
            .collect()
    }
}

/// Parsed shortcut result.
pub struct ParsedShortcut {
    pub mods: Option<Modifiers>,
    pub key: Code,
}

/// Parse a human-readable shortcut string like "Shift+Cmd+P" into Tauri types.
pub fn parse_shortcut(keys: &str) -> Result<ParsedShortcut, String> {
    let parts: Vec<&str> = keys.split('+').map(|s| s.trim()).collect();
    if parts.is_empty() {
        return Err("Empty shortcut".to_string());
    }

    let mut mods = Modifiers::empty();
    let key_str = parts.last().ok_or("No key specified")?;

    for &part in &parts[..parts.len() - 1] {
        match part.to_lowercase().as_str() {
            "shift" => mods |= Modifiers::SHIFT,
            "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "alt" | "option" | "opt" => mods |= Modifiers::ALT,
            "cmd" | "command" | "meta" | "super" => mods |= Modifiers::META,
            _ => return Err(format!("Unknown modifier: {part}")),
        }
    }

    let code = parse_key_code(key_str)?;
    let mods_opt = if mods.is_empty() { None } else { Some(mods) };

    Ok(ParsedShortcut {
        mods: mods_opt,
        key: code,
    })
}

fn parse_key_code(s: &str) -> Result<Code, String> {
    match s.to_lowercase().as_str() {
        "a" => Ok(Code::KeyA),
        "b" => Ok(Code::KeyB),
        "c" => Ok(Code::KeyC),
        "d" => Ok(Code::KeyD),
        "e" => Ok(Code::KeyE),
        "f" => Ok(Code::KeyF),
        "g" => Ok(Code::KeyG),
        "h" => Ok(Code::KeyH),
        "i" => Ok(Code::KeyI),
        "j" => Ok(Code::KeyJ),
        "k" => Ok(Code::KeyK),
        "l" => Ok(Code::KeyL),
        "m" => Ok(Code::KeyM),
        "n" => Ok(Code::KeyN),
        "o" => Ok(Code::KeyO),
        "p" => Ok(Code::KeyP),
        "q" => Ok(Code::KeyQ),
        "r" => Ok(Code::KeyR),
        "s" => Ok(Code::KeyS),
        "t" => Ok(Code::KeyT),
        "u" => Ok(Code::KeyU),
        "v" => Ok(Code::KeyV),
        "w" => Ok(Code::KeyW),
        "x" => Ok(Code::KeyX),
        "y" => Ok(Code::KeyY),
        "z" => Ok(Code::KeyZ),
        "0" | "digit0" => Ok(Code::Digit0),
        "1" | "digit1" => Ok(Code::Digit1),
        "2" | "digit2" => Ok(Code::Digit2),
        "3" | "digit3" => Ok(Code::Digit3),
        "4" | "digit4" => Ok(Code::Digit4),
        "5" | "digit5" => Ok(Code::Digit5),
        "6" | "digit6" => Ok(Code::Digit6),
        "7" | "digit7" => Ok(Code::Digit7),
        "8" | "digit8" => Ok(Code::Digit8),
        "9" | "digit9" => Ok(Code::Digit9),
        "space" => Ok(Code::Space),
        "enter" | "return" => Ok(Code::Enter),
        "escape" | "esc" => Ok(Code::Escape),
        "tab" => Ok(Code::Tab),
        "backspace" => Ok(Code::Backspace),
        "delete" => Ok(Code::Delete),
        "up" | "arrowup" => Ok(Code::ArrowUp),
        "down" | "arrowdown" => Ok(Code::ArrowDown),
        "left" | "arrowleft" => Ok(Code::ArrowLeft),
        "right" | "arrowright" => Ok(Code::ArrowRight),
        "f1" => Ok(Code::F1),
        "f2" => Ok(Code::F2),
        "f3" => Ok(Code::F3),
        "f4" => Ok(Code::F4),
        "f5" => Ok(Code::F5),
        "f6" => Ok(Code::F6),
        "f7" => Ok(Code::F7),
        "f8" => Ok(Code::F8),
        "f9" => Ok(Code::F9),
        "f10" => Ok(Code::F10),
        "f11" => Ok(Code::F11),
        "f12" => Ok(Code::F12),
        "-" | "minus" => Ok(Code::Minus),
        "=" | "equal" => Ok(Code::Equal),
        "[" | "bracketleft" => Ok(Code::BracketLeft),
        "]" | "bracketright" => Ok(Code::BracketRight),
        "\\" | "backslash" => Ok(Code::Backslash),
        ";" | "semicolon" => Ok(Code::Semicolon),
        "'" | "quote" => Ok(Code::Quote),
        "," | "comma" => Ok(Code::Comma),
        "." | "period" => Ok(Code::Period),
        "/" | "slash" => Ok(Code::Slash),
        "`" | "backquote" => Ok(Code::Backquote),
        _ => Err(format!("Unknown key: {s}")),
    }
}

// ── Tauri commands ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_shortcuts(
    registry: tauri::State<'_, SharedShortcutRegistry>,
) -> Result<Vec<ShortcutBinding>, String> {
    let reg = registry.read().await;
    Ok(reg.all_bindings())
}

#[tauri::command]
pub async fn rebind_shortcut(
    id: String,
    keys: String,
    app: tauri::AppHandle,
    registry: tauri::State<'_, SharedShortcutRegistry>,
) -> Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    let mut reg = registry.write().await;

    // Unregister old shortcut
    if let Some(binding) = reg.bindings.get(&id) {
        if let Ok(old) = parse_shortcut(&binding.keys) {
            let shortcut = Shortcut::new(old.mods, old.key);
            let _ = app.global_shortcut().unregister(shortcut);
        }
    }

    // Update binding
    reg.rebind(&id, &keys)?;

    // Register new shortcut
    if let Ok(new) = parse_shortcut(&keys) {
        let shortcut = Shortcut::new(new.mods, new.key);
        app.global_shortcut()
            .register(shortcut)
            .map_err(|e| format!("Failed to register shortcut: {e}"))?;
    }

    // Save user overrides to settings
    let overrides = reg.user_overrides();
    drop(reg);

    let mut settings = crate::settings::Settings::load();
    settings.shortcuts = overrides;
    settings.save()?;

    Ok(())
}
