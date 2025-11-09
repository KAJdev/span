mod commands;
mod config;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "span", version, about = "Distributed personal cloud control plane")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize or join a cluster
    Init(commands::init::InitArgs),
    /// Show status of local node and cluster
    Status,
    /// Manage nodes (stub)
    Node,
    /// Manage apps (stub)
    App,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init(args) => commands::init::run(args).await?,
        Commands::Status => commands::node::status().await?,
        Commands::Node => commands::node::run()?,
        Commands::App => { println!("app command stub"); },
    }
    Ok(())
}
