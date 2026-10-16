// src/main.rs
//
// MCP Server entry point — RMCP 3.5.1
// Transport: stdio (with optional SSH tunneling via client config)
// Registration: Zero-duplication macro pattern

use dotenv::dotenv;
use rmcp::{
    ServerHandler, ServiceExt, tool_handler,
    handler::server::tool::ToolRouter,
    model::{ServerConfig, ServerCapabilities, Implementation},
    transport::stdio,
};

mod tools;

#[derive(Clone)]
pub struct MCPHandler {
    tool_router: ToolRouter<Self>,
}

// ============================================================================
// ZERO-DUPLICATION TOOL REGISTRATION
// ============================================================================
// This macro registers standalone tool functions from src/tools/.
// The #[tool] macro on each function generates a companion `<name>_tool_attr()`
// function containing the tool's metadata (name, description, schema, annotations).
//
// TO ADD A NEW TOOL: append `, new_tool_name` to the invocation below.
// The tool's definition (name, description, annotations, schema) is pulled
// directly from the tool file — nothing is duplicated here.
macro_rules! register_tools {
    ($router:ident, $($mod:ident),* $(,)?) => {
        $(
            rmcp::paste! {
                $router = $router.with_route((
                    tools::$mod::[<$mod _tool_attr>](),
                    tools::$mod::$mod
                ));
            }
        )*
    };
}

impl MCPHandler {
    pub fn new() -> Self {
        let mut router = ToolRouter::new();

        // ─── REGISTER TOOLS HERE ───────────────────────────────────────
        // Add new tools by appending: , tool_name
        register_tools!(router, get_time, boilerplate_example);
        // ────────────────────────────────────────────────────────────────

        Self { tool_router: router }
    }
}

// ============================================================================
// SERVER HANDLER
// ============================================================================
// #[tool_handler] wires the tool_router into the ServerHandler trait.
// The trait provides default implementations for all standard MCP methods
// (initialize, list_resources, read_resource, list_prompts, etc.).
// We only need to implement get_info() to declare our capabilities.
#[tool_handler(router = self.tool_router)]
impl ServerHandler for MCPHandler {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .build(),
        )
        .with_server_info(Implementation::new("mcp-server", "0.1.0"))
        .with_instructions(
            "MCP server exposing tools via JSON-RPC over stdio. \
             Available tools: get_time, boilerplate_example.",
        )
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    // Optional SSH mode flag.
    // In practice, MCP over SSH is configured in the client's mcp.json:
    //   { "command": "ssh", "args": ["user@host", "/path/to/mcp-server"] }
    // The server itself always uses stdio — SSH just tunnels it.
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"--ssh".to_string()) {
        eprintln!("[mcp-server] SSH mode flag detected. Ensure your mcp.json uses 'ssh' as the command.");
    }

    let service = MCPHandler::new().serve(stdio()).await?;
    eprintln!("[mcp-server] Running on stdio. Waiting for client connection…");
    service.waiting().await?;

    Ok(())
}
