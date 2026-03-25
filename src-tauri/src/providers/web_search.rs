use async_trait::async_trait;
use crate::launcher::CommandEntry;
use super::CommandProvider;

pub struct WebSearchProvider;

impl WebSearchProvider {
    pub fn new() -> Self { Self }
}

fn url_encode(s: &str) -> String {
    s.bytes().map(|b| match b {
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
            String::from(b as char)
        }
        b' ' => "+".into(),
        _ => format!("%{:02X}", b),
    }).collect()
}

const ENGINES: &[(&str, &str, &str)] = &[
    ("google", "Google", "https://www.google.com/search?q="),
    ("duckduckgo", "DuckDuckGo", "https://duckduckgo.com/?q="),
    ("stackoverflow", "Stack Overflow", "https://stackoverflow.com/search?q="),
    ("github", "GitHub", "https://github.com/search?q="),
];

#[async_trait]
impl CommandProvider for WebSearchProvider {
    fn name(&self) -> &str { "WebSearch" }

    async fn commands(&self) -> Vec<CommandEntry> { vec![] }

    fn is_dynamic(&self) -> bool { true }

    async fn search(&self, query: &str) -> Vec<CommandEntry> {
        let trimmed = query.trim();
        if trimmed.len() < 2 { return vec![]; }

        let encoded = url_encode(trimmed);
        ENGINES.iter().map(|(id, name, base_url)| {
            CommandEntry {
                id: format!("web.{}.{}", id, encoded),
                name: format!("Search {} for '{}'", name, trimmed),
                description: format!("{}{}", base_url, encoded),
                category: "Web Search".into(),
                icon: None,
                match_indices: vec![],
                score: 0,
            }
        }).collect()
    }

    fn execute(&self, id: &str) -> Option<Result<String, String>> {
        if !id.starts_with("web.") { return None; }
        let rest = id.strip_prefix("web.")?;
        let dot_pos = rest.find('.')?;
        let engine_id = &rest[..dot_pos];
        let encoded_query = &rest[dot_pos + 1..];

        let (_, _, base_url) = ENGINES.iter().find(|(eid, _, _)| *eid == engine_id)?;
        let url = format!("{}{}", base_url, encoded_query);

        Some(
            std::process::Command::new("open")
                .arg(&url)
                .spawn()
                .map(|_| format!("Opened {}", url))
                .map_err(|e| e.to_string()),
        )
    }
}
