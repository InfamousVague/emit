use async_trait::async_trait;
use std::time::SystemTime;
use crate::launcher::CommandEntry;
use super::CommandProvider;

const MAX_FILE_RESULTS: usize = 15;

pub struct FileSearchProvider;

impl FileSearchProvider {
    pub fn new() -> Self {
        Self
    }
}

fn format_file_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn format_relative_time(time: SystemTime) -> String {
    let elapsed = time.elapsed().unwrap_or_default();
    let secs = elapsed.as_secs();
    if secs < 60 {
        "just now".into()
    } else if secs < 3600 {
        let mins = secs / 60;
        format!("{} min{} ago", mins, if mins == 1 { "" } else { "s" })
    } else if secs < 86400 {
        let hours = secs / 3600;
        format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" })
    } else if secs < 86400 * 30 {
        let days = secs / 86400;
        format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
    } else if secs < 86400 * 365 {
        let months = secs / (86400 * 30);
        format!("{} month{} ago", months, if months == 1 { "" } else { "s" })
    } else {
        let years = secs / (86400 * 365);
        format!("{} year{} ago", years, if years == 1 { "" } else { "s" })
    }
}

#[async_trait]
impl CommandProvider for FileSearchProvider {
    fn name(&self) -> &str {
        "Files"
    }

    async fn commands(&self) -> Vec<CommandEntry> {
        vec![]
    }

    fn is_dynamic(&self) -> bool {
        true
    }

    async fn search(&self, query: &str) -> Vec<CommandEntry> {
        if query.len() < 2 {
            return vec![];
        }

        let home = dirs::home_dir()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".into());

        let output = tokio::process::Command::new("mdfind")
            .args(["-name", query, "-onlyin", &home])
            .output()
            .await;

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                stdout
                    .lines()
                    .filter(|line| !line.is_empty() && line.starts_with('/'))
                    .take(MAX_FILE_RESULTS)
                    .map(|path| {
                        let file_name = std::path::Path::new(path)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| path.to_string());

                        let description = match std::fs::metadata(path) {
                            Ok(meta) => {
                                let size = format_file_size(meta.len());
                                let modified = meta
                                    .modified()
                                    .ok()
                                    .map(|t| format_relative_time(t))
                                    .unwrap_or_default();
                                if modified.is_empty() {
                                    format!("{} · {}", path, size)
                                } else {
                                    format!("{} · {} · {}", path, size, modified)
                                }
                            }
                            Err(_) => path.to_string(),
                        };

                        CommandEntry {
                            id: format!("file.{}", path.replace('/', ":")),
                            name: file_name,
                            description,
                            category: "Files".into(),
                            icon: None,
                            match_indices: vec![],
                            score: 0,
                        }
                    })
                    .collect()
            }
            Err(_) => vec![],
        }
    }

    fn execute(&self, id: &str) -> Option<Result<String, String>> {
        if !id.starts_with("file.") {
            return None;
        }
        let path = id.strip_prefix("file.")?.replace(':', "/");
        Some(
            std::process::Command::new("open")
                .arg(&path)
                .spawn()
                .map(|_| format!("Opened {path}"))
                .map_err(|e| e.to_string()),
        )
    }
}
