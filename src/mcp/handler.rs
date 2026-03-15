use crate::mcp::state::ServerState;
use crate::mcp::tools::*;
use crate::config::AppConfig;

use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    service::RequestContext,
    tool, tool_router,
};
use std::sync::Arc;
use tracing::{debug, info};

#[derive(Clone)]
pub struct McpHandler {
    state: Arc<ServerState>,
    tool_router: ToolRouter<Self>,
}

impl McpHandler {
    pub fn new(state: Arc<ServerState>, _config: &AppConfig) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    /// Get current working directory from state config
    async fn get_working_dir(&self) -> std::path::PathBuf {
        self.state.config.read().await.working_dir.clone()
    }
}

#[tool_router]
impl McpHandler {
    // ===== Basic Tools (Always Enabled) =====

    #[tool(description = "List directory contents with tree structure (max depth 1)")]
    async fn dir_list(
        &self,
        params: Parameters<dir_list::DirListParams>,
    ) -> Result<CallToolResult, McpError> {
        dir_list::dir_list(params).await.map_err(|e| {
            McpError::invalid_params(e, None)
        })
    }

    #[tool(description = "Read text file content")]
    async fn file_read(
        &self,
        params: Parameters<file_read::FileReadParams>,
    ) -> Result<CallToolResult, McpError> {
        file_read::file_read(params).await.map_err(|e| {
            McpError::invalid_params(e, None)
        })
    }

    #[tool(description = "Search for keyword in file or directory")]
    async fn file_search(
        &self,
        params: Parameters<file_search::FileSearchParams>,
    ) -> Result<CallToolResult, McpError> {
        file_search::file_search(params).await.map_err(|e| {
            McpError::invalid_params(e, None)
        })
    }

    #[tool(description = "Write content to file (create/append/overwrite)")]
    async fn file_write(
        &self,
        params: Parameters<file_write::FileWriteParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        file_write::file_write(params, &working_dir).await.map_err(|e| {
            McpError::invalid_params(e, None)
        })
    }

    #[tool(description = "Copy a file to a new location")]
    async fn file_copy(
        &self,
        params: Parameters<file_ops::FileCopyParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        file_ops::file_copy(params, &working_dir).await.map_err(|e| {
            McpError::invalid_params(e, None)
        })
    }

    #[tool(description = "Move a file to a new location")]
    async fn file_move(
        &self,
        params: Parameters<file_ops::FileMoveParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        file_ops::file_move(params, &working_dir).await.map_err(|e| {
            McpError::invalid_params(e, None)
        })
    }

    #[tool(description = "Delete a file")]
    async fn file_delete(
        &self,
        params: Parameters<file_ops::FileDeleteParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        file_ops::file_delete(params, &working_dir).await.map_err(|e| {
            McpError::invalid_params(e, None)
        })
    }

    #[tool(description = "Rename a file")]
    async fn file_rename(
        &self,
        params: Parameters<file_ops::FileRenameParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        file_ops::file_rename(params, &working_dir).await.map_err(|e| {
            McpError::invalid_params(e, None)
        })
    }

    #[tool(description = "Calculate mathematical expressions (supports +, -, *, /, ^, sqrt, sin, cos, tan, log, ln, abs, pi, e)")]
    async fn calculator(
        &self,
        params: Parameters<calculator::CalculatorParams>,
    ) -> Result<CallToolResult, McpError> {
        calculator::calculator(params).await.map_err(|e| {
            McpError::invalid_params(e, None)
        })
    }

    #[tool(description = "Make HTTP GET or POST requests")]
    async fn http_request(
        &self,
        params: Parameters<http_request::HttpRequestParams>,
    ) -> Result<CallToolResult, McpError> {
        http_request::http_request(params).await.map_err(|e| {
            McpError::invalid_params(e, None)
        })
    }

    #[tool(description = "Get current date and time in China format")]
    async fn datetime(&self) -> Result<CallToolResult, McpError> {
        datetime::datetime().await.map_err(|e| {
            McpError::internal_error(e, None)
        })
    }

    #[tool(description = "Read image file and return base64 encoded data with MIME type")]
    async fn image_read(
        &self,
        params: Parameters<image_read::ImageReadParams>,
    ) -> Result<CallToolResult, McpError> {
        image_read::image_read(params).await.map_err(|e| {
            McpError::invalid_params(e, None)
        })
    }

    // ===== Extended Tools (Potentially Disabled) =====

