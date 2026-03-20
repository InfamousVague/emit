//! Shared ioreg output parsing utilities for macOS.

/// Parse an integer value for a given key from ioreg output.
///
/// Looks for lines containing `key` and extracts the integer after `=`.
#[cfg(target_os = "macos")]
pub fn parse_ioreg_int(text: &str, key: &str) -> Option<i64> {
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.contains(key) {
            if let Some(val_str) = trimmed.split('=').nth(1) {
                return val_str.trim().parse::<i64>().ok();
            }
        }
    }
    None
}

/// Check whether a boolean key is present and set to `Yes`.
#[cfg(target_os = "macos")]
pub fn parse_ioreg_bool(text: &str, key: &str) -> bool {
    for line in text.lines() {
        if line.contains(key) {
            return line.contains("Yes");
        }
    }
    false
}

/// Extract a string value for a given key from ioreg output.
///
/// Strips surrounding double-quotes from the value.
#[cfg(target_os = "macos")]
pub fn parse_ioreg_string(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.contains(key) {
            let val = trimmed.split('=').nth(1)?.trim();
            let val = val.trim_matches('"');
            return Some(val.to_string());
        }
    }
    None
}

/// Parse a floating-point value for a given key from ioreg output.
#[cfg(target_os = "macos")]
pub fn parse_ioreg_float(text: &str, key: &str) -> Option<f64> {
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.contains(key) {
            if let Some(val_str) = trimmed.split('=').nth(1) {
                return val_str.trim().parse::<f64>().ok();
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_IOREG: &str = r#"
    +-o AppleSmartBattery  <class AppleSmartBattery>
      {
        "CurrentCapacity" = 85
        "MaxCapacity" = 100
        "DesignCapacity" = 104
        "IsCharging" = Yes
        "CycleCount" = 42
        "Temperature" = 2950
        "Voltage" = 12800
        "model" = "Apple M1 GPU"
        "Device Utilization %" = 55
      }
    "#;

    #[test]
    fn parse_int_found() {
        assert_eq!(parse_ioreg_int(SAMPLE_IOREG, "\"CurrentCapacity\""), Some(85));
    }

    #[test]
    fn parse_int_missing() {
        assert_eq!(parse_ioreg_int(SAMPLE_IOREG, "\"MissingKey\""), None);
    }

    #[test]
    fn parse_bool_true() {
        assert!(parse_ioreg_bool(SAMPLE_IOREG, "\"IsCharging\""));
    }

    #[test]
    fn parse_bool_false() {
        assert!(!parse_ioreg_bool(SAMPLE_IOREG, "\"MissingKey\""));
    }

    #[test]
    fn parse_string_found() {
        assert_eq!(
            parse_ioreg_string(SAMPLE_IOREG, "\"model\""),
            Some("Apple M1 GPU".to_string()),
        );
    }

    #[test]
    fn parse_string_missing() {
        assert_eq!(parse_ioreg_string(SAMPLE_IOREG, "\"NoSuchKey\""), None);
    }

    #[test]
    fn parse_float_found() {
        assert_eq!(parse_ioreg_float(SAMPLE_IOREG, "\"Device Utilization %\""), Some(55.0));
    }

    #[test]
    fn parse_float_missing() {
        assert_eq!(parse_ioreg_float(SAMPLE_IOREG, "\"Nope\""), None);
    }
}
