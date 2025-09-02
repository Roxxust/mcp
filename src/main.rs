use dotenv::dotenv;
use rmcp::{
    ServerHandler, ServiceExt,
    tool_router, tool_handler, tool,
    handler::server::tool::ToolRouter,
    model::{
        ServerInfo, ServerCapabilities, ProtocolVersion, Implementation,
        InitializeResult, InitializeRequestParam,
        ListResourcesResult, ReadResourceResult, ReadResourceRequestParam,
        ListPromptsResult, GetPromptResult, GetPromptRequestParam,
        ListResourceTemplatesResult, PaginatedRequestParam,
        CallToolResult,
    },
    transport::stdio, ErrorData,
};
use std::future::Future;
mod tools;

#[derive(Clone)]
pub struct MCPHandler {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl MCPHandler {
    pub fn new() -> Self {
        Self { tool_router: Self::tool_router() }
    }

    #[tool(name = "get_time", description = "Current timestamp in ms")]
    async fn get_time(
        &self,
        args: rmcp::handler::server::tool::Parameters<tools::get_time::GetTimeArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::get_time::get_time(args).await
    }

    #[tool(name = "boilerplate_example", description = "Echo parameter back")]
    async fn boilerplate_example(
        &self,
        args: rmcp::handler::server::tool::Parameters<
            tools::boilerplate_example::BoilerplateArgs
        >,
    ) -> Result<CallToolResult, ErrorData> {
        tools::boilerplate_example::boilerplate_example(args).await
    }
    #[tool(name = "query_rustdocs", description = "figure out the crate to use based off the user's inquiry, then lookup the crates you intend to use and scrape the latest version and documentation, then use the updated crate version's documentation to write the code. do this for all crates you intend to use in the project.")]
    async fn query_rustdocs(
        &self,
        args: rmcp::handler::server::tool::Parameters<
            tools::query_rustdocs::QueryRustDocsArgs
        >,
    ) -> Result<CallToolResult, ErrorData> {
        tools::query_rustdocs::query_rustdocs(args).await
    }
    #[tool(name = "internet_lookup", description = "Echo parameter back")]
    async fn internet_lookup(
        &self,
        args: rmcp::handler::server::tool::Parameters<
            tools::internet_lookup::InternetLookupArgs
        >,
    ) -> Result<CallToolResult, ErrorData> {
        tools::internet_lookup::internet_lookup(args).await
    }
}

#[tool_handler]
impl ServerHandler for MCPHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation {
                name: "mcp-server".into(),
                version: "0.1.0".into(),
            },
            instructions: Some("Tools: get_time, boilerplate_example".into()),
        }
    }

    async fn initialize(
        &self,
        _param: InitializeRequestParam,
        _ctx: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<InitializeResult, ErrorData> {
        Ok(InitializeResult {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation {
                name: "mcp-server".into(),
                version: "0.1.0".into(),
            },
            instructions: Some("Use tools via JSON-RPC".into()),
        })
    }

    async fn list_resources(
        &self,
        _req: Option<PaginatedRequestParam>,
        _ctx: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(ListResourcesResult { resources: vec![], next_cursor: None })
    }

    async fn read_resource(
        &self,
        _req: ReadResourceRequestParam,
        _ctx: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        Err(ErrorData::resource_not_found("Not found", None))
    }

    async fn list_prompts(
        &self,
        _req: Option<PaginatedRequestParam>,
        _ctx: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<ListPromptsResult, ErrorData> {
        Ok(ListPromptsResult { prompts: vec![], next_cursor: None })
    }

    async fn get_prompt(
        &self,
        _req: GetPromptRequestParam,
        _ctx: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<GetPromptResult, ErrorData> {
        Err(ErrorData::invalid_params("Not found", None))
    }

    async fn list_resource_templates(
        &self,
        _req: Option<PaginatedRequestParam>,
        _ctx: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<ListResourceTemplatesResult, ErrorData> {
        Ok(ListResourceTemplatesResult {
            resource_templates: vec![],
            next_cursor: None,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let service = MCPHandler::new().serve(stdio()).await?;
    eprintln!("MCP server running on stdio…");
    service.waiting().await?;
    Ok(())
}
