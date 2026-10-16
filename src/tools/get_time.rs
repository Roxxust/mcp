// src/tools/get_time.rs
use chrono::{Local, Datelike, Timelike};
use rmcp::tool;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use serde::Deserialize;
use rmcp::schemars::JsonSchema;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetTimeArgs {
    #[serde(default)]
    #[schemars(description = "Optional format style: 12hr, 24hr, iso, or unix")]
    pub format: Option<String>,
}

#[tool(
    name = "get_time",
    description = "Returns the current time in the server's local timezone.",
    annotations(
        title = "Get Current Time",
        read_only_hint = true,
        idempotent_hint = true,
        open_world_hint = false,
        destructive_hint = false
    )
)]
pub async fn get_time(
    Parameters(args): Parameters<GetTimeArgs>,
) -> Result<CallToolResult, rmcp::ErrorData> {
    let now = Local::now();
    let tz_str = now.format("%Z").to_string();
    let tz_str = if tz_str.is_empty() { "Local Time".to_string() } else { tz_str };

    let output = match args.format.as_deref().unwrap_or("12hr").to_lowercase().as_str() {
        "24hr" => format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02} {} (24hr)",
            now.year(), now.month(), now.day(),
            now.hour(), now.minute(), now.second(), tz_str
        ),
        "12hr" => {
            let h = now.hour();
            let hour12 = if h == 0 || h == 12 { 12 } else { h % 12 };
            let ampm = if h < 12 { "AM" } else { "PM" };
            format!(
                "{} at {:02}:{:02}:{:02} {} ({})",
                now.format("%A, %B %-d, %Y"), hour12, now.minute(), now.second(), ampm, tz_str
            )
        }
        "iso" => now.to_rfc3339(),
        "unix" => now.timestamp().to_string(),
        invalid => {
            // 3.5.1 Pattern: Use CallToolResult::error for expected tool-level failures
            return Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                "Unsupported format: '{}'. Valid options: 12hr, 24hr, iso, unix.", invalid
            ))]));
        }
    };

    Ok(CallToolResult::success(vec![ContentBlock::text(output)]))
}
