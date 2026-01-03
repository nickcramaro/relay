use crate::config::{interpolate_env_map, Config, ConfigStore, TransportConfig};
use crate::mcp::transport::{HttpTransport, StdioTransport, Transport};
use crate::mcp::McpClient;
use anyhow::{Context, Result};

/// Resolve server name, using default if not specified
pub fn resolve_server_name(config: &Config, server: Option<String>) -> Result<String> {
    match server {
        Some(name) => Ok(name),
        None => config.default_server.clone().context(
            "No server specified and no default server set. Use `relay add` to add a server.",
        ),
    }
}

/// Create a connected MCP client for a server
pub async fn connect(store: &ConfigStore, server_name: &str) -> Result<McpClient> {
    let config = store.load()?;

    let server_config = config
        .servers
        .get(server_name)
        .with_context(|| format!("Server '{}' not found", server_name))?;

    let env = interpolate_env_map(&server_config.env);

    let transport: Box<dyn Transport> = match &server_config.transport {
        TransportConfig::Stdio { command } => Box::new(StdioTransport::spawn(command, env).await?),
        TransportConfig::Http { url } => Box::new(HttpTransport::new(url.clone())),
    };

    let mut client = McpClient::new(transport);
    client.initialize().await?;

    Ok(client)
}
