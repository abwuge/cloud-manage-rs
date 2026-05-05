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
    RefreshIp {
        instance_id: Option<String>,
    },
    Dns {
        #[command(subcommand)]
        command: DnsCommand,
    },
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
    /// Start a local web UI
    Serve {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value_t = 7878)]
        port: u16,
        /// Bearer token required for all /api/* requests. Overrides [web].token.
        #[arg(long)]
        token: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum DnsCommand {
    List {
        #[arg(long = "type")]
        record_type: Option<String>,
        #[arg(long)]
        name: Option<String>,
    },
    Upsert {
        #[arg(long = "type")]
        record_type: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        content: String,
        #[arg(long, default_value_t = 1)]
        ttl: u32,
        #[arg(long)]
        proxied: Option<bool>,
    },
    Delete {
        record_id: String,
    },
}
