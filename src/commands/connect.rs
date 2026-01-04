use crate::auth::AuthStore;
use crate::config::{interpolate_env_map, Config, ConfigStore, TransportConfig};
use crate::mcp::transport::{HttpTransport, SseTransport, StdioTransport, Transport};
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

    // Load auth tokens
    let auth_store = AuthStore::load().ok();
    let access_token = auth_store
        .as_ref()
        .and_then(|s| s.get_token(server_name))
        .map(|t| t.access_token.clone());

    let transport: Box<dyn Transport> = match &server_config.transport {
        TransportConfig::Stdio { command } => Box::new(StdioTransport::spawn(command, env).await?),
        TransportConfig::Http { url } => {
            // Use SSE transport for URLs ending with /sse
            if url.ends_with("/sse") {
                Box::new(
                    SseTransport::new(url.clone(), server_name.to_string()).with_token(access_token),
                )
            } else {
                Box::new(
                    HttpTransport::new(url.clone(), server_name.to_string()).with_token(access_token),
                )
            }
        }
    };

    let mut client = McpClient::new(transport);
    client.initialize().await?;

    Ok(client)
}
