use super::Transport;
use crate::mcp::{JsonRpcRequest, JsonRpcResponse};
use anyhow::Result;
use async_trait::async_trait;

pub struct StdioTransport;

#[async_trait]
impl Transport for StdioTransport {
    async fn request(&mut self, _req: JsonRpcRequest) -> Result<JsonRpcResponse> {
        todo!("Will be implemented in Task 9")
    }

    async fn close(&mut self) -> Result<()> {
        todo!("Will be implemented in Task 9")
    }
}
