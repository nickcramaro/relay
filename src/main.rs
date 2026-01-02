mod cli;
mod commands;
mod config;
mod mcp;
mod schema;

use clap::Parser;
use cli::{Cli, Commands};
use config::ConfigStore;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(if cli.verbose {
                    tracing::Level::DEBUG.into()
                } else {
                    tracing::Level::INFO.into()
                }),
        )
        .init();

    // Support RELAY_CONFIG env var for testing
    let store = if let Ok(path) = std::env::var("RELAY_CONFIG") {
        ConfigStore::with_path(path.into())
    } else {
        ConfigStore::new()?
    };

    match cli.command {
        Commands::Add { name, transport, cmd, url, env } => {
            commands::add_server(&store, name, transport, cmd, url, env, cli.format)?;
        }
        Commands::List => {
            commands::list_servers(&store, cli.format)?;
        }
        Commands::Remove { name } => {
            commands::remove_server(&store, name, cli.format)?;
        }
        Commands::Ping { name } => {
            commands::ping_server(&store, &name, cli.format).await?;
        }
        Commands::Tools { server } => {
            commands::list_tools(&store, server, cli.format).await?;
        }
        Commands::Describe { server, tool } => {
            commands::describe_tool(&store, server, &tool, cli.format).await?;
        }
        Commands::Run { server, tool, input_json, args } => {
            commands::run_tool(&store, server, &tool, input_json, args, cli.format).await?;
        }
    }

    Ok(())
}
