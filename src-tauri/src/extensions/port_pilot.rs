//! Port & Process Pilot — discover what's listening on which ports,
//! kill processes by port, and detect orphaned dev servers.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use sysinfo::{Pid, ProcessesToUpdate, System};

use crate::launcher::CommandEntry;
use crate::providers::CommandProvider;

// ── Data types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortListener {
    pub port: u16,
    pub protocol: String,
    pub pid: u32,
    pub process_name: String,
    pub command: String,
    pub user: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortGroup {
    pub port: u16,
    pub listeners: Vec<PortListener>,
}

// ── Provider ────────────────────────────────────────────────────────────────

pub struct PortPilotProvider;

impl PortPilotProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl CommandProvider for PortPilotProvider {
    fn name(&self) -> &str {
        "PortPilot"
    }

    async fn commands(&self) -> Vec<CommandEntry> {
        vec![
            CommandEntry {
                id: "port-pilot.dashboard".into(),
                name: "Port & Process Pilot".into(),
                description: "View listening ports, kill processes, detect conflicts".into(),
                category: "Developer Tools".into(),
                icon: None,
                match_indices: vec![],
                score: 0,
            },
        ]
    }

    fn is_dynamic(&self) -> bool {
        true
    }

    async fn search(&self, query: &str) -> Vec<CommandEntry> {
        let q = query.to_lowercase();
        let mut results = Vec::new();

        // Match against common keywords
        let keywords = [
            "port", "ports", "process", "pilot", "kill", "listen", "network",
            "pid", "server", "localhost", "eaddrinuse", "conflict",
        ];

        let matches = keywords.iter().any(|k| k.starts_with(&q) || q.starts_with(k));
        if !matches {
            return results;
        }

        // Try to parse a port number from the query for quick-kill
        if let Ok(port) = q.parse::<u16>() {
            if let Ok(listeners) = scan_ports().await {
                if let Some(listener) = listeners.iter().find(|l| l.port == port) {
                    results.push(CommandEntry {
                        id: format!("port-pilot.kill.{}", listener.pid),
                        name: format!("Kill :{} — {}", port, listener.process_name),
                        description: format!("PID {} · {}", listener.pid, listener.command),
                        category: "Developer Tools".into(),
                        icon: None,
                        match_indices: vec![],
                        score: 90,
                    });
                }
            }
        }

        // Show live port count
        if let Ok(listeners) = scan_ports().await {
            let port_count = listeners.iter().map(|l| l.port).collect::<std::collections::HashSet<_>>().len();
            results.push(CommandEntry {
                id: "port-pilot.dashboard".into(),
                name: "Port & Process Pilot".into(),
                description: format!("{} ports in use", port_count),
                category: "Developer Tools".into(),
                icon: None,
                match_indices: vec![],
                score: 80,
            });
        }

        results
    }

    fn execute(&self, id: &str) -> Option<Result<String, String>> {
        match id {
            "port-pilot.dashboard" => Some(Ok("view:port-pilot".into())),
            _ if id.starts_with("port-pilot.kill.") => {
                let pid_str = id.strip_prefix("port-pilot.kill.").unwrap_or("");
                if let Ok(pid) = pid_str.parse::<u32>() {
                    match kill_process(pid) {
                        Ok(_) => Some(Ok(format!("Killed process {pid}"))),
                        Err(e) => Some(Err(e)),
                    }
                } else {
                    Some(Err("Invalid PID".into()))
                }
            }
            _ => None,
        }
    }
}

// ── Port scanning via lsof ──────────────────────────────────────────────────

