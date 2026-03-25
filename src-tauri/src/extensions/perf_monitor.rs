//! Real-time system performance monitoring with CPU, RAM, disk, network, GPU, and battery metrics.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::launcher::CommandEntry;
use crate::providers::CommandProvider;
use crate::shortcuts::ShortcutBinding;

use super::perf_store::SharedMetricsStore;

// ── Data Structures ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSnapshot {
    pub timestamp: u64,
    pub cpu: CpuMetrics,
    pub memory: MemoryMetrics,
    pub disks: Vec<DiskMetrics>,
    pub network: NetworkMetrics,
    pub gpu: Option<GpuMetrics>,
    pub battery: Option<BatteryMetrics>,
    pub system: SystemMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuMetrics {
    pub total_usage: f32,
    pub per_core: Vec<f32>,
    pub load_avg_1: f64,
    pub load_avg_5: f64,
    pub load_avg_15: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetrics {
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub app_memory: u64,
    pub wired: u64,
    pub compressed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskMetrics {
    pub name: String,
    pub mount_point: String,
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub fs_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    pub upload_speed: u64,
    pub download_speed: u64,
    pub total_uploaded: u64,
    pub total_downloaded: u64,
    pub interfaces: Vec<NetworkInterface>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub ip: String,
    pub is_wifi: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuMetrics {
    pub utilization: f32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryMetrics {
    pub charge_percent: f32,
    pub health_percent: f32,
    pub temperature: f32,
    pub cycle_count: u32,
    pub power_draw: f32,
    pub is_charging: bool,
    pub time_remaining: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub uptime_secs: u64,
    pub hostname: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AlertConfig {
    pub thresholds: Vec<AlertThreshold>,
    pub cooldown_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThreshold {
    pub metric: String,
    pub threshold: f32,
    pub enabled: bool,
}

pub type SharedAlertConfig = Arc<RwLock<AlertConfig>>;

// ── Command Provider ─────────────────────────────────────────────────────────

pub struct PerfMonitorProvider {
    store: Option<SharedMetricsStore>,
}

impl PerfMonitorProvider {
    pub fn with_store(store: SharedMetricsStore) -> Self {
        Self { store: Some(store) }
    }
}

#[async_trait]
impl CommandProvider for PerfMonitorProvider {
    fn name(&self) -> &str {
        "PerfMonitor"
    }

    async fn commands(&self) -> Vec<CommandEntry> {
        vec![CommandEntry {
            id: "perf.open".into(),
            name: "Performance Monitor".into(),
            description: "View real-time system metrics and performance dashboard".into(),
            category: "Developer Tools".into(),
            icon: None,
            match_indices: vec![],
            score: 0,
        }]
    }

    fn is_dynamic(&self) -> bool {
        true
    }

    async fn search(&self, query: &str) -> Vec<CommandEntry> {
        let q = query.to_lowercase();
        let mut results = Vec::new();

        // Get live snapshot for inline readouts
        let snap = if let Some(ref store) = self.store {
            let s = store.read().await;
            s.buffer.back().cloned()
        } else {
            None
        };

        let metrics: Vec<(&str, &str, String, &[&str])> = vec![
            ("perf.cpu", "CPU Usage", {
                if let Some(ref s) = snap {
                    format!("{:.1}% — {} cores", s.cpu.total_usage, s.cpu.per_core.len())
                } else { "View CPU utilization and per-core stats".into() }
            }, &["cpu", "processor", "perf", "monitor", "performance"]),
            ("perf.memory", "Memory Usage", {
                if let Some(ref s) = snap {
                    let pct = if s.memory.total > 0 { (s.memory.used as f64 / s.memory.total as f64) * 100.0 } else { 0.0 };
                    format!("{:.1}% — {} / {}", pct, format_bytes_inline(s.memory.used), format_bytes_inline(s.memory.total))
                } else { "View RAM utilization breakdown".into() }
            }, &["ram", "memory", "mem", "perf", "monitor", "performance"]),
            ("perf.disk", "Disk Usage", {
                if let Some(ref s) = snap {
                    if let Some(d) = s.disks.first() {
                        let pct = if d.total > 0 { (d.used as f64 / d.total as f64) * 100.0 } else { 0.0 };
                        format!("{:.1}% — {} / {}", pct, format_bytes_inline(d.used), format_bytes_inline(d.total))
                    } else { "View storage usage for all volumes".into() }
                } else { "View storage usage for all volumes".into() }
            }, &["disk", "storage", "drive", "ssd", "perf", "monitor", "performance"]),
            ("perf.network", "Network Activity", {
                if let Some(ref s) = snap {
                    format!("↓ {}/s  ↑ {}/s", format_bytes_inline(s.network.download_speed), format_bytes_inline(s.network.upload_speed))
                } else { "View upload/download speeds and connections".into() }
            }, &["network", "net", "wifi", "internet", "bandwidth", "perf", "monitor", "performance"]),
            ("perf.gpu", "GPU Usage", {
                if let Some(ref s) = snap {
                    if let Some(ref g) = s.gpu { format!("{:.1}% — {}", g.utilization, g.name) } else { "No GPU data".into() }
                } else { "View GPU utilization".into() }
            }, &["gpu", "graphics", "perf", "monitor", "performance"]),
            ("perf.battery", "Battery Status", {
                if let Some(ref s) = snap {
                    if let Some(ref b) = s.battery {
                        format!("{:.0}%{} — Health {:.0}%", b.charge_percent, if b.is_charging { " ⚡" } else { "" }, b.health_percent)
                    } else { "No battery detected".into() }
                } else { "View battery charge, health, and power info".into() }
            }, &["battery", "power", "charge", "perf", "monitor", "performance"]),
            ("perf.uptime", "System Uptime", {
                if let Some(ref s) = snap {
                    let secs = s.system.uptime_secs;
                    let d = secs / 86400; let h = (secs % 86400) / 3600; let m = (secs % 3600) / 60;
                    let mut parts = Vec::new();
                    if d > 0 { parts.push(format!("{}d", d)); }
                    if h > 0 { parts.push(format!("{}h", h)); }
                    parts.push(format!("{}m", m));
                    format!("{} — Load {:.2} {:.2} {:.2}", parts.join(" "), s.cpu.load_avg_1, s.cpu.load_avg_5, s.cpu.load_avg_15)
                } else { "View system uptime and load averages".into() }
            }, &["uptime", "load", "perf", "monitor", "performance"]),
        ];

        for (id, name, desc, keywords) in &metrics {
            if keywords.iter().any(|k| k.starts_with(&q) || q.starts_with(k)) {
                results.push(CommandEntry {
                    id: id.to_string(),
                    name: name.to_string(),
                    description: desc.clone(),
                    category: "Developer Tools".into(),
                    icon: None,
                    match_indices: vec![],
                    score: 80,
                });
            }
        }

        results
    }

    fn execute(&self, id: &str) -> Option<Result<String, String>> {
        match id {
            "perf.open" => Some(Ok("view:perf".into())),
            "perf.cpu" => Some(Ok("view:perf:cpu".into())),
            "perf.memory" => Some(Ok("view:perf:memory".into())),
            "perf.disk" => Some(Ok("view:perf:disk".into())),
            "perf.network" => Some(Ok("view:perf:network".into())),
            "perf.gpu" => Some(Ok("view:perf:gpu".into())),
            "perf.battery" => Some(Ok("view:perf:battery".into())),
            "perf.uptime" => Some(Ok("view:perf".into())),
            _ => None,
        }
    }

    fn shortcuts(&self) -> Vec<ShortcutBinding> {
        vec![ShortcutBinding {
            id: "perf.dashboard".into(),
            label: "Performance Monitor".into(),
            default_keys: "Shift+Cmd+P".into(),
            keys: "Shift+Cmd+P".into(),
            extension_id: "perf-monitor".into(),
        }]
    }
}

fn format_bytes_inline(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;
    const TB: u64 = 1024 * 1024 * 1024 * 1024;
    if bytes >= TB { format!("{:.1} TB", bytes as f64 / TB as f64) }
    else if bytes >= GB { format!("{:.1} GB", bytes as f64 / GB as f64) }
    else if bytes >= MB { format!("{:.1} MB", bytes as f64 / MB as f64) }
    else if bytes >= KB { format!("{:.1} KB", bytes as f64 / KB as f64) }
    else { format!("{} B", bytes) }
}

// ── Collector ────────────────────────────────────────────────────────────────

pub fn start_collector(
    app: tauri::AppHandle,
    store: SharedMetricsStore,
    alert_config: SharedAlertConfig,
) {
    use tauri::Emitter;

    tauri::async_runtime::spawn(async move {
        let mut sys = sysinfo::System::new_all();
        let mut networks = sysinfo::Networks::new_with_refreshed_list();
        let mut disks = sysinfo::Disks::new_with_refreshed_list();
        let mut prev_rx: u64 = 0;
        let mut prev_tx: u64 = 0;
        let mut last_alert_time: std::collections::HashMap<String, std::time::Instant> =
            std::collections::HashMap::new();
        let hostname_str = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".into());

        // Initial refresh to populate baseline
        sys.refresh_all();
        std::thread::sleep(Duration::from_millis(500));

        let mut tick: u64 = 0;

        loop {
            sys.refresh_cpu_usage();
            sys.refresh_memory();
            networks.refresh(true);

            // Heavy operations on staggered intervals to reduce memory/CPU pressure
            // Processes: every 5 seconds (tick % 10 == 0)
            if tick % 10 == 0 {
                sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
            }
            // Disks: every 10 seconds (tick % 20 == 0)
            if tick % 20 == 0 {
                disks.refresh(true);
            }

            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            // CPU
            let total_cpu = sys.global_cpu_usage();
            let per_core: Vec<f32> = sys.cpus().iter().map(|c| c.cpu_usage()).collect();
            let load_avg = sysinfo::System::load_average();

            // Memory
            let total_mem = sys.total_memory();
            let used_mem = sys.used_memory();
            let available_mem = sys.available_memory();

            // Memory breakdown via vm_stat (macOS-specific) — every 5 seconds
            let (wired, compressed, app_memory) = if tick % 10 == 2 || tick == 0 {
                get_memory_breakdown()
            } else {
                let s = store.read().await;
                s.buffer.back().map(|snap| (snap.memory.wired, snap.memory.compressed, snap.memory.app_memory)).unwrap_or((0, 0, 0))
            };

            // Disk
            let disk_metrics: Vec<DiskMetrics> = disks
                .iter()
                .filter(|d| {
                    let mp = d.mount_point().to_string_lossy();
                    !mp.starts_with("/System/Volumes/") || mp == "/System/Volumes/Data"
                })
                .map(|d| DiskMetrics {
                    name: d.name().to_string_lossy().to_string(),
                    mount_point: d.mount_point().to_string_lossy().to_string(),
                    total: d.total_space(),
                    used: d.total_space() - d.available_space(),
                    available: d.available_space(),
                    fs_type: format!("{:?}", d.file_system()),
                })
                .collect();

            // Network
            let mut total_rx: u64 = 0;
            let mut total_tx: u64 = 0;
            let mut net_interfaces = Vec::new();

            for (name, data) in networks.iter() {
                total_rx += data.total_received();
                total_tx += data.total_transmitted();

                // Only list non-loopback interfaces
                if name != "lo0" && name != "lo" {
                    net_interfaces.push(NetworkInterface {
                        name: name.clone(),
                        ip: String::new(), // sysinfo doesn't provide IPs directly
                        is_wifi: name.starts_with("en") || name.contains("Wi-Fi"),
                    });
                }
            }

            let download_speed = total_rx.saturating_sub(prev_rx) * 2; // per second (500ms interval)
            let upload_speed = total_tx.saturating_sub(prev_tx) * 2;
            prev_rx = total_rx;
            prev_tx = total_tx;

            // GPU: every 5 seconds (tick % 10 == 5, offset from processes)
            let gpu = if tick % 10 == 5 || tick == 0 {
                super::perf_gpu::get_gpu_utilization()
            } else {
                // Reuse last snapshot's GPU data
                let s = store.read().await;
                s.buffer.back().and_then(|snap| snap.gpu.clone())
            };

            // Battery: every 30 seconds (tick % 60 == 0)
            let battery = if tick % 60 == 0 {
                super::perf_battery::get_battery_info()
            } else {
                let s = store.read().await;
                s.buffer.back().and_then(|snap| snap.battery.clone())
            };

            // System
            let uptime = sysinfo::System::uptime();

            let snapshot = MetricSnapshot {
                timestamp,
                cpu: CpuMetrics {
                    total_usage: total_cpu,
                    per_core,
                    load_avg_1: load_avg.one,
                    load_avg_5: load_avg.five,
                    load_avg_15: load_avg.fifteen,
                },
                memory: MemoryMetrics {
                    total: total_mem,
                    used: used_mem,
                    available: available_mem,
                    app_memory,
                    wired,
                    compressed,
                },
                disks: disk_metrics,
                network: NetworkMetrics {
                    upload_speed,
                    download_speed,
                    total_uploaded: total_tx,
                    total_downloaded: total_rx,
                    interfaces: net_interfaces,
                },
                gpu,
                battery,
                system: SystemMetrics {
                    uptime_secs: uptime,
                    hostname: hostname_str.clone(),
                },
            };

            // Collect top processes (only when we refreshed them)
            if tick % 10 == 0 {
                let mut procs: Vec<ProcessInfo> = sys
                    .processes()
                    .values()
                    .map(|p| ProcessInfo {
                        pid: p.pid().as_u32(),
                        name: p.name().to_string_lossy().to_string(),
                        cpu_usage: p.cpu_usage(),
                        memory_bytes: p.memory(),
                    })
                    .filter(|p| p.cpu_usage > 0.0 || p.memory_bytes > 10_000_000)
                    .collect();
                procs.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap_or(std::cmp::Ordering::Equal));
                procs.truncate(50);

                let mut s = store.write().await;
                s.push(snapshot.clone());
                s.processes = procs;
            } else {
                let mut s = store.write().await;
                s.push(snapshot.clone());
            }

            // Check alerts
            check_alerts(&snapshot, &alert_config, &mut last_alert_time).await;

            // Emit to frontend
            let _ = app.emit("perf-update", &snapshot);

            tick = tick.wrapping_add(1);
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });
}

fn get_memory_breakdown() -> (u64, u64, u64) {
    // Parse vm_stat output for detailed memory info
    if let Ok(output) = std::process::Command::new("vm_stat").output() {
        if let Ok(text) = String::from_utf8(output.stdout) {
            let page_size: u64 = 16384; // Apple Silicon default
            let mut wired: u64 = 0;
            let mut compressed: u64 = 0;
            let mut app: u64 = 0;

            for line in text.lines() {
                if line.contains("Pages wired down") {
                    if let Some(val) = parse_vm_stat_value(line) {
                        wired = val * page_size;
                    }
                } else if line.contains("Pages occupied by compressor") {
                    if let Some(val) = parse_vm_stat_value(line) {
                        compressed = val * page_size;
                    }
                } else if line.contains("Pages active") {
                    if let Some(val) = parse_vm_stat_value(line) {
                        app = val * page_size;
                    }
                }
            }

            return (wired, compressed, app);
        }
    }
    (0, 0, 0)
}

fn parse_vm_stat_value(line: &str) -> Option<u64> {
    line.split(':')
        .nth(1)?
        .trim()
        .trim_end_matches('.')
        .parse::<u64>()
        .ok()
}

async fn check_alerts(
    snapshot: &MetricSnapshot,
    config: &SharedAlertConfig,
    last_alert: &mut std::collections::HashMap<String, std::time::Instant>,
) {
    let cfg = config.read().await;
    let cooldown = Duration::from_secs(cfg.cooldown_secs.max(30));

    for threshold in &cfg.thresholds {
        if !threshold.enabled {
            continue;
        }

        let current_value = match threshold.metric.as_str() {
            "cpu" => snapshot.cpu.total_usage,
            "ram" => {
                if snapshot.memory.total > 0 {
                    (snapshot.memory.used as f32 / snapshot.memory.total as f32) * 100.0
                } else {
                    0.0
                }
            }
            "gpu" => snapshot.gpu.as_ref().map(|g| g.utilization).unwrap_or(0.0),
            _ => continue,
        };

        if current_value > threshold.threshold {
            if let Some(last) = last_alert.get(&threshold.metric) {
                if last.elapsed() < cooldown {
                    continue;
                }
            }

            // Send macOS notification
            let title = format!("{} Alert", threshold.metric.to_uppercase());
            let body = format!(
                "{} usage is at {:.1}% (threshold: {:.0}%)",
                threshold.metric.to_uppercase(),
                current_value,
                threshold.threshold
            );
            send_notification(&title, &body);
            last_alert.insert(threshold.metric.clone(), std::time::Instant::now());
        }
    }
}

fn send_notification(title: &str, body: &str) {
    let script = format!(
        r#"display notification "{}" with title "{}""#,
        body.replace('"', "\\\""),
        title.replace('"', "\\\"")
    );
    std::process::Command::new("osascript")
        .args(["-e", &script])
        .spawn()
        .ok();
}

// ── Tauri Commands ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn perf_get_snapshot(
    store: tauri::State<'_, SharedMetricsStore>,
) -> Result<MetricSnapshot, String> {
    let s = store.read().await;
    s.buffer
        .back()
        .cloned()
        .ok_or_else(|| "No data yet".into())
}

#[tauri::command]
pub async fn perf_get_history(
    range: String,
    store: tauri::State<'_, SharedMetricsStore>,
) -> Result<Vec<MetricSnapshot>, String> {
    let range_ms = match range.as_str() {
        "1m" => 60_000u64,
        "5m" => 300_000,
        "15m" => 900_000,
        "1hr" => 3_600_000,
        _ => 300_000,
    };

    let s = store.read().await;
    Ok(s.query(range_ms, 200))
}

#[tauri::command]
pub async fn perf_get_processes(
    sort_by: String,
    limit: usize,
    store: tauri::State<'_, SharedMetricsStore>,
) -> Result<Vec<ProcessInfo>, String> {
    let s = store.read().await;
    let mut procs = s.processes.clone();

    match sort_by.as_str() {
        "ram" | "memory" => {
            procs.sort_by(|a, b| {
                b.memory_bytes
                    .partial_cmp(&a.memory_bytes)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        "pid" => {
            procs.sort_by_key(|p| p.pid);
        }
        _ => {
            // Default: sort by CPU
            procs.sort_by(|a, b| {
                b.cpu_usage
                    .partial_cmp(&a.cpu_usage)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
    }

    procs.truncate(limit.min(50));
    Ok(procs)
}

#[tauri::command]
pub async fn perf_get_alerts(
    config: tauri::State<'_, SharedAlertConfig>,
) -> Result<AlertConfig, String> {
    let cfg = config.read().await;
    Ok(cfg.clone())
}

#[tauri::command]
pub async fn perf_save_alerts(
    new_config: AlertConfig,
    config: tauri::State<'_, SharedAlertConfig>,
) -> Result<(), String> {
    let mut cfg = config.write().await;
    *cfg = new_config.clone();

    // Persist to disk
    let dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("com.infamousvague.emit");
    std::fs::create_dir_all(&dir).ok();
    let path = dir.join("perf_alerts.json");
    let json = serde_json::to_string_pretty(&new_config).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn perf_resize_window(app: tauri::AppHandle, height: u32) -> Result<(), String> {
    use tauri::Manager;
    if let Some(window) = app.get_webview_window("main") {
        let current_size = window.outer_size().map_err(|e| e.to_string())?;
        let new_size = tauri::PhysicalSize::new(current_size.width, height);
        window.set_size(new_size).map_err(|e| e.to_string())?;
        window.center().map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn load_alert_config() -> AlertConfig {
    let path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("com.infamousvague.emit")
        .join("perf_alerts.json");

    match std::fs::read_to_string(&path) {
        Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
        Err(_) => AlertConfig {
            thresholds: vec![
                AlertThreshold {
                    metric: "cpu".into(),
                    threshold: 90.0,
                    enabled: false,
                },
                AlertThreshold {
                    metric: "ram".into(),
                    threshold: 90.0,
                    enabled: false,
                },
                AlertThreshold {
                    metric: "gpu".into(),
                    threshold: 95.0,
                    enabled: false,
                },
            ],
            cooldown_secs: 60,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perf_provider() {
        let provider = PerfMonitorProvider { store: None };
        assert_eq!(provider.name(), "PerfMonitor");
        assert!(provider.is_dynamic());
        assert_eq!(
            provider.execute("perf.open"),
            Some(Ok("view:perf".into()))
        );
        assert_eq!(
            provider.execute("perf.cpu"),
            Some(Ok("view:perf:cpu".into()))
        );
        assert_eq!(provider.execute("unknown"), None);
    }

    #[test]
    fn test_parse_vm_stat_value() {
        assert_eq!(parse_vm_stat_value("Pages wired down:    123456."), Some(123456));
        assert_eq!(parse_vm_stat_value("Invalid line"), None);
    }
}
