use super::Transport;
use crate::mcp::{JsonRpcRequest, JsonRpcResponse};
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;

pub struct HttpTransport {
    client: Client,
    url: String,
}

impl HttpTransport {
    pub fn new(url: String) -> Self {
        Self {
            client: Client::new(),
            url,
        }
    }
}

#[async_trait]
impl Transport for HttpTransport {
    async fn request(&mut self, req: JsonRpcRequest) -> Result<JsonRpcResponse> {
        let response = self
            .client
            .post(&self.url)
            .json(&req)
            .send()
            .await
            .with_context(|| format!("Failed to send request to {}", self.url))?;

        let response: JsonRpcResponse = response
            .json()
            .await
            .context("Failed to parse JSON-RPC response")?;

        Ok(response)
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}