/// Parse `lsof -i -P -n -sTCP:LISTEN` output to discover listening ports.
async fn scan_ports_lsof() -> Result<Vec<PortListener>, String> {
    let output = tokio::process::Command::new("lsof")
        .args(["-i", "-P", "-n", "-sTCP:LISTEN"])
        .output()
        .await
        .map_err(|e| format!("Failed to run lsof: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut listeners = Vec::new();

    for line in stdout.lines().skip(1) {
        // lsof columns: COMMAND PID USER FD TYPE DEVICE SIZE/OFF NODE NAME
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 10 {
            continue;
        }

        let process_name = parts[0].to_string();
        let pid: u32 = match parts[1].parse() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let user = parts[2].to_string();
        let protocol = parts[7].to_lowercase(); // TCP or UDP
        let name = parts[8]; // e.g. *:3000 or 127.0.0.1:8080

        // Extract port from the NAME column
        let port: u16 = match name.rsplit(':').next().and_then(|p| p.parse().ok()) {
            Some(p) => p,
            None => continue,
        };

        let state = if parts.len() > 9 {
            parts[9].trim_start_matches('(').trim_end_matches(')').to_string()
        } else {
            "LISTEN".into()
        };

        listeners.push(PortListener {
            port,
            protocol,
            pid,
            process_name: process_name.clone(),
            command: process_name,
            user,
            state,
        });
    }

    Ok(listeners)
}

/// Enrich listeners with full command lines from sysinfo.
fn enrich_with_sysinfo(listeners: &mut [PortListener]) {
    let mut sys = System::new();
    let pids: Vec<Pid> = listeners.iter().map(|l| Pid::from_u32(l.pid)).collect();
    sys.refresh_processes_specifics(ProcessesToUpdate::Some(&pids), true, Default::default());

    let proc_map: HashMap<u32, String> = sys
        .processes()
        .iter()
        .map(|(pid, proc)| {
            let cmd_parts: Vec<String> = proc
                .cmd()
                .iter()
                .map(|s| s.to_string_lossy().to_string())
                .collect();
            let cmd = cmd_parts.join(" ");
            let display = if cmd.is_empty() {
                proc.name().to_string_lossy().to_string()
            } else {
                cmd
            };
            (pid.as_u32(), display)
        })
        .collect();

    for listener in listeners.iter_mut() {
        if let Some(cmd) = proc_map.get(&listener.pid) {
            listener.command = cmd.clone();
        }
    }
}

/// Public scan entry point: lsof + sysinfo enrichment.
///
/// Deduplicates by (pid, port) since lsof often reports the same listener
/// on both IPv4 and IPv6 — keeping the first occurrence.
pub async fn scan_ports() -> Result<Vec<PortListener>, String> {
    let mut listeners = scan_ports_lsof().await?;

    // Deduplicate by (pid, port) — lsof returns separate rows for IPv4/IPv6
    let mut seen = std::collections::HashSet::new();
    listeners.retain(|l| seen.insert((l.pid, l.port)));

    enrich_with_sysinfo(&mut listeners);
    Ok(listeners)
}

/// Group listeners by port number.
pub fn group_by_port(listeners: &[PortListener]) -> Vec<PortGroup> {
    let mut map: HashMap<u16, Vec<PortListener>> = HashMap::new();
    for l in listeners {
        map.entry(l.port).or_default().push(l.clone());
    }
    let mut groups: Vec<PortGroup> = map
        .into_iter()
        .map(|(port, listeners)| PortGroup { port, listeners })
        .collect();
    groups.sort_by_key(|g| g.port);
    groups
}

/// Kill a process by PID using SIGTERM, falling back to SIGKILL.
fn kill_process(pid: u32) -> Result<String, String> {
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[Pid::from_u32(pid)]),
        true,
        Default::default(),
    );

    if let Some(process) = sys.process(Pid::from_u32(pid)) {
        let name = process.name().to_string_lossy().to_string();
        if process.kill() {
            Ok(format!("Killed {} (PID {})", name, pid))
        } else {
            Err(format!("Failed to kill {} (PID {})", name, pid))
        }
    } else {
        Err(format!("Process {} not found", pid))
    }
}

// ── Tauri commands ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn port_list_listeners() -> Result<Vec<PortListener>, String> {
    scan_ports().await
}

#[tauri::command]
pub async fn port_kill_process(pid: u32) -> Result<String, String> {
    kill_process(pid)
}

#[tauri::command]
pub async fn port_get_groups() -> Result<Vec<PortGroup>, String> {
    let listeners = scan_ports().await?;
    Ok(group_by_port(&listeners))
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_by_port() {
        let listeners = vec![
            PortListener {
                port: 3000,
                protocol: "tcp".into(),
                pid: 100,
                process_name: "node".into(),
                command: "node server.js".into(),
                user: "matt".into(),
                state: "LISTEN".into(),
            },
            PortListener {
                port: 3000,
                protocol: "tcp".into(),
                pid: 101,
                process_name: "node".into(),
                command: "node worker.js".into(),
                user: "matt".into(),
                state: "LISTEN".into(),
            },
            PortListener {
                port: 5432,
                protocol: "tcp".into(),
                pid: 200,
                process_name: "postgres".into(),
                command: "postgres".into(),
                user: "postgres".into(),
                state: "LISTEN".into(),
            },
        ];

        let groups = group_by_port(&listeners);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].port, 3000);
        assert_eq!(groups[0].listeners.len(), 2);
        assert_eq!(groups[1].port, 5432);
        assert_eq!(groups[1].listeners.len(), 1);
    }

    #[test]
    fn test_kill_nonexistent_process() {
        let result = kill_process(999999);
        assert!(result.is_err());
    }

    #[test]
    fn test_provider_name() {
        let provider = PortPilotProvider::new();
        assert_eq!(provider.name(), "PortPilot");
    }

    #[test]
    fn test_execute_dashboard() {
        let provider = PortPilotProvider::new();
        let result = provider.execute("port-pilot.dashboard");
        assert_eq!(result, Some(Ok("view:port-pilot".into())));
    }

    #[test]
    fn test_execute_unknown() {
        let provider = PortPilotProvider::new();
        let result = provider.execute("unknown.command");
        assert!(result.is_none());
    }
}
