use crate::config::config::InstanceConfigFile;
use crate::providers::oracle::InstanceConfig;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn build_instance_config(config: &InstanceConfigFile) -> InstanceConfig {
    let base = if config.instance.instance_type == "amd" {
        InstanceConfig::amd_micro(&config.instance.display_name)
    } else {
        let ocpus = config.instance.arm_ocpus.unwrap_or(2);
        let memory = config.instance.arm_memory_gb.unwrap_or(12);
        InstanceConfig::arm_flex(&config.instance.display_name, ocpus, memory)
    };
    base.with_public_ip(config.network.assign_public_ip)
        .with_boot_volume_size(config.instance.boot_volume_size_gb)
        .with_tag("managed-by", "cloud-manage-rs")
}

pub fn parse_positive_f64(s: &str, fallback: f64) -> f64 {
    s.trim().parse::<f64>().ok().filter(|v| *v >= 0.0).unwrap_or(fallback)
}

pub fn humanize_oci_error(msg: &str) -> String {
    if let Some(start) = msg.find('{') {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&msg[start..]) {
            let code = v.get("code").and_then(|s| s.as_str());
            let message = v.get("message").and_then(|s| s.as_str());
            return match (code, message) {
                (Some(c), Some(m)) => format!("{} ({})", m, c),
                (_, Some(m)) => m.to_string(),
                (Some(c), _) => c.to_string(),
                _ => msg.to_string(),
            };
        }
    }
    msg.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn is_retryable_oci_error(msg: &str) -> bool {
    if let Some(rest) = msg.strip_prefix("API error ") {
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(status) = digits.parse::<u16>() {
            return status == 429 || (500..=599).contains(&status);
        }
    }
    let lower = msg.to_lowercase();
    lower.contains("timed out")
        || lower.contains("timeout")
        || lower.contains("connection")
        || lower.contains("dns")
        || lower.contains("reset by peer")
}

pub fn random_in_range(min: f64, max: f64) -> f64 {
    if (max - min).abs() < f64::EPSILON {
        return min;
    }
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let r = (nanos as f64) / 1_000_000_000.0;
    min + r * (max - min)
}

pub fn format_secs(v: f64) -> String {
    if (v.fract()).abs() < f64::EPSILON {
        format!("{}", v as i64)
    } else {
        format!("{}", v)
    }
}
