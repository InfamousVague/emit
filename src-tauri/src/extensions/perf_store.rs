//! Ring buffer for metric history + persistence.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::perf_monitor::{MetricSnapshot, ProcessInfo};

pub type SharedMetricsStore = Arc<RwLock<MetricsStore>>;

/// Maximum entries: 24 hours at 2 samples/sec = 172,800
const MAX_BUFFER_SIZE: usize = 172_800;

#[derive(Debug, Default)]
pub struct MetricsStore {
    pub buffer: VecDeque<MetricSnapshot>,
    pub processes: Vec<ProcessInfo>,
}

impl MetricsStore {
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::with_capacity(8192),
            processes: Vec::new(),
        }
    }

    pub fn push(&mut self, snapshot: MetricSnapshot) {
        if self.buffer.len() >= MAX_BUFFER_SIZE {
            self.buffer.pop_front();
        }
        self.buffer.push_back(snapshot);
    }

    /// Query snapshots within a time range, downsampled to target_points.
    pub fn query(&self, range_ms: u64, target_points: usize) -> Vec<MetricSnapshot> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let cutoff = now.saturating_sub(range_ms);

        let entries: Vec<&MetricSnapshot> = self
            .buffer
            .iter()
            .filter(|s| s.timestamp >= cutoff)
            .collect();

        if entries.len() <= target_points {
            return entries.into_iter().cloned().collect();
        }

        // Downsample by taking evenly spaced entries
        let step = entries.len() as f64 / target_points as f64;
        (0..target_points)
            .map(|i| {
                let idx = (i as f64 * step) as usize;
                entries[idx.min(entries.len() - 1)].clone()
            })
            .collect()
    }

    fn storage_path() -> PathBuf {
        let dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("com.infamousvague.emit");
        std::fs::create_dir_all(&dir).ok();
        dir.join("perf_history.bin")
    }

    #[allow(dead_code)]
    pub fn save_to_disk(&self) {
        let path = Self::storage_path();
        // Only save the last hour of data to keep file size reasonable
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let one_hour_ago = now.saturating_sub(3_600_000);

        let recent: Vec<&MetricSnapshot> = self
            .buffer
            .iter()
            .filter(|s| s.timestamp >= one_hour_ago)
            .collect();

        if let Ok(encoded) = bincode::serialize(&recent) {
            std::fs::write(path, encoded).ok();
        }
    }

    pub fn load_from_disk() -> Self {
        let path = Self::storage_path();
        let mut store = Self::new();

        if let Ok(data) = std::fs::read(&path) {
            if let Ok(snapshots) = bincode::deserialize::<Vec<MetricSnapshot>>(&data) {
                for s in snapshots {
                    store.buffer.push_back(s);
                }
            }
        }

        store
    }
}
