mod cli;
mod common;
mod config;
mod dns;
mod instance;
mod providers;
mod ui;

use clap::Parser;
use cli::{Cli, Command};
use config::InstanceConfigFile;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();

    if let Some(cmd) = cli.command {
        return run_command(cmd).await;
    }

    run_interactive_mode().await
}

async fn run_interactive_mode() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    ui::print_banner();
    let mut config = config::load_or_create_config().await?;

    loop {
        let selection = ui::show_main_menu()?;

        if handle_main_menu_selection(selection, &mut config).await? {
            break;
        }
    }

    Ok(())
}

async fn handle_main_menu_selection(
    selection: usize,
    config: &mut InstanceConfigFile,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    match selection {
        0 => {
            handle_oci_menu(config).await?;
            Ok(false)
        }
        1 => {
            handle_cloudflare_menu(config).await?;
            Ok(false)
        }
        2 => {
            println!("\n👋 Goodbye!");
            Ok(true)
        }
        _ => unreachable!(),
    }
}

async fn handle_oci_menu(
    config: &mut InstanceConfigFile,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        let selection = ui::show_oci_menu()?;
        match selection {
            0 => {
                instance::create_instance(config).await?;
                ui::pause_for_user()?;
            }
            1 => {
                instance::snipe_instance_interactive(config).await?;
            }
            2 => {
                let new_config = config::reconfigure_quick(&config).await?;
                config::save_config_and_exit(&new_config)?;
                *config = new_config;
                break;
            }
            3 => {
                let new_config = config::reconfigure_full().await?;
                config::save_config_and_exit(&new_config)?;
                *config = new_config;
                break;
            }
            4 => {
                config::display_oracle_config(config)?;
                ui::pause_for_user()?;
            }
            5 => break,
            _ => unreachable!(),
        }
    }

    Ok(())
}

async fn handle_cloudflare_menu(
    config: &mut InstanceConfigFile,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        let selection = ui::show_cloudflare_menu()?;
        match selection {
            0 => {
                dns::handle_dns_list_interactive(config).await?;
                ui::pause_for_user()?;
            }
            1 => {
                dns::handle_dns_upsert_interactive(config).await?;
                ui::pause_for_user()?;
            }
            2 => {
                dns::handle_dns_delete_interactive(config).await?;
                ui::pause_for_user()?;
            }
            3 => {
                let new_config = config::reconfigure_cloudflare(config).await?;
                config::save_config_and_exit(&new_config)?;
                *config = new_config;
                break;
            }
            4 => {
                config::display_cloudflare_config(config)?;
                ui::pause_for_user()?;
            }
            5 => break,
            _ => unreachable!(),
        }
    }

    Ok(())
}

async fn run_command(cmd: Command) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match cmd {
        Command::ShowConfig => {
            let config = config::load_existing_config()?;
            config::display_config(&config)?;
        }
        Command::Create => {
            let config = config::load_existing_config()?;
            instance::handle_create_command(&config).await?;
        }
        Command::Dns { command } => {
            let config = config::load_existing_config()?;
            dns::handle_dns_command(&config, command).await?;
        }
        Command::Snipe {
            min_delay,
            max_delay,
            max_attempts,
            save,
            bypass,
        } => {
            let config = config::load_existing_config()?;
            instance::handle_snipe_command(
                &config,
                min_delay,
                max_delay,
                max_attempts,
                save,
                bypass,
            )
            .await?;
        }
        Command::Reconfigure => {
            let new_config = config::reconfigure_full().await?;
            config::save_config_and_exit(&new_config)?;
        }
        Command::QuickConfig => {
            let config = config::load_existing_config()?;
            let new_config = config::reconfigure_quick(&config).await?;
            config::save_config_and_exit(&new_config)?;
        }
    }
    Ok(())
}
