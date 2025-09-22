// src/tools/get_time.rs

use chrono::{Local, Datelike, Timelike};
use rmcp::tool;
use rmcp::handler::server::tool::Parameters;
use rmcp::model::{CallToolResult, Content};
use serde::Deserialize;
use rmcp::schemars::JsonSchema;
use rmcp::schemars;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetTimeArgs {
    /// Optional format: "12hr", "24hr", "iso", or "unix"
    #[serde(default)]
    #[schemars(description = "Optional format style: 12hr, 24hr, iso, or unix")]
    format: Option<String>,
}

#[tool(
    name = "get_time",
    description = "Returns the current time in the server's local timezone. Defaults to a readable 12-hour AM/PM format."
)]
pub async fn get_time(
    Parameters(args): Parameters<GetTimeArgs>,
) -> Result<CallToolResult, rmcp::ErrorData> {
    // Get current time in system's local timezone
    let now = Local::now();
    
    // Get timezone name (e.g., "PST", "EST", "CET") using format specifier
    let tz_str = now.format("%Z").to_string();
    let tz_str = if tz_str.is_empty() { "Local Time".to_string() } else { tz_str };

    // Match format
    let output = match args.format.as_deref().unwrap_or("12hr").to_lowercase().as_str() {
        "24hr" => format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02} {} (24hr)",
            now.year(),
            now.month(),
            now.day(),
            now.hour(),
            now.minute(),
            now.second(),
            tz_str
        ),

        "12hr" => {
            let (hour12, ampm) = {
                let h = now.hour();
                ((if h == 0 || h == 12 { 12 } else { h % 12 }), if h < 12 { "AM" } else { "PM" })
            };
            format!(
                "{} at {:02}:{:02}:{:02} {} ({})",
                now.format("%A, %B %-d, %Y"),
                hour12,
                now.minute(),
                now.second(),
                ampm,
                tz_str
            )
        }

        "iso" => now.to_rfc3339(),

        "unix" => now.timestamp().to_string(),

        invalid => format!(
            "Unsupported format: '{}'. Try 12hr, 24hr, iso, or unix.",
            invalid
        ),
    };

    Ok(CallToolResult::success(vec![Content::text(output)]))
}
