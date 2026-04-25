use crate::config::config::InstanceConfigFile;
use crate::config::wizard::ConfigWizard;

const CONFIG_FILE: &str = "./config/config";

pub async fn load_or_create_config() -> Result<InstanceConfigFile, Box<dyn std::error::Error + Send + Sync>> {
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

    let wizard = ConfigWizard::new();
    let default_config = InstanceConfigFile::default();
    let config: InstanceConfigFile = wizard.run(&default_config).await?;

    config.save_to_file(CONFIG_FILE)?;
    println!("\n✅ Configuration saved to: {}", CONFIG_FILE);

    Ok(config)
}

pub async fn reconfigure_full() -> Result<InstanceConfigFile, Box<dyn std::error::Error + Send + Sync>> {
    println!("\n🔧 Full Reconfiguration");

    let current_config = if InstanceConfigFile::exists(CONFIG_FILE) {
        InstanceConfigFile::load_from_file(CONFIG_FILE).unwrap_or_default()
    } else {
        InstanceConfigFile::default()
    };

    ConfigWizard::new().run(&current_config).await
}

pub async fn reconfigure_quick(base_config: &InstanceConfigFile) -> Result<InstanceConfigFile, Box<dyn std::error::Error + Send + Sync>> {
    ConfigWizard::new().quick_config(base_config).await
}

pub fn display_config(config: &InstanceConfigFile) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

    println!("\n🔹 Snipe Config:");
    println!("  Min delay:    {}s", config.snipe.min_delay_secs);
    println!("  Max delay:    {}s", config.snipe.max_delay_secs);
    println!(
        "  Max attempts: {}",
        if config.snipe.max_attempts == 0 {
            "unlimited".to_string()
        } else {
            config.snipe.max_attempts.to_string()
        }
    );
    println!();

    Ok(())
}

pub fn load_existing_config() -> Result<InstanceConfigFile, Box<dyn std::error::Error + Send + Sync>> {
    if !InstanceConfigFile::exists(CONFIG_FILE) {
        return Err(format!(
            "config file not found at {}. Run `cloud-manage reconfigure` first.",
            CONFIG_FILE
        ).into());
    }
    Ok(InstanceConfigFile::load_from_file(CONFIG_FILE)?)
}

pub fn save_config_and_exit(config: &InstanceConfigFile) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    config.save_to_file(CONFIG_FILE)?;
    println!("\n✅ Configuration saved. Please restart the program.");
    Ok(())
}
