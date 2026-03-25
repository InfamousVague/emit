use async_trait::async_trait;

use crate::launcher::CommandEntry;
use super::CommandProvider;

pub struct ClipboardProvider;

impl ClipboardProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl CommandProvider for ClipboardProvider {
    fn name(&self) -> &str {
        "Clipboard"
    }

    async fn commands(&self) -> Vec<CommandEntry> {
        vec![CommandEntry {
            id: "clipboard.open".into(),
            name: "Clipboard".into(),
            description: "View and search clipboard history".into(),
            category: "Productivity".into(),
            icon: None,
            match_indices: vec![],
            score: 0,
        }]
    }

    fn execute(&self, id: &str) -> Option<Result<String, String>> {
        if id == "clipboard.open" {
            Some(Ok("view:clipboard".into()))
        } else {
            None
        }
    }
}
