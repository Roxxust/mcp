// src/tools/internet_lookup.rs

// Import RMCP macros and primitives
use rmcp::tool;
use rmcp::handler::server::tool::Parameters;
use rmcp::model::{CallToolResult, Content, ErrorData, ErrorCode};
use rmcp::serde_json;

// Derive traits
use serde::Deserialize;
use rmcp::schemars::JsonSchema;
use rmcp::schemars;

// Required for HTTP requests
use reqwest;
use urlencoding;

// ------------------------------------------------------------------------------------------------
// TOOL ARGUMENT STRUCT
// ------------------------------------------------------------------------------------------------

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InternetLookupArgs {
    #[schemars(description = "The search query string to look up on the internet")]
    query: String,
}

// ------------------------------------------------------------------------------------------------
// TOOL FUNCTION
// ------------------------------------------------------------------------------------------------

#[tool(
    name = "internet_lookup",
    description = "Search the internet for current information"
)]
pub async fn internet_lookup(
    Parameters(args): Parameters<InternetLookupArgs>,
) -> Result<CallToolResult, ErrorData> {
    // First try Wikipedia for factual information
    match search_wikipedia(&args.query).await {
        Ok(results) => {
            if !results.is_empty() {
                return Ok(CallToolResult::success(vec![Content::text(results)]));
            }
        }
        Err(_) => {} // Continue to general search if Wikipedia fails
    }
    
    // Fallback to DuckDuckGo
    match search_duckduckgo(&args.query).await {
        Ok(results) => {
            if !results.is_empty() {
                return Ok(CallToolResult::success(vec![Content::text(results)]));
            }
        }
        Err(e) => return Err(e),
    }
    
    // If all else fails
    let output = format!("Searched for: \"{}\"\n\nNo detailed results available. Try rephrasing your query.", args.query);
    Ok(CallToolResult::success(vec![Content::text(output)]))
}

// Search Wikipedia for factual information
async fn search_wikipedia(query: &str) -> Result<String, ErrorData> {
    let encoded_query = urlencoding::encode(query);
    let url = format!("https://en.wikipedia.org/api/rest_v1/page/summary/{}", encoded_query);
    
    let response = reqwest::get(&url)
        .await
        .map_err(|_e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                "Failed to make HTTP request to Wikipedia",
                None
            )
        })?;
    
    // If page not found, return empty string to try other methods
    if response.status() == 404 {
        return Ok(String::new());
    }
    
    let json: serde_json::Value = response.json()
        .await
        .map_err(|_e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                "Failed to parse Wikipedia response",
                None
            )
        })?;
    
    let mut output = String::new();
    
    if let Some(title) = json["title"].as_str() {
        output.push_str(&format!("**{}**\n", title));
    }
    
    if let Some(extract) = json["extract"].as_str() {
        output.push_str(&format!("{}\n", extract));
    }
    
    if let Some(page_url) = json["content_urls"]["desktop"]["page"].as_str() {
        output.push_str(&format!("\n[Read more on Wikipedia]({})", page_url));
    }
    
    Ok(output)
}

// Search DuckDuckGo as fallback
async fn search_duckduckgo(query: &str) -> Result<String, ErrorData> {
    let encoded_query = urlencoding::encode(query);
    let url = format!("https://api.duckduckgo.com/?q={}&format=json&no_html=1", encoded_query);
    
    let response = reqwest::get(&url)
        .await
        .map_err(|_e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                "Failed to make HTTP request to search engine",
                None
            )
        })?;
    
    let json: serde_json::Value = response.json()
        .await
        .map_err(|_e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                "Failed to parse search engine response",
                None
            )
        })?;
    
    let mut output = String::new();
    
    // Add abstract if available
    if let Some(abstract_text) = json["AbstractText"].as_str() {
        if !abstract_text.is_empty() {
            output.push_str(&format!("**Summary**: {}\n\n", abstract_text));
        }
    }
    
    // Add related topics
    if let Some(related) = json["RelatedTopics"].as_array() {
        if !related.is_empty() {
            output.push_str("**Related Information**:\n");
            
            for (i, topic) in related.iter().take(5).enumerate() {
                if let Some(text) = topic["Text"].as_str() {
                    output.push_str(&format!("{}. {}\n", i + 1, text));
                }
            }
        }
    }
    
    Ok(output)
}