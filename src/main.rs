mod cli;
mod commands;
mod config;
mod mcp;
mod schema;

use clap::Parser;
use cli::{Cli, Commands};
use config::ConfigStore;
use owo_colors::OwoColorize;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive(if cli.verbose {
                tracing::Level::DEBUG.into()
            } else {
                tracing::Level::INFO.into()
            }),
        )
        .init();

    if let Err(err) = run(cli.verbose, cli.format, cli.command).await {
        print_error(&err, cli.verbose);
        std::process::exit(1);
    }
}

async fn run(_verbose: bool, format: cli::OutputFormat, command: Commands) -> anyhow::Result<()> {
    // Support RELAY_CONFIG env var for testing
    let store = if let Ok(path) = std::env::var("RELAY_CONFIG") {
        ConfigStore::with_path(path.into())
    } else {
        ConfigStore::new()?
    };

    match command {
        Commands::Add {
            name,
            transport,
            cmd,
            url,
            env,
        } => {
            commands::add_server(&store, name, transport, cmd, url, env, format)?;
        }
        Commands::List => {
            commands::list_servers(&store, format)?;
        }
        Commands::Remove { name } => {
            commands::remove_server(&store, name, format)?;
        }
        Commands::Ping { name } => {
            commands::ping_server(&store, &name, format).await?;
        }
        Commands::Tools { server } => {
            commands::list_tools(&store, server, format).await?;
        }
        Commands::Describe { server, tool } => {
            commands::describe_tool(&store, server, &tool, format).await?;
        }
        Commands::Run {
            server,
            tool,
            input_json,
            args,
        } => {
            commands::run_tool(&store, server, &tool, input_json, args, format).await?;
        }
        Commands::Update => {
            commands::update(format).await?;
        }
    }

    Ok(())
}

fn print_error(err: &anyhow::Error, verbose: bool) {
    eprintln!("{} {}", "error:".red().bold(), err);

    if verbose {
        let mut source = err.source();
        while let Some(cause) = source {
            eprintln!("  {} {}", "caused by:".yellow(), cause);
            source = cause.source();
        }
    }
}
