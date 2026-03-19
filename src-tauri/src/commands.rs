use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use serde::Serialize;
use tauri::AppHandle;

use crate::clipboard::{self, ClipboardItem, SharedClipboardState};
use crate::command_schema::{CommandDefinition, CommandResult, SelectOption};
use crate::extensions::manifest::all_manifests;
use crate::extensions::registry::ExtensionRegistry;
use crate::frecency::FrecencyTracker;
use crate::launcher::{CommandEntry, CommandRegistry};
use crate::settings::Settings;
use crate::undo::{UndoEntry, UndoStack};
use crate::window;

pub type SharedExtensionRegistry = Arc<RwLock<ExtensionRegistry>>;
pub type SharedFrecency = Arc<RwLock<FrecencyTracker>>;
pub type SharedUndoStack = Arc<RwLock<UndoStack>>;

#[tauri::command]
pub async fn search(
    query: String,
    registry: tauri::State<'_, Arc<RwLock<CommandRegistry>>>,
) -> Result<Vec<CommandEntry>, String> {
    let reg = registry.read().await;
    Ok(reg.search(&query).await)
}

/// Fast static-only search — no network/IO, returns immediately.
#[tauri::command]
pub async fn search_static(
    query: String,
    registry: tauri::State<'_, Arc<RwLock<CommandRegistry>>>,
) -> Result<Vec<CommandEntry>, String> {
    let reg = registry.read().await;
    Ok(reg.search_static(&query))
}

#[tauri::command]
pub async fn execute_command(
    id: String,
    registry: tauri::State<'_, Arc<RwLock<CommandRegistry>>>,
) -> Result<String, String> {
    let reg = registry.read().await;
    reg.execute(&id)
}

#[tauri::command]
pub fn hide_window(app: AppHandle) {
    window::hide(&app);
}

#[tauri::command]
pub fn get_settings() -> Result<Settings, String> {
    Ok(Settings::load())
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use objc2::MainThreadMarker;
        use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
        let policy = if settings.show_in_dock {
            NSApplicationActivationPolicy::Regular
        } else {
            NSApplicationActivationPolicy::Accessory
        };
        if let Some(mtm) = MainThreadMarker::new() {
            let ns_app = NSApplication::sharedApplication(mtm);
            ns_app.setActivationPolicy(policy);
        }
    }
    let _ = app; // suppress unused warning on non-mac
    settings.save()
}

#[tauri::command]
pub async fn get_clipboard_history(
    state: tauri::State<'_, SharedClipboardState>,
) -> Result<Vec<ClipboardItem>, String> {
    let s = state.read().await;
    Ok(s.items.clone())
}

#[tauri::command]
pub async fn clipboard_copy(
    id: String,
    state: tauri::State<'_, SharedClipboardState>,
) -> Result<(), String> {
    let s = state.read().await;
    let item = s.items.iter().find(|i| i.id == id);
    match item {
        Some(item) => {
            if item.content_type == "image" {
                // Copy image back to clipboard via osascript
                if let Some(path) = &item.image_path {
                    std::process::Command::new("osascript")
                        .arg("-e")
                        .arg(format!(
                            r#"set the clipboard to (read (POSIX file "{}") as «class PNGf»)"#,
                            path
                        ))
                        .output()
                        .map_err(|e| e.to_string())?;
                }
            } else {
                use std::process::{Command, Stdio};
                use std::io::Write;
                let mut child = Command::new("pbcopy")
                    .stdin(Stdio::piped())
                    .spawn()
                    .map_err(|e| e.to_string())?;
                child
                    .stdin
                    .take()
                    .unwrap()
                    .write_all(item.content.as_bytes())
                    .map_err(|e| e.to_string())?;
                child.wait().map_err(|e| e.to_string())?;
            }
            Ok(())
        }
        None => Err("Clipboard item not found".into()),
    }
}

#[tauri::command]
pub async fn clipboard_get_image(
    id: String,
    state: tauri::State<'_, SharedClipboardState>,
) -> Result<String, String> {
    let s = state.read().await;
    clipboard::get_image_base64(&s.items, &id)
}

