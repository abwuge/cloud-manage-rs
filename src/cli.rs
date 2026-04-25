use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "cloud-manage", version, about = "Oracle Cloud Instance Manager", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Create,
    Snipe {
        #[arg(long)]
        min_delay: Option<f64>,
        #[arg(long)]
        max_delay: Option<f64>,
        #[arg(long)]
        max_attempts: Option<u32>,
        #[arg(long)]
        save: bool,
        #[arg(long)]
        bypass: bool,
    },
    ShowConfig,
    Reconfigure,
    QuickConfig,
}
