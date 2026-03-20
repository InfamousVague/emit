//! GPU metrics via macOS ioreg command.

use super::perf_monitor::GpuMetrics;

#[cfg(target_os = "macos")]
pub fn get_gpu_utilization() -> Option<GpuMetrics> {
    let output = std::process::Command::new("ioreg")
        .args(["-rc", "IOAccelerator", "-d", "2"])
        .output()
        .ok()?;

    let text = String::from_utf8(output.stdout).ok()?;
    if text.is_empty() {
        return None;
    }

    use super::ioreg::{parse_ioreg_float, parse_ioreg_string};

    let name = parse_ioreg_string(&text, "\"model\"").unwrap_or_else(|| String::from("GPU"));

    // Try different utilization keys
    let utilization = parse_ioreg_float(&text, "\"Device Utilization %\"")
        .or_else(|| parse_ioreg_float(&text, "\"GPU Core Utilization\""))
        .or_else(|| parse_ioreg_float(&text, "\"GPU Activity\""));

    utilization.map(|val| {
        // Some keys report as 0-100, others as 0-100000000
        let normalized = if val > 100.0 {
            val / 10000000.0
        } else {
            val
        };
        GpuMetrics {
            utilization: normalized as f32,
            name,
        }
    })
}

#[cfg(not(target_os = "macos"))]
pub fn get_gpu_utilization() -> Option<GpuMetrics> {
    None
}
