mod config;
mod ghost_input;
mod providers;
mod wizard;

use config::InstanceConfigFile;
use ghost_input::ghost_input;
use providers::oracle::{InstanceConfig, OracleInstanceCreator};
use wizard::ConfigWizard;
use dialoguer::{theme::ColorfulTheme, Select};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;

const CONFIG_FILE: &str = "./config/instance_config.toml";
const OCI_CONFIG_FILE: &str = "./config/oci_config";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("\n╔════════════════════════════════════════╗");
    println!("║  Oracle Cloud Instance Manager         ║");
    println!("╚════════════════════════════════════════╝\n");
    
    let config = load_or_create_config().await?;
    
    loop {
        let options = vec![
            "Create Instance",
            "Snipe Instance (retry until success)",
            "Reconfigure",
            "Quick Config (Instance Only)",
            "View Current Config",
            "Exit",
        ];
        
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select operation")
            .items(&options)
            .default(0)
            .interact()?;
        
        match selection {
            0 => create_instance(&config).await?,
            1 => snipe_instance(&config).await?,
            2 => {
                let new_config = reconfigure_full().await?;
                new_config.save_to_file(CONFIG_FILE)?;
                println!("\n✅ Configuration saved. Please restart the program.");
                break;
            }
            3 => {
                let new_config = reconfigure_quick(&config).await?;
                new_config.save_to_file(CONFIG_FILE)?;
                println!("\n✅ Configuration saved. Please restart the program.");
                break;
            }
            4 => display_config(&config)?,
            5 => {
                println!("\n👋 Goodbye!");
                break;
            }
            _ => unreachable!(),
        }
    }
    
    Ok(())
}

async fn load_or_create_config() -> Result<InstanceConfigFile, Box<dyn std::error::Error + Send + Sync>> {
    if InstanceConfigFile::exists(CONFIG_FILE) {
        println!("📂 Loading config from: {}", CONFIG_FILE);
        match InstanceConfigFile::load_from_file(CONFIG_FILE) {
            Ok(config) => {
                println!("✅ Configuration loaded successfully\n");
                return Ok(config);
            }
            Err(e) => {
                println!("⚠️  Failed to load config: {}", e);
                println!("Creating new configuration...\n");
            }
        }
    } else {
        println!("📝 Config file not found. Starting configuration wizard...\n");
    }
    
    if !Path::new(OCI_CONFIG_FILE).exists() {
        return Err(format!(
            "❌ OCI config file not found: {}\nPlease create OCI config file first",
            OCI_CONFIG_FILE
        ).into());
    }
    
    let wizard = ConfigWizard::new(OCI_CONFIG_FILE);
    let default_config = InstanceConfigFile::default();
    let config = wizard.run(&default_config).await?;
    
    config.save_to_file(CONFIG_FILE)?;
    println!("\n✅ Configuration saved to: {}", CONFIG_FILE);
    
    Ok(config)
}

async fn reconfigure_full() -> Result<InstanceConfigFile, Box<dyn std::error::Error + Send + Sync>> {
    println!("\n🔧 Full Reconfiguration");
    
    let current_config = if InstanceConfigFile::exists(CONFIG_FILE) {
        InstanceConfigFile::load_from_file(CONFIG_FILE).unwrap_or_default()
    } else {
        InstanceConfigFile::default()
    };
    
    let wizard = ConfigWizard::new(OCI_CONFIG_FILE);
    wizard.run(&current_config).await
}

async fn reconfigure_quick(base_config: &InstanceConfigFile) -> Result<InstanceConfigFile, Box<dyn std::error::Error + Send + Sync>> {
    let wizard = ConfigWizard::new(OCI_CONFIG_FILE);
    wizard.quick_config(base_config).await
}

fn display_config(config: &InstanceConfigFile) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("\n📋 Current Configuration");
    println!("═══════════════════════════════════════");
    println!("\n🔹 Oracle Cloud Config:");
    println!("  Compartment ID: {}", config.oracle.compartment_id);
    println!("  Availability Domain: {}", config.oracle.availability_domain);
    println!("  Subnet ID: {}", config.oracle.subnet_id);
    println!("  AMD Image: {}", config.oracle.image_id_amd);
    println!("  ARM Image: {}", config.oracle.image_id_arm);
    println!("  SSH Public Key: {}...", &config.oracle.ssh_public_key.chars().take(50).collect::<String>());
    
    println!("\n🔹 Instance Config:");
    println!("  Type: {}", if config.instance.instance_type == "amd" { "AMD Micro" } else { "ARM Flex" });
    println!("  Name: {}", config.instance.display_name);
    if let (Some(ocpus), Some(memory)) = (config.instance.arm_ocpus, config.instance.arm_memory_gb) {
        println!("  OCPU: {}", ocpus);
        println!("  Memory: {} GB", memory);
    }
    println!("  Boot Volume: {} GB", config.instance.boot_volume_size_gb);
    
    println!("\n🔹 Network Config:");
    println!("  Public IPv4: {}", if config.network.assign_public_ip { "Yes" } else { "No" });
    println!("  IPv6:        {}", if config.network.assign_ipv6 { "Yes" } else { "No" });
    if let Some(ip) = &config.network.private_ip {
        println!("  Private IPv4: {}", ip);
    }
    if let Some(ipv6) = &config.network.ipv6_address {
        println!("  IPv6 Address: {}", ipv6);
    }
    if let Some(host) = &config.network.hostname_label {
        println!("  Hostname:     {}", host);
    }
    println!();

    Ok(())
}

