pub mod applications;
pub mod builtin;
pub mod clipboard;
pub mod files;

use crate::command_schema::{CommandDefinition, CommandResult, SelectOption};
use crate::launcher::CommandEntry;
use async_trait::async_trait;

#[async_trait]
pub trait CommandProvider: Send + Sync {
    fn name(&self) -> &str;

    async fn commands(&self) -> Vec<CommandEntry>;

    fn execute(&self, id: &str) -> Option<Result<String, String>>;

    fn is_dynamic(&self) -> bool {
        false
    }

    async fn search(&self, _query: &str) -> Vec<CommandEntry> {
        vec![]
    }

    /// Populate icon data for cached entries. Override in providers that
    /// support real icon extraction (e.g. ApplicationProvider).
    async fn enrich_icons(&mut self) {}

    /// Return parameterized command definitions for slash-command mode.
    fn command_definitions(&self) -> Vec<CommandDefinition> {
        vec![]
    }

    /// Execute a parameterized command with the given params.
    async fn execute_action(
        &self,
        _command_id: &str,
        _params: serde_json::Value,
    ) -> Option<CommandResult> {
        None
    }

    /// Resolve dynamic autocomplete options for a parameter.
    async fn resolve_autocomplete(
        &self,
        _command_id: &str,
        _param_id: &str,
        _query: &str,
    ) -> Vec<SelectOption> {
        vec![]
    }

    /// Undo a previously executed action.
    async fn undo_action(
        &self,
        _action_id: &str,
        _undo_data: serde_json::Value,
    ) -> Option<CommandResult> {
        None
    }

    /// Return keyboard shortcuts this provider wants to register.
    fn shortcuts(&self) -> Vec<crate::shortcuts::ShortcutBinding> {
        vec![]
    }
}
