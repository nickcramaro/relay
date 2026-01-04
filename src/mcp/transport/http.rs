use super::Transport;
use crate::mcp::{JsonRpcRequest, JsonRpcResponse};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HttpTransportError {
    #[error("Authentication required. Run: relay auth {server_name}")]
    AuthRequired { server_name: String },
}

pub struct HttpTransport {
    client: Client,
    url: String,
    access_token: Option<String>,
    server_name: String,
}

impl HttpTransport {
    pub fn new(url: String, server_name: String) -> Self {
        Self {
            client: Client::new(),
            url,
            access_token: None,
            server_name,
        }
    }

    pub fn with_token(mut self, token: Option<String>) -> Self {
        self.access_token = token;
        self
    }
}

#[async_trait]
impl Transport for HttpTransport {
    async fn request(&mut self, req: JsonRpcRequest) -> Result<JsonRpcResponse> {
        let mut request = self
            .client
            .post(&self.url)
            .header("Accept", "application/json, text/event-stream");

        if let Some(token) = &self.access_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .json(&req)
            .send()
            .await
            .with_context(|| format!("Failed to send request to {}", self.url))?;

        // Check for authentication errors
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(HttpTransportError::AuthRequired {
                server_name: self.server_name.clone(),
            }
            .into());
        }

        // Check for other HTTP errors with OAuth error format
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            // Try to parse as OAuth error
            if let Ok(oauth_error) = serde_json::from_str::<serde_json::Value>(&body) {
                if let Some(error) = oauth_error.get("error").and_then(|e| e.as_str()) {
                    let description = oauth_error
                        .get("error_description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("");

                    if error == "invalid_token" {
                        return Err(HttpTransportError::AuthRequired {
                            server_name: self.server_name.clone(),
                        }
                        .into());
                    }

                    return Err(anyhow!("{}: {}", error, description));
                }
            }

            return Err(anyhow!("HTTP error {}: {}", status, body));
        }

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
