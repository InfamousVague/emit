use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const DECAY_RATE: f64 = 0.01; // half-life ~69 hours (~3 days)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrecencyEntry {
    pub frequency: u32,
    pub last_used: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrecencyTracker {
    entries: HashMap<String, FrecencyEntry>,
}

impl FrecencyTracker {
    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("com.infamousvague.emit")
            .join("frecency.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_else(|| FrecencyTracker {
                    entries: HashMap::new(),
                })
        } else {
            FrecencyTracker {
                entries: HashMap::new(),
            }
        }
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(&path, json);
        }
    }

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    pub fn record_use(&mut self, id: &str) {
        let now = Self::now_secs();
        let entry = self.entries.entry(id.to_string()).or_insert(FrecencyEntry {
            frequency: 0,
            last_used: now,
        });
        entry.frequency += 1;
        entry.last_used = now;
        self.save();
    }

    pub fn score(&self, id: &str) -> f64 {
        let entry = match self.entries.get(id) {
            Some(e) => e,
            None => return 0.0,
        };
        let now = Self::now_secs();
        let hours_since = (now.saturating_sub(entry.last_used)) as f64 / 3600.0;
        entry.frequency as f64 * (-DECAY_RATE * hours_since).exp()
    }

    #[allow(dead_code)]
    /// Return IDs sorted by score descending.
    pub fn ranked(&self) -> Vec<String> {
        let mut scored: Vec<(String, f64)> = self
            .entries
            .keys()
            .map(|id| (id.clone(), self.score(id)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().map(|(id, _)| id).collect()
    }

    #[allow(dead_code)]
    /// Return recently used IDs matching a prefix, sorted by score.
    pub fn recent_with_prefix(&self, prefix: &str) -> Vec<String> {
        let mut scored: Vec<(String, f64)> = self
            .entries
            .keys()
            .filter(|id| id.starts_with(prefix))
            .map(|id| (id.clone(), self.score(id)))
            .filter(|(_, s)| *s > 0.0)
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().map(|(id, _)| id).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_score() {
        let mut tracker = FrecencyTracker {
            entries: HashMap::new(),
        };
        tracker.record_use("test.command");
        assert!(tracker.score("test.command") > 0.0);
        assert_eq!(tracker.score("nonexistent"), 0.0);
    }

    #[test]
    fn test_frequency_increases_score() {
        let mut tracker = FrecencyTracker {
            entries: HashMap::new(),
        };
        tracker.record_use("cmd-a");
        let score_1 = tracker.score("cmd-a");
        tracker.record_use("cmd-a");
        let score_2 = tracker.score("cmd-a");
        assert!(score_2 > score_1);
    }

    #[test]
    fn test_ranked_order() {
        let now = FrecencyTracker::now_secs();
        let tracker = FrecencyTracker {
            entries: HashMap::from([
                (
                    "low".to_string(),
                    FrecencyEntry {
                        frequency: 1,
                        last_used: now,
                    },
                ),
                (
                    "high".to_string(),
                    FrecencyEntry {
                        frequency: 10,
                        last_used: now,
                    },
                ),
            ]),
        };
        let ranked = tracker.ranked();
        assert_eq!(ranked[0], "high");
        assert_eq!(ranked[1], "low");
    }
}
