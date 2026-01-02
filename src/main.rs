mod cli;

use clap::Parser;
use cli::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive(if cli.verbose {
                tracing::Level::DEBUG.into()
            } else {
                tracing::Level::INFO.into()
            }),
        )
        .init();

    match cli.command {
        _ => {
            println!("Command not yet implemented");
        }
    }

    Ok(())
}
