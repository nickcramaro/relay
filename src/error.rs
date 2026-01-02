use thiserror::Error;

#[derive(Error, Debug)]
pub enum RelayError {
    #[error("Server '{0}' not found")]
    ServerNotFound(String),

    #[error("Tool '{0}' not found on server '{1}'")]
    ToolNotFound(String, String),

    #[error("No default server configured. Use `relay add` to add a server.")]
    NoDefaultServer,

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("MCP error: {message} (code {code})")]
    McpError { code: i32, message: String },

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Missing required flag: --{0}")]
    MissingRequiredFlag(String),
}
