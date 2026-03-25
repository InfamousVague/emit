use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde::{Deserialize, Serialize};

use crate::command_schema::{CommandDefinition, CommandResult, SelectOption};
use crate::frecency::FrecencyTracker;
use crate::providers::CommandProvider;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub icon: Option<String>,
    /// Character indices in `name` that matched the fuzzy query.
    #[serde(default)]
    pub match_indices: Vec<usize>,
    /// Fuzzy match score — higher is better. Used by the frontend to merge
    /// static and dynamic results into a single sorted list.
    #[serde(default)]
    pub score: i64,
}

pub struct CommandRegistry {
    providers: Vec<Box<dyn CommandProvider>>,
    cached_static: Vec<CommandEntry>,
    cached_command_defs: Vec<CommandDefinition>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            cached_static: Vec::new(),
            cached_command_defs: Vec::new(),
        }
    }

    pub fn register(&mut self, provider: Box<dyn CommandProvider>) {
        self.providers.push(provider);
    }

    pub async fn refresh_cache(&mut self) {
        let mut all = Vec::new();
        let mut all_defs = Vec::new();
        for provider in &self.providers {
            if !provider.is_dynamic() {
                all.extend(provider.commands().await);
            }
            all_defs.extend(provider.command_definitions());
        }
        self.cached_static = all;
        self.cached_command_defs = all_defs;
    }

    /// Fast search against cached static commands only — no network/IO.
    pub fn search_static(&self, query: &str) -> Vec<CommandEntry> {
        if query.is_empty() {
            return self.cached_static.clone();
        }

        let matcher = SkimMatcherV2::default();

        let mut scored: Vec<(i64, CommandEntry)> = self
            .cached_static
            .iter()
            .filter_map(|cmd| {
                let name_result = matcher.fuzzy_indices(&cmd.name, query);
                let desc_score = matcher.fuzzy_match(&cmd.description, query).map(|s| s / 2);

                let (best_score, indices) = match (name_result, desc_score) {
                    (Some((ns, idx)), Some(ds)) if ns >= ds => (ns, idx),
                    (Some((ns, idx)), None) => (ns, idx),
                    (_, Some(ds)) => (ds, vec![]),
                    (None, None) => return None,
                };

                let mut entry = cmd.clone();
                entry.match_indices = indices;
                entry.score = best_score;
                Some((best_score, entry))
            })
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().map(|(_, cmd)| cmd).collect()
    }

    /// Full search: static results + dynamic providers (network/IO).
    pub async fn search(&self, query: &str) -> Vec<CommandEntry> {
        let matcher = SkimMatcherV2::default();
        let mut scored: Vec<(i64, CommandEntry)> = Vec::new();

        for provider in &self.providers {
            if provider.is_dynamic() {
                let dynamic = provider.search(query).await;
                for mut cmd in dynamic {
                    let (fuzzy_score, indices) =
                        matcher.fuzzy_indices(&cmd.name, query).unwrap_or((50, vec![]));
                    cmd.match_indices = indices;
                    // If provider already set a high score (e.g. calculator/converter),
                    // preserve it instead of overwriting with fuzzy score
                    let final_score = if cmd.score >= 100 { cmd.score } else { fuzzy_score };
                    cmd.score = final_score;
                    scored.push((final_score, cmd));
                }
            }
        }

        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().map(|(_, cmd)| cmd).collect()
    }

    /// Search slash-commands by query, weighted by frecency.
    pub fn search_commands(
        &self,
        query: &str,
        frecency: &FrecencyTracker,
    ) -> Vec<CommandDefinition> {
        if query.is_empty() {
            let mut defs = self.cached_command_defs.clone();
            defs.sort_by(|a, b| {
                let sa = frecency.score(&a.id);
                let sb = frecency.score(&b.id);
                sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
            });
            return defs;
        }

        let matcher = SkimMatcherV2::default();
        let mut scored: Vec<(f64, CommandDefinition)> = self
            .cached_command_defs
            .iter()
            .filter_map(|def| {
                let display_name = format!("{}: {}", def.extension_id, def.name);
                let name_score = matcher.fuzzy_match(&display_name, query);
                let desc_score = matcher.fuzzy_match(&def.description, query).map(|s| s / 2);

                let fuzzy_score = match (name_score, desc_score) {
                    (Some(ns), Some(ds)) => ns.max(ds),
                    (Some(ns), None) => ns,
                    (None, Some(ds)) => ds,
                    (None, None) => return None,
                };

                let frecency_score = frecency.score(&def.id);
                let blended = fuzzy_score as f64 * 0.7 + frecency_score * 0.3;
                Some((blended, def.clone()))
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().map(|(_, def)| def).collect()
    }

    /// Execute a parameterized command.
    pub async fn execute_action(
        &self,
        command_id: &str,
        params: serde_json::Value,
    ) -> Result<CommandResult, String> {
        for provider in &self.providers {
            if let Some(result) = provider.execute_action(command_id, params.clone()).await {
                return Ok(result);
            }
        }
        Err(format!("Unknown command: {command_id}"))
    }

    /// Resolve autocomplete options for a parameter.
    pub async fn resolve_autocomplete(
        &self,
        command_id: &str,
        param_id: &str,
        query: &str,
    ) -> Vec<SelectOption> {
        for provider in &self.providers {
            let options = provider
                .resolve_autocomplete(command_id, param_id, query)
                .await;
            if !options.is_empty() {
                return options;
            }
        }
        vec![]
    }

    /// Undo an action by dispatching to the owning provider.
    pub async fn undo_action(
        &self,
        extension_id: &str,
        action_id: &str,
        undo_data: serde_json::Value,
    ) -> Result<CommandResult, String> {
        for provider in &self.providers {
            if provider.name() == extension_id {
                if let Some(result) = provider.undo_action(action_id, undo_data.clone()).await {
                    return Ok(result);
                }
            }
        }
        Err(format!("No provider found for undo: {extension_id}"))
    }

    /// Run icon enrichment on all providers, then rebuild the static cache.
    pub async fn enrich_icons(&mut self) {
        for provider in &mut self.providers {
            provider.enrich_icons().await;
        }
        self.refresh_cache().await;
    }

    pub fn execute(&self, id: &str) -> Result<String, String> {
        for provider in &self.providers {
            if let Some(result) = provider.execute(id) {
                return result;
            }
        }
        Err(format!("Unknown command: {id}"))
    }
}
