use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::sync::RwLock;
use zeroize::Zeroize;

use super::vault_crypto::{
    decrypt_vault, encrypt_vault, reencrypt_vault, PasswordHistoryEntry, VaultData,
};
use crate::launcher::CommandEntry;
use crate::providers::CommandProvider;

// ── Paths ────────────────────────────────────────────────────────────────────

fn config_dir() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("com.infamousvague.emit");
    std::fs::create_dir_all(&dir).ok();
    dir
}

fn vault_path() -> PathBuf {
    config_dir().join("vault.enc")
}

// ── Session state ────────────────────────────────────────────────────────────

pub struct VaultSession {
    vault_key: Option<Vec<u8>>,
    salt: Option<Vec<u8>>,
    passwords: Vec<PasswordHistoryEntry>,
    unlocked_at: Option<Instant>,
    lock_timeout_secs: u64,
}

impl Default for VaultSession {
    fn default() -> Self {
        Self {
            vault_key: None,
            salt: None,
            passwords: Vec::new(),
            unlocked_at: None,
            lock_timeout_secs: 300,
        }
    }
}

impl VaultSession {
    pub fn is_unlocked(&self) -> bool {
        if self.vault_key.is_none() {
            return false;
        }
        if let Some(at) = self.unlocked_at {
            if at.elapsed().as_secs() > self.lock_timeout_secs {
                return false;
            }
        }
        true
    }

    pub fn check_auto_lock(&mut self) {
        if let Some(at) = self.unlocked_at {
            if at.elapsed().as_secs() > self.lock_timeout_secs {
                self.lock();
            }
        }
    }

    pub fn lock(&mut self) {
        if let Some(ref mut key) = self.vault_key {
            key.zeroize();
        }
        self.vault_key = None;
        self.salt = None;
        self.passwords.clear();
        self.unlocked_at = None;
    }

    fn touch(&mut self) {
        self.unlocked_at = Some(Instant::now());
    }
}

pub type SharedVaultSession = Arc<RwLock<VaultSession>>;

// ── Vault file I/O ──────────────────────────────────────────────────────────

fn save_vault(session: &VaultSession) -> Result<(), String> {
    let key = session.vault_key.as_ref().ok_or("Vault is locked")?;
    let salt = session.salt.as_ref().ok_or("No salt available")?;

    let data = VaultData {
        passwords: session.passwords.clone(),
        version: 1,
    };

    let encrypted = reencrypt_vault(&data, key, salt)?;
    std::fs::write(vault_path(), &encrypted)
        .map_err(|e| format!("Failed to write vault: {e}"))?;

    Ok(())
}

// ── Provider ─────────────────────────────────────────────────────────────────

pub struct PasswordGeneratorProvider;

impl PasswordGeneratorProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl CommandProvider for PasswordGeneratorProvider {
    fn name(&self) -> &str {
        "PasswordGenerator"
    }

    async fn commands(&self) -> Vec<CommandEntry> {
        vec![CommandEntry {
            id: "pwgen.open".into(),
            name: "Password Generator".into(),
            description: "Generate secure passwords with encrypted history".into(),
            category: "Security".into(),
            icon: None,
            match_indices: vec![],
            score: 0,
        }]
    }

    fn execute(&self, id: &str) -> Option<Result<String, String>> {
        if id == "pwgen.open" {
            Some(Ok("view:password-generator".into()))
        } else {
            None
        }
    }

    fn is_dynamic(&self) -> bool {
        false
    }

    async fn search(&self, _query: &str) -> Vec<CommandEntry> {
        Vec::new()
    }
}

// ── Tauri commands ───────────────────────────────────────────────────────────

#[tauri::command]
pub fn pwgen_has_vault() -> bool {
    vault_path().exists()
}

