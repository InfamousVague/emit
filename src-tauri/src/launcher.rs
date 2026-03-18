use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub icon: Option<String>,
}

/// Built-in commands — will be replaced by a plugin/extension system
fn builtin_commands() -> Vec<CommandEntry> {
    vec![
        CommandEntry {
            id: "app.calculator".into(),
            name: "Calculator".into(),
            description: "Open Calculator".into(),
            category: "Applications".into(),
            icon: None,
        },
        CommandEntry {
            id: "app.terminal".into(),
            name: "Terminal".into(),
            description: "Open Terminal".into(),
            category: "Applications".into(),
            icon: None,
        },
        CommandEntry {
            id: "app.finder".into(),
            name: "Finder".into(),
            description: "Open Finder".into(),
            category: "Applications".into(),
            icon: None,
        },
        CommandEntry {
            id: "system.sleep".into(),
            name: "Sleep".into(),
            description: "Put the system to sleep".into(),
            category: "System".into(),
            icon: None,
        },
        CommandEntry {
            id: "system.lock".into(),
            name: "Lock Screen".into(),
            description: "Lock the screen".into(),
            category: "System".into(),
            icon: None,
        },
    ]
}

pub fn search_commands(query: &str) -> Vec<CommandEntry> {
    if query.is_empty() {
        return builtin_commands();
    }

    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(i64, CommandEntry)> = builtin_commands()
        .into_iter()
        .filter_map(|cmd| {
            matcher
                .fuzzy_match(&cmd.name, query)
                .map(|score| (score, cmd))
        })
        .collect();

    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored.into_iter().map(|(_, cmd)| cmd).collect()
}

pub fn execute(id: &str) -> Result<String, String> {
    match id {
        "app.calculator" => open_app("Calculator"),
        "app.terminal" => open_app("Terminal"),
        "app.finder" => open_app("Finder"),
        "system.sleep" => run_cmd("pmset", &["sleepnow"]),
        "system.lock" => run_cmd(
            "osascript",
            &["-e", r#"tell application "System Events" to keystroke "q" using {command down, control down}"#],
        ),
        _ => Err(format!("Unknown command: {id}")),
    }
}

fn open_app(name: &str) -> Result<String, String> {
    std::process::Command::new("open")
        .arg("-a")
        .arg(name)
        .spawn()
        .map(|_| format!("Opened {name}"))
        .map_err(|e| e.to_string())
}

fn run_cmd(cmd: &str, args: &[&str]) -> Result<String, String> {
    std::process::Command::new(cmd)
        .args(args)
        .spawn()
        .map(|_| "OK".into())
        .map_err(|e| e.to_string())
}
