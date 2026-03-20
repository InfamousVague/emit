//! Battery information via ioreg command (macOS).

use super::perf_monitor::BatteryMetrics;

#[cfg(target_os = "macos")]
pub fn get_battery_info() -> Option<BatteryMetrics> {
    let output = std::process::Command::new("ioreg")
        .args(["-rc", "AppleSmartBattery"])
        .output()
        .ok()?;

    let text = String::from_utf8(output.stdout).ok()?;
    if text.is_empty() {
        return None;
    }

    let get_int = |key: &str| -> Option<i64> {
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.contains(key) {
                if let Some(val_str) = trimmed.split('=').nth(1) {
                    return val_str.trim().parse::<i64>().ok();
                }
            }
        }
        None
    };

    let get_bool = |key: &str| -> bool {
        for line in text.lines() {
            if line.contains(key) {
                return line.contains("Yes");
            }
        }
        false
    };

    let current_capacity = get_int("\"CurrentCapacity\"")? as f64;
    let max_capacity = get_int("\"MaxCapacity\"").unwrap_or(100) as f64;
    let design_capacity = get_int("\"DesignCapacity\"").unwrap_or(max_capacity as i64) as f64;
    let is_charging = get_bool("\"IsCharging\"");
    let cycle_count = get_int("\"CycleCount\"").unwrap_or(0) as u32;
    let temperature = get_int("\"Temperature\"").unwrap_or(0) as f64 / 100.0;
    let amperage = get_int("\"InstantAmperage\"").unwrap_or(0) as f64;
    let voltage = get_int("\"Voltage\"").unwrap_or(0) as f64 / 1000.0;
    let power_draw = (amperage.abs() * voltage) / 1000.0;
    let time_remaining = get_int("\"TimeRemaining\"").map(|t| t as u32);

    let charge_percent = if max_capacity > 0.0 {
        (current_capacity / max_capacity * 100.0) as f32
    } else {
        0.0
    };

    let health_percent = if design_capacity > 0.0 {
        (max_capacity / design_capacity * 100.0) as f32
    } else {
        100.0
    };

    Some(BatteryMetrics {
        charge_percent,
        health_percent,
        temperature: temperature as f32,
        cycle_count,
        power_draw: power_draw as f32,
        is_charging,
        time_remaining: if time_remaining == Some(65535) {
            None
        } else {
            time_remaining
        },
    })
}

#[cfg(not(target_os = "macos"))]
pub fn get_battery_info() -> Option<BatteryMetrics> {
    None
}
