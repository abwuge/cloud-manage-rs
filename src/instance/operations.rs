use crate::config::config::InstanceConfigFile;
use crate::providers::oracle::OracleInstanceCreator;
use crate::ui::ghost_input::ghost_input;
use std::time::Duration;
use tokio::time::sleep;

use crate::common::utils::{
    build_instance_config, format_secs, humanize_oci_error, is_retryable_oci_error,
    parse_positive_f64, random_in_range,
};

const CONFIG_FILE: &str = "./config/config";

pub async fn create_instance(
    config: &InstanceConfigFile,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("\n🚀 Creating instance...\n");

    let instance_config = build_instance_config(config);
    let creator = OracleInstanceCreator::new(config.clone());

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

    Ok(())
}

pub async fn snipe_instance_interactive(
    config: &InstanceConfigFile,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let snipe = &config.snipe;
    let min_delay = parse_positive_f64(
        &ghost_input(
            "Min delay between attempts (seconds)",
            "",
            &format_secs(snipe.min_delay_secs),
        )?,
        snipe.min_delay_secs,
    );
    let max_delay = parse_positive_f64(
        &ghost_input(
            "Max delay between attempts (seconds)",
            "",
            &format_secs(snipe.max_delay_secs),
        )?,
        snipe.max_delay_secs,
    );
    let max_attempts: u32 = ghost_input(
        "Max attempts (0 = unlimited)",
        "",
        &snipe.max_attempts.to_string(),
    )?
    .trim()
    .parse()
    .unwrap_or(snipe.max_attempts);

    maybe_save_snipe_config(config, min_delay, max_delay, max_attempts)?;

    snipe_instance(config, min_delay, max_delay, max_attempts, true, false).await
}

fn maybe_save_snipe_config(
    config: &InstanceConfigFile,
    min_delay: f64,
    max_delay: f64,
    max_attempts: u32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let snipe = &config.snipe;
    if min_delay != snipe.min_delay_secs
        || max_delay != snipe.max_delay_secs
        || max_attempts != snipe.max_attempts
    {
        let mut updated = config.clone();
        updated.snipe.min_delay_secs = min_delay;
        updated.snipe.max_delay_secs = max_delay;
        updated.snipe.max_attempts = max_attempts;
        if let Err(e) = updated.save_to_file(CONFIG_FILE) {
            println!("⚠️  Failed to persist snipe settings: {}", e);
        } else {
            println!("💾 Snipe settings saved to {}", CONFIG_FILE);
        }
    }
    Ok(())
}

pub async fn snipe_instance(
    config: &InstanceConfigFile,
    min_delay: f64,
    max_delay: f64,
    max_attempts: u32,
    pause_at_end: bool,
    bypass: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("\n🎯 Snipe Mode: keep retrying until an instance is launched");
    if bypass {
        println!("   ⚠️  Bypass mode enabled: all errors will be retried");
    }
    println!("   (Ctrl+C to stop at any time)\n");

    let (min_delay, max_delay) = if min_delay <= max_delay {
        (min_delay, max_delay)
    } else {
        (max_delay, min_delay)
    };

    let creator = OracleInstanceCreator::new(config.clone());
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
                let raw: String = e.to_string();
                let pretty = humanize_oci_error(&raw);
                if bypass || is_retryable_oci_error(&raw) {
                    println!("   ✖ {}", pretty);
                } else {
                    println!("   ✖ {} (non-retryable)", pretty);
                    break Err(format!("aborted on non-retryable error: {}", pretty));
                }
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

    if pause_at_end {
        println!("\nPress Enter to continue...");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
    }
    Ok(())
}

pub async fn handle_create_command(
    config: &InstanceConfigFile,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let instance_config = build_instance_config(config);
    let creator = OracleInstanceCreator::new(config.clone());
    match creator.create_and_wait(&instance_config, 300).await {
        Ok(id) => {
            println!("\n✅ Instance created successfully!");
            println!("📌 Instance ID: {}", id);
        }
        Err(e) => {
            println!("\n❌ Instance creation failed: {}", e);
            std::process::exit(1);
        }
    }
    Ok(())
}

pub async fn handle_snipe_command(
    config: &InstanceConfigFile,
    min_delay: Option<f64>,
    max_delay: Option<f64>,
    max_attempts: Option<u32>,
    save: bool,
    bypass: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let min = min_delay.unwrap_or(config.snipe.min_delay_secs);
    let max = max_delay.unwrap_or(config.snipe.max_delay_secs);
    let attempts = max_attempts.unwrap_or(config.snipe.max_attempts);
    if save {
        let mut updated = config.clone();
        updated.snipe.min_delay_secs = min;
        updated.snipe.max_delay_secs = max;
        updated.snipe.max_attempts = attempts;
        updated.save_to_file(CONFIG_FILE)?;
        println!("💾 Snipe settings saved to {}", CONFIG_FILE);
    }
    snipe_instance(config, min, max, attempts, false, bypass).await?;
    Ok(())
}
