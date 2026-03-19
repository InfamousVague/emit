use async_trait::async_trait;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::icons;
use crate::launcher::CommandEntry;
use super::CommandProvider;

/// Standard macOS application directories to scan.
const APP_DIRS: &[&str] = &[
    "/Applications",
    "/System/Applications",
    "/System/Applications/Utilities",
];

/// Tracks an application entry alongside its filesystem path for icon extraction.
struct AppEntry {
    command: CommandEntry,
    path: PathBuf,
}

pub struct ApplicationProvider {
    apps: Vec<AppEntry>,
}

impl ApplicationProvider {
    pub fn new() -> Self {
        let mut apps = Vec::new();
        let mut seen_ids = HashSet::new();

        for dir in APP_DIRS {
            scan_dir(Path::new(dir), &mut apps, &mut seen_ids);
        }
        if let Some(home) = dirs::home_dir() {
            scan_dir(&home.join("Applications"), &mut apps, &mut seen_ids);
        }

        apps.sort_by(|a, b| a.command.name.cmp(&b.command.name));
        Self { apps }
    }
}

fn scan_dir(dir: &Path, apps: &mut Vec<AppEntry>, seen: &mut HashSet<String>) {
    if !dir.exists() {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.ends_with(".app") {
                let app_name = name_str.trim_end_matches(".app").to_string();
                let id = format!(
                    "app.{}",
                    app_name.to_lowercase().replace(' ', "-").replace('.', "-")
                );

                // Skip duplicates across directories
                if !seen.insert(id.clone()) {
                    continue;
                }

                apps.push(AppEntry {
                    command: CommandEntry {
                        id,
                        name: app_name.clone(),
                        description: format!("Open {app_name}"),
                        category: "Applications".into(),
                        icon: None, match_indices: vec![], score: 0,
                    },
                    path: entry.path(),
                });
            }
        }
    }
}

#[async_trait]
impl CommandProvider for ApplicationProvider {
    fn name(&self) -> &str {
        "Applications"
    }

    async fn commands(&self) -> Vec<CommandEntry> {
        self.apps.iter().map(|a| a.command.clone()).collect()
    }

    fn execute(&self, id: &str) -> Option<Result<String, String>> {
        if !id.starts_with("app.") {
            return None;
        }
        let app = self.apps.iter().find(|a| a.command.id == id)?;
        Some(
            std::process::Command::new("open")
                .arg("-a")
                .arg(&app.command.name)
                .spawn()
                .map(|_| format!("Opened {}", app.command.name))
                .map_err(|e| e.to_string()),
        )
    }

    async fn enrich_icons(&mut self) {
        for app in &mut self.apps {
            if app.command.icon.is_some() {
                continue;
            }
            if let Some(data_uri) = icons::extract_icon(&app.path, &app.command.name).await {
                app.command.icon = Some(data_uri);
            }
        }
    }
}
