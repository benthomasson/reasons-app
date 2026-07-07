# Reasons.app

A macOS menu bar app that runs a [Reasons](https://github.com/benthomasson/reasons-rust) MCP server — a justification-based truth maintenance system (JTMS) for managing beliefs with dependency tracking, contradiction detection, and truth-value propagation.

Reasons.app manages the MCP server lifecycle and provides one-click installation into Claude Desktop and Claude Code. It supports multiple databases (domains) so a single MCP endpoint can serve beliefs from different knowledge areas.

## Install

### From source

```bash
cargo install tauri-cli
cargo tauri build
```

The built `.app` bundle will be in `src-tauri/target/release/bundle/macos/`.

### Development

```bash
cargo tauri dev
```

## Features

- **Menu bar app** — runs as a macOS tray icon (no Dock icon, no main window)
- **Auto-start MCP server** — HTTP transport on `localhost:6519`
- **Multi-domain support** — serve multiple reasons databases through one MCP endpoint
- **One-click install** — register with Claude Desktop or Claude Code from the tray menu
- **Domain configuration** — `~/.reasons/domains.toml` (auto-created on first run)

## Architecture

Cargo workspace with three crates:

| Crate | Purpose |
|-------|---------|
| `reasons-core` | Library — TMS engine, database, MCP server, domain config |
| `reasons-cli` | Binary `reasons` — CLI and stdio MCP server |
| `src-tauri` | Tauri v2 app — menu bar, HTTP MCP server, installer |

## Domains

Domains are configured in `~/.reasons/domains.toml`:

```toml
default = "product"

[[domain]]
name = "product"
path = "~/reasons.db"

[[domain]]
name = "code"
path = "~/git/my-project/reasons.db"

[[domain]]
name = "research"
path = "~/git/papers/reasons.db"
```

All MCP tools accept an optional `domain` parameter. When omitted, the default domain is used. The `domains` tool lists all configured domains.

## MCP Server

### Stdio transport (Claude Desktop / Claude Code)

```bash
reasons mcp --db /path/to/reasons.db
```

The stdio server reads `~/.reasons/domains.toml` and serves all configured domains, with the `--db` path included as an additional domain if not already listed.

Claude Desktop configuration (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "reasons": {
      "command": "reasons",
      "args": ["mcp", "--db", "/path/to/reasons.db"]
    }
  }
}
```

### HTTP transport (Reasons.app)

The Tauri app runs an HTTP MCP server on `http://localhost:6519/mcp` using Streamable HTTP transport.

### Tools

| Tool | Purpose |
|------|---------|
| `domains` | List configured domains and their database paths |
| `search` | Full-text search with neighbor expansion |
| `show` | Node details, justifications, and dependents |
| `explain` | Trace why a node is IN or OUT |
| `tree` | Dependency tree visualization |
| `list` | List nodes with status/type/impact filters |
| `add` | Create a premise or derived node |
| `retract` | Mark a node OUT with cascading propagation |
| `assert_node` | Restore a retracted node to IN |
| `challenge` | Attack a belief with a counter-argument |
| `defend` | Counter a challenge to restore a belief |
| `nogood` | Record a contradiction with auto-backtracking |

## CLI

The `reasons` binary includes a full CLI for direct database operations. See [`reasons-rust`](https://github.com/benthomasson/reasons-rust) for CLI documentation, or run `reasons --help`.

## Related Projects

- [reasons-rust](https://github.com/benthomasson/reasons-rust) — standalone CLI and MCP server (this app is forked from it)
- [ftl-reasons](https://github.com/benthomasson/ftl-reasons) — original Python implementation with LLM-powered commands

## License

MIT
