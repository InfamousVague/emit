//! Env Vault — recursively discover all .env files across your development
//! directories and display them in a flat, searchable list.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::launcher::CommandEntry;
use crate::providers::CommandProvider;

// ── Data types ──────────────────────────────────────────────────────────────

/// A single discovered .env file anywhere in the scan tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvFile {
    /// Full absolute path to the .env file.
    pub file_path: String,
    /// Filename (e.g. ".env", ".env.production").
    pub filename: String,
    /// Friendly environment label (e.g. "Default", "Production").
    pub env_label: String,
    /// Name of the project (parent directory name).
    pub project: String,
    /// Relative path from the scan root to the file's parent directory.
    pub relative_dir: String,
    /// Number of variables in the file.
    pub var_count: usize,
    /// The parsed variables.
    pub variables: Vec<EnvVariable>,
}

/// A single environment variable entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVariable {
    pub key: String,
    pub value: String,
}

/// Configuration for scan directories.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnvVaultConfig {
    pub scan_dirs: Vec<String>,
}

// ── Provider ────────────────────────────────────────────────────────────────

pub struct EnvVaultProvider;

impl EnvVaultProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl CommandProvider for EnvVaultProvider {
    fn name(&self) -> &str {
        "EnvVault"
    }

    async fn commands(&self) -> Vec<CommandEntry> {
        vec![CommandEntry {
            id: "env-vault.open".into(),
            name: "Env Vault".into(),
            description: "Manage .env files across projects".into(),
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
        let keywords = [
            "env", "environment", "dotenv", ".env", "vault", "secret",
            "variable", "config", "staging", "production",
        ];

        if !keywords.iter().any(|k| k.starts_with(&q) || q.starts_with(k)) {
            return vec![];
        }

        let config = load_config();
        let file_count: usize = config
            .scan_dirs
            .iter()
            .filter_map(|d| deep_scan(d).ok())
            .map(|files| files.len())
            .sum();

        vec![CommandEntry {
            id: "env-vault.open".into(),
            name: "Env Vault".into(),
            description: format!("{} env files found", file_count),
            category: "Security".into(),
            icon: None,
            match_indices: vec![],
            score: 80,
        }]
    }

    fn execute(&self, id: &str) -> Option<Result<String, String>> {
        match id {
            "env-vault.open" => Some(Ok("view:env-vault".into())),
            _ => None,
        }
    }
}

// ── Config persistence ──────────────────────────────────────────────────────

fn config_dir() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("com.infamousvague.emit");
    std::fs::create_dir_all(&dir).ok();
    dir
}

fn config_path() -> PathBuf {
    config_dir().join("env_vault.json")
}

fn load_config() -> EnvVaultConfig {
    std::fs::read_to_string(config_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_config(config: &EnvVaultConfig) -> Result<(), String> {
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {e}"))?;
    std::fs::write(config_path(), json)
        .map_err(|e| format!("Failed to write config: {e}"))?;
    Ok(())
}

// ── .env file parsing ───────────────────────────────────────────────────────

/// Parse a .env file into key-value pairs, respecting comments and blank lines.
fn parse_env_file(path: &Path) -> Result<Vec<EnvVariable>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;

    let mut vars = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim().to_string();
            let value = value
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            vars.push(EnvVariable { key, value });
        }
    }

    Ok(vars)
}

/// Map filename to a friendly environment label.
fn env_label(filename: &str) -> String {
    match filename {
        ".env" => "Default".into(),
        ".env.local" => "Local".into(),
        ".env.development" | ".env.dev" => "Development".into(),
        ".env.staging" => "Staging".into(),
        ".env.production" | ".env.prod" => "Production".into(),
        ".env.test" => "Test".into(),
        ".env.example" => "Example".into(),
        other => other.strip_prefix(".env.").unwrap_or(other).to_string(),
    }
}

/// Return true if a filename looks like an env file.
fn is_env_file(filename: &str) -> bool {
    filename == ".env" || filename.starts_with(".env.")
}

