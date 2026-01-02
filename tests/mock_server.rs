use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};

/// A simple mock MCP server for testing
pub fn run_mock_server() {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let reader = BufReader::new(stdin);

    for line in reader.lines() {
        let line = line.unwrap();
        let request: Value = serde_json::from_str(&line).unwrap();

        let method = request["method"].as_str().unwrap_or("");
        let id = &request["id"];

        let response = match method {
            "initialize" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "mock-server", "version": "1.0.0" }
                }
            }),
            "tools/list" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": [
                        {
                            "name": "echo",
                            "description": "Echo back a message",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "message": { "type": "string", "description": "Message to echo" }
                                },
                                "required": ["message"]
                            }
                        }
                    ]
                }
            }),
            "tools/call" => {
                let args = &request["params"]["arguments"];
                let message = args["message"].as_str().unwrap_or("(no message)");
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [{ "type": "text", "text": format!("Echo: {}", message) }],
                        "isError": false
                    }
                })
            }
            _ => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": "Method not found" }
            }),
        };

        writeln!(stdout, "{}", serde_json::to_string(&response).unwrap()).unwrap();
        stdout.flush().unwrap();
    }
}

fn main() {
    run_mock_server();
}