#[tauri::command]
pub async fn clipboard_delete(
    id: String,
    state: tauri::State<'_, SharedClipboardState>,
) -> Result<(), String> {
    let mut s = state.write().await;
    s.delete(&id);
    Ok(())
}

#[tauri::command]
pub async fn clipboard_clear(
    state: tauri::State<'_, SharedClipboardState>,
) -> Result<(), String> {
    let mut s = state.write().await;
    s.clear();
    Ok(())
}

// --- Extension commands ---

#[derive(Debug, Clone, Serialize)]
pub struct ExtensionInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub category: String,
    pub bundled: bool,
    pub enabled: bool,
    pub configured: bool,
    pub has_settings: bool,
}

#[tauri::command]
pub async fn get_extensions(
    registry: tauri::State<'_, SharedExtensionRegistry>,
) -> Result<Vec<ExtensionInfo>, String> {
    let reg = registry.read().await;
    let manifests = all_manifests();
    let extensions = manifests
        .into_iter()
        .map(|m| {
            let enabled = reg.is_enabled(&m.id);
            let settings = reg.get_settings(&m.id);
            let configured = settings
                .get("api_key")
                .and_then(|v| v.as_str())
                .map(|s| !s.is_empty())
                .unwrap_or(false);
            ExtensionInfo {
                id: m.id,
                name: m.name,
                description: m.description,
                icon: m.icon,
                category: m.category,
                bundled: m.bundled,
                enabled,
                configured,
                has_settings: m.has_settings,
            }
        })
        .collect();
    Ok(extensions)
}

#[tauri::command]
pub async fn set_extension_enabled(
    id: String,
    enabled: bool,
    registry: tauri::State<'_, SharedExtensionRegistry>,
) -> Result<(), String> {
    let mut reg = registry.write().await;
    reg.set_enabled(&id, enabled);
    Ok(())
}

#[tauri::command]
pub async fn get_extension_settings(
    id: String,
    registry: tauri::State<'_, SharedExtensionRegistry>,
) -> Result<serde_json::Value, String> {
    let reg = registry.read().await;
    Ok(reg.get_settings(&id))
}

#[tauri::command]
pub async fn save_extension_settings(
    id: String,
    settings: serde_json::Value,
    registry: tauri::State<'_, SharedExtensionRegistry>,
) -> Result<(), String> {
    let mut reg = registry.write().await;
    reg.set_settings(&id, settings);
    Ok(())
}

// --- Notion-specific commands ---

