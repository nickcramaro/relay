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
cp target/release/relay /usr/local/bin/
```

## Quick Start

```bash
# Add an MCP server
relay add context7 \
  --transport http \
  --url https://mcp.context7.com/mcp

# List available tools
relay tools

# Describe a tool
relay describe resolve-library-id

# Run a tool
relay run resolve-library-id \
  --query "how to parse JSON" \
  --library-name "serde"
```

## Commands

| Command | Description |
|---------|-------------|
| `relay add <name>` | Register an MCP server |
| `relay list` | List registered servers |
| `relay remove <name>` | Remove a server |
| `relay ping <name>` | Test server connectivity |
| `relay tools [--server <name>]` | List available tools |
| `relay describe <tool> [--server <name>]` | Show tool details |
| `relay run <tool> [--server <name>] [args]` | Execute a tool |

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
