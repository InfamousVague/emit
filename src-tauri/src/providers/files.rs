use async_trait::async_trait;
use crate::launcher::CommandEntry;
use super::CommandProvider;

const MAX_FILE_RESULTS: usize = 8;

pub struct FileSearchProvider;

impl FileSearchProvider {
    pub fn new() -> Self {
        Self
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
                        CommandEntry {
                            id: format!("file.{}", path.replace('/', ":")),
                            name: file_name,
                            description: path.to_string(),
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
