// Import RMCP macros and primitives for declaring tools
use rmcp::tool; // #[tool] macro: defines a new callable tool
use rmcp::handler::server::tool::Parameters; // Wrapper type that deserializes JSON-RPC parameters
use rmcp::model::{CallToolResult, Content}; // Types used for the return value of a tool

// Derive traits for automatic argument parsing and schema generation
use serde::Deserialize; // Deserialize tool parameters from JSON
use rmcp::schemars::JsonSchema; // Enables automatic OpenAPI / JSON Schema generation for the tool
use rmcp::schemars; // Re-export of the schemars crate

use std::future::Future; // Required for async tool functions (often implicit)

// ------------------------------------------------------------------------------------------------
// TOOL ARGUMENT STRUCT
// ------------------------------------------------------------------------------------------------

// This struct defines what arguments your tool will accept.
// It will be automatically deserialized from the incoming JSON input,
// and used to auto-generate documentation for your tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct BoilerplateArgs {
    // This is a single named argument. You can add more fields as needed.
    // The `#[schemars(description = "...")]` adds a helpful description for schema generation,
    // which the server can expose to clients or for documentation tools.
    #[schemars(description = "Example parameter for the tool")]
    example_param: String,
}

// ------------------------------------------------------------------------------------------------
// TOOL FUNCTION
// ------------------------------------------------------------------------------------------------

// This macro marks this function as a server-exposed tool.
// The `name` is what the client will use to call it, and the `description` is for documentation.
// Clients using JSON-RPC can now invoke "boilerplate_example" with the appropriate args.
#[tool(
    name = "boilerplate_example",
    description = "Echoes back the `example_param` argument"
)]
pub async fn boilerplate_example(
    // This wraps your input struct in RMCP's Parameters<T> type.
    // This enables schema validation and type-safe access to your arguments.
    Parameters(args): Parameters<BoilerplateArgs>,
) -> Result<CallToolResult, rmcp::ErrorData> {
    // Your tool logic goes here.
    // Here we're simply echoing back the input in a formatted string.
    let output = format!("Received parameter: {}", args.example_param);

    // Tools return `CallToolResult`, which wraps a list of `Content`.
    // Each `Content::text()` adds a textual reply for the client.
    Ok(CallToolResult::success(vec![Content::text(output)]))
}
