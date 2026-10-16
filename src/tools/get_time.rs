// src/tools/get_time.rs
//
// Returns the current time, optionally in a user-specified IANA timezone.

use chrono::{DateTime, Datelike, Local, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::tool;
use rmcp::schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetTimeArgs {
    /// Optional output format: "12hr", "24hr", "iso", or "unix"
    #[serde(default)]
    #[schemars(description = "Optional format style: 12hr, 24hr, iso, or unix")]
    pub format: Option<String>,

    /// Optional IANA timezone name. If omitted, uses the server's local timezone.
    #[serde(default)]
    #[schemars(description = "Optional IANA timezone name, e.g. 'Asia/Tokyo' for Japan, 'America/New_York', 'Europe/London', 'UTC'. Case-sensitive. Omit to use the server's local timezone.")]
    pub timezone: Option<String>,
}

#[tool(
    name = "get_time",
    description = "Returns the current time. Defaults to the server's local timezone, but accepts an optional IANA timezone name to report the time anywhere in the world (e.g. 'Asia/Tokyo' for Japan). Supports 12hr, 24hr, iso, and unix output formats.",
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
    let format = args.format.as_deref().unwrap_or("12hr").to_lowercase();

    let output = if let Some(tz_name) = args.timezone.as_deref() {
        // Parse the IANA timezone name (e.g. "Asia/Tokyo")
        let tz: Tz = match tz_name.parse() {
            Ok(tz) => tz,
            Err(_) => {
                return Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                    "Unknown timezone: '{}'. Use IANA names like 'Asia/Tokyo', 'America/New_York', or 'Europe/London'. Names are case-sensitive.",
                    tz_name
                ))]));
            }
        };
        let now = Utc::now().with_timezone(&tz);
        build_output(now, &format, tz_name)
    } else {
        let now = Local::now();
        build_output(now, &format, "Local Time")
    };

    Ok(CallToolResult::success(vec![ContentBlock::text(output)]))
}

// Generic formatter that works for both Local and chrono-tz timezones.
// We add `Tz::Offset: std::fmt::Display` to satisfy chrono's `.format()` requirements.
fn build_output<Tz>(now: DateTime<Tz>, format: &str, fallback_tz: &str) -> String
where
    Tz: TimeZone,
    Tz::Offset: std::fmt::Display, // <--- THIS FIXES THE COMPILER ERROR
{
    let tz_str = now.format("%Z").to_string();
    let tz_str = if tz_str.is_empty() { fallback_tz.to_string() } else { tz_str };

    match format {
        "24hr" => format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02} {} (24hr)",
            now.year(), now.month(), now.day(),
            now.hour(), now.minute(), now.second(),
            tz_str
        ),
        "12hr" => {
            let h = now.hour();
            let hour12 = if h == 0 || h == 12 { 12 } else { h % 12 };
            let ampm = if h < 12 { "AM" } else { "PM" };
            format!(
                "{} at {:02}:{:02}:{:02} {} ({})",
                now.format("%A, %B %-d, %Y"),
                hour12, now.minute(), now.second(),
                ampm, tz_str
            )
        }
        "iso" => now.to_rfc3339(),
        "unix" => now.timestamp().to_string(),
        invalid => format!(
            "Unsupported format: '{}'. Try 12hr, 24hr, iso, or unix.",
            invalid
        ),
    }
}
