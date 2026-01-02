use crate::mcp::{JsonRpcRequest, JsonRpcResponse};
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Transport: Send + Sync {
    /// Send a request and receive a response
    async fn request(&mut self, req: JsonRpcRequest) -> Result<JsonRpcResponse>;

    /// Close the transport
    async fn close(&mut self) -> Result<()>;
}