// ── Directories to skip during recursive walk ───────────────────────────────

const SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    "dist",
    "build",
    ".next",
    ".nuxt",
    ".turbo",
    ".cache",
    "__pycache__",
    ".venv",
    "venv",
    ".tox",
    "vendor",
    "Pods",
    ".gradle",
    ".idea",
    ".vscode",
];

// ── Deep recursive scanning ─────────────────────────────────────────────────

/// Recursively walk a directory tree and collect every .env* file.
fn deep_scan(root: &str) -> Result<Vec<EnvFile>, String> {
    let root_path = Path::new(root);
    if !root_path.exists() {
        return Err(format!("Directory not found: {root}"));
    }

    let mut results = Vec::new();
    walk_dir(root_path, root_path, &mut results, 0);
    results.sort_by(|a, b| a.file_path.cmp(&b.file_path));
    Ok(results)
}

/// Recursive directory walker with depth limit to avoid runaway traversal.
fn walk_dir(dir: &Path, root: &Path, out: &mut Vec<EnvFile>, depth: usize) {
    // Safety: don't recurse deeper than 8 levels
    if depth > 8 {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry
            .file_name()
            .to_string_lossy()
            .to_string();

        if path.is_dir() {
            // Skip common noise directories
            if SKIP_DIRS.contains(&name.as_str()) || name.starts_with('.') && name != ".env" {
                continue;
            }
            walk_dir(&path, root, out, depth + 1);
        } else if is_env_file(&name) {
            let variables = parse_env_file(&path).unwrap_or_default();
            let var_count = variables.len();

            let project = dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let relative_dir = dir
                .strip_prefix(root)
                .unwrap_or(dir)
                .to_string_lossy()
                .to_string();

            out.push(EnvFile {
                file_path: path.to_string_lossy().to_string(),
                filename: name.clone(),
                env_label: env_label(&name),
                project,
                relative_dir,
                var_count,
                variables,
            });
        }
    }
}

// ── Tauri commands ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn env_vault_get_config() -> Result<EnvVaultConfig, String> {
    Ok(load_config())
}

#[tauri::command]
pub async fn env_vault_save_config(config: EnvVaultConfig) -> Result<(), String> {
    save_config(&config)
}

#[tauri::command]
pub async fn env_vault_scan() -> Result<Vec<EnvFile>, String> {
    let config = load_config();
    let mut all_files = Vec::new();
    for dir in &config.scan_dirs {
        match deep_scan(dir) {
            Ok(files) => all_files.extend(files),
            Err(e) => log::warn!("Failed to scan {dir}: {e}"),
        }
    }
    Ok(all_files)
}

#[tauri::command]
pub async fn env_vault_read_file(file_path: String) -> Result<Vec<EnvVariable>, String> {
    parse_env_file(Path::new(&file_path))
}

/// Update a single variable in a specific .env file (in place).
#[tauri::command]
pub async fn env_vault_update_var(
    file_path: String,
    key: String,
    value: String,
) -> Result<(), String> {
    let path = Path::new(&file_path);
    if !path.exists() {
        return Err(format!("File not found: {file_path}"));
    }

    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {e}"))?;

    let mut found = false;
    let lines: Vec<String> = content
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            if let Some((k, _)) = trimmed.split_once('=') {
                if k.trim() == key {
                    found = true;
                    return format!("{key}={value}");
                }
            }
            line.to_string()
        })
        .collect();

    if !found {
        return Err(format!("Variable {key} not found in {file_path}"));
    }

    let mut output = lines.join("\n");
    if !output.ends_with('\n') {
        output.push('\n');
    }

    std::fs::write(path, output)
        .map_err(|e| format!("Failed to write file: {e}"))?;

    Ok(())
}

