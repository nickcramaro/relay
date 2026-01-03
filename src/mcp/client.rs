use super::protocol::*;
use super::transport::Transport;
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

const PROTOCOL_VERSION: &str = "2024-11-05";

pub struct McpClient {
    transport: Box<dyn Transport>,
    request_id: AtomicU64,
    server_info: Option<ServerInfo>,
}

impl McpClient {
    pub fn new(transport: Box<dyn Transport>) -> Self {
        Self {
            transport,
            request_id: AtomicU64::new(1),
            server_info: None,
        }
    }

    fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Initialize the MCP connection
    pub async fn initialize(&mut self) -> Result<InitializeResult> {
        let params = InitializeParams {
            protocol_version: PROTOCOL_VERSION.to_string(),
            capabilities: ClientCapabilities::default(),
            client_info: ClientInfo {
                name: "relay".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };

        let req = JsonRpcRequest::new(
            self.next_id(),
            "initialize",
            Some(serde_json::to_value(params)?),
        );

        let response = self.transport.request(req).await?;

        if let Some(error) = response.error {
            bail!("Initialize failed: {} (code {})", error.message, error.code);
        }

        let result: InitializeResult = serde_json::from_value(
            response
                .result
                .context("No result in initialize response")?,
        )?;

        self.server_info = Some(result.server_info.clone());

        Ok(result)
    }

    /// List all available tools
    pub async fn list_tools(&mut self) -> Result<Vec<Tool>> {
        let mut all_tools = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let params = cursor.as_ref().map(|c| json!({ "cursor": c }));
            let req = JsonRpcRequest::new(self.next_id(), "tools/list", params);
            let response = self.transport.request(req).await?;

            if let Some(error) = response.error {
                bail!("tools/list failed: {} (code {})", error.message, error.code);
            }

            let result: ToolsListResult = serde_json::from_value(
                response
                    .result
                    .context("No result in tools/list response")?,
            )?;

            all_tools.extend(result.tools);

            match result.next_cursor {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }

        Ok(all_tools)
    }

    /// Call a tool with arguments
    pub async fn call_tool(
        &mut self,
        name: &str,
        arguments: HashMap<String, Value>,
    ) -> Result<ToolCallResult> {
        let params = ToolCallParams {
            name: name.to_string(),
            arguments,
        };

        let req = JsonRpcRequest::new(
            self.next_id(),
            "tools/call",
            Some(serde_json::to_value(params)?),
        );

        let response = self.transport.request(req).await?;

        if let Some(error) = response.error {
            bail!("tools/call failed: {} (code {})", error.message, error.code);
        }

        let result: ToolCallResult = serde_json::from_value(
            response
                .result
                .context("No result in tools/call response")?,
        )?;

        Ok(result)
    }

    /// Get server info (after initialization)
    pub fn server_info(&self) -> Option<&ServerInfo> {
        self.server_info.as_ref()
    }

    /// Close the connection
    pub async fn close(&mut self) -> Result<()> {
        self.transport.close().await
    }
}
