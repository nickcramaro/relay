# Relay

CLI interface for MCP (Model Context Protocol) servers.

## Installation

```bash
cargo install --path .
```

## Quick Start

```bash
# Add an MCP server
relay add linear \
  --transport stdio \
  --cmd "npx @linear/mcp-server" \
  --env LINEAR_API_KEY=your-key

# List available tools
relay tools

# Describe a tool
relay describe issue.create

# Run a tool
relay run issue.create \
  --title "Bug report" \
  --team ENG \
  --description "Found an issue"
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
  linear:
    transport: stdio
    command: "npx @linear/mcp-server"
    env:
      LINEAR_API_KEY: "${env:LINEAR_API_KEY}"

default_server: linear
```

## License

MIT
