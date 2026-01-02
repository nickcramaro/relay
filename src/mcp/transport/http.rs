use super::Transport;
use crate::mcp::{JsonRpcRequest, JsonRpcResponse};
use anyhow::Result;
use async_trait::async_trait;

pub struct HttpTransport;

#[async_trait]
impl Transport for HttpTransport {
    async fn request(&mut self, _req: JsonRpcRequest) -> Result<JsonRpcResponse> {
        todo!("Will be implemented in Task 10")
    }

    async fn close(&mut self) -> Result<()> {
        todo!("Will be implemented in Task 10")
    }
}
