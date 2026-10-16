<div align="center">

# Rust MCP Server

### A modular Model Context Protocol server built in Rust

[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![rmcp](https://img.shields.io/badge/rmcp-3.5.1-blue?style=flat-square)](https://crates.io/crates/rmcp)
[![License: GPL v3](https://img.shields.io/badge/license-GPL--3.0-green?style=flat-square)](https://www.gnu.org/licenses/gpl-3.0)
[![Transport](https://img.shields.io/badge/transport-stdio-lightgrey?style=flat-square)]()

</div>

---

A lightweight, extensible **Model Context Protocol (MCP)** server written in Rust. It exposes callable tools to AI clients (LM Studio, Claude Desktop, Cursor, etc.) over **stdio** using JSON-RPC.

---

## Available Tools

| Tool | Description | Read-Only | Idempotent | Online | Destructive |
|------|-------------|:---------:|:----------:|:--------:|:------------:|
| `get_time` | Returns current server time in 12hr, 24hr, iso, or unix format | ✅ | ✅ | ❌ | ❌ |
| `boilerplate_example` | Echoes parameters back — template for building new tools | ✅ | ✅ | ❌ | ❌ |

---

## Quick Start

### Prerequisites

- [Rust 1.85+](https://rustup.rs)
- An MCP-compatible client (LM Studio, Claude Desktop, Cursor, etc.)

The server starts on **stdio** and waits for a client connection.

### Build

```bash
git clone https://github.com/Roxxust/mcp.git
cd mcp-server
cargo build --release
```

### Run

```bash
cargo run
```


---


## Client Configuration

### Local (stdio)

```json
{
  "mcpServers": {
    "mcp-server": {
      "command": "./target/release/mcp",
      "args": []
    }
  }
}
```

### Remote (SSH tunnel)

```json
{
  "mcpServers": {
    "mcp-server": {
      "command": "ssh",
      "args": ["user@your-host", "/path/to/mcp-server"]
    }
  }
}
```

---

## Adding a New Tool

**Three steps. That's it.**

### 1. Create `src/tools/my_tool.rs`
`boilerplate_example.rs`:

```rust
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::schemars::JsonSchema;
use rmcp::service::{RequestContext, RoleServer};
use rmcp::{tool, Peer};
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BoilerplateArgs {
    #[schemars(description = "The primary input parameter.")]
    pub example_param: String,

    #[serde(default)]
    #[schemars(description = "An optional numeric modifier.")]
    pub optional_modifier: Option<i32>,
}

#[tool(
    name = "boilerplate_example",
    description = "A template tool that echoes back input parameters.",
    annotations(
        title = "Boilerplate Example Tool",
        read_only_hint = true,
        idempotent_hint = true,
        open_world_hint = false,
        destructive_hint = false
    )
)]
pub async fn boilerplate_example(
    Parameters(args): Parameters<BoilerplateArgs>,
    req_ctx: RequestContext<RoleServer>,
    _peer: Peer<RoleServer>,
) -> Result<CallToolResult, rmcp::ErrorData> {
    if req_ctx.ct.is_cancelled() {
        return Ok(CallToolResult::error(vec![
            ContentBlock::text("Operation cancelled by user"),
        ]));
    }

    if args.example_param.is_empty() {
        return Ok(CallToolResult::error(vec![
            ContentBlock::text("example_param cannot be empty"),
        ]));
    }

    let mut output = format!("Received parameter: {}", args.example_param);

    if let Some(modifier) = args.optional_modifier {
        output.push_str(&format!("\nApplied modifier: {}", modifier));
    }

    Ok(CallToolResult::success(vec![ContentBlock::text(output)]))
}
```
### 2. Register in [`src/tools/mod.rs`](https://github.com/Roxxust/mcp/blob/cec52933f2ac80cf229416fb8751305d48478fff/src/tools/mod.rs#L1)

```rust
pub mod my_tool;
```

### 3. Add to the macro in [`src/main.rs`](https://github.com/Roxxust/mcp/blob/cec52933f2ac80cf229416fb8751305d48478fff/src/main.rs#L51)

```rust
register_tools!(router, get_time, boilerplate_example, my_tool);
```

---

## Project Structure

```
mcp-server/
├── Cargo.toml
├── LICENSE
├── README.md
└── src/
    ├── main.rs                    # Server lifecycle + zero-duplication router
    └── tools/
        ├── mod.rs                 # Module declarations
        ├── get_time.rs            # Time formatting tool
        └── boilerplate_example.rs # Template for new tools
```

---

## License

This project is licensed under the **GNU General Public License v3.0**.

See [LICENSE](LICENSE) for the full text, or visit [gnu.org/licenses/gpl-3.0](https://www.gnu.org/licenses/gpl-3.0).
 
---

<div align="center">

**Built with 🦀 and the [Model Context Protocol](https://modelcontextprotocol.io)**

</div>
