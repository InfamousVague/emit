use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_TTL_SECS: u64 = 300; // 5 minutes

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoEntry {
    pub action_id: String,
    pub command_id: String,
    pub extension_id: String,
    pub undo_data: serde_json::Value,
    pub description: String,
    pub timestamp: u64,
}

pub struct UndoStack {
    last_action: Option<UndoEntry>,
}

impl UndoStack {
    pub fn new() -> Self {
        Self { last_action: None }
    }

    pub fn push(&mut self, entry: UndoEntry) {
        self.last_action = Some(entry);
    }

    pub fn pop(&mut self) -> Option<UndoEntry> {
        if self.is_expired() {
            self.last_action = None;
            return None;
        }
        self.last_action.take()
    }

    pub fn peek(&self) -> Option<&UndoEntry> {
        if self.is_expired() {
            return None;
        }
        self.last_action.as_ref()
    }

    fn is_expired(&self) -> bool {
        match &self.last_action {
            Some(entry) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                now.saturating_sub(entry.timestamp) > DEFAULT_TTL_SECS
            }
            None => true,
        }
    }
}
