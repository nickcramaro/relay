use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// JSON-RPC 2.0 request ID (can be string or number per spec)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum RequestId {
    Number(u64),
    String(String),
}

impl From<u64> for RequestId {
    fn from(n: u64) -> Self {
        RequestId::Number(n)
    }
}

impl From<String> for RequestId {
    fn from(s: String) -> Self {
        RequestId::String(s)
    }
}

impl From<&str> for RequestId {
    fn from(s: &str) -> Self {
        RequestId::String(s.to_string())
    }
}

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: RequestId,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    pub fn new(id: impl Into<RequestId>, method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            method: method.into(),
            params,
        }
    }
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: RequestId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    pub fn is_success(&self) -> bool {
        self.result.is_some() && self.error.is_none()
    }

    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// MCP Initialize params
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    pub client_info: ClientInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClientCapabilities {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

/// MCP Initialize result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub server_info: ServerInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolsCapability {
    #[serde(default)]
    pub list_changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// MCP Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<Value>,
}

/// MCP tools/list result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsListResult {
    pub tools: Vec<Tool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// MCP tools/call params
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallParams {
    pub name: String,
    #[serde(default)]
    pub arguments: HashMap<String, Value>,
}

/// MCP tools/call result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallResult {
    pub content: Vec<ContentItem>,
    #[serde(default)]
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ContentItem {
    Text { text: String },
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
    Resource { resource: Value },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jsonrpc_request_serialization() {
        let req = JsonRpcRequest::new(1, "tools/list", None);
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"tools/list\""));
    }

    #[test]
    fn test_jsonrpc_request_with_string_id() {
        let req = JsonRpcRequest::new("req-123", "tools/list", None);
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"id\":\"req-123\""));
    }

    #[test]
    fn test_jsonrpc_request_with_number_id() {
        let req = JsonRpcRequest::new(42u64, "tools/call", None);
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"id\":42"));
    }

    #[test]
    fn test_request_id_from_conversions() {
        let id1: RequestId = 123u64.into();
        assert_eq!(id1, RequestId::Number(123));

        let id2: RequestId = "test".into();
        assert_eq!(id2, RequestId::String("test".to_string()));

        let id3: RequestId = String::from("hello").into();
        assert_eq!(id3, RequestId::String("hello".to_string()));
    }

    #[test]
    fn test_jsonrpc_response_deserialization_with_number_id() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": 42,
            "result": {"status": "ok"}
        }"#;

        let response: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, RequestId::Number(42));
        assert!(response.is_success());
        assert!(!response.is_error());
    }

    #[test]
    fn test_jsonrpc_response_deserialization_with_string_id() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": "request-abc",
            "result": null
        }"#;

        let response: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, RequestId::String("request-abc".to_string()));
    }

    #[test]
    fn test_jsonrpc_response_is_error() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "error": {"code": -32600, "message": "Invalid Request"}
        }"#;

        let response: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert!(response.is_error());
        assert!(!response.is_success());
    }

    #[test]
    fn test_tool_deserialization() {
        let json = r#"{
            "name": "get_weather",
            "description": "Get weather for a location",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "location": { "type": "string" }
                },
                "required": ["location"]
            }
        }"#;

        let tool: Tool = serde_json::from_str(json).unwrap();
        assert_eq!(tool.name, "get_weather");
        assert!(tool.input_schema.is_some());
    }

    #[test]
    fn test_tool_call_result_deserialization() {
        let json = r#"{
            "content": [
                { "type": "text", "text": "Weather: Sunny, 72F" }
            ],
            "isError": false
        }"#;

        let result: ToolCallResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.content.len(), 1);
        assert!(!result.is_error);
    }

    #[test]
    fn test_content_item_image_mime_type_serialization() {
        let image = ContentItem::Image {
            data: "base64data".to_string(),
            mime_type: "image/png".to_string(),
        };
        let json = serde_json::to_string(&image).unwrap();
        assert!(json.contains("\"mimeType\":\"image/png\""));
        assert!(!json.contains("\"mime_type\""));
    }

    #[test]
    fn test_content_item_image_deserialization() {
        let json = r#"{
            "type": "image",
            "data": "base64data",
            "mimeType": "image/jpeg"
        }"#;

        let item: ContentItem = serde_json::from_str(json).unwrap();
        match item {
            ContentItem::Image { data, mime_type } => {
                assert_eq!(data, "base64data");
                assert_eq!(mime_type, "image/jpeg");
            }
            _ => panic!("Expected Image variant"),
        }
    }
}
