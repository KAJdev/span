mod commands;
mod config;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "span", version, about = "Span distributed personal cloud CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Control Plane URL
    #[arg(long, env = "SPAN_CP_URL", default_value = "http://localhost:8080")]
    cp_url: String,

    /// Auth token
    #[arg(long, env = "SPAN_TOKEN")]
    token: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize or join a cluster
    Init(commands::init::InitArgs),

    /// Node management
    #[command(subcommand)]
    Node(NodeCommands),

    /// App management
    #[command(subcommand)]
    App(AppCommands),

    /// Function management
    #[command(subcommand)]
    Function(FunctionCommands),

    /// Bucket management
    #[command(subcommand)]
    Bucket(BucketCommands),

    /// Secret management
    #[command(subcommand)]
    Secret(SecretCommands),

    /// Route management
    #[command(subcommand)]
    Route(RouteCommands),

    /// View logs for an app
    Logs {
        /// App namespace/name
        app: String,
        /// Follow logs
        #[arg(short, long)]
        follow: bool,
        /// Tail last N lines (optional, server-dependent)
        #[arg(short, long)]
        tail: Option<usize>,
    },
}

#[derive(Subcommand, Debug)]
enum NodeCommands {
    /// List nodes
    List,
    /// Show node details
    Get { id: String },
    /// Join a node to the cluster
    Join { #[arg(long)] token: Option<String> },
    /// Cordon a node (prevent new workloads)
    Cordon { id: String },
    /// Drain a node (evict workloads)
    Drain { id: String },
    /// Remove a node
    Remove { id: String },
}

#[derive(Subcommand, Debug)]
enum AppCommands {
    /// List apps
    List { #[arg(short, long)] namespace: Option<String> },
    /// Get app details
    Get { namespace: String, name: String },
    /// Apply app configuration from a YAML file
    Apply { #[arg(short, long)] file: String },
    /// Delete an app
    Delete { namespace: String, name: String },
    /// Deploy a specific release
    Deploy { namespace: String, name: String, #[arg(long)] version: Option<u32> },
    /// Rollback an app
    Rollback { namespace: String, name: String },
    /// Scale an app
    Scale { namespace: String, name: String, replicas: u32 },
}

#[derive(Subcommand, Debug)]
enum FunctionCommands {
    /// List functions
    List { #[arg(short, long)] namespace: Option<String> },
    /// Invoke a function with optional JSON payload
    Invoke {
        namespace: String,
        name: String,
        /// Inline JSON payload
        #[arg(short = 'd', long)]
        data: Option<String>,
        /// Read JSON payload from file
        #[arg(short = 'f', long)]
        file: Option<String>,
        /// HTTP path/query to append when invoking
        #[arg(long)]
        path: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum BucketCommands {
    /// Create a bucket
    Create { namespace: String, name: String, #[arg(long)] public: bool },
    /// Upload a file to a bucket
    Upload { bucket: String, key: String, file: String },
    /// Download an object from a bucket
    Download { bucket: String, key: String, file: String },
    /// List objects in a bucket
    ListObjects { bucket: String, #[arg(long)] prefix: Option<String> },
}

#[derive(Subcommand, Debug)]
enum SecretCommands {
    /// Set a secret (prompts for value)
    Set { namespace: String, name: String },
    /// List secrets in a namespace
    List { namespace: String },
}

#[derive(Subcommand, Debug)]
enum RouteCommands {
    /// List routes
    List { #[arg(short, long)] namespace: Option<String> },
    /// Apply routes from file
    Apply { #[arg(short, long)] file: String },
    /// Delete a route
    Delete { namespace: String, name: String },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init(args) => commands::init::run(args).await?,

        Commands::Node(cmd) => match cmd {
            NodeCommands::List => commands::node::list(&cli.cp_url, cli.token.as_deref()).await?,
            NodeCommands::Get { id } => commands::node::get(&id, &cli.cp_url, cli.token.as_deref()).await?,
            NodeCommands::Join { token } => commands::node::join(token.as_deref(), &cli.cp_url, cli.token.as_deref()).await?,
            NodeCommands::Cordon { id } => commands::node::cordon(&id, &cli.cp_url, cli.token.as_deref()).await?,
            NodeCommands::Drain { id } => commands::node::drain(&id, &cli.cp_url, cli.token.as_deref()).await?,
            NodeCommands::Remove { id } => commands::node::remove(&id, &cli.cp_url, cli.token.as_deref()).await?,
        },

        Commands::App(cmd) => match cmd {
            AppCommands::List { namespace } => commands::app::list(namespace.as_deref(), &cli.cp_url, cli.token.as_deref()).await?,
            AppCommands::Get { namespace, name } => commands::app::get(&namespace, &name, &cli.cp_url, cli.token.as_deref()).await?,
            AppCommands::Apply { file } => commands::app::apply(&file, &cli.cp_url, cli.token.as_deref()).await?,
            AppCommands::Delete { namespace, name } => commands::app::delete(&namespace, &name, &cli.cp_url, cli.token.as_deref()).await?,
            AppCommands::Deploy { namespace, name, version } => commands::app::deploy(&namespace, &name, version, &cli.cp_url, cli.token.as_deref()).await?,
            AppCommands::Rollback { namespace, name } => commands::app::rollback(&namespace, &name, &cli.cp_url, cli.token.as_deref()).await?,
            AppCommands::Scale { namespace, name, replicas } => commands::app::scale(&namespace, &name, replicas, &cli.cp_url, cli.token.as_deref()).await?,
        },

        Commands::Function(cmd) => match cmd {
            FunctionCommands::List { namespace } => commands::function::list(namespace.as_deref(), &cli.cp_url, cli.token.as_deref()).await?,
            FunctionCommands::Invoke { namespace, name, data, file, path } => commands::function::invoke(&namespace, &name, data.as_deref(), file.as_deref(), path.as_deref(), &cli.cp_url, cli.token.as_deref()).await?,
        },

        Commands::Bucket(cmd) => match cmd {
            BucketCommands::Create { namespace, name, public } => commands::bucket::create(&namespace, &name, public, &cli.cp_url, cli.token.as_deref()).await?,
            BucketCommands::Upload { bucket, key, file } => commands::bucket::upload(&bucket, &key, &file, &cli.cp_url, cli.token.as_deref()).await?,
            BucketCommands::Download { bucket, key, file } => commands::bucket::download(&bucket, &key, &file, &cli.cp_url, cli.token.as_deref()).await?,
            BucketCommands::ListObjects { bucket, prefix } => commands::bucket::list_objects(&bucket, prefix.as_deref(), &cli.cp_url, cli.token.as_deref()).await?,
        },

        Commands::Secret(cmd) => match cmd {
            SecretCommands::Set { namespace, name } => commands::secret::set(&namespace, &name, &cli.cp_url, cli.token.as_deref()).await?,
            SecretCommands::List { namespace } => commands::secret::list(&namespace, &cli.cp_url, cli.token.as_deref()).await?,
        },

        Commands::Route(cmd) => match cmd {
            RouteCommands::List { namespace } => commands::route::list(namespace.as_deref(), &cli.cp_url, cli.token.as_deref()).await?,
            RouteCommands::Apply { file } => commands::route::apply(&file, &cli.cp_url, cli.token.as_deref()).await?,
            RouteCommands::Delete { namespace, name } => commands::route::delete(&namespace, &name, &cli.cp_url, cli.token.as_deref()).await?,
        },

        Commands::Logs { app, follow, tail } => {
            commands::logs::stream_logs(&app, follow, tail, &cli.cp_url, cli.token.as_deref()).await?
        }
    }

    Ok(())
}
