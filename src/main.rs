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
use providers::oracle::PublicIpv4Target;
use tokio::task::JoinHandle;

type AppResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[tokio::main]
async fn main() -> AppResult<()> {
    let cli = Cli::parse();

    if let Some(cmd) = cli.command {
        return run_command(cmd).await;
    }

    run_interactive_mode().await
}

async fn run_interactive_mode() -> AppResult<()> {
    ui::print_banner();
    let mut config = config::load_or_create_config().await?;
    let mut oci_state = OciState::Loading(spawn_oci_status_load(config.clone()));

    loop {
        let selection = ui::show_main_menu()?;

        if handle_main_menu_selection(selection, &mut config, &mut oci_state).await? {
            break;
        }
    }

    Ok(())
}

async fn handle_main_menu_selection(
    selection: usize,
    config: &mut InstanceConfigFile,
    oci_state: &mut OciState,
) -> AppResult<bool> {
    match selection {
        0 => {
            handle_oci_menu(config, oci_state).await?;
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
    oci_state: &mut OciState,
) -> AppResult<()> {
    let public_ipv4_targets = oci_state.targets_mut().await?;
    instance::show_public_ipv4_status(public_ipv4_targets);

    loop {
        let selection = ui::show_oci_menu()?;
        match selection {
            0 => {
                instance::create_instance(config).await?;
                ui::pause_for_user()?;
            }
            1 => {
                instance::refresh_public_ipv4_from_targets(config, public_ipv4_targets, false)
                    .await?;
                ui::pause_for_user()?;
            }
            2 => {
                instance::snipe_instance_interactive(config).await?;
            }
            3 => {
                let new_config = config::reconfigure_quick(&config).await?;
                config::save_config_and_exit(&new_config)?;
                *config = new_config;
                break;
            }
            4 => {
                let new_config = config::reconfigure_full().await?;
                config::save_config_and_exit(&new_config)?;
                *config = new_config;
                break;
            }
            5 => {
                config::display_oracle_config(config)?;
                ui::pause_for_user()?;
            }
            6 => break,
            _ => unreachable!(),
        }
    }

    Ok(())
}

async fn handle_cloudflare_menu(config: &mut InstanceConfigFile) -> AppResult<()> {
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

async fn run_command(cmd: Command) -> AppResult<()> {
    match cmd {
        Command::ShowConfig => {
            let config = config::load_existing_config()?;
            config::display_config(&config)?;
        }
        Command::Create => {
            let config = config::load_existing_config()?;
            instance::handle_create_command(&config).await?;
        }
        Command::RefreshIp { instance_id } => {
            let config = config::load_existing_config()?;
            if let Some(instance_id) = instance_id {
                instance::refresh_public_ipv4(&config, &instance_id, true).await?;
            } else {
                instance::refresh_public_ipv4_auto(&config).await?;
            }
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

enum OciState {
    Loading(JoinHandle<AppResult<Vec<PublicIpv4Target>>>),
    Ready(Vec<PublicIpv4Target>),
}

impl OciState {
    async fn targets_mut(&mut self) -> AppResult<&mut Vec<PublicIpv4Target>> {
        if matches!(self, Self::Loading(_)) {
            let Self::Loading(handle) = std::mem::replace(self, Self::Ready(Vec::new())) else {
                unreachable!();
            };
            let targets = handle.await??;
            *self = Self::Ready(targets);
        }

        match self {
            Self::Ready(targets) => Ok(targets),
            Self::Loading(_) => unreachable!(),
        }
    }
}

fn spawn_oci_status_load(
    config: InstanceConfigFile,
) -> JoinHandle<AppResult<Vec<PublicIpv4Target>>> {
    tokio::spawn(async move { instance::load_public_ipv4_targets(&config).await })
}
