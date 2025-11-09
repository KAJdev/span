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
    /// Initialize and start the control plane
    Init(commands::init::InitArgs),
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
        Commands::Node => commands::node::run()?,
        Commands::App => { println!("app command stub"); },
    }
    Ok(())
}
