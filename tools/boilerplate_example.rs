// src/tools/boilerplate_example.rs
use rmcp::tool;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
// Import the specific extractors we need
use rmcp::service::{RequestContext, RoleServer};
use rmcp::Peer;
use serde::Deserialize;
use rmcp::schemars::JsonSchema;

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
    // 1. Standard parameter extraction
    Parameters(args): Parameters<BoilerplateArgs>,

    // 2. Extract the Request Context (contains the cancellation token)
    req_ctx: RequestContext<RoleServer>,

    // 3. Extract the Peer (allows sending logs/notifications back to the client)
    _peer: Peer<RoleServer>,

) -> Result<CallToolResult, rmcp::ErrorData> {

    // Check if the user hit "Stop" in the client UI
    // The cancellation token lives inside the request context's `ct` field
    if req_ctx.ct.is_cancelled() {
        return Ok(CallToolResult::error(vec![
            ContentBlock::text("Operation cancelled by user")
        ]));
    }

    if args.example_param.is_empty() {
        return Ok(CallToolResult::error(vec![
            ContentBlock::text("example_param cannot be empty")
        ]));
    }

    // Example: Send a structured log to the client's developer console
    // (Uncomment to test in LM Studio/Claude Desktop)
    /*
    use rmcp::model::{LoggingLevel, LoggingMessageNotificationParam};
    peer.send_logging_message(LoggingMessageNotificationParam {
        level: LoggingLevel::Info,
        logger: Some("boilerplate_example".into()),
        data: format!("Processing: {}", args.example_param).into(),
    }).await.ok();
    */

    let mut output = format!("Received parameter: {}", args.example_param);
    if let Some(modifier) = args.optional_modifier {
        output.push_str(&format!("\nApplied modifier: {}", modifier));
    }

    Ok(CallToolResult::success(vec![ContentBlock::text(output)]))
}
