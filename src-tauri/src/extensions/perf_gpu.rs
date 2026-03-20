//! GPU utilization via ioreg command (macOS).

use super::perf_monitor::GpuMetrics;

#[cfg(target_os = "macos")]
pub fn get_gpu_utilization() -> Option<GpuMetrics> {
    // Use ioreg to query GPU performance statistics
    let output = std::process::Command::new("ioreg")
        .args(["-rc", "IOAccelerator", "-d", "2"])
        .output()
        .ok()?;

    let text = String::from_utf8(output.stdout).ok()?;
    if text.is_empty() {
        return None;
    }

    // Look for GPU utilization values
    let mut utilization: Option<f64> = None;
    let mut name = String::from("GPU");

    for line in text.lines() {
        let trimmed = line.trim();

        // Get GPU name
        if trimmed.contains("\"model\"") {
            if let Some(val) = extract_string_value(trimmed) {
                name = val;
            }
        }

        // Try different utilization keys
        if trimmed.contains("\"Device Utilization %\"")
            || trimmed.contains("\"GPU Core Utilization\"")
            || trimmed.contains("\"GPU Activity\"")
        {
            if let Some(val) = extract_number_value(trimmed) {
                // Some keys report as 0-100, others as 0-100000000
                utilization = Some(if val > 100.0 {
                    val / 10000000.0 // Normalize from 0-10^9 range
                } else {
                    val
                });
            }
        }
    }

    utilization.map(|u| GpuMetrics {
        utilization: u as f32,
        name,
    })
}

#[cfg(target_os = "macos")]
fn extract_number_value(line: &str) -> Option<f64> {
    line.split('=')
        .nth(1)?
        .trim()
        .parse::<f64>()
        .ok()
}

#[cfg(target_os = "macos")]
fn extract_string_value(line: &str) -> Option<String> {
    let val = line.split('=').nth(1)?.trim();
    let val = val.trim_matches('"');
    Some(val.to_string())
}

#[cfg(not(target_os = "macos"))]
pub fn get_gpu_utilization() -> Option<GpuMetrics> {
    None
}
