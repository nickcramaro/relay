use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "relay")]
#[command(version, about = "CLI interface for MCP servers", long_about = None)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Output format
    #[arg(long, global = true, default_value = "human")]
    pub format: OutputFormat,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Clone, Copy, Default, clap::ValueEnum)]
pub enum OutputFormat {
    #[default]
    Human,
    Json,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a new MCP server
    Add {
        /// Server name
        name: String,
        /// Transport type
        #[arg(long, value_enum)]
        transport: Transport,
        /// Command to spawn (for stdio transport)
        #[arg(long)]
        cmd: Option<String>,
        /// URL (for http transport)
        #[arg(long)]
        url: Option<String>,
        /// Environment variables (KEY=value format)
        #[arg(long, value_parser = parse_env_var)]
        env: Vec<(String, String)>,
    },
    /// List registered servers
    List,
    /// Remove a server
    Remove {
        /// Server name
        name: String,
    },
    /// Ping a server to check connectivity
    Ping {
        /// Server name
        name: String,
    },
    /// List tools from a server
    Tools {
        /// Server name (uses default if not specified)
        server: Option<String>,
    },
    /// Describe a specific tool
    Describe {
        /// Server name (uses default if not specified)
        server: Option<String>,
        /// Tool name
        tool: String,
    },
    /// Run a tool
    Run {
        /// Server name (uses default if not specified)
        server: Option<String>,
        /// Tool name
        tool: String,
        /// JSON input for the tool
        #[arg(long)]
        input_json: Option<String>,
        /// Tool arguments as flags (collected dynamically)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum Transport {
    Stdio,
    Http,
}

fn parse_env_var(s: &str) -> Result<(String, String), String> {
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{s}`"))?;
    Ok((s[..pos].to_string(), s[pos + 1..].to_string()))
}