#[tauri::command]
pub async fn pwgen_setup(
    password: String,
    session: tauri::State<'_, SharedVaultSession>,
) -> Result<(), String> {
    if vault_path().exists() {
        return Err("Vault already exists".into());
    }

    let vault = VaultData::default();
    let encrypted = encrypt_vault(&vault, &password)?;
    std::fs::write(vault_path(), &encrypted)
        .map_err(|e| format!("Failed to write vault: {e}"))?;

    // Auto-unlock after setup
    let salt = encrypted[..32].to_vec();
    let key = super::vault_crypto::derive_key(&password, &salt)?;

    let mut s = session.write().await;
    s.vault_key = Some(key);
    s.salt = Some(salt);
    s.passwords = Vec::new();
    s.touch();

    Ok(())
}

#[tauri::command]
pub async fn pwgen_unlock(
    password: String,
    session: tauri::State<'_, SharedVaultSession>,
) -> Result<(), String> {
    let data = std::fs::read(vault_path())
        .map_err(|e| format!("Failed to read vault: {e}"))?;

    let salt = data[..32].to_vec();
    let (vault, key) = decrypt_vault(&data, &password)?;

    let mut s = session.write().await;
    s.vault_key = Some(key);
    s.salt = Some(salt);
    s.passwords = vault.passwords;
    s.touch();

    log::info!("Vault unlocked ({} passwords in history)", s.passwords.len());
    Ok(())
}

#[tauri::command]
pub async fn pwgen_lock(session: tauri::State<'_, SharedVaultSession>) -> Result<(), String> {
    let mut s = session.write().await;
    s.lock();
    log::info!("Vault locked");
    Ok(())
}

#[tauri::command]
pub async fn pwgen_is_unlocked(
    session: tauri::State<'_, SharedVaultSession>,
) -> Result<bool, String> {
    let mut s = session.write().await;
    s.check_auto_lock();
    Ok(s.is_unlocked())
}

// ── Password generation ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct GeneratePasswordOpts {
    pub length: usize,
    pub uppercase: bool,
    pub lowercase: bool,
    pub numbers: bool,
    pub symbols: bool,
    pub passphrase: bool,
    pub word_count: Option<usize>,
    pub separator: Option<String>,
    #[allow(dead_code)]
    pub label: Option<String>,
}

/// Generate-only: returns the password string without saving to history.
#[tauri::command]
pub fn pwgen_generate(opts: GeneratePasswordOpts) -> Result<String, String> {
    generate_password_string(&opts)
}

/// Save an already-generated password to the encrypted history.
#[tauri::command]
pub async fn pwgen_save_to_history(
    password: String,
    mode: String,
    length: usize,
    label: Option<String>,
    session: tauri::State<'_, SharedVaultSession>,
) -> Result<PasswordHistoryEntry, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let entry = PasswordHistoryEntry {
        id: uuid::Uuid::new_v4().to_string(),
        password,
        generated_at: now,
        label,
        mode,
        length,
    };

    let mut s = session.write().await;
    s.check_auto_lock();

    if !s.is_unlocked() {
        return Err("Vault is locked".into());
    }

    s.passwords.insert(0, entry.clone());
    s.touch();
    save_vault(&s)?;

    Ok(entry)
}

