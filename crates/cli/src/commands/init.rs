use clap::Args;
use models::{create_pool, run_migrations};
use std::env;

#[derive(Args, Debug)]
pub struct InitArgs {
    #[arg(long)]
    pub run: bool,
    #[arg(long)]
    pub config: Option<std::path::PathBuf>,
}

pub async fn run(args: InitArgs) -> anyhow::Result<()> {
    common::telemetry::init_tracing();

    let db_url = env::var("DATABASE_URL").or_else(|_| env::var("SPAN_DATABASE_URL"))
        .map_err(|_| anyhow::anyhow!("set DATABASE_URL or SPAN_DATABASE_URL"))?;

    let pool = create_pool(&db_url).await?;
    run_migrations(&pool).await?;

    let cfg = crate::config::Config { database_url: db_url, http_bind: "0.0.0.0:8080".into(), grpc_bind: "0.0.0.0:50051".into(), nats_url: None };
    let path = crate::config::write_config(&cfg, args.config)?;
    println!("Config written to {}", path.display());
    println!("Control plane initialized. You can run 'span-control-plane'.");

    if args.run {
        println!("Starting control plane...");
        let status = std::process::Command::new("span-control-plane").status();
        match status {
            Ok(s) => println!("span-control-plane exited with status {}", s),
            Err(e) => eprintln!("failed to start span-control-plane: {}", e),
        }
    }

    Ok(())
}