async fn create_instance(config: &InstanceConfigFile) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("\n🚀 Creating instance...\n");
    
    if !Path::new(OCI_CONFIG_FILE).exists() {
        return Err(format!("OCI config file not found: {}", OCI_CONFIG_FILE).into());
    }
    
    let instance_config = build_instance_config(config);
    let creator = OracleInstanceCreator::from_config(OCI_CONFIG_FILE, config.clone());
    
    match creator.create_and_wait(&instance_config, 300).await {
        Ok(instance_id) => {
            println!("\n✅ Instance created successfully!");
            println!("📌 Instance ID: {}", instance_id);
            println!("\nTip: Use OCI Console to view instance details and IP address");
        }
        Err(e) => {
            println!("\n❌ Instance creation failed: {}", e);
        }
    }
    
    println!("\nPress Enter to continue...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    
    Ok(())
}

async fn snipe_instance(config: &InstanceConfigFile) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("\n🎯 Snipe Mode: keep retrying until an instance is launched");
    println!("   (Ctrl+C to stop at any time)\n");

    if !Path::new(OCI_CONFIG_FILE).exists() {
        return Err(format!("OCI config file not found: {}", OCI_CONFIG_FILE).into());
    }

    let min_delay = parse_positive_f64(
        &ghost_input("Min delay between attempts (seconds)", "", "5")?,
        5.0,
    );
    let max_delay = parse_positive_f64(
        &ghost_input("Max delay between attempts (seconds)", "", "30")?,
        30.0,
    );
    let (min_delay, max_delay) = if min_delay <= max_delay {
        (min_delay, max_delay)
    } else {
        (max_delay, min_delay)
    };
    let max_attempts: u32 = ghost_input("Max attempts (0 = unlimited)", "", "0")?
        .trim()
        .parse()
        .unwrap_or(0);

    let creator = OracleInstanceCreator::from_config(OCI_CONFIG_FILE, config.clone());
    let instance_config = build_instance_config(config);

    let mut attempt: u32 = 0;
    let outcome = loop {
        attempt += 1;
        if max_attempts != 0 && attempt > max_attempts {
            break Err(format!("reached max attempts ({})", max_attempts));
        }

        println!("\n[#{}] launching...", attempt);
        match creator.create_instance(&instance_config).await {
            Ok(id) => break Ok(id),
            Err(e) => {
                println!("   ✖ {}", humanize_oci_error(&e.to_string()));
            }
        }

        let delay = random_in_range(min_delay, max_delay);
        println!("   ⏳ retrying in {:.1}s...", delay);
        sleep(Duration::from_secs_f64(delay)).await;
    };

    match outcome {
        Ok(instance_id) => {
            println!("\n✅ Snipe successful on attempt #{}!", attempt);
            println!("📌 Instance ID: {}", instance_id);
            println!("\nTip: Use OCI Console to view instance details and IP address");
        }
        Err(reason) => {
            println!("\n⏹️  Stopped: {}", reason);
        }
    }

    println!("\nPress Enter to continue...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(())
}

fn build_instance_config(config: &InstanceConfigFile) -> InstanceConfig {
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

fn parse_positive_f64(s: &str, fallback: f64) -> f64 {
    s.trim().parse::<f64>().ok().filter(|v| *v >= 0.0).unwrap_or(fallback)
}

/// Extract the meaningful piece of an OCI API error. The SDK formats errors
/// as `API error <STATUS>: <json-body>`, where the body looks like
/// `{ "code": "...", "message": "..." }`. We pull out `message (code)` when
/// possible; otherwise we collapse the multi-line text into a single line.
fn humanize_oci_error(msg: &str) -> String {
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

/// Cheap uniform-ish PRNG seeded from the system clock; good enough for
/// jittering retry intervals (no security requirement).
fn random_in_range(min: f64, max: f64) -> f64 {
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
