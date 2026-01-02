use crate::cli::OutputFormat;
use crate::commands::{connect, resolve_server_name};
use crate::config::ConfigStore;
use anyhow::Result;
use owo_colors::OwoColorize;

pub async fn list_tools(
    store: &ConfigStore,
    server: Option<String>,
    format: OutputFormat,
) -> Result<()> {
    let config = store.load()?;
    let server_name = resolve_server_name(&config, server)?;

    let mut client = connect(store, &server_name).await?;
    let tools = client.list_tools().await?;
    client.close().await?;

    match format {
        OutputFormat::Human => {
            if tools.is_empty() {
                println!(
                    "{}",
                    format!("No tools available from server '{}'", server_name).dimmed()
                );
                return Ok(());
            }

            println!("Tools from {}:", server_name.cyan());
            println!();

            for tool in &tools {
                println!("  {}", tool.name.green().bold());
                if let Some(desc) = &tool.description {
                    for line in textwrap::wrap(desc, 56) {
                        println!("    {}", line.dimmed());
                    }
                }
                println!();
            }

            println!(
                "{}",
                format!("Total: {} tool(s)", tools.len()).dimmed()
            );
        }
        OutputFormat::Json => {
            let output = serde_json::json!({
                "server": server_name,
                "tools": tools
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
    }

    Ok(())
}

pub async fn describe_tool(
    store: &ConfigStore,
    server: Option<String>,
    tool_name: &str,
    format: OutputFormat,
) -> Result<()> {
    let config = store.load()?;
    let server_name = resolve_server_name(&config, server)?;

    let mut client = connect(store, &server_name).await?;
    let tools = client.list_tools().await?;
    client.close().await?;

    let tool = tools
        .iter()
        .find(|t| t.name == tool_name)
        .ok_or_else(|| anyhow::anyhow!("Tool '{}' not found on server '{}'", tool_name, server_name))?;

    match format {
        OutputFormat::Human => {
            println!("{}: {}", "Tool".bold(), tool.name.cyan());
            println!();

            if let Some(desc) = &tool.description {
                println!("{}:", "Description".bold());
                for line in textwrap::wrap(desc, 70) {
                    println!("  {}", line);
                }
                println!();
            }

            if let Some(schema) = &tool.input_schema {
                println!("{}:", "Input Schema".bold());
                println!("{}", serde_json::to_string_pretty(schema)?);
            } else {
                println!("{}", "No input schema defined".dimmed());
            }
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(tool)?);
        }
    }

    Ok(())
}