#[tauri::command]
pub async fn notion_get_databases(
    registry: tauri::State<'_, SharedExtensionRegistry>,
) -> Result<Vec<serde_json::Value>, String> {
    let reg = registry.read().await;
    let settings = reg.get_settings("notion");
    let api_key = settings
        .get("api_key")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "Notion API key not configured".to_string())?
        .to_string();

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.notion.com/v1/search")
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Notion-Version", "2022-06-28")
        .json(&serde_json::json!({
            "filter": { "value": "database", "property": "object" }
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let results = body
        .get("results")
        .and_then(|r| r.as_array())
        .cloned()
        .unwrap_or_default();

    let databases: Vec<serde_json::Value> = results
        .into_iter()
        .map(|db| {
            let id = db.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let title = db
                .get("title")
                .and_then(|t| t.as_array())
                .and_then(|a| a.first())
                .and_then(|t| t.get("plain_text"))
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled")
                .to_string();
            serde_json::json!({ "id": id, "title": title })
        })
        .collect();

    Ok(databases)
}

#[tauri::command]
pub async fn notion_query_database(
    database_id: String,
    filters: serde_json::Value,
    registry: tauri::State<'_, SharedExtensionRegistry>,
) -> Result<Vec<serde_json::Value>, String> {
    let reg = registry.read().await;
    let settings = reg.get_settings("notion");
    let api_key = settings
        .get("api_key")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "Notion API key not configured".to_string())?
        .to_string();

    let client = reqwest::Client::new();
    let mut filter_conditions: Vec<serde_json::Value> = vec![];

    if let Some(status) = filters.get("status").and_then(|v| v.as_str()) {
        if !status.is_empty() {
            filter_conditions.push(serde_json::json!({
                "property": "Status",
                "status": { "equals": status }
            }));
        }
    }

    if let Some(assignee) = filters.get("assignee").and_then(|v| v.as_str()) {
        if !assignee.is_empty() {
            filter_conditions.push(serde_json::json!({
                "property": "Assignee",
                "people": { "contains": assignee }
            }));
        }
    }

    let mut body = serde_json::json!({ "page_size": 20 });
    if !filter_conditions.is_empty() {
        if filter_conditions.len() == 1 {
            body["filter"] = filter_conditions.into_iter().next().unwrap();
        } else {
            body["filter"] = serde_json::json!({ "and": filter_conditions });
        }
    }

    let resp = client
        .post(format!(
            "https://api.notion.com/v1/databases/{database_id}/query"
        ))
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Notion-Version", "2022-06-28")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let resp_body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let results = resp_body
        .get("results")
        .and_then(|r| r.as_array())
        .cloned()
        .unwrap_or_default();

    let pages: Vec<serde_json::Value> = results
        .into_iter()
        .map(|page| {
            let id = page.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let url = page.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();

            let title = page
                .get("properties")
                .and_then(|p| p.as_object())
                .and_then(|props| {
                    props.values().find_map(|v| {
                        if v.get("type").and_then(|t| t.as_str()) == Some("title") {
                            v.get("title")
                                .and_then(|t| t.as_array())
                                .and_then(|a| a.first())
                                .and_then(|t| t.get("plain_text"))
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                })
                .unwrap_or_else(|| "Untitled".into());

            let status = page
                .get("properties")
                .and_then(|p| p.as_object())
                .and_then(|props| {
                    props.values().find_map(|v| {
                        if v.get("type").and_then(|t| t.as_str()) == Some("status") {
                            v.get("status")
                                .and_then(|s| s.get("name"))
                                .and_then(|n| n.as_str())
                                .map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                })
                .unwrap_or_default();

            let assignee = page
                .get("properties")
                .and_then(|p| p.as_object())
                .and_then(|props| {
                    props.values().find_map(|v| {
                        if v.get("type").and_then(|t| t.as_str()) == Some("people") {
                            v.get("people")
                                .and_then(|p| p.as_array())
                                .and_then(|a| a.first())
                                .and_then(|person| person.get("name"))
                                .and_then(|n| n.as_str())
                                .map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                })
                .unwrap_or_default();

            serde_json::json!({
                "id": id,
                "title": title,
                "status": status,
                "assignee": assignee,
                "url": url,
            })
        })
        .collect();

    Ok(pages)
}

// --- Slash command system ---

#[tauri::command]
pub async fn search_commands(
    query: String,
    registry: tauri::State<'_, Arc<RwLock<CommandRegistry>>>,
    frecency: tauri::State<'_, SharedFrecency>,
) -> Result<Vec<CommandDefinition>, String> {
    let reg = registry.read().await;
    let frec = frecency.read().await;
    Ok(reg.search_commands(&query, &frec))
}

#[tauri::command]
pub async fn execute_action(
    command_id: String,
    params: serde_json::Value,
    registry: tauri::State<'_, Arc<RwLock<CommandRegistry>>>,
    frecency: tauri::State<'_, SharedFrecency>,
    undo_stack: tauri::State<'_, SharedUndoStack>,
    _app: AppHandle,
) -> Result<CommandResult, String> {
    let reg = registry.read().await;
    let result = reg.execute_action(&command_id, params).await?;

    // Record frecency
    {
        let mut frec = frecency.write().await;
        frec.record_use(&command_id);
    }

    // Push to undo stack if undoable
    if let (Some(action_id), Some(undo_data)) = (&result.action_id, &result.undo_data) {
        let extension_id = command_id
            .split('.')
            .next()
            .unwrap_or("")
            .to_string();
        let mut stack = undo_stack.write().await;
        stack.push(UndoEntry {
            action_id: action_id.clone(),
            command_id: command_id.clone(),
            extension_id,
            undo_data: undo_data.clone(),
            description: result.message.clone(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        });
    }

    // Send system notification via osascript, then open URL if present
    {
        let title = if result.success { "Command Succeeded" } else { "Command Failed" };
        let body = result.message.replace('"', r#"\""#);

        let _ = std::process::Command::new("osascript")
            .arg("-e")
            .arg(format!(
                r#"display notification "{body}" with title "{title}""#
            ))
            .spawn();

        // Open page URL in default browser if present
        if let Some(url) = result.data.as_ref()
            .and_then(|d| d.get("url"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
        {
            let _ = std::process::Command::new("open")
                .arg(url)
                .spawn();
        }
    }

    Ok(result)
}

#[tauri::command]
pub async fn resolve_param_options(
    command_id: String,
    param_id: String,
    query: String,
    registry: tauri::State<'_, Arc<RwLock<CommandRegistry>>>,
) -> Result<Vec<SelectOption>, String> {
    let reg = registry.read().await;
    Ok(reg.resolve_autocomplete(&command_id, &param_id, &query).await)
}

#[tauri::command]
pub async fn undo_last_action(
    undo_stack: tauri::State<'_, SharedUndoStack>,
    registry: tauri::State<'_, Arc<RwLock<CommandRegistry>>>,
    _app: AppHandle,
) -> Result<CommandResult, String> {
    let entry = {
        let mut stack = undo_stack.write().await;
        stack.pop().ok_or_else(|| "Nothing to undo".to_string())?
    };

    let reg = registry.read().await;
    let result = reg
        .undo_action(&entry.extension_id, &entry.action_id, entry.undo_data)
        .await?;

    if result.success {
        let body = result.message.replace('"', r#"\""#);
        let _ = std::process::Command::new("osascript")
            .arg("-e")
            .arg(format!(
                r#"display notification "{body}" with title "Undo Succeeded""#
            ))
            .spawn();
    }

    Ok(result)
}

// --- Notion CRUD commands ---

fn get_notion_api_key(settings: &serde_json::Value) -> Result<String, String> {
    settings
        .get("api_key")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .ok_or_else(|| "Notion API key not configured".to_string())
}

#[tauri::command]
pub async fn notion_get_database_schema(
    database_id: String,
    registry: tauri::State<'_, SharedExtensionRegistry>,
) -> Result<serde_json::Value, String> {
    let reg = registry.read().await;
    let settings = reg.get_settings("notion");
    let api_key = get_notion_api_key(&settings)?;

    let resp = reqwest::Client::new()
        .get(format!("https://api.notion.com/v1/databases/{database_id}"))
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Notion-Version", "2022-06-28")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    // Extract just the properties schema
    Ok(body.get("properties").cloned().unwrap_or(serde_json::json!({})))
}

#[tauri::command]
pub async fn notion_create_page(
    database_id: String,
    properties: serde_json::Value,
    registry: tauri::State<'_, SharedExtensionRegistry>,
) -> Result<serde_json::Value, String> {
    let reg = registry.read().await;
    let settings = reg.get_settings("notion");
    let api_key = get_notion_api_key(&settings)?;

    let body = serde_json::json!({
        "parent": { "database_id": database_id },
        "properties": properties,
    });

    let resp = reqwest::Client::new()
        .post("https://api.notion.com/v1/pages")
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Notion-Version", "2022-06-28")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let result: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if result.get("object").and_then(|v| v.as_str()) == Some("error") {
        let msg = result.get("message").and_then(|v| v.as_str()).unwrap_or("Unknown error");
        return Err(msg.to_string());
    }

    Ok(result)
}

#[tauri::command]
pub async fn notion_update_page(
    page_id: String,
    properties: serde_json::Value,
    registry: tauri::State<'_, SharedExtensionRegistry>,
) -> Result<serde_json::Value, String> {
    let reg = registry.read().await;
    let settings = reg.get_settings("notion");
    let api_key = get_notion_api_key(&settings)?;

    let body = serde_json::json!({ "properties": properties });

    let resp = reqwest::Client::new()
        .patch(format!("https://api.notion.com/v1/pages/{page_id}"))
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Notion-Version", "2022-06-28")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let result: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if result.get("object").and_then(|v| v.as_str()) == Some("error") {
        let msg = result.get("message").and_then(|v| v.as_str()).unwrap_or("Unknown error");
        return Err(msg.to_string());
    }

    Ok(result)
}

#[tauri::command]
pub async fn notion_archive_page(
    page_id: String,
    archive: bool,
    registry: tauri::State<'_, SharedExtensionRegistry>,
) -> Result<(), String> {
    let reg = registry.read().await;
    let settings = reg.get_settings("notion");
    let api_key = get_notion_api_key(&settings)?;

    let body = serde_json::json!({ "archived": archive });

    let resp = reqwest::Client::new()
        .patch(format!("https://api.notion.com/v1/pages/{page_id}"))
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Notion-Version", "2022-06-28")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let result: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if result.get("object").and_then(|v| v.as_str()) == Some("error") {
        let msg = result.get("message").and_then(|v| v.as_str()).unwrap_or("Unknown error");
        return Err(msg.to_string());
    }

    Ok(())
}

#[tauri::command]
pub async fn notion_add_comment(
    page_id: String,
    content: String,
    registry: tauri::State<'_, SharedExtensionRegistry>,
) -> Result<(), String> {
    let reg = registry.read().await;
    let settings = reg.get_settings("notion");
    let api_key = get_notion_api_key(&settings)?;

    let body = serde_json::json!({
        "parent": { "page_id": page_id },
        "rich_text": [{
            "type": "text",
            "text": { "content": content }
        }]
    });

    let resp = reqwest::Client::new()
        .post("https://api.notion.com/v1/comments")
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Notion-Version", "2022-06-28")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let result: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if result.get("object").and_then(|v| v.as_str()) == Some("error") {
        let msg = result.get("message").and_then(|v| v.as_str()).unwrap_or("Unknown error");
        return Err(msg.to_string());
    }

    Ok(())
}

#[tauri::command]
pub async fn notion_search_pages(
    query: String,
    database_id: Option<String>,
    registry: tauri::State<'_, SharedExtensionRegistry>,
) -> Result<Vec<serde_json::Value>, String> {
    let reg = registry.read().await;
    let settings = reg.get_settings("notion");
    let api_key = get_notion_api_key(&settings)?;

    // If database_id is provided, query that database; otherwise search globally
    let (url, body) = if let Some(db_id) = database_id {
        (
            format!("https://api.notion.com/v1/databases/{db_id}/query"),
            serde_json::json!({ "page_size": 20 }),
        )
    } else {
        (
            "https://api.notion.com/v1/search".to_string(),
            serde_json::json!({
                "query": query,
                "filter": { "value": "page", "property": "object" },
                "page_size": 20,
            }),
        )
    };

    let resp = reqwest::Client::new()
        .post(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Notion-Version", "2022-06-28")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let resp_body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let results = resp_body
        .get("results")
        .and_then(|r| r.as_array())
        .cloned()
        .unwrap_or_default();

    let pages: Vec<serde_json::Value> = results
        .into_iter()
        .map(|page| {
            let id = page.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let url = page.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let title = page
                .get("properties")
                .and_then(|p| p.as_object())
                .and_then(|props| {
                    props.values().find_map(|v| {
                        if v.get("type").and_then(|t| t.as_str()) == Some("title") {
                            v.get("title")
                                .and_then(|t| t.as_array())
                                .and_then(|a| a.first())
                                .and_then(|t| t.get("plain_text"))
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                })
                .unwrap_or_else(|| "Untitled".into());
            serde_json::json!({ "id": id, "title": title, "url": url })
        })
        .collect();

    Ok(pages)
}

