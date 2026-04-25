mod config;
mod ghost_input;
mod providers;
mod wizard;

use config::InstanceConfigFile;
use providers::oracle::{InstanceConfig, OracleInstanceCreator};
use wizard::ConfigWizard;
use dialoguer::{theme::ColorfulTheme, Select};
use std::path::Path;

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
            1 => {
                let new_config = reconfigure_full().await?;
                new_config.save_to_file(CONFIG_FILE)?;
                println!("\n✅ Configuration saved. Please restart the program.");
                break;
            }
            2 => {
                let new_config = reconfigure_quick(&config).await?;
                new_config.save_to_file(CONFIG_FILE)?;
                println!("\n✅ Configuration saved. Please restart the program.");
                break;
            }
            3 => display_config(&config)?,
            4 => {
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
    println!("  Public IP: {}", if config.network.assign_public_ip { "Yes" } else { "No" });
    println!();
    
    Ok(())
}

async fn create_instance(config: &InstanceConfigFile) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("\n🚀 Creating instance...\n");
    
    if !Path::new(OCI_CONFIG_FILE).exists() {
        return Err(format!("OCI config file not found: {}", OCI_CONFIG_FILE).into());
    }
    
    let instance_config = if config.instance.instance_type == "amd" {
        InstanceConfig::amd_micro(&config.instance.display_name)
    } else {
        let ocpus = config.instance.arm_ocpus.unwrap_or(2);
        let memory = config.instance.arm_memory_gb.unwrap_or(12);
        InstanceConfig::arm_flex(&config.instance.display_name, ocpus, memory)
    }
    .with_public_ip(config.network.assign_public_ip)
    .with_boot_volume_size(config.instance.boot_volume_size_gb)
    .with_tag("managed-by", "cloud-manage-rs");
    
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
