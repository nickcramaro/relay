use crate::cli::{OutputFormat, Transport};
use crate::config::{ConfigStore, ServerConfig, TransportConfig};
use anyhow::{bail, Result};
use owo_colors::OwoColorize;

pub fn add_server(
    store: &ConfigStore,
    name: String,
    transport: Transport,
    cmd: Option<String>,
    url: Option<String>,
    env: Vec<(String, String)>,
    format: OutputFormat,
) -> Result<()> {
    let mut config = store.load()?;

    let transport_config = match transport {
        Transport::Stdio => {
            let command = cmd.ok_or_else(|| anyhow::anyhow!("--cmd required for stdio transport"))?;
            TransportConfig::Stdio { command }
        }
        Transport::Http => {
            let url = url.ok_or_else(|| anyhow::anyhow!("--url required for http transport"))?;
            TransportConfig::Http { url }
        }
    };

    let server_config = ServerConfig {
        transport: transport_config,
        env: env.into_iter().collect(),
    };

    config.servers.insert(name.clone(), server_config);

    // Set as default if it's the first server
    if config.default_server.is_none() {
        config.default_server = Some(name.clone());
    }

    store.save(&config)?;

    match format {
        OutputFormat::Human => {
            println!("{} Added server: {}", "✓".green(), name.cyan());
        }
        OutputFormat::Json => {
            let output = serde_json::json!({ "added": name });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
    }

    Ok(())
}

pub fn list_servers(store: &ConfigStore, format: OutputFormat) -> Result<()> {
    let config = store.load()?;

    match format {
        OutputFormat::Human => {
            if config.servers.is_empty() {
                println!("{}", "No servers registered. Use `relay add` to add one.".dimmed());
                return Ok(());
            }

            println!(
                "{:<20} {:<10} {}",
                "NAME".bold(),
                "TRANSPORT".bold(),
                "TARGET".bold()
            );
            println!("{}", "─".repeat(60).dimmed());

            for (name, server) in &config.servers {
                let (transport, target) = match &server.transport {
                    TransportConfig::Stdio { command } => ("stdio", command.as_str()),
                    TransportConfig::Http { url } => ("http", url.as_str()),
                };
                let is_default = config.default_server.as_ref() == Some(name);
                let name_display = if is_default {
                    format!("{} {}", name.cyan(), "(default)".dimmed())
                } else {
                    name.cyan().to_string()
                };
                println!("{:<20} {:<10} {}", name_display, transport.yellow(), target);
            }
        }
        OutputFormat::Json => {
            let output = serde_json::to_string_pretty(&config.servers)?;
            println!("{}", output);
        }
    }

    Ok(())
}

pub fn remove_server(store: &ConfigStore, name: String, format: OutputFormat) -> Result<()> {
    let mut config = store.load()?;

    if !config.servers.contains_key(&name) {
        bail!("Server '{}' not found", name);
    }

    config.servers.remove(&name);

    // Clear default if it was the removed server
    if config.default_server.as_ref() == Some(&name) {
        config.default_server = config.servers.keys().next().cloned();
    }

    store.save(&config)?;

    match format {
        OutputFormat::Human => {
            println!("{} Removed server: {}", "✓".green(), name.cyan());
        }
        OutputFormat::Json => {
            let output = serde_json::json!({ "removed": name });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
    }

    Ok(())
}
