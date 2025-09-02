// src/tools/get_time.rs

use chrono::{Utc, FixedOffset, Datelike, Timelike};
use rmcp::tool;
use rmcp::handler::server::tool::Parameters;
use rmcp::model::{CallToolResult, Content};
use serde::Deserialize;
use rmcp::schemars::JsonSchema;
use rmcp::schemars;
use std::future::Future;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetTimeArgs {
    /// Optional format: "12hr", "24hr", "iso", or "unix"
    #[serde(default)]
    #[schemars(description = "Optional format style: 12hr, 24hr, iso, or unix")]
    format: Option<String>,
}

#[tool(
    name = "get_time",
    description = "Returns the current time. Defaults to a readable 12-hour AM/PM format."
)]
pub async fn get_time(
    Parameters(args): Parameters<GetTimeArgs>,
) -> Result<CallToolResult, rmcp::ErrorData> {
    // Always compute "now" in fixed UTC-7 (Pacific)
    let offset = FixedOffset::west_opt(7 * 3600).unwrap();
    let now = Utc::now().with_timezone(&offset);

    // Match format
    let output = match args.format.as_deref().unwrap_or("12hr").to_lowercase().as_str() {
        "24hr" => format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02} (24hr)",
                          now.year(), now.month(), now.day(),
                          now.hour(), now.minute(), now.second()),

        "12hr" => {
            let (hour12, ampm) = {
                let h = now.hour();
                ((if h == 0 || h == 12 { 12 } else { h % 12 }), if h < 12 { "AM" } else { "PM" })
            };
            format!(
                "{} at {:02}:{:02}:{:02} {} (Pacific Time)",
                now.format("%A, %B %-d, %Y"),
                hour12,
                now.minute(),
                now.second(),
                ampm
            )
        }

        "iso" => now.to_rfc3339(),

        "unix" => now.timestamp().to_string(),

        invalid => format!("Unsupported format: '{}'. Try 12hr, 24hr, iso, or unix.", invalid),
    };

    Ok(CallToolResult::success(vec![Content::text(output)]))
}
