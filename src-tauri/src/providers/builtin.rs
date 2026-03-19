use async_trait::async_trait;
use crate::launcher::CommandEntry;
use super::CommandProvider;

/// Declarative system command definitions.
/// Icons are resolved on the frontend via Phosphor React.
/// Each tuple: (id, name, description, shell_command, shell_args)
const SYSTEM_COMMANDS: &[(&str, &str, &str, &str, &[&str])] = &[
    (
        "system.sleep",
        "Sleep",
        "Put the system to sleep",
        "pmset",
        &["sleepnow"],
    ),
    (
        "system.lock",
        "Lock Screen",
        "Lock the screen",
        "osascript",
        &["-e", r#"tell application "System Events" to keystroke "q" using {command down, control down}"#],
    ),
    (
        "system.restart",
        "Restart",
        "Restart the computer",
        "osascript",
        &["-e", r#"tell application "System Events" to restart"#],
    ),
    (
        "system.shutdown",
        "Shut Down",
        "Shut down the computer",
        "osascript",
        &["-e", r#"tell application "System Events" to shut down"#],
    ),
    (
        "system.logout",
        "Log Out",
        "Log out current user",
        "osascript",
        &["-e", r#"tell application "System Events" to log out"#],
    ),
    (
        "system.trash",
        "Empty Trash",
        "Empty the Trash",
        "osascript",
        &["-e", r#"tell application "Finder" to empty the trash"#],
    ),
];

pub struct BuiltinProvider;

impl BuiltinProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl CommandProvider for BuiltinProvider {
    fn name(&self) -> &str {
        "Builtin"
    }

    async fn commands(&self) -> Vec<CommandEntry> {
        let mut cmds: Vec<CommandEntry> = SYSTEM_COMMANDS
            .iter()
            .map(|(id, name, desc, _, _)| CommandEntry {
                id: (*id).into(),
                name: (*name).into(),
                description: (*desc).into(),
                category: "System".into(),
                icon: None,
                match_indices: vec![],
                score: 0,
            })
            .collect();

        cmds.push(CommandEntry {
            id: "system.marketplace".into(),
            name: "Extension Marketplace".into(),
            description: "Browse and manage extensions".into(),
            category: "Extensions".into(),
            icon: None,
            match_indices: vec![],
            score: 0,
        });

        cmds
    }

    fn execute(&self, id: &str) -> Option<Result<String, String>> {
        if id == "system.marketplace" {
            return Some(Ok("view:marketplace".into()));
        }

        let (_, _, _, cmd, args) = SYSTEM_COMMANDS.iter().find(|(cid, ..)| *cid == id)?;
        Some(
            std::process::Command::new(cmd)
                .args(*args)
                .spawn()
                .map(|_| "OK".into())
                .map_err(|e| e.to_string()),
        )
    }
}