#[tauri::command]
pub async fn env_vault_open_dir(dir_path: String) -> Result<(), String> {
    std::process::Command::new("open")
        .arg(&dir_path)
        .spawn()
        .map_err(|e| format!("Failed to open directory: {e}"))?;
    Ok(())
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn test_parse_env_file() {
        let dir = temp_dir();
        let path = dir.path().join(".env");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "# Comment").unwrap();
        writeln!(f, "").unwrap();
        writeln!(f, "API_KEY=abc123").unwrap();
        writeln!(f, "DB_URL=\"postgres://localhost\"").unwrap();
        writeln!(f, "SECRET='my_secret'").unwrap();

        let vars = parse_env_file(&path).unwrap();
        assert_eq!(vars.len(), 3);
        assert_eq!(vars[0].key, "API_KEY");
        assert_eq!(vars[0].value, "abc123");
        assert_eq!(vars[1].key, "DB_URL");
        assert_eq!(vars[1].value, "postgres://localhost");
        assert_eq!(vars[2].key, "SECRET");
        assert_eq!(vars[2].value, "my_secret");
    }

    #[test]
    fn test_env_label() {
        assert_eq!(env_label(".env"), "Default");
        assert_eq!(env_label(".env.production"), "Production");
        assert_eq!(env_label(".env.staging"), "Staging");
        assert_eq!(env_label(".env.example"), "Example");
        assert_eq!(env_label(".env.custom"), "custom");
    }

    #[test]
    fn test_is_env_file() {
        assert!(is_env_file(".env"));
        assert!(is_env_file(".env.production"));
        assert!(is_env_file(".env.local"));
        assert!(is_env_file(".env.example"));
        assert!(!is_env_file("README.md"));
        assert!(!is_env_file(".gitignore"));
        assert!(!is_env_file("package.json"));
    }

    #[test]
    fn test_deep_scan() {
        let dir = temp_dir();

        // Project A at top level
        let proj_a = dir.path().join("project-a");
        std::fs::create_dir(&proj_a).unwrap();
        std::fs::write(proj_a.join(".env"), "A=1\n").unwrap();
        std::fs::write(proj_a.join(".env.production"), "A=prod\n").unwrap();

        // Project B nested one level deep
        let nested = dir.path().join("apps");
        let proj_b = nested.join("project-b");
        std::fs::create_dir_all(&proj_b).unwrap();
        std::fs::write(proj_b.join(".env"), "B=2\nC=3\n").unwrap();

        // node_modules should be skipped
        let nm = dir.path().join("project-a").join("node_modules");
        std::fs::create_dir_all(&nm).unwrap();
        std::fs::write(nm.join(".env"), "SKIP=me\n").unwrap();

        let files = deep_scan(&dir.path().to_string_lossy()).unwrap();

        // Should find 3 files: project-a/.env, project-a/.env.production, apps/project-b/.env
        assert_eq!(files.len(), 3);

        // Verify project names
        let projects: Vec<&str> = files.iter().map(|f| f.project.as_str()).collect();
        assert!(projects.contains(&"project-a"));
        assert!(projects.contains(&"project-b"));

        // Verify node_modules was skipped
        assert!(!files.iter().any(|f| f.file_path.contains("node_modules")));
    }

    #[test]
    fn test_deep_scan_var_counts() {
        let dir = temp_dir();
        let proj = dir.path().join("my-app");
        std::fs::create_dir(&proj).unwrap();
        std::fs::write(proj.join(".env"), "A=1\nB=2\nC=3\n").unwrap();

        let files = deep_scan(&dir.path().to_string_lossy()).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].var_count, 3);
        assert_eq!(files[0].variables.len(), 3);
        assert_eq!(files[0].env_label, "Default");
        assert_eq!(files[0].project, "my-app");
    }

    #[test]
    fn test_provider_basics() {
        let provider = EnvVaultProvider::new();
        assert_eq!(provider.name(), "EnvVault");
        assert_eq!(
            provider.execute("env-vault.open"),
            Some(Ok("view:env-vault".into()))
        );
        assert!(provider.execute("unknown").is_none());
    }
}