fn generate_password_string(opts: &GeneratePasswordOpts) -> Result<String, String> {
    use rand::Rng;

    if opts.passphrase {
        let word_count = opts.word_count.unwrap_or(4);
        let separator = opts.separator.as_deref().unwrap_or("-");
        let words = generate_passphrase(word_count);
        return Ok(words.join(separator));
    }

    let mut charset = String::new();
    if opts.lowercase {
        charset.push_str("abcdefghijklmnopqrstuvwxyz");
    }
    if opts.uppercase {
        charset.push_str("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
    }
    if opts.numbers {
        charset.push_str("0123456789");
    }
    if opts.symbols {
        charset.push_str("!@#$%^&*()_+-=[]{}|;:,.<>?");
    }

    if charset.is_empty() {
        return Err("Select at least one character type".into());
    }

    let chars: Vec<char> = charset.chars().collect();
    let mut rng = rand::thread_rng();
    let password: String = (0..opts.length)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect();

    Ok(password)
}

fn generate_passphrase(word_count: usize) -> Vec<String> {
    use rand::seq::SliceRandom;

    const WORDS: &[&str] = &[
        "acid", "acme", "aged", "also", "arch", "army", "atom", "aunt",
        "baby", "back", "bail", "bake", "ball", "band", "bank", "barn",
        "base", "bath", "bead", "beam", "bean", "bear", "beat", "beef",
        "bell", "belt", "bird", "bite", "blow", "blue", "blur", "boat",
        "body", "bold", "bolt", "bomb", "bond", "bone", "book", "boot",
        "born", "boss", "both", "bowl", "bulk", "bump", "burn", "bush",
        "busy", "buzz", "cafe", "cage", "cake", "calm", "camp", "cape",
        "card", "care", "cart", "case", "cash", "cast", "cave", "cell",
        "chat", "chef", "chin", "chip", "chop", "city", "clad", "clam",
        "clap", "clay", "clip", "club", "clue", "coat", "code", "coil",
        "coin", "cold", "cole", "colt", "comb", "come", "cook", "cool",
        "cope", "copy", "cord", "core", "cork", "corn", "cost", "cozy",
        "crew", "crop", "crow", "cube", "cult", "cure", "curl", "cute",
        "dare", "dark", "dart", "dash", "data", "dawn", "deal", "dean",
        "dear", "debt", "deck", "deep", "deer", "demo", "dent", "deny",
        "desk", "dial", "dice", "diet", "dine", "dirt", "dish", "disk",
        "dock", "does", "doll", "dome", "done", "doom", "door", "dose",
        "dove", "down", "draw", "drip", "drop", "drum", "dual", "duck",
        "dull", "dumb", "dump", "dune", "dusk", "dust", "duty", "each",
        "earn", "ease", "east", "echo", "edge", "edit", "else", "emit",
        "envy", "epic", "even", "evil", "exam", "exit", "face", "fact",
        "fade", "fail", "fair", "fake", "fall", "fame", "farm", "fast",
        "fate", "fear", "feat", "feed", "feel", "feet", "fell", "felt",
        "file", "fill", "film", "find", "fine", "fire", "firm", "fish",
        "fist", "five", "flag", "flat", "flee", "flew", "flip", "flock",
        "flow", "foam", "fold", "folk", "fond", "font", "food", "fool",
        "foot", "fork", "form", "fort", "foul", "four", "free", "from",
        "fuel", "full", "fund", "fury", "fuse", "fuss", "gain", "gala",
        "gale", "game", "gang", "gate", "gave", "gaze", "gear", "gene",
        "gift", "girl", "give", "glad", "glow", "glue", "goat", "goes",
        "gold", "golf", "gone", "good", "grab", "gray", "grew", "grid",
        "grim", "grin", "grip", "grow", "gulf", "guru", "gust", "hack",
        "hail", "hair", "half", "hall", "halt", "hand", "hang", "hard",
        "harm", "harp", "hate", "have", "hawk", "haze", "head", "heal",
        "heap", "hear", "heat", "heel", "held", "help", "herb", "herd",
        "here", "hero", "hide", "high", "hike", "hill", "hint", "hire",
        "hold", "hole", "holy", "home", "hood", "hook", "hope", "horn",
        "host", "hour", "huge", "hull", "hung", "hunt", "hurt", "hymn",
        "icon", "idea", "inch", "info", "iron", "isle", "item", "jade",
        "jail", "jazz", "jean", "jerk", "jest", "jobs", "join", "joke",
        "jump", "june", "jury", "just", "keen", "keep", "kept", "kick",
        "kill", "kind", "king", "kiss", "kite", "knee", "knew", "knit",
        "knob", "knot", "know", "lace", "lack", "laid", "lake", "lamp",
        "land", "lane", "last", "late", "lawn", "lazy", "lead", "leaf",
        "lean", "leap", "left", "lend", "lens", "less", "lick", "life",
        "lift", "like", "limb", "lime", "limp", "line", "link", "lion",
        "list", "live", "load", "loaf", "loan", "lock", "loft", "logo",
        "lone", "long", "look", "lord", "lose", "loss", "lost", "loud",
        "love", "luck", "lump", "lung", "lure", "lurk", "lush", "made",
        "mail", "main", "make", "male", "mall", "malt", "mane", "many",
        "mark", "mask", "mass", "mate", "maze", "meal", "mean", "meat",
        "meet", "melt", "memo", "menu", "mere", "mesh", "mess", "mild",
        "milk", "mill", "mind", "mine", "mint", "miss", "mist", "mode",
        "mold", "mood", "moon", "more", "moss", "most", "moth", "move",
        "much", "mule", "muse", "must", "myth", "nail", "name", "navy",
        "near", "neat", "neck", "need", "nest", "next", "nice", "nine",
        "node", "none", "noon", "norm", "nose", "note", "noun", "odds",
        "once", "only", "onto", "open", "oral", "oven", "over", "owed",
        "pace", "pack", "page", "paid", "pain", "pair", "pale", "palm",
    ];

    let mut rng = rand::thread_rng();
    (0..word_count)
        .map(|_| WORDS.choose(&mut rng).unwrap_or(&"word").to_string())
        .collect()
}

// ── History management ───────────────────────────────────────────────────────

#[tauri::command]
pub async fn pwgen_get_history(
    session: tauri::State<'_, SharedVaultSession>,
) -> Result<Vec<PasswordHistoryEntry>, String> {
    let mut s = session.write().await;
    s.check_auto_lock();

    if !s.is_unlocked() {
        return Err("Vault is locked".into());
    }

    s.touch();
    Ok(s.passwords.clone())
}

#[tauri::command]
pub async fn pwgen_delete_history_entry(
    id: String,
    session: tauri::State<'_, SharedVaultSession>,
) -> Result<(), String> {
    let mut s = session.write().await;
    s.check_auto_lock();

    if !s.is_unlocked() {
        return Err("Vault is locked".into());
    }

    s.passwords.retain(|e| e.id != id);
    s.touch();
    save_vault(&s)?;
    Ok(())
}

#[tauri::command]
pub async fn pwgen_clear_history(
    session: tauri::State<'_, SharedVaultSession>,
) -> Result<(), String> {
    let mut s = session.write().await;
    s.check_auto_lock();

    if !s.is_unlocked() {
        return Err("Vault is locked".into());
    }

    s.passwords.clear();
    s.touch();
    save_vault(&s)?;
    Ok(())
}

#[tauri::command]
pub async fn pwgen_copy_password(
    id: String,
    session: tauri::State<'_, SharedVaultSession>,
) -> Result<String, String> {
    let mut s = session.write().await;
    s.check_auto_lock();

    if !s.is_unlocked() {
        return Err("Vault is locked".into());
    }

    let pw = s
        .passwords
        .iter()
        .find(|e| e.id == id)
        .ok_or("Entry not found")?
        .password
        .clone();

    s.touch();
    Ok(pw)
}

// ── Lock timeout settings ───────────────────────────────────────────────────

#[tauri::command]
pub async fn pwgen_set_lock_timeout(
    seconds: u64,
    session: tauri::State<'_, SharedVaultSession>,
) -> Result<(), String> {
    let mut s = session.write().await;
    s.lock_timeout_secs = seconds;
    Ok(())
}

#[tauri::command]
pub async fn pwgen_get_lock_timeout(
    session: tauri::State<'_, SharedVaultSession>,
) -> Result<u64, String> {
    let s = session.read().await;
    Ok(s.lock_timeout_secs)
}
