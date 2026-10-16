// src/tools/boilerplate_example.rs
use rmcp::tool;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
// Import the specific extractors we need
use rmcp::service::{RequestContext, RoleServer};
use rmcp::Peer;
use serde::Deserialize;
use rmcp::schemars::JsonSchema;

/// Arguments accepted by the `boilerplate_example` tool.
///
/// `Deserialize` allows RMCP to parse incoming tool arguments from JSON.
/// `JsonSchema` generates the schema exposed to clients so they know
/// which arguments are available and what types they expect.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct BoilerplateArgs {
    /// Required string input. The schema description helps clients
    /// understand what value to provide.
    #[schemars(description = "The primary input parameter.")]
    pub example_param: String,

    /// Optional numeric modifier. If omitted from the incoming JSON,
    /// this field defaults to `None`.
    #[serde(default)]
    #[schemars(description = "An optional numeric modifier.")]
    pub optional_modifier: Option<i32>,
}

/// Registers this function as an MCP tool.
///
/// The name and description are presented to clients when they discover
/// available tools. Annotations communicate the tool's expected behavior;
/// they describe its capabilities but do not enforce them.
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
    // Deserializes the incoming JSON arguments into `BoilerplateArgs`.
    Parameters(args): Parameters<BoilerplateArgs>,

    // Provides request-scoped information, including the cancellation
    // token used to detect when the client cancels the operation.
    req_ctx: RequestContext<RoleServer>,

    // Represents the connected client peer. Use it to send supported
    // notifications, such as logging messages, back to the client.
    // Prefix with `_` because this boilerplate does not use it by default. rename _peer to peer in the function signature to enable it if needed
    _peer: Peer<RoleServer>,

) -> Result<CallToolResult, rmcp::ErrorData> {
    // Check for cancellation before starting the work. Longer-running tools
    // should also check cancellation at appropriate points during execution.
    if req_ctx.ct.is_cancelled() {
        return Ok(CallToolResult::error(vec![
            ContentBlock::text("Operation cancelled by user")
        ]));
    }

    // Validate application-level constraints that are not fully expressed
    // by the argument types or generated JSON schema.
    if args.example_param.is_empty() {
        return Ok(CallToolResult::error(vec![
            ContentBlock::text("example_param cannot be empty")
        ]));
    }

    // Optional: send a structured log notification to the connected client.
    // Uncomment this block and rename `_peer` to `peer` in the function
    // signature to enable it. Client support for displaying logs may vary.
    //
    // use rmcp::model::{LoggingLevel, LoggingMessageNotificationParam};
    //
    // peer.send_logging_message(LoggingMessageNotificationParam {
    //     level: LoggingLevel::Info,
    //     logger: Some("boilerplate_example".into()),
    //     data: format!("Processing: {}", args.example_param).into(),
    // })
    // .await
    // .ok();

    // Build the response from the required input and include the modifier
    // only when the caller supplied one.
    let mut output = format!("Received parameter: {}", args.example_param);
    if let Some(modifier) = args.optional_modifier {
        output.push_str(&format!("\nApplied modifier: {}", modifier));
    }

    // Return a successful MCP tool result containing a text response.
    // Use CallToolResult::error for tool-level failures that should be
    // reported to the client as an unsuccessful tool invocation.
    Ok(CallToolResult::success(vec![ContentBlock::text(output)]))
}
