//! Battery metrics via macOS ioreg command.

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

    use super::ioreg::{parse_ioreg_bool, parse_ioreg_int};

    let current_capacity = parse_ioreg_int(&text, "\"CurrentCapacity\"")? as f64;
    let max_capacity = parse_ioreg_int(&text, "\"MaxCapacity\"").unwrap_or(100) as f64;
    let design_capacity =
        parse_ioreg_int(&text, "\"DesignCapacity\"").unwrap_or(max_capacity as i64) as f64;
    let is_charging = parse_ioreg_bool(&text, "\"IsCharging\"");
    let cycle_count = parse_ioreg_int(&text, "\"CycleCount\"").unwrap_or(0) as u32;
    let temperature = parse_ioreg_int(&text, "\"Temperature\"").unwrap_or(0) as f64 / 100.0;
    let amperage = parse_ioreg_int(&text, "\"InstantAmperage\"").unwrap_or(0) as f64;
    let voltage = parse_ioreg_int(&text, "\"Voltage\"").unwrap_or(0) as f64 / 1000.0;
    let power_draw = (amperage.abs() * voltage) / 1000.0;
    let time_remaining = parse_ioreg_int(&text, "\"TimeRemaining\"").map(|t| t as u32);

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
