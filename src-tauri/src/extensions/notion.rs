use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use async_trait::async_trait;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use crate::command_schema::{
    CommandCategory, CommandDefinition, CommandResult, ParamDefinition, ParamGroup, ParamType,
    SelectOption,
};
use crate::launcher::CommandEntry;
use crate::providers::CommandProvider;
use super::registry::ExtensionRegistry;

const MAX_NOTION_RESULTS: usize = 8;
const CACHE_TTL: Duration = Duration::from_secs(90);

/// Cached Notion page/database entry for instant local search.
#[derive(Clone)]
struct CachedNotionItem {
    id: String,
    title: String,
    object_type: String,
}

struct NotionCache {
    items: Vec<CachedNotionItem>,
    fetched_at: Option<Instant>,
}

impl NotionCache {
    fn new() -> Self {
        Self { items: Vec::new(), fetched_at: None }
    }

    fn is_stale(&self) -> bool {
        self.fetched_at.map_or(true, |t| t.elapsed() > CACHE_TTL)
    }
}

pub struct NotionProvider {
    registry: Arc<RwLock<ExtensionRegistry>>,
    cache: Arc<RwLock<NotionCache>>,
}

impl NotionProvider {
    pub fn new(registry: Arc<RwLock<ExtensionRegistry>>) -> Self {
        Self {
            registry,
            cache: Arc::new(RwLock::new(NotionCache::new())),
        }
    }

    fn get_api_key(settings: &serde_json::Value) -> Option<String> {
        settings
            .get("api_key")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    }

    /// Fetch all recent pages and databases from Notion, updating the cache.
    async fn refresh_cache(api_key: &str, cache: &RwLock<NotionCache>) {
        let client = reqwest::Client::new();
        let mut all_items = Vec::new();

        // Fetch recent pages (up to 50)
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            client
                .post("https://api.notion.com/v1/search")
                .header("Authorization", format!("Bearer {api_key}"))
                .header("Notion-Version", "2022-06-28")
                .json(&serde_json::json!({
                    "page_size": 50,
                    "sort": { "direction": "descending", "timestamp": "last_edited_time" },
                }))
                .send(),
        )
        .await;

        if let Ok(Ok(resp)) = result {
            if let Ok(body) = resp.json::<serde_json::Value>().await {
                if let Some(results) = body.get("results").and_then(|r| r.as_array()) {
                    for item in results {
                        if let Some(cached) = Self::parse_search_item(item) {
                            all_items.push(cached);
                        }
                    }
                }
            }
        }

