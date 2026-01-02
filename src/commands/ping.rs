use crate::cli::OutputFormat;
use crate::commands::connect;
use crate::config::ConfigStore;
use anyhow::Result;
use owo_colors::OwoColorize;
use std::time::Instant;

pub async fn ping_server(store: &ConfigStore, name: &str, format: OutputFormat) -> Result<()> {
    let start = Instant::now();

    let mut client = connect(store, name).await?;
    let elapsed = start.elapsed();

    let server_info = client.server_info().cloned();
    client.close().await?;

    match format {
        OutputFormat::Human => {
            let ms = elapsed.as_secs_f64() * 1000.0;
            if let Some(info) = server_info {
                println!(
                    "{} Connected to {} {} in {}",
                    "✓".green(),
                    info.name.cyan(),
                    format!("v{}", info.version.unwrap_or_else(|| "?".to_string())).dimmed(),
                    format!("{:.2}ms", ms).yellow()
                );
            } else {
                println!(
                    "{} Connected in {}",
                    "✓".green(),
                    format!("{:.2}ms", ms).yellow()
                );
            }
        }
        OutputFormat::Json => {
            let output = serde_json::json!({
                "status": "ok",
                "server": server_info,
                "elapsed_ms": elapsed.as_secs_f64() * 1000.0
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
    }

    Ok(())
}
