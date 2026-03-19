use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandCategory {
    Read,
    Write,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParamGroup {
    Required,
    Advanced,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ParamType {
    Text,
    RichText,
    Number,
    Boolean,
    Date,
    Select { options: Vec<SelectOption> },
    MultiSelect { options: Vec<SelectOption> },
    DatabasePicker,
    PagePicker { database_id: Option<String> },
    People,
    Url,
    DynamicSelect { resolver: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamDefinition {
    pub id: String,
    pub name: String,
    pub param_type: ParamType,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    pub group: ParamGroup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDefinition {
    pub id: String,
    pub extension_id: String,
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub category: CommandCategory,
    pub requires_confirmation: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shortcut: Option<String>,
    #[serde(default)]
    pub follow_ups: Vec<String>,
    pub params: Vec<ParamDefinition>,
    pub undoable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(default)]
    pub follow_ups: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub undo_data: Option<serde_json::Value>,
}

impl CommandResult {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            action_id: None,
            data: None,
            follow_ups: vec![],
            undo_data: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            action_id: None,
            data: None,
            follow_ups: vec![],
            undo_data: None,
        }
    }

    pub fn with_undo(mut self, action_id: String, undo_data: serde_json::Value) -> Self {
        self.action_id = Some(action_id);
        self.undo_data = Some(undo_data);
        self
    }

    pub fn with_follow_ups(mut self, follow_ups: Vec<String>) -> Self {
        self.follow_ups = follow_ups;
        self
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }
}