    #[tool(description = "Execute shell command in specified directory (use with caution)")]
    async fn execute_command(
        &self,
        params: Parameters<execute_command::ExecuteCommandParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        execute_command::execute_command(params, &working_dir, self.state.clone(), context).await.map_err(|e| {
            McpError::invalid_params(e, None)
        })
    }

    #[tool(description = "List system processes with CPU and memory usage")]
    async fn process_list(&self) -> Result<CallToolResult, McpError> {
        process_list::process_list().await.map_err(|e| {
            McpError::internal_error(e, None)
        })
    }

    #[tool(description = "Encode string to base64")]
    async fn base64_encode(
        &self,
        params: Parameters<base64_codec::Base64EncodeParams>,
    ) -> Result<CallToolResult, McpError> {
        base64_codec::base64_encode(params).await.map_err(|e| {
            McpError::invalid_params(e, None)
        })
    }

    #[tool(description = "Decode base64 to string")]
    async fn base64_decode(
        &self,
        params: Parameters<base64_codec::Base64DecodeParams>,
    ) -> Result<CallToolResult, McpError> {
        base64_codec::base64_decode(params).await.map_err(|e| {
            McpError::invalid_params(e, None)
        })
    }

    #[tool(description = "Compute hash of string or file (MD5, SHA1, SHA256). Prefix file path with 'file:' for files")]
    async fn hash_compute(
        &self,
        params: Parameters<hash_computer::HashComputeParams>,
    ) -> Result<CallToolResult, McpError> {
        hash_computer::hash_compute(params).await.map_err(|e| {
            McpError::invalid_params(e, None)
        })
    }

    #[tool(description = "Get system information including OS, CPU, memory")]
    async fn system_info(&self) -> Result<CallToolResult, McpError> {
        system_info::system_info().await.map_err(|e| {
            McpError::internal_error(e, None)
        })
    }
}

impl ServerHandler for McpHandler {
    fn get_info(&self) -> ServerInfo {
        info!("Getting server info for MCP initialization");
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()  // Enable tool list change notifications
                .build(),
        )
        .with_server_info(Implementation::from_build_env())
        .with_protocol_version(ProtocolVersion::V_2024_11_05)
        .with_instructions(
            "A comprehensive MCP server with file operations, calculations, HTTP requests, and system tools. \
            Use WebUI at http://127.0.0.1:2233 to manage tool settings."
        )
    }

    async fn initialize(
        &self,
        request: InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        info!("Received initialize request from client: {:?}", request.client_info);
        
        // Check for HTTP request parts (available when running over HTTP)
        if let Some(http_parts) = context.extensions.get::<axum::http::request::Parts>() {
            debug!("Initialize request headers: {:?}", http_parts.headers);
        }
        
        // Return server info as initialize result
        Ok(self.get_info())
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        // Get all tools from router
        let all_tools = self.tool_router.list_all();
        let total_count = all_tools.len();
        
        // Filter to only return enabled tools
        let tools: Vec<Tool> = all_tools
            .into_iter()
            .filter(|tool| {
                // Check synchronously using the DashMap
                self.state.tool_status.get(&tool.name.to_string())
                    .map(|s| s.enabled)
                    .unwrap_or(true) // Default to enabled if not found
            })
            .collect();
        
        info!("Listing {} enabled tools ({} total)", tools.len(), total_count);
        Ok(ListToolsResult {
            tools,
            meta: None,
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let tool_name = request.name.clone();
        
        // Check if tool is enabled
        if !self.state.is_tool_enabled(&tool_name).await {
            info!("Tool '{}' is disabled, rejecting call", tool_name);
            return Err(McpError::invalid_params(
                format!("Tool '{}' is disabled", tool_name),
                None,
            ));
        }

        // Acquire concurrency permit
        let _permit = self.state.concurrency_limiter
            .acquire()
            .await
            .map_err(|_| McpError::internal_error("Failed to acquire concurrency permit", None))?;

        info!("Calling tool: {}", tool_name);
        
        // Record call start
        self.state.record_call_start(&tool_name).await;
        
        // Create tool call context
        let tcc = rmcp::handler::server::tool::ToolCallContext::new(
            self,
            request,
            context,
        );
        
        // Call the tool through the router
        let result = self.tool_router.call(tcc).await;
        
        // Record call end
        self.state.record_call_end(&tool_name).await;
        
        // Permit is automatically released when _permit goes out of scope
        
        result
    }
}
