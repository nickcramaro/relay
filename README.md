# Relay

CLI interface for MCP (Model Context Protocol) servers.

## Installation

### Quick Install

```bash
curl -fsSL https://raw.githubusercontent.com/nickcramaro/relay/main/install.sh | sh
```

### Build from Source

```bash
git clone https://github.com/nickcramaro/relay.git
cd relay
cargo build --release
mkdir -p ~/.local/bin
cp target/release/relay ~/.local/bin/
```

## Quick Start

```bash
# Add MCP servers
relay add context7 \
  --transport http \
  --url https://mcp.context7.com/mcp

relay add linear \
  --transport http \
  --url https://mcp.linear.app/sse

# List available tools
relay tools

# Describe a tool
relay describe resolve-library-id

# Run a tool
relay run resolve-library-id \
  --query "how to parse JSON" \
  --library-name "serde"

# Run Linear tools
relay run --server linear list_issues --query "bugs"
```

## Commands

| Command | Description |
|---------|-------------|
| `relay add <name>` | Register an MCP server |
| `relay list` | List registered servers |
| `relay remove <name>` | Remove a server |
| `relay ping <name>` | Test server connectivity |
| `relay tools [server]` | List available tools |
| `relay describe <tool> [-s server]` | Show tool details |
| `relay run <tool> [-s server] [args]` | Execute a tool |
| `relay auth <name>` | Authenticate with a server |
| `relay logout <name>` | Remove authentication |
| `relay update` | Update to latest version |

## Configuration

Config stored at `~/.config/relay/config.yaml`:

```yaml
servers:
  context7:
    transport: http
    url: https://mcp.context7.com/mcp

default_server: context7
```

## License

MIT
