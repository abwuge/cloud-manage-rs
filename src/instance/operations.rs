use crate::config::config::InstanceConfigFile;
use crate::dns;
use crate::providers::oracle::{OracleInstanceCreator, PublicIpv4Target};
use crate::ui::ghost_input::ghost_input;
use dialoguer::{Confirm, Select, theme::ColorfulTheme};
use oci_rust_sdk::compute::models::LifecycleState;
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

pub async fn refresh_public_ipv4_from_targets(
    config: &InstanceConfigFile,
    targets: &mut Vec<PublicIpv4Target>,
    exit_on_error: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let Some(selection) = pick_instance_for_refresh(targets)? else {
        return Ok(());
    };
    let result = refresh_public_ipv4_target(config, &targets[selection], exit_on_error).await?;
    if let Some(result) = result {
        targets[selection].public_ip = Some(result.new_public_ip.clone());
        targets[selection].public_ip_error = None;
    }
    Ok(())
}

pub async fn refresh_public_ipv4(
    config: &InstanceConfigFile,
    instance_id: &str,
    exit_on_error: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if instance_id.is_empty() {
        println!("\n❌ Instance OCID is required");
        return Ok(());
    }

    println!("\n🔄 Refreshing public IPv4...");
    println!("📌 Instance ID: {}", instance_id);

    let creator = OracleInstanceCreator::new(config.clone());
    let result = match creator
        .public_ipv4_target_for_instance_id(instance_id)
        .await
    {
        Ok(target) => creator.refresh_public_ipv4_target(&target).await,
        Err(error) => Err(error),
    };

    match result {
        Ok(result) => {
            println!("\n✅ Public IPv4 refreshed successfully!");
            match &result.old_public_ip {
                Some(ip) => println!("Old public IPv4: {}", ip),
                None => println!("Old public IPv4: none"),
            }
            println!("New public IPv4: {}", result.new_public_ip);
            maybe_update_dns_after_refresh(config, &result).await?;
        }
        Err(e) => {
            println!("\n❌ Public IPv4 refresh failed: {}", e);
            if exit_on_error {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

pub async fn refresh_public_ipv4_auto(
    config: &InstanceConfigFile,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut targets = load_public_ipv4_targets(config).await?;
    show_public_ipv4_status(&targets);
    refresh_public_ipv4_from_targets(config, &mut targets, true).await
}

async fn refresh_public_ipv4_target(
    config: &InstanceConfigFile,
    target: &PublicIpv4Target,
    exit_on_error: bool,
) -> Result<
    Option<crate::providers::oracle::PublicIpRefreshResult>,
    Box<dyn std::error::Error + Send + Sync>,
> {
    println!("\n🔄 Refreshing public IPv4...");
    println!("Instance: {}", target_name(target));
    println!(
        "Current public IPv4: {}",
        target.public_ip.as_deref().unwrap_or("none")
    );

    let creator = OracleInstanceCreator::new(config.clone());
    match creator.refresh_public_ipv4_target(target).await {
        Ok(result) => {
            println!("\n✅ Public IPv4 refreshed successfully!");
            match &result.old_public_ip {
                Some(ip) => println!("Old public IPv4: {}", ip),
                None => println!("Old public IPv4: none"),
            }
            println!("New public IPv4: {}", result.new_public_ip);
            maybe_update_dns_after_refresh(config, &result).await?;
            Ok(Some(result))
        }
        Err(e) => {
            println!("\n❌ Public IPv4 refresh failed: {}", e);
            if exit_on_error {
                std::process::exit(1);
            }
            Ok(None)
        }
    }
}

async fn maybe_update_dns_after_refresh(
    config: &InstanceConfigFile,
    result: &crate::providers::oracle::PublicIpRefreshResult,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let Some(old_ip) = result.old_public_ip.as_deref() else {
        println!("No old public IPv4 found; skipping DNS record update.");
        return Ok(());
    };

    let update_dns = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Update Cloudflare DNS records pointing to the old IP?")
        .default(true)
        .interact()?;

    if !update_dns {
        return Ok(());
    }

    println!(
        "\n🔄 Updating Cloudflare A records from {} to {}...",
        old_ip, result.new_public_ip
    );

    match dns::update_a_records_pointing_to_ip(config, old_ip, &result.new_public_ip).await {
        Ok(records) if records.is_empty() => {
            println!("No Cloudflare A records pointed to {}.", old_ip);
        }
        Ok(records) => {
            println!("✅ Updated {} Cloudflare DNS record(s):", records.len());
            for record in records {
                println!("  {} -> {}", record.name, record.content);
            }
        }
        Err(error) => {
            println!("❌ Cloudflare DNS update failed: {}", error);
        }
    }

    Ok(())
}

pub async fn load_public_ipv4_targets(
    config: &InstanceConfigFile,
) -> Result<Vec<PublicIpv4Target>, Box<dyn std::error::Error + Send + Sync>> {
    let creator = OracleInstanceCreator::new(config.clone());
    let mut targets = creator.list_public_ipv4_targets().await?;
    targets.sort_by(|a, b| {
        target_sort_key(a)
            .cmp(&target_sort_key(b))
            .then_with(|| a.instance_id.cmp(&b.instance_id))
    });

    Ok(targets)
}

pub fn show_public_ipv4_status(targets: &[PublicIpv4Target]) {
    println!("\nOracle Cloud instances:");
    if targets.is_empty() {
        println!("No active instances found in configured compartment.");
        return;
    }
    for target in targets {
        println!("  {}", format_target_option(target));
    }
}

fn pick_instance_for_refresh(
    targets: &[PublicIpv4Target],
) -> Result<Option<usize>, Box<dyn std::error::Error + Send + Sync>> {
    if targets.is_empty() {
        println!("No active instances found in configured compartment.");
        return Ok(None);
    }
    let items: Vec<String> = targets.iter().map(format_target_option).collect();
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select instance to refresh public IPv4")
        .items(&items)
        .default(0)
        .interact()?;

    Ok(Some(selection))
}

fn target_sort_key(target: &PublicIpv4Target) -> (u8, String) {
    let state_rank = if target.lifecycle_state == LifecycleState::Running {
        0
    } else {
        1
    };
    let name = target.display_name.clone().unwrap_or_default();
    (state_rank, name)
}

fn format_target_option(target: &PublicIpv4Target) -> String {
    let public_ip = match (&target.public_ip, &target.public_ip_error) {
        (Some(ip), _) => ip.as_str(),
        (None, Some(_)) => "unavailable",
        (None, None) => "none",
    };
    format!(
        "{:<12} {:<28} {}",
        format!("{:?}", target.lifecycle_state),
        target_name(target),
        public_ip
    )
}

fn target_name(target: &PublicIpv4Target) -> &str {
    target.display_name.as_deref().unwrap_or("(unnamed)")
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