        let mut c = cache.write().await;
        c.items = all_items;
        c.fetched_at = Some(Instant::now());
    }

    fn parse_search_item(item: &serde_json::Value) -> Option<CachedNotionItem> {
        let id = item.get("id")?.as_str()?.to_string();
        let object_type = item.get("object")?.as_str()?.to_string();

        let title = if object_type == "page" {
            item.get("properties")
                .and_then(|p| p.as_object())
                .and_then(|props| {
                    props.values().find_map(|v| {
                        if v.get("type")?.as_str()? == "title" {
                            v.get("title")?
                                .as_array()?
                                .first()?
                                .get("plain_text")?
                                .as_str()
                                .map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                })
        } else if object_type == "database" {
            item.get("title")
                .and_then(|t| t.as_array())
                .and_then(|a| a.first())
                .and_then(|t| t.get("plain_text"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        } else {
            None
        };

        Some(CachedNotionItem {
            id,
            title: title.unwrap_or_else(|| "Untitled".into()),
            object_type,
        })
    }
}

#[async_trait]
impl CommandProvider for NotionProvider {
    fn name(&self) -> &str {
        "Notion"
    }

    async fn commands(&self) -> Vec<CommandEntry> {
        let reg = self.registry.read().await;
        if !reg.is_enabled("notion") {
            return vec![];
        }

        vec![CommandEntry {
            id: "notion.open".into(),
            name: "Notion".into(),
            description: "Search and browse Notion pages".into(),
            category: "Extensions".into(),
            icon: None,
            match_indices: vec![],
            score: 0,
        }]
    }

    fn is_dynamic(&self) -> bool {
        true
    }

    async fn search(&self, query: &str) -> Vec<CommandEntry> {
        if query.len() < 2 {
            return vec![];
        }

        let reg = self.registry.read().await;
        if !reg.is_enabled("notion") {
            return vec![];
        }

        let settings = reg.get_settings("notion");
        let api_key = match Self::get_api_key(&settings) {
            Some(key) => key,
            None => return vec![],
        };
        drop(reg);

        // Ensure cache is populated; refresh in background if stale
        {
            let cache = self.cache.read().await;
            if cache.is_stale() {
                drop(cache);
                // Refresh synchronously on first load, background on subsequent
                let cache_ref = Arc::clone(&self.cache);
                let key = api_key.clone();
                if self.cache.read().await.fetched_at.is_none() {
                    Self::refresh_cache(&key, &cache_ref).await;
                } else {
                    tokio::spawn(async move {
                        Self::refresh_cache(&key, &cache_ref).await;
                    });
                }
            }
        }

        // Fuzzy match against cached items locally — instant
        let cache = self.cache.read().await;
        let matcher = SkimMatcherV2::default();

        let mut scored: Vec<(i64, CommandEntry)> = cache
            .items
            .iter()
            .filter_map(|item| {
                let (score, indices) = matcher.fuzzy_indices(&item.title, query)?;
                let desc = if item.object_type == "database" {
                    "Database".to_string()
                } else {
                    "Page".to_string()
                };
                Some((score, CommandEntry {
                    id: format!("notion.{}.{}", item.object_type, item.id),
                    name: item.title.clone(),
                    description: desc,
                    category: "Notion".into(),
                    icon: None,
                    match_indices: indices,
                    score,
                }))
            })
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().take(MAX_NOTION_RESULTS).map(|(_, e)| e).collect()
    }

    fn command_definitions(&self) -> Vec<CommandDefinition> {
        vec![
            CommandDefinition {
                id: "notion.create".into(),
                extension_id: "notion".into(),
                name: "Create".into(),
                description: "Create a new page in a Notion database".into(),
                icon: None,
                category: CommandCategory::Write,
                requires_confirmation: false,
                shortcut: None,
                follow_ups: vec!["notion.update".into()],
                params: vec![
                    ParamDefinition {
                        id: "database_id".into(),
                        name: "Database".into(),
                        param_type: ParamType::DatabasePicker,
                        required: true,
                        default_value: None,
                        placeholder: Some("Select a database...".into()),
                        group: ParamGroup::Required,
                    },
                    ParamDefinition {
                        id: "title".into(),
                        name: "Title".into(),
                        param_type: ParamType::Text,
                        required: true,
                        default_value: None,
                        placeholder: Some("Page title...".into()),
                        group: ParamGroup::Required,
                    },
                    ParamDefinition {
                        id: "status".into(),
                        name: "Status".into(),
                        param_type: ParamType::Select { options: vec![
                            SelectOption { value: "Backlog".into(), label: "Backlog".into(), color: None },
                            SelectOption { value: "To Do".into(), label: "To Do".into(), color: None },
                            SelectOption { value: "In Progress".into(), label: "In Progress".into(), color: None },
                            SelectOption { value: "In Review".into(), label: "In Review".into(), color: None },
                            SelectOption { value: "Done".into(), label: "Done".into(), color: None },
                        ] },
                        required: false,
                        default_value: None,
                        placeholder: Some("Select status...".into()),
                        group: ParamGroup::Advanced,
                    },
                    ParamDefinition {
                        id: "priority".into(),
                        name: "Priority".into(),
                        param_type: ParamType::Select { options: vec![
                            SelectOption { value: "Critical".into(), label: "Critical".into(), color: None },
                            SelectOption { value: "High".into(), label: "High".into(), color: None },
                            SelectOption { value: "Medium".into(), label: "Medium".into(), color: None },
                            SelectOption { value: "Low".into(), label: "Low".into(), color: None },
                        ] },
                        required: false,
                        default_value: None,
                        placeholder: Some("Select priority...".into()),
                        group: ParamGroup::Advanced,
                    },
                    ParamDefinition {
                        id: "page_type".into(),
                        name: "Type".into(),
                        param_type: ParamType::Select { options: vec![
                            SelectOption { value: "Feature".into(), label: "Feature".into(), color: None },
                            SelectOption { value: "Bug".into(), label: "Bug".into(), color: None },
                            SelectOption { value: "Improvement".into(), label: "Improvement".into(), color: None },
                            SelectOption { value: "Chore".into(), label: "Chore".into(), color: None },
                            SelectOption { value: "Refactor".into(), label: "Refactor".into(), color: None },
                        ] },
                        required: false,
                        default_value: None,
                        placeholder: Some("Select type...".into()),
                        group: ParamGroup::Advanced,
                    },
                    ParamDefinition {
                        id: "area".into(),
                        name: "Area".into(),
                        param_type: ParamType::Select { options: vec![
                            SelectOption { value: "Frontend".into(), label: "Frontend".into(), color: None },
                            SelectOption { value: "Backend".into(), label: "Backend".into(), color: None },
                            SelectOption { value: "UI-UX".into(), label: "UI/UX".into(), color: None },
                            SelectOption { value: "Commands".into(), label: "Commands".into(), color: None },
                            SelectOption { value: "Extensions".into(), label: "Extensions".into(), color: None },
                            SelectOption { value: "Performance".into(), label: "Performance".into(), color: None },
                            SelectOption { value: "Notion".into(), label: "Notion".into(), color: None },
                        ] },
                        required: false,
                        default_value: None,
                        placeholder: Some("Select area...".into()),
                        group: ParamGroup::Advanced,
                    },
                    ParamDefinition {
                        id: "assignee".into(),
                        name: "Assignee".into(),
                        param_type: ParamType::DynamicSelect { resolver: "assignee".into() },
                        required: false,
                        default_value: None,
                        placeholder: Some("Search for a user...".into()),
                        group: ParamGroup::Advanced,
                    },
                ],
                undoable: true,
            },
            CommandDefinition {
                id: "notion.update".into(),
                extension_id: "notion".into(),
                name: "Update".into(),
                description: "Update properties on an existing Notion page".into(),
                icon: None,
                category: CommandCategory::Write,
                requires_confirmation: false,
                shortcut: None,
                follow_ups: vec![],
                params: vec![ParamDefinition {
                    id: "page_id".into(),
                    name: "Page".into(),
                    param_type: ParamType::PagePicker { database_id: None },
                    required: true,
                    default_value: None,
                    placeholder: Some("Search for a page...".into()),
                    group: ParamGroup::Required,
                }],
                undoable: true,
            },
            CommandDefinition {
                id: "notion.archive".into(),
                extension_id: "notion".into(),
                name: "Archive".into(),
                description: "Archive a Notion page".into(),
                icon: None,
                category: CommandCategory::Write,
                requires_confirmation: true,
                shortcut: None,
                follow_ups: vec![],
                params: vec![ParamDefinition {
                    id: "page_id".into(),
                    name: "Page".into(),
                    param_type: ParamType::PagePicker { database_id: None },
                    required: true,
                    default_value: None,
                    placeholder: Some("Search for a page...".into()),
                    group: ParamGroup::Required,
                }],
                undoable: true,
            },
            CommandDefinition {
                id: "notion.comment".into(),
                extension_id: "notion".into(),
                name: "Comment".into(),
                description: "Add a quick comment to a Notion page".into(),
                icon: None,
                category: CommandCategory::Write,
                requires_confirmation: false,
                shortcut: None,
                follow_ups: vec![],
                params: vec![
                    ParamDefinition {
                        id: "page_id".into(),
                        name: "Page".into(),
                        param_type: ParamType::PagePicker { database_id: None },
                        required: true,
                        default_value: None,
                        placeholder: Some("Search for a page...".into()),
                        group: ParamGroup::Required,
                    },
                    ParamDefinition {
                        id: "content".into(),
                        name: "Comment".into(),
                        param_type: ParamType::RichText,
                        required: true,
                        default_value: None,
                        placeholder: Some("Write your comment...".into()),
                        group: ParamGroup::Required,
                    },
                ],
                undoable: false,
            },
            CommandDefinition {
                id: "notion.query".into(),
                extension_id: "notion".into(),
                name: "Query".into(),
                description: "View pages in a Notion database".into(),
                icon: None,
                category: CommandCategory::Read,
                requires_confirmation: false,
                shortcut: None,
                follow_ups: vec!["notion.update".into(), "notion.archive".into()],
                params: vec![ParamDefinition {
                    id: "database_id".into(),
                    name: "Database".into(),
                    param_type: ParamType::DatabasePicker,
                    required: true,
                    default_value: None,
                    placeholder: Some("Select a database...".into()),
                    group: ParamGroup::Required,
                }],
                undoable: false,
            },
        ]
    }

    async fn execute_action(
        &self,
        command_id: &str,
        params: serde_json::Value,
    ) -> Option<CommandResult> {
        if !command_id.starts_with("notion.") {
            return None;
        }

        let reg = self.registry.read().await;
        let settings = reg.get_settings("notion");
        let api_key = match Self::get_api_key(&settings) {
            Some(key) => key,
            None => return Some(CommandResult::error("Notion API key not configured")),
        };
        drop(reg);

        let client = reqwest::Client::new();

        match command_id {
            "notion.create" => {
                let database_id = params.get("database_id")?.as_str()?;
                let title = params.get("title")?.as_str()?;

                let mut properties = serde_json::json!({
                    "title": {
                        "title": [{ "text": { "content": title } }]
                    }
                });

                if let Some(status) = params.get("status").and_then(|v| v.as_str()) {
                    properties["Status"] = serde_json::json!({"status": {"name": status}});
                }
                if let Some(pri) = params.get("priority").and_then(|v| v.as_str()) {
                    properties["Priority"] = serde_json::json!({"select": {"name": pri}});
                }
                if let Some(pt) = params.get("page_type").and_then(|v| v.as_str()) {
                    properties["Type"] = serde_json::json!({"select": {"name": pt}});
                }
                if let Some(area) = params.get("area").and_then(|v| v.as_str()) {
                    properties["Area"] = serde_json::json!({"multi_select": [{"name": area}]});
                }
                if let Some(assignee) = params.get("assignee").and_then(|v| v.as_str()) {
                    properties["Assignee"] = serde_json::json!({"people": [{"id": assignee}]});
                }

                let body = serde_json::json!({
                    "parent": { "database_id": database_id },
                    "properties": properties,
                });

                let resp = client
                    .post("https://api.notion.com/v1/pages")
                    .header("Authorization", format!("Bearer {api_key}"))
                    .header("Notion-Version", "2022-06-28")
                    .json(&body)
                    .send()
                    .await
                    .ok()?;

                let result: serde_json::Value = resp.json().await.ok()?;

                if result.get("object").and_then(|v| v.as_str()) == Some("error") {
                    let msg = result
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    return Some(CommandResult::error(msg));
                }

                let page_id = result.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let page_url = result.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();

                Some(
                    CommandResult::success(format!("Created page '{title}'"))
                        .with_data(serde_json::json!({ "url": page_url }))
                        .with_undo(
                            format!("archive:{page_id}"),
                            serde_json::json!({ "page_id": page_id }),
                        )
                        .with_follow_ups(vec!["notion.update".into()]),
                )
            }
            "notion.archive" => {
                let page_id = params.get("page_id")?.as_str()?;

                let body = serde_json::json!({ "archived": true });

                let resp = client
                    .patch(format!("https://api.notion.com/v1/pages/{page_id}"))
                    .header("Authorization", format!("Bearer {api_key}"))
                    .header("Notion-Version", "2022-06-28")
                    .json(&body)
                    .send()
                    .await
                    .ok()?;

                let result: serde_json::Value = resp.json().await.ok()?;

                if result.get("object").and_then(|v| v.as_str()) == Some("error") {
                    let msg = result
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    return Some(CommandResult::error(msg));
                }

                Some(
                    CommandResult::success("Page archived")
                        .with_undo(
                            format!("unarchive:{page_id}"),
                            serde_json::json!({ "page_id": page_id }),
                        ),
                )
            }
            "notion.update" => {
                let page_id = params.get("page_id")?.as_str()?;
                let properties = params.get("properties").cloned().unwrap_or(serde_json::json!({}));

                let body = serde_json::json!({ "properties": properties });

                let resp = client
                    .patch(format!("https://api.notion.com/v1/pages/{page_id}"))
                    .header("Authorization", format!("Bearer {api_key}"))
                    .header("Notion-Version", "2022-06-28")
                    .json(&body)
                    .send()
                    .await
                    .ok()?;

                let result: serde_json::Value = resp.json().await.ok()?;

                if result.get("object").and_then(|v| v.as_str()) == Some("error") {
                    let msg = result
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    return Some(CommandResult::error(msg));
                }

                Some(CommandResult::success("Page updated"))
            }
            "notion.comment" => {
                let page_id = params.get("page_id")?.as_str()?;
                let content = params.get("content")?.as_str()?;

                let body = serde_json::json!({
                    "parent": { "page_id": page_id },
                    "rich_text": [{
                        "type": "text",
                        "text": { "content": content }
                    }]
                });

                let resp = client
                    .post("https://api.notion.com/v1/comments")
                    .header("Authorization", format!("Bearer {api_key}"))
                    .header("Notion-Version", "2022-06-28")
                    .json(&body)
                    .send()
                    .await
                    .ok()?;

                let result: serde_json::Value = resp.json().await.ok()?;

                if result.get("object").and_then(|v| v.as_str()) == Some("error") {
                    let msg = result
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    return Some(CommandResult::error(msg));
                }

                Some(CommandResult::success("Comment added"))
            }
            "notion.query" => {
                let database_id = params.get("database_id")?.as_str()?;

                let resp = client
                    .post(format!(
                        "https://api.notion.com/v1/databases/{database_id}/query"
                    ))
                    .header("Authorization", format!("Bearer {api_key}"))
                    .header("Notion-Version", "2022-06-28")
                    .json(&serde_json::json!({ "page_size": 20 }))
                    .send()
                    .await
                    .ok()?;

                let body: serde_json::Value = resp.json().await.ok()?;
                let results = body
                    .get("results")
                    .and_then(|r| r.as_array())
                    .cloned()
                    .unwrap_or_default();

                let pages: Vec<serde_json::Value> = results
                    .into_iter()
                    .filter_map(|page| {
                        let id = page.get("id")?.as_str()?.to_string();
                        let url = page.get("url")?.as_str().unwrap_or("").to_string();
                        let title = page
                            .get("properties")
                            .and_then(|p| p.as_object())
                            .and_then(|props| {
                                props.values().find_map(|v| {
                                    if v.get("type")?.as_str()? == "title" {
                                        v.get("title")?
                                            .as_array()?
                                            .first()?
                                            .get("plain_text")?
                                            .as_str()
                                            .map(|s| s.to_string())
                                    } else {
                                        None
                                    }
                                })
                            })
                            .unwrap_or_else(|| "Untitled".into());
                        Some(serde_json::json!({ "id": id, "title": title, "url": url }))
                    })
                    .collect();

                Some(
                    CommandResult::success(format!("Found {} pages", pages.len()))
                        .with_data(serde_json::json!(pages))
                        .with_follow_ups(vec![
                            "notion.update".into(),
                            "notion.archive".into(),
                        ]),
                )
            }
            _ => None,
        }
    }

    async fn resolve_autocomplete(
        &self,
        command_id: &str,
        param_id: &str,
        query: &str,
    ) -> Vec<SelectOption> {
        if !command_id.starts_with("notion.") {
            return vec![];
        }

        let reg = self.registry.read().await;
        let settings = reg.get_settings("notion");
        let api_key = match Self::get_api_key(&settings) {
            Some(key) => key,
            None => return vec![],
        };
        drop(reg);

        let client = reqwest::Client::new();

        match param_id {
            "database_id" => {
                let resp = client
                    .post("https://api.notion.com/v1/search")
                    .header("Authorization", format!("Bearer {api_key}"))
                    .header("Notion-Version", "2022-06-28")
                    .json(&serde_json::json!({
                        "filter": { "value": "database", "property": "object" },
                        "page_size": 10,
                    }))
                    .send()
                    .await;

                let resp = match resp {
                    Ok(r) => r,
                    Err(_) => return vec![],
                };

                let body: serde_json::Value = match resp.json().await {
                    Ok(b) => b,
                    Err(_) => return vec![],
                };

                body.get("results")
                    .and_then(|r| r.as_array())
                    .map(|results| {
                        results
                            .iter()
                            .filter_map(|db| {
                                let id = db.get("id")?.as_str()?.to_string();
                                let title = db
                                    .get("title")
                                    .and_then(|t| t.as_array())
                                    .and_then(|a| a.first())
                                    .and_then(|t| t.get("plain_text"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Untitled")
                                    .to_string();

                                // Filter by query if non-empty
                                if !query.is_empty()
                                    && !title.to_lowercase().contains(&query.to_lowercase())
                                {
                                    return None;
                                }

                                Some(SelectOption {
                                    value: id,
                                    label: title,
                                    color: None,
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            }
            "page_id" => {
                if query.len() < 2 {
                    return vec![];
                }

                let resp = client
                    .post("https://api.notion.com/v1/search")
                    .header("Authorization", format!("Bearer {api_key}"))
                    .header("Notion-Version", "2022-06-28")
                    .json(&serde_json::json!({
                        "query": query,
                        "filter": { "value": "page", "property": "object" },
                        "page_size": 10,
                    }))
                    .send()
                    .await;

                let resp = match resp {
                    Ok(r) => r,
                    Err(_) => return vec![],
                };

                let body: serde_json::Value = match resp.json().await {
                    Ok(b) => b,
                    Err(_) => return vec![],
                };

                body.get("results")
                    .and_then(|r| r.as_array())
                    .map(|results| {
                        results
                            .iter()
                            .filter_map(|page| {
                                let id = page.get("id")?.as_str()?.to_string();
                                let title = page
                                    .get("properties")
                                    .and_then(|p| p.as_object())
                                    .and_then(|props| {
                                        props.values().find_map(|v| {
                                            if v.get("type")?.as_str()? == "title" {
                                                v.get("title")?
                                                    .as_array()?
                                                    .first()?
                                                    .get("plain_text")?
                                                    .as_str()
                                                    .map(|s| s.to_string())
                                            } else {
                                                None
                                            }
                                        })
                                    })
                                    .unwrap_or_else(|| "Untitled".into());

                                Some(SelectOption {
                                    value: id,
                                    label: title,
                                    color: None,
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            }
            "assignee" => {
                let resp = client
                    .get("https://api.notion.com/v1/users")
                    .header("Authorization", format!("Bearer {api_key}"))
                    .header("Notion-Version", "2022-06-28")
                    .send()
                    .await;

                let resp = match resp {
                    Ok(r) => r,
                    Err(_) => return vec![],
                };

                let body: serde_json::Value = match resp.json().await {
                    Ok(b) => b,
                    Err(_) => return vec![],
                };

                body.get("results")
                    .and_then(|r| r.as_array())
                    .map(|results| {
                        results
                            .iter()
                            .filter_map(|user| {
                                let id = user.get("id")?.as_str()?.to_string();
                                let name = user.get("name")?.as_str()?.to_string();
                                let user_type = user.get("type")?.as_str()?;
                                if user_type != "person" {
                                    return None;
                                }
                                if !query.is_empty()
                                    && !name.to_lowercase().contains(&query.to_lowercase())
                                {
                                    return None;
                                }
                                Some(SelectOption {
                                    value: id,
                                    label: name,
                                    color: None,
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            }
            _ => vec![],
        }
    }

    async fn undo_action(
        &self,
        _action_id: &str,
        undo_data: serde_json::Value,
    ) -> Option<CommandResult> {
        let page_id = undo_data.get("page_id")?.as_str()?;

        let reg = self.registry.read().await;
        let settings = reg.get_settings("notion");
        let api_key = Self::get_api_key(&settings)?;
        drop(reg);

        let client = reqwest::Client::new();

        // For create-page undo, archive the page. For archive-page undo, unarchive.
        let body = if _action_id.starts_with("archive:") {
            serde_json::json!({ "archived": true })
        } else {
            serde_json::json!({ "archived": false })
        };

        let resp = client
            .patch(format!("https://api.notion.com/v1/pages/{page_id}"))
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Notion-Version", "2022-06-28")
            .json(&body)
            .send()
            .await
            .ok()?;

        let result: serde_json::Value = resp.json().await.ok()?;

        if result.get("object").and_then(|v| v.as_str()) == Some("error") {
            let msg = result
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Undo failed");
            return Some(CommandResult::error(msg));
        }

        if _action_id.starts_with("archive:") {
            Some(CommandResult::success("Page archived (undo create)"))
        } else {
            Some(CommandResult::success("Page unarchived"))
        }
    }

    fn execute(&self, id: &str) -> Option<Result<String, String>> {
        if id == "notion.open" {
            return Some(Ok("view:notion".into()));
        }

        if !id.starts_with("notion.") {
            return None;
        }

        // Extract the notion page/database ID (last segment after second dot)
        let page_id = id
            .splitn(3, '.')
            .nth(2)
            .unwrap_or("")
            .replace('-', "");

        if page_id.is_empty() {
            return Some(Err("Invalid Notion ID".into()));
        }

        Some(
            std::process::Command::new("open")
                .arg(format!("https://notion.so/{page_id}"))
                .spawn()
                .map(|_| "Opened Notion page".into())
                .map_err(|e| e.to_string()),
        )
    }
}
