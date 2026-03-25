//! Bitwarden integration — search vault items and copy passwords/usernames/TOTPs.
//!
//! Uses the `bw` CLI under the hood. Requires the user to have `bw` installed
//! and logged in (`bw login`). The vault must be unlocked per-session; we store
//! the session key in memory and auto-clear it after the configured timeout.
//!
//! Vault item metadata (names, usernames, URIs — NOT passwords) is persisted to
//! disk so items appear in search immediately after restart. When the user selects
//! a result while locked, they're prompted to unlock inline.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

use crate::launcher::CommandEntry;
use crate::providers::CommandProvider;

// ── Paths ───────────────────────────────────────────────────────────────────

fn config_dir() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("com.infamousvague.emit");
    std::fs::create_dir_all(&dir).ok();
    dir
}

fn vault_cache_path() -> PathBuf {
    config_dir().join("bw_vault_cache.json")
}

// ── Data types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultItem {
    pub id: String,
    pub name: String,
    pub username: String,
    pub uri: String,
    pub folder: String,
    pub item_type: String,
    pub has_totp: bool,
}

/// Cached credentials for an item (kept in memory only, never persisted).
#[derive(Debug, Clone)]
struct CachedCredentials {
    password: Option<String>,
    username: Option<String>,
    totp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BwItem {
    id: String,
    name: String,
    #[serde(rename = "type")]
    item_type: i32,
    folder_id: Option<String>,
    login: Option<BwLogin>,
    notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BwLogin {
    username: Option<String>,
    password: Option<String>,
    totp: Option<String>,
    uris: Option<Vec<BwUri>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BwUri {
    uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BwFolder {
    id: String,
    name: String,
}

// ── Session state ───────────────────────────────────────────────────────────

pub struct BitwardenSession {
    session_key: Option<String>,
    unlocked_at: Option<Instant>,
    lock_timeout_secs: u64,
    folders: Vec<BwFolder>,
    /// Full vault item cache (metadata for search — persisted to disk)
    item_cache: Vec<VaultItem>,
    /// Credential cache: item_id → credentials (in-memory only, never persisted)
    credential_cache: HashMap<String, CachedCredentials>,
    /// Whether the full vault has been loaded this session
    vault_loaded: bool,
}

impl BitwardenSession {
    /// Create a new session, loading any persisted vault metadata from disk.
    pub fn load() -> Self {
        let item_cache = load_vault_cache();
        let has_cache = !item_cache.is_empty();
        Self {
            session_key: None,
            unlocked_at: None,
            lock_timeout_secs: 14400,
            folders: Vec::new(),
            item_cache,
            credential_cache: HashMap::new(),
            vault_loaded: has_cache,
        }
    }

    pub fn is_unlocked(&self) -> bool {
        if self.session_key.is_none() {
            return false;
        }
        if self.lock_timeout_secs == 0 {
            return true;
        }
        if let Some(at) = self.unlocked_at {
            if at.elapsed().as_secs() > self.lock_timeout_secs {
                return false;
            }
        }
        true
    }

    /// Lock the vault — clears credentials but keeps item metadata for search.
    pub fn lock(&mut self) {
        self.session_key = None;
        self.unlocked_at = None;
        self.folders.clear();
        self.credential_cache.clear();
        // Keep item_cache and vault_loaded so items remain searchable
    }
}

pub type SharedBwSession = Arc<RwLock<BitwardenSession>>;

// ── Persistence ─────────────────────────────────────────────────────────────

fn save_vault_cache(items: &[VaultItem]) {
    if let Ok(json) = serde_json::to_string(items) {
        let _ = std::fs::write(vault_cache_path(), json);
    }
}

fn load_vault_cache() -> Vec<VaultItem> {
    match std::fs::read_to_string(vault_cache_path()) {
        Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

// ── CLI helpers ─────────────────────────────────────────────────────────────

fn bw_path() -> String {
    for path in &["/opt/homebrew/bin/bw", "/usr/local/bin/bw"] {
        if std::path::Path::new(path).exists() {
            return path.to_string();
        }
    }
    if let Ok(output) = Command::new("npm").args(["root", "-g"]).output() {
        if output.status.success() {
            let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let npm_bw = format!("{}/@bitwarden/cli/build/bw.js", root);
            if std::path::Path::new(&npm_bw).exists() {
                return npm_bw;
            }
        }
    }
    "bw".into()
}

fn run_bw(args: &[&str], session: Option<&str>) -> Result<String, String> {
    let bw = bw_path();
    let mut cmd = if bw.ends_with(".js") {
        let mut c = Command::new("node");
        c.arg(&bw);
        c
    } else {
        Command::new(&bw)
    };

    cmd.args(args);
    cmd.env("BW_NOINTERACTION", "true");

    if let Some(key) = session {
        cmd.env("BW_SESSION", key);
    }

    let output = cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "Bitwarden CLI (bw) not found. Install with: npm install -g @bitwarden/cli".into()
        } else {
            format!("Failed to run bw: {e}")
        }
    })?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
}

fn type_name(t: i32) -> &'static str {
    match t {
        1 => "login",
        2 => "secureNote",
        3 => "card",
        4 => "identity",
        _ => "unknown",
    }
}

/// Parse BwItems into VaultItems + CachedCredentials, given a folder map.
fn parse_bw_items(
    items: Vec<BwItem>,
    folder_map: &HashMap<String, String>,
) -> (Vec<VaultItem>, HashMap<String, CachedCredentials>) {
    let mut vault_items = Vec::with_capacity(items.len());
    let mut creds = HashMap::new();

    for item in items {
        let username = item.login.as_ref().and_then(|l| l.username.clone()).unwrap_or_default();
        let password = item.login.as_ref().and_then(|l| l.password.clone());
        let totp = item.login.as_ref().and_then(|l| l.totp.clone());
        let uri = item
            .login.as_ref()
            .and_then(|l| l.uris.as_ref())
            .and_then(|uris| uris.first())
            .and_then(|u| u.uri.clone())
            .unwrap_or_default();
        let has_totp = totp.as_ref().map(|t| !t.is_empty()).unwrap_or(false);
        let folder = item
            .folder_id.as_ref()
            .and_then(|fid| folder_map.get(fid))
            .cloned()
            .unwrap_or_else(|| "No Folder".into());

        creds.insert(
            item.id.clone(),
            CachedCredentials {
                password,
                username: Some(username.clone()),
                totp,
            },
        );

        vault_items.push(VaultItem {
            id: item.id,
            name: item.name,
            username,
            uri,
            folder,
            item_type: type_name(item.item_type).into(),
            has_totp,
        });
    }

    (vault_items, creds)
}

/// Load the entire vault into the session cache and persist metadata to disk.
fn load_vault_into_session(session: &mut BitwardenSession) -> Result<(), String> {
    let key = session.session_key.as_ref().ok_or("Not unlocked")?;
    let output = run_bw(&["list", "items"], Some(key))?;
    let items: Vec<BwItem> =
        serde_json::from_str(&output).map_err(|e| format!("Failed to parse items: {e}"))?;

    let folder_map: HashMap<String, String> = session
        .folders.iter()
        .map(|f| (f.id.clone(), f.name.clone()))
        .collect();

    let (vault_items, creds) = parse_bw_items(items, &folder_map);

    // Persist metadata (no passwords) to disk
    save_vault_cache(&vault_items);

    session.item_cache = vault_items;
    session.credential_cache = creds;
    session.vault_loaded = true;
    Ok(())
}

// ── Provider ────────────────────────────────────────────────────────────────

pub struct BitwardenProvider {
    session: SharedBwSession,
}

impl BitwardenProvider {
    pub fn new(session: SharedBwSession) -> Self {
        Self { session }
    }
}

#[async_trait]
impl CommandProvider for BitwardenProvider {
    fn name(&self) -> &str {
        "Bitwarden"
    }

    async fn commands(&self) -> Vec<CommandEntry> {
        vec![CommandEntry {
            id: "bitwarden.dashboard".into(),
            name: "Bitwarden".into(),
            description: "Search passwords, copy credentials, manage vault".into(),
            category: "Security".into(),
            icon: None,
            match_indices: vec![],
            score: 0,
        }]
    }

    fn is_dynamic(&self) -> bool {
        true
    }

    async fn search(&self, query: &str) -> Vec<CommandEntry> {
        let q = query.to_lowercase();
        let mut results = Vec::new();

        // Always show the dashboard entry for keyword matches
        let keywords = [
            "bitwarden", "password", "vault", "credential", "bw", "secret",
        ];
        let kw_match = keywords.iter().any(|k| k.starts_with(&q) || q.starts_with(k));
        if kw_match {
            results.push(CommandEntry {
                id: "bitwarden.dashboard".into(),
                name: "Bitwarden".into(),
                description: "Search passwords, copy credentials, manage vault".into(),
                category: "Security".into(),
                icon: None,
                match_indices: vec![],
                score: 90,
            });
        }

        // Search cached items (available even when locked — persisted from disk)
        if q.len() >= 2 {
            let s = self.session.read().await;
            if s.vault_loaded {
                let matching: Vec<_> = s.item_cache.iter()
                    .filter(|item| {
                        item.item_type == "login" && (
                            item.name.to_lowercase().contains(&q) ||
                            item.username.to_lowercase().contains(&q) ||
                            item.uri.to_lowercase().contains(&q)
                        )
                    })
                    .take(5)
                    .collect();

                for item in matching {
                    let desc = if item.username.is_empty() {
                        item.uri.clone()
                    } else if item.uri.is_empty() {
                        item.username.clone()
                    } else {
                        format!("{} · {}", item.username, item.uri)
                    };

                    results.push(CommandEntry {
                        id: format!("bitwarden.copy.{}", item.id),
                        name: item.name.clone(),
                        description: desc,
                        category: "Bitwarden".into(),
                        icon: None,
                        match_indices: vec![],
                        score: 95,
                    });
                }
            }
        }

        results
    }

    fn execute(&self, id: &str) -> Option<Result<String, String>> {
        if id == "bitwarden.dashboard" {
            return Some(Ok("view:bitwarden".into()));
        }

        if let Some(item_id) = id.strip_prefix("bitwarden.copy.") {
            // Return an action for the frontend to handle asynchronously.
            // We can't use blocking_read() here because we're inside a tokio runtime.
            Some(Ok(format!("action:bw_copy:{}", item_id)))
        } else {
            None
        }
    }
}

// ── Clipboard helper ────────────────────────────────────────────────────────

fn copy_to_clipboard(value: &str) -> Result<(), String> {
    let mut child = Command::new("pbcopy")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn pbcopy: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(value.as_bytes())
            .map_err(|e| format!("Failed to write to pbcopy: {e}"))?;
    }
    child.wait().map_err(|e| format!("pbcopy failed: {e}"))?;
    Ok(())
}

// ── Tauri commands ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn bw_status() -> Result<String, String> {
    let output = run_bw(&["status"], None)?;
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&output) {
        if let Some(status) = val.get("status").and_then(|s| s.as_str()) {
            return Ok(status.to_string());
        }
    }
    Ok(output)
}

