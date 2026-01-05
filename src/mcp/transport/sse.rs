use super::Transport;
use crate::mcp::{JsonRpcRequest, JsonRpcResponse};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

struct SseConnection {
    message_endpoint: String,
    response_rx: mpsc::Receiver<String>,
}

pub struct SseTransport {
    client: Client,
    base_url: String,
    connection: Arc<Mutex<Option<SseConnection>>>,
    access_token: Option<String>,
    server_name: String,
}

impl SseTransport {
    pub fn new(url: String, server_name: String) -> Self {
        Self {
            client: Client::new(),
            base_url: url,
            connection: Arc::new(Mutex::new(None)),
            access_token: None,
            server_name,
        }
    }

    pub fn with_token(mut self, token: Option<String>) -> Self {
        self.access_token = token;
        self
    }

    async fn ensure_connected(&self) -> Result<String> {
        // Check if we already have a connection
        {
            let conn = self.connection.lock().await;
            if let Some(ref c) = *conn {
                return Ok(c.message_endpoint.clone());
            }
        }

        // Open SSE connection
        let mut request = self
            .client
            .get(&self.base_url)
            .header("Accept", "text/event-stream");

        if let Some(ref token) = self.access_token {
            // Support different auth formats: if token already has a prefix, use as-is
            let auth_value = if token.starts_with("Bearer ")
                || token.starts_with("token ")
                || token.starts_with("Basic ")
            {
                token.clone()
            } else {
                format!("Bearer {}", token)
            };
            request = request.header("Authorization", auth_value);
        }

        let response = request
            .send()
            .await
            .with_context(|| format!("Failed to connect to SSE endpoint: {}", self.base_url))?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(anyhow!(
                "Authentication required. Run: relay auth {}",
                self.server_name
            ));
        }

        if !response.status().is_success() {
            return Err(anyhow!("SSE connection failed: HTTP {}", response.status()));
        }

        // Create channel for responses
        let (tx, rx) = mpsc::channel::<String>(100);

        // Read SSE stream to get endpoint and start background reader
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut endpoint_url: Option<String> = None;

        // Read until we get the endpoint
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.with_context(|| "Failed to read SSE stream")?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            for line in buffer.lines() {
                if line.starts_with("data: ") {
                    let data = line.strip_prefix("data: ").unwrap_or("");
                    if data.contains("sessionId=") || data.starts_with("/") {
                        let base = self.base_url.trim_end_matches("/sse");
                        endpoint_url = Some(format!("{}{}", base, data));
                        break;
                    }
                }
            }

            if endpoint_url.is_some() {
                break;
            }
        }

        let endpoint = endpoint_url.ok_or_else(|| anyhow!("No endpoint received"))?;

        // Spawn background task to read SSE responses
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            let mut buf = buffer;
            while let Some(chunk) = stream.next().await {
                if let Ok(chunk) = chunk {
                    buf.push_str(&String::from_utf8_lossy(&chunk));

                    // Process complete SSE events
                    while let Some(pos) = buf.find("\n\n") {
                        let event = buf[..pos].to_string();
                        buf = buf[pos + 2..].to_string();

                        // Extract data from event
                        for line in event.lines() {
                            if line.starts_with("data: ") {
                                let data = line.strip_prefix("data: ").unwrap_or("");
                                if data.starts_with("{") {
                                    let _ = tx_clone.send(data.to_string()).await;
                                }
                            }
                        }
                    }
                }
            }
        });

        // Store connection
        {
            let mut conn = self.connection.lock().await;
            *conn = Some(SseConnection {
                message_endpoint: endpoint.clone(),
                response_rx: rx,
            });
        }

        Ok(endpoint)
    }
}

#[async_trait]
impl Transport for SseTransport {
    async fn request(&mut self, req: JsonRpcRequest) -> Result<JsonRpcResponse> {
        let message_url = self.ensure_connected().await?;
        let request_id = req.id.clone();

        let mut request = self
            .client
            .post(&message_url)
            .header("Content-Type", "application/json");

        if let Some(ref token) = self.access_token {
            // Support different auth formats: if token already has a prefix, use as-is
            let auth_value = if token.starts_with("Bearer ")
                || token.starts_with("token ")
                || token.starts_with("Basic ")
            {
                token.clone()
            } else {
                format!("Bearer {}", token)
            };
            request = request.header("Authorization", auth_value);
        }

        let response = request
            .json(&req)
            .send()
            .await
            .with_context(|| format!("Failed to send request to {}", message_url))?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(anyhow!(
                "Authentication required. Run: relay auth {}",
                self.server_name
            ));
        }

        // 202 Accepted means response will come via SSE
        if response.status() == reqwest::StatusCode::ACCEPTED {
            // Wait for response on SSE channel
            let mut conn = self.connection.lock().await;
            if let Some(ref mut c) = *conn {
                // Read responses until we find the one matching our request ID
                while let Some(data) = c.response_rx.recv().await {
                    if let Ok(resp) = serde_json::from_str::<JsonRpcResponse>(&data) {
                        if resp.id == request_id {
                            return Ok(resp);
                        }
                    }
                }
            }
            return Err(anyhow!("Connection closed before response received"));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("HTTP error {}: {}", status, body));
        }

        // Direct response (non-SSE servers)
        let text = response.text().await?;
        let json_text = if text.contains("data: ") {
            text.lines()
                .filter(|l| l.starts_with("data: "))
                .map(|l| l.strip_prefix("data: ").unwrap_or(""))
                .collect::<Vec<_>>()
                .join("")
        } else {
            text
        };

        if json_text.is_empty() {
            return Err(anyhow!("Empty response from server"));
        }

        let response: JsonRpcResponse = serde_json::from_str(&json_text)
            .with_context(|| format!("Failed to parse response: {}", json_text))?;

        Ok(response)
    }

    async fn close(&mut self) -> Result<()> {
        let mut conn = self.connection.lock().await;
        *conn = None;
        Ok(())
    }
}
