use super::Transport;
use crate::mcp::{JsonRpcRequest, JsonRpcResponse};
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

pub struct StdioTransport {
    child: Child,
    stdin: tokio::process::ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
}

impl StdioTransport {
    pub async fn spawn(command: &str, env: HashMap<String, String>) -> Result<Self> {
        // Parse command - first word is the program, rest are args
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            bail!("Empty command");
        }

        let (program, args) = parts.split_first().unwrap();

        let mut cmd = Command::new(program);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .envs(env);

        let mut child = cmd.spawn().with_context(|| format!("Failed to spawn: {}", command))?;

        let stdin = child.stdin.take().context("Failed to get stdin")?;
        let stdout = child.stdout.take().context("Failed to get stdout")?;

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn request(&mut self, req: JsonRpcRequest) -> Result<JsonRpcResponse> {
        // Write request as JSON line
        let mut json = serde_json::to_string(&req)?;
        json.push('\n');

        self.stdin.write_all(json.as_bytes()).await?;
        self.stdin.flush().await?;

        // Read response line
        let mut line = String::new();
        self.stdout.read_line(&mut line).await?;

        if line.is_empty() {
            bail!("Server closed connection unexpectedly");
        }

        let response: JsonRpcResponse = serde_json::from_str(&line)
            .with_context(|| format!("Failed to parse response: {}", line))?;

        Ok(response)
    }

    async fn close(&mut self) -> Result<()> {
        self.child.kill().await.ok();
        Ok(())
    }
}
