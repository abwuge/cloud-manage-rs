mod cli;
mod common;
mod config;
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
    let config = config::load_or_create_config().await?;

    loop {
        let selection = ui::show_menu()?;

        if handle_menu_selection(selection, &config).await? {
            break;
        }
    }

    Ok(())
}

async fn handle_menu_selection(
    selection: usize,
    config: &InstanceConfigFile,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    match selection {
        0 => {
            instance::create_instance(config).await?;
            ui::pause_for_user()?;
            Ok(false)
        }
        1 => {
            instance::snipe_instance_interactive(config).await?;
            Ok(false)
        }
        2 => {
            let new_config = config::reconfigure_full().await?;
            config::save_config_and_exit(&new_config)?;
            Ok(true)
        }
        3 => {
            let new_config = config::reconfigure_quick(&config).await?;
            config::save_config_and_exit(&new_config)?;
            Ok(true)
        }
        4 => {
            config::display_config(config)?;
            Ok(false)
        }
        5 => {
            println!("\n👋 Goodbye!");
            Ok(true)
        }
        _ => unreachable!(),
    }
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
        Command::Snipe { min_delay, max_delay, max_attempts, save } => {
            let config = config::load_existing_config()?;
            instance::handle_snipe_command(&config, min_delay, max_delay, max_attempts, save).await?;
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
