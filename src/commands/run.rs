use crate::cli::OutputFormat;
use crate::commands::{connect, resolve_server_name};
use crate::config::ConfigStore;
use crate::mcp::ContentItem;
use crate::schema::{parse_args, parse_schema};
use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use serde_json::Value;
use std::collections::HashMap;

pub async fn run_tool(
    store: &ConfigStore,
    server: Option<String>,
    tool_name: &str,
    input_json: Option<String>,
    args: Vec<String>,
    format: OutputFormat,
) -> Result<()> {
    let config = store.load()?;
    let server_name = resolve_server_name(&config, server)?;

    let mut client = connect(store, &server_name).await?;

    // Find the tool to get its schema
    let tools = client.list_tools().await?;
    let tool = tools.iter().find(|t| t.name == tool_name).ok_or_else(|| {
        anyhow::anyhow!("Tool '{}' not found on server '{}'", tool_name, server_name)
    })?;

    // Build arguments
    let arguments: HashMap<String, Value> = if let Some(json_str) = input_json {
        serde_json::from_str(&json_str).context("Invalid --input-json")?
    } else if !args.is_empty() {
        // Parse args using schema
        let schema = tool.input_schema.as_ref().unwrap_or(&Value::Null);
        let flags = parse_schema(schema)?;
        parse_args(&args, &flags)?
    } else {
        HashMap::new()
    };

    // Call the tool
    let result = client.call_tool(tool_name, arguments).await?;
    client.close().await?;

    // Render output
    match format {
        OutputFormat::Human => {
            if result.is_error {
                eprintln!("{} {}", "✗".red(), "Error from tool:".red().bold());
            }

            for item in &result.content {
                match item {
                    ContentItem::Text { text } => {
                        println!("{}", text);
                    }
                    ContentItem::Image { data, mime_type } => {
                        println!(
                            "{} {} {}",
                            "[Image]".magenta(),
                            mime_type.dimmed(),
                            format!("({} bytes)", data.len()).dimmed()
                        );
                    }
                    ContentItem::Resource { resource } => {
                        println!(
                            "{}\n{}",
                            "[Resource]".magenta(),
                            serde_json::to_string_pretty(resource)?
                        );
                    }
                }
            }
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    }

    if result.is_error {
        std::process::exit(1);
    }

    Ok(())
}