#[tauri::command]
pub async fn bw_unlock(
    password: String,
    session: tauri::State<'_, SharedBwSession>,
) -> Result<(), String> {
    let output = run_bw(&["unlock", &password, "--raw"], None)?;
    let key = output.trim().to_string();
    if key.is_empty() {
        return Err("Unlock failed — no session key returned".into());
    }

    // Sync vault after unlock
    let _ = run_bw(&["sync"], Some(&key));

    // Load folders
    let folders = match run_bw(&["list", "folders"], Some(&key)) {
        Ok(json) => serde_json::from_str::<Vec<BwFolder>>(&json).unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    let mut s = session.write().await;
    s.session_key = Some(key);
    s.unlocked_at = Some(Instant::now());
    s.folders = folders;
    s.credential_cache.clear();

    // Load entire vault into cache and persist metadata
    if let Err(e) = load_vault_into_session(&mut s) {
        log::warn!("Failed to pre-load vault: {e}");
    }

    Ok(())
}

/// Unlock and immediately copy a specific item's password.
/// Used when the user selects a cached item while locked.
#[tauri::command]
pub async fn bw_unlock_and_copy(
    password: String,
    item_id: String,
    session: tauri::State<'_, SharedBwSession>,
) -> Result<String, String> {
    let output = run_bw(&["unlock", &password, "--raw"], None)?;
    let key = output.trim().to_string();
    if key.is_empty() {
        return Err("Unlock failed — no session key returned".into());
    }

    let _ = run_bw(&["sync"], Some(&key));

    let folders = match run_bw(&["list", "folders"], Some(&key)) {
        Ok(json) => serde_json::from_str::<Vec<BwFolder>>(&json).unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    let mut s = session.write().await;
    s.session_key = Some(key.clone());
    s.unlocked_at = Some(Instant::now());
    s.folders = folders;
    s.credential_cache.clear();

    if let Err(e) = load_vault_into_session(&mut s) {
        log::warn!("Failed to pre-load vault: {e}");
    }

    // Now copy the requested password
    if let Some(creds) = s.credential_cache.get(&item_id) {
        if let Some(ref pw) = creds.password {
            let pw = pw.clone();
            drop(s);
            copy_to_clipboard(&pw)?;
            return Ok("Password copied to clipboard".into());
        }
    }
    drop(s);

    // Fallback to CLI
    let pw = run_bw(&["get", "password", &item_id], Some(&key))?;
    copy_to_clipboard(&pw)?;
    Ok("Password copied to clipboard".into())
}

#[tauri::command]
pub async fn bw_lock(session: tauri::State<'_, SharedBwSession>) -> Result<(), String> {
    let mut s = session.write().await;
    if let Some(ref key) = s.session_key {
        let _ = run_bw(&["lock"], Some(key));
    }
    s.lock();
    Ok(())
}

#[tauri::command]
pub async fn bw_is_unlocked(session: tauri::State<'_, SharedBwSession>) -> Result<bool, String> {
    let s = session.read().await;
    Ok(s.is_unlocked())
}

#[tauri::command]
pub async fn bw_search(
    query: String,
    session: tauri::State<'_, SharedBwSession>,
) -> Result<Vec<VaultItem>, String> {
    let s = session.read().await;
    if !s.is_unlocked() {
        return Err("Vault is locked".into());
    }

    if s.vault_loaded {
        let q = query.to_lowercase();
        let filtered: Vec<VaultItem> = s.item_cache.iter()
            .filter(|item| {
                item.name.to_lowercase().contains(&q) ||
                item.username.to_lowercase().contains(&q) ||
                item.uri.to_lowercase().contains(&q)
            })
            .cloned()
            .collect();
        return Ok(filtered);
    }

    let key = s.session_key.as_ref().unwrap().clone();
    let folder_map: HashMap<String, String> =
        s.folders.iter().map(|f| (f.id.clone(), f.name.clone())).collect();
    drop(s);

    let output = run_bw(&["list", "items", "--search", &query], Some(&key))?;
    let items: Vec<BwItem> =
        serde_json::from_str(&output).map_err(|e| format!("Failed to parse items: {e}"))?;

    let (vault_items, new_creds) = parse_bw_items(items, &folder_map);

    let mut s = session.write().await;
    s.credential_cache.extend(new_creds);

    Ok(vault_items)
}

#[tauri::command]
pub async fn bw_get_password(
    item_id: String,
    session: tauri::State<'_, SharedBwSession>,
) -> Result<String, String> {
    let s = session.read().await;
    if !s.is_unlocked() {
        return Err("Vault is locked".into());
    }

    if let Some(creds) = s.credential_cache.get(&item_id) {
        if let Some(ref pw) = creds.password {
            return Ok(pw.clone());
        }
    }

    let key = s.session_key.as_ref().unwrap().clone();
    drop(s);
    run_bw(&["get", "password", &item_id], Some(&key))
}

#[tauri::command]
pub async fn bw_get_username(
    item_id: String,
    session: tauri::State<'_, SharedBwSession>,
) -> Result<String, String> {
    let s = session.read().await;
    if !s.is_unlocked() {
        return Err("Vault is locked".into());
    }

    if let Some(creds) = s.credential_cache.get(&item_id) {
        if let Some(ref un) = creds.username {
            return Ok(un.clone());
        }
    }

    let key = s.session_key.as_ref().unwrap().clone();
    drop(s);
    run_bw(&["get", "username", &item_id], Some(&key))
}

#[tauri::command]
pub async fn bw_get_totp(
    item_id: String,
    session: tauri::State<'_, SharedBwSession>,
) -> Result<String, String> {
    let s = session.read().await;
    if !s.is_unlocked() {
        return Err("Vault is locked".into());
    }
    let key = s.session_key.as_ref().unwrap().clone();
    drop(s);
    run_bw(&["get", "totp", &item_id], Some(&key))
}

#[tauri::command]
pub async fn bw_copy_to_clipboard(value: String) -> Result<(), String> {
    copy_to_clipboard(&value)
}

#[tauri::command]
pub async fn bw_get_lock_timeout(
    session: tauri::State<'_, SharedBwSession>,
) -> Result<u64, String> {
    let s = session.read().await;
    Ok(s.lock_timeout_secs)
}

#[tauri::command]
pub async fn bw_set_lock_timeout(
    seconds: u64,
    session: tauri::State<'_, SharedBwSession>,
) -> Result<(), String> {
    let mut s = session.write().await;
    s.lock_timeout_secs = seconds;
    if s.unlocked_at.is_some() {
        s.unlocked_at = Some(Instant::now());
    }
    Ok(())
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_name() {
        assert_eq!(type_name(1), "login");
        assert_eq!(type_name(2), "secureNote");
        assert_eq!(type_name(3), "card");
        assert_eq!(type_name(4), "identity");
        assert_eq!(type_name(99), "unknown");
    }

    #[test]
    fn test_bw_item_deserialize() {
        let json = r#"{
            "id": "abc-123",
            "name": "GitHub",
            "type": 1,
            "folderId": null,
            "login": {
                "username": "matt@example.com",
                "password": "secret123",
                "totp": null,
                "uris": [{"uri": "https://github.com"}]
            },
            "notes": null
        }"#;
        let item: BwItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.name, "GitHub");
        assert_eq!(item.login.as_ref().unwrap().password.as_deref(), Some("secret123"));
    }

    #[test]
    fn test_parse_bw_items() {
        let items = vec![BwItem {
            id: "abc".into(),
            name: "Test".into(),
            item_type: 1,
            folder_id: Some("f1".into()),
            login: Some(BwLogin {
                username: Some("user@test.com".into()),
                password: Some("pass".into()),
                totp: Some("otpauth://totp/test".into()),
                uris: Some(vec![BwUri { uri: Some("https://test.com".into()) }]),
            }),
            notes: None,
        }];

        let folder_map: HashMap<String, String> = [("f1".into(), "Work".into())].into();
        let (vault_items, creds) = parse_bw_items(items, &folder_map);

        assert_eq!(vault_items.len(), 1);
        assert_eq!(vault_items[0].name, "Test");
        assert_eq!(vault_items[0].folder, "Work");
        assert!(vault_items[0].has_totp);

        let c = creds.get("abc").unwrap();
        assert_eq!(c.password.as_deref(), Some("pass"));
    }

    #[test]
    fn test_session_load_empty() {
        // When no cache file exists, load() returns empty
        let session = BitwardenSession::load();
        assert!(!session.is_unlocked());
        assert_eq!(session.lock_timeout_secs, 14400);
    }

    #[test]
    fn test_session_never_lock() {
        let session = BitwardenSession {
            session_key: Some("key".into()),
            unlocked_at: Some(Instant::now()),
            lock_timeout_secs: 0,
            folders: Vec::new(),
            item_cache: Vec::new(),
            credential_cache: HashMap::new(),
            vault_loaded: false,
        };
        assert!(session.is_unlocked());
    }

    #[test]
    fn test_session_lock_keeps_item_cache() {
        let items = vec![VaultItem {
            id: "i1".into(), name: "Item".into(), username: "u".into(),
            uri: "u".into(), folder: "f".into(), item_type: "login".into(),
            has_totp: false,
        }];
        let mut session = BitwardenSession {
            session_key: Some("test-key".into()),
            unlocked_at: Some(Instant::now()),
            lock_timeout_secs: 900,
            folders: vec![BwFolder { id: "1".into(), name: "Test".into() }],
            item_cache: items,
            credential_cache: HashMap::from([(
                "i1".into(),
                CachedCredentials { password: Some("pw".into()), username: Some("u".into()), totp: None },
            )]),
            vault_loaded: true,
        };
        assert!(session.is_unlocked());
        session.lock();
        assert!(!session.is_unlocked());
        // item_cache preserved, credential_cache cleared
        assert_eq!(session.item_cache.len(), 1);
        assert!(session.credential_cache.is_empty());
        assert!(session.vault_loaded);
    }

    #[test]
    fn test_vault_cache_serialization() {
        // Test serialization round-trip without touching the production config dir
        let items = vec![VaultItem {
            id: "test-persist".into(),
            name: "Persisted Item".into(),
            username: "user@test.com".into(),
            uri: "https://test.com".into(),
            folder: "Work".into(),
            item_type: "login".into(),
            has_totp: false,
        }];

        let json = serde_json::to_string(&items).unwrap();
        let loaded: Vec<VaultItem> = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "Persisted Item");
        assert_eq!(loaded[0].username, "user@test.com");

        // VaultItem has no password field — only metadata
        assert!(!json.contains("password"));
    }

    #[test]
    fn test_item_cache_search_filter() {
        let items = vec![
            VaultItem {
                id: "1".into(), name: "GitHub".into(), username: "matt@gh.com".into(),
                uri: "https://github.com".into(), folder: "Dev".into(),
                item_type: "login".into(), has_totp: false,
            },
            VaultItem {
                id: "2".into(), name: "Gmail".into(), username: "matt@gmail.com".into(),
                uri: "https://gmail.com".into(), folder: "Personal".into(),
                item_type: "login".into(), has_totp: true,
            },
        ];

        let q = "git";
        let filtered: Vec<_> = items.iter()
            .filter(|item| {
                item.name.to_lowercase().contains(q) ||
                item.username.to_lowercase().contains(q) ||
                item.uri.to_lowercase().contains(q)
            })
            .collect();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "GitHub");
    }
}
