use crate::cli::DnsCommand;
use crate::config::InstanceConfigFile;
use crate::ui::ghost_input::ghost_input;
use cloudflare_rust_sdk::dns::{DnsClient, DnsRecord, DnsRecordRequest};
use dialoguer::{Confirm, theme::ColorfulTheme};

pub async fn handle_dns_command(
    config: &InstanceConfigFile,
    command: DnsCommand,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = DnsClient::new(&config.cloudflare)?;

    match command {
        DnsCommand::List { record_type, name } => {
            let records = client
                .list_records(record_type.as_deref(), name.as_deref())
                .await?;
            print_records(&records);
        }
        DnsCommand::Upsert {
            record_type,
            name,
            content,
            ttl,
            proxied,
        } => {
            let record = client
                .upsert_record(&DnsRecordRequest {
                    record_type,
                    name,
                    content,
                    ttl,
                    proxied,
                })
                .await?;
            println!("✅ DNS record saved");
            print_records(&[record]);
        }
        DnsCommand::Delete { record_id } => {
            client.delete_record(&record_id).await?;
            println!("✅ DNS record deleted: {}", record_id);
        }
    }

    Ok(())
}

pub async fn handle_dns_list_interactive(
    config: &InstanceConfigFile,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let record_type = optional_input("Record type filter")?;
    let name = optional_input_with_default(
        "Name filter",
        config.cloudflare.record_name.as_deref().unwrap_or(""),
    )?;
    handle_dns_command(config, DnsCommand::List { record_type, name }).await
}

pub async fn handle_dns_upsert_interactive(
    config: &InstanceConfigFile,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let record_type = ghost_input("Record type", "", "A")?;
    let name = ghost_input(
        "Name",
        "",
        config.cloudflare.record_name.as_deref().unwrap_or(""),
    )?;
    let content = ghost_input("Content", "", "")?;
    let ttl = ghost_input("TTL (1 = automatic)", "", "1")?
        .trim()
        .parse()
        .unwrap_or(1);
    let set_proxied = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Set proxied flag?")
        .default(false)
        .interact()?;
    let proxied = if set_proxied {
        Some(
            Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Proxied?")
                .default(false)
                .interact()?,
        )
    } else {
        None
    };
    handle_dns_command(
        config,
        DnsCommand::Upsert {
            record_type,
            name,
            content,
            ttl,
            proxied,
        },
    )
    .await
}

pub async fn handle_dns_delete_interactive(
    config: &InstanceConfigFile,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let record_id = ghost_input("Record ID", "", "")?;
    handle_dns_command(config, DnsCommand::Delete { record_id }).await
}

fn optional_input(label: &str) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    optional_input_with_default(label, "")
}

fn optional_input_with_default(
    label: &str,
    default_value: &str,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    let value = ghost_input(label, "", default_value)?;
    if value.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

fn print_records(records: &[DnsRecord]) {
    if records.is_empty() {
        println!("No DNS records found.");
        return;
    }

    println!(
        "{:<34} {:<6} {:<36} {:<8} {:<7} {}",
        "ID", "TYPE", "NAME", "TTL", "PROXY", "CONTENT"
    );
    for record in records {
        println!(
            "{:<34} {:<6} {:<36} {:<8} {:<7} {}",
            record.id,
            record.record_type,
            record.name,
            record.ttl,
            record
                .proxied
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".to_string()),
            record.content,
        );
    }
}
