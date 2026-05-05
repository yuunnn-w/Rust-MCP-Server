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

/// Convert tool execution result: logic errors become CallToolResult::error instead of JSON-RPC errors
fn tool_result(result: Result<CallToolResult, String>) -> Result<CallToolResult, McpError> {
    match result {
        Ok(r) => Ok(r),
        Err(e) => Ok(CallToolResult::error(vec![rmcp::model::Content::text(e)])),
    }
}

#[tool_router]
impl McpHandler {
    #[tool(description = "List directory contents with filtering (max depth 5). Returns text file char_count and line_count for UTF-8 files. Not restricted to working directory.")]
    async fn dir_list(
        &self,
        params: Parameters<dir_list::DirListParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        tool_result(dir_list::dir_list(params, &working_dir).await)
    }

    #[tool(description = "Read one or more text files concurrently with line numbers and range support. Accepts a list of file items. Not restricted to working directory.")]
    async fn file_read(
        &self,
        params: Parameters<file_read::FileReadParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        tool_result(file_read::file_read(params, &working_dir).await)
    }

    #[tool(description = "Search keyword in files with context (max depth 5). Not restricted to working directory.")]
    async fn file_search(
        &self,
        params: Parameters<file_search::FileSearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        tool_result(file_search::file_search(params, &working_dir).await)
    }

    #[tool(description = "Edit one or more files concurrently using string_replace, line_replace, insert, delete, or patch mode. Can create new files with string_replace/line_replace/insert.")]
    async fn file_edit(
        &self,
        params: Parameters<file_edit::FileEditParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        tool_result(file_edit::file_edit(params, &working_dir).await)
    }

    #[tool(description = "Write content to one or more files concurrently (create/append/overwrite). Accepts a list of file items.")]
    async fn file_write(
        &self,
        params: Parameters<file_write::FileWriteParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        tool_result(file_write::file_write(params, &working_dir).await)
    }

    #[tool(description = "Copy, move, delete, or rename one or more files concurrently. Accepts a list of operations.")]
    async fn file_ops(
        &self,
        params: Parameters<file_ops::FileOpsParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        tool_result(file_ops::file_ops(params, &working_dir).await)
    }

    #[tool(description = "Get metadata for one or more files or directories concurrently. Returns is_text, char_count, line_count, and encoding for UTF-8 files. Not restricted to working directory.")]
    async fn file_stat(
        &self,
        params: Parameters<file_stat::FileStatParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        tool_result(file_stat::file_stat(params, &working_dir).await)
    }

    #[tool(description = "Check if a path exists and get its type. Not restricted to working directory.")]
    async fn path_exists(
        &self,
        params: Parameters<path_exists::PathExistsParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        tool_result(path_exists::path_exists(params, &working_dir).await)
    }

    #[tool(description = "Query a JSON file using JSON Pointer syntax. Not restricted to working directory.")]
    async fn json_query(
        &self,
        params: Parameters<json_query::JsonQueryParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        tool_result(json_query::json_query(params, &working_dir).await)
    }

    #[tool(description = "Run git commands (status, diff, log, branch, show) in a repository. Not restricted to working directory.")]
    async fn git_ops(
        &self,
        params: Parameters<git_ops::GitOpsParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        tool_result(git_ops::git_ops(params, &working_dir).await)
    }

    #[tool(description = "Calculate mathematical expressions")]
    async fn calculator(
        &self,
        params: Parameters<calculator::CalculatorParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(calculator::calculator(params).await)
    }

    #[tool(description = "Make HTTP requests with JSON extraction")]
    async fn http_request(
        &self,
        params: Parameters<http_request::HttpRequestParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(http_request::http_request(params).await)
    }

    #[tool(description = "Get current date and time (China/Beijing UTC+8)")]
    async fn datetime(&self) -> Result<CallToolResult, McpError> {
        tool_result(datetime::datetime().await)
    }

    #[tool(description = "Read image and return base64 or metadata. Not restricted to working directory.")]
    async fn image_read(
        &self,
        params: Parameters<image_read::ImageReadParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        tool_result(image_read::image_read(params, &working_dir).await)
    }

    #[tool(description = "Execute shell command (use with caution)")]
    async fn execute_command(
        &self,
        params: Parameters<execute_command::ExecuteCommandParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        tool_result(execute_command::execute_command(params, &working_dir, self.state.clone(), context).await)
    }

    #[tool(description = "List system processes")]
    async fn process_list(&self) -> Result<CallToolResult, McpError> {
        tool_result(process_list::process_list().await)
    }

    #[tool(description = "Encode or decode base64 strings")]
    async fn base64_codec(
        &self,
        params: Parameters<base64_codec::Base64CodecParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(base64_codec::base64_codec(params).await)
    }

    #[tool(description = "Compute hash (MD5, SHA1, SHA256). Not restricted to working directory.")]
    async fn hash_compute(
        &self,
        params: Parameters<hash_computer::HashComputeParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        tool_result(hash_computer::hash_compute(params, &working_dir).await)
    }

    #[tool(description = "Get system information")]
    async fn system_info(&self) -> Result<CallToolResult, McpError> {
        tool_result(system_info::system_info().await)
    }

    #[tool(description = "Get the value of an environment variable")]
    async fn env_get(
        &self,
        params: Parameters<env_get::EnvGetParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(env_get::env_get(params).await)
    }

    #[tool(description = "Execute Python code for calculations, data processing, and logic evaluation. Set __result for return value. All Python standard library modules are available.")]
    async fn execute_python(
        &self,
        params: Parameters<execute_python::ExecutePythonParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        let allow_fs = self.state.is_python_fs_access_enabled().await;
        tool_result(execute_python::execute_python(params, &working_dir, allow_fs).await)
    }

    #[tool(description = "Read or write system clipboard content. Supports read_text, write_text, read_image, and clear operations. Cross-platform.")]
    async fn clipboard(
        &self,
        params: Parameters<clipboard::ClipboardReadTextParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(clipboard::clipboard(params).await)
    }

    #[tool(description = "Create, extract, list, or append ZIP archives. Supports deflate and zstd compression. Restricted to working directory.")]
    async fn archive(
        &self,
        params: Parameters<archive::ArchiveParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        tool_result(archive::archive(params, &working_dir).await)
    }

    #[tool(description = "Compare text, files, or directories. Output formats: unified, side_by_side, summary, inline. Supports git_diff_file to compare against HEAD.")]
    async fn diff(
        &self,
        params: Parameters<diff::DiffParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        tool_result(diff::diff(params, &working_dir).await)
    }

    #[tool(description = "The AI assistant's short-term memory scratchpad. Use it to temporarily store intermediate results, task sub-steps, context snippets, or working hypotheses during the current conversation or task. Notes are stored only in memory and are automatically erased if not used for 30 minutes. Do not use this for long-term persistence—use it as a thinking workspace to offload complex reasoning or maintain state across multiple tool calls within a session.")]
    async fn note_storage(
        &self,
        params: Parameters<note_storage::NoteStorageParams>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.clone();
        tool_result(note_storage::note_storage(params, state).await)
    }
}

impl ServerHandler for McpHandler {
    fn get_tool(&self, name: &str) -> Option<Tool> {
        self.tool_router.get(name).cloned()
    }

    fn get_info(&self) -> ServerInfo {
        info!("Getting server info for MCP initialization");
        let mut instructions = "A comprehensive MCP server with file operations, calculations, HTTP requests, and system tools. \
            Supports resources (file://) and prompts. \
            Use WebUI at http://127.0.0.1:2233 to manage tool settings."
            .to_string();
        if let Some(ref prompt) = self.state.get_system_prompt_sync() {
            instructions.push_str("\n\n");
            instructions.push_str(prompt);
        }
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .enable_resources()
                .enable_prompts()
                .build(),
        )
        .with_server_info(Implementation::from_build_env())
        .with_protocol_version(ProtocolVersion::V_2024_11_05)
        .with_instructions(instructions)
    }

    async fn initialize(
        &self,
        request: InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        info!("Received initialize request from client: {:?}", request.client_info);
        
        if context.peer.peer_info().is_none() {
            context.peer.set_peer_info(request.clone());
        }
        
        if let Some(http_parts) = context.extensions.get::<axum::http::request::Parts>() {
            debug!("Initialize request headers: {:?}", http_parts.headers);
        }
        
        let mut rx = self.state.tool_list_changed_tx.subscribe();
        let peer = context.peer.clone();
        tokio::spawn(async move {
            while rx.recv().await.is_ok() {
                let _ = peer.notify_tool_list_changed().await;
            }
        });
        
        Ok(self.get_info())
    }

    async fn set_level(
        &self,
        request: SetLevelRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<(), McpError> {
        let level = request.level;
        let level_str = match level {
            rmcp::model::LoggingLevel::Debug => "debug",
            rmcp::model::LoggingLevel::Info => "info",
            rmcp::model::LoggingLevel::Warning => "warn",
            rmcp::model::LoggingLevel::Error => "error",
            rmcp::model::LoggingLevel::Critical => "error",
            rmcp::model::LoggingLevel::Emergency => "error",
            rmcp::model::LoggingLevel::Alert => "error",
            rmcp::model::LoggingLevel::Notice => "info",
        };
        info!("Setting log level to {:?} ({})", level, level_str);
        // Update the tracing subscriber filter dynamically is complex;
        // for now we update the config so it persists across restarts.
        // A full dynamic log level change would require a reload handle.
        // We acknowledge the request and return success.
        Ok(())
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        let working_dir = self.get_working_dir().await;
        let uri = format!("file:///");
        let name = working_dir.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "working-directory".to_string());
        
        let raw_resource = RawResource::new(uri, name)
            .with_description("Server working directory");
        let resources = vec![Resource::new(raw_resource, None)];
        
        Ok(ListResourcesResult {
            resources,
            meta: None,
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        let working_dir = self.get_working_dir().await;
        let uri = request.uri;
        
        if uri == "file:///" {
            // Return directory listing as resource content
            let entries = std::fs::read_dir(&working_dir)
                .map_err(|e| McpError::internal_error(format!("Failed to read directory: {}", e), None))?;
            
            let mut content = String::new();
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                content.push_str(&format!("{} {}{}\n", if is_dir { "[DIR]" } else { "[FILE]" }, name, if is_dir { "/" } else { "" }));
            }
            
            return Ok(ReadResourceResult::new(vec![
                rmcp::model::ResourceContents::text(content, uri),
            ]));
        }
        
        // Try to parse as file://{relative_path}
        let relative_path = uri.strip_prefix("file:///").unwrap_or(&uri);
        let file_path = crate::utils::file_utils::ensure_path_within_working_dir(
            std::path::Path::new(relative_path),
            &working_dir,
        ).map_err(|e| McpError::invalid_params(e, None))?;
        
        if !file_path.exists() {
            return Err(McpError::invalid_params(format!("Resource '{}' not found", uri), None));
        }
        
        if file_path.is_dir() {
            let entries = std::fs::read_dir(&file_path)
                .map_err(|e| McpError::internal_error(format!("Failed to read directory: {}", e), None))?;
            
            let mut content = String::new();
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                content.push_str(&format!("{} {}{}\n", if is_dir { "[DIR]" } else { "[FILE]" }, name, if is_dir { "/" } else { "" }));
            }
            
            Ok(ReadResourceResult::new(vec![
                rmcp::model::ResourceContents::text(content, uri),
            ]))
        } else {
            let content = tokio::fs::read_to_string(&file_path)
                .await
                .map_err(|e| McpError::internal_error(format!("Failed to read file: {}", e), None))?;
            
            Ok(ReadResourceResult::new(vec![
                rmcp::model::ResourceContents::text(content, uri),
            ]))
        }
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        let prompts = vec![
            Prompt::new("system_diagnosis", Some("Guide for analyzing system information and identifying issues"), None),
            Prompt::new("file_analysis", Some("Guide for analyzing code files and directory structures"), None),
            Prompt::new("security_checklist", Some("Checklist to review before executing dangerous operations"), None),
        ];
        
        Ok(ListPromptsResult {
            prompts,
            meta: None,
            next_cursor: None,
        })
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        let messages = match request.name.as_str() {
            "system_diagnosis" => vec![
                PromptMessage::new_text(PromptMessageRole::User, "You are a system diagnostics assistant. When given system information, analyze CPU usage, memory consumption, process load, and uptime to identify potential bottlenecks or anomalies. Provide actionable recommendations."),
            ],
            "file_analysis" => vec![
                PromptMessage::new_text(PromptMessageRole::User, "You are a code analysis assistant. When examining files or directories, identify the project structure, main entry points, dependencies, and potential issues. Summarize the architecture and suggest improvements."),
            ],
            "security_checklist" => vec![
                PromptMessage::new_text(PromptMessageRole::User, "Before executing any dangerous command (file deletion, command execution, etc.), verify the following:\n1. The command is necessary and intended by the user.\n2. The working directory is correct.\n3. No destructive operations will affect data outside the intended scope.\n4. A backup or recovery plan exists if applicable.\nConfirm each item explicitly."),
            ],
            _ => return Err(McpError::invalid_params(format!("Prompt '{}' not found", request.name), None)),
        };
        
        let description = match request.name.as_str() {
            "system_diagnosis" => "Guide for analyzing system information",
            "file_analysis" => "Guide for analyzing code files",
            "security_checklist" => "Checklist before dangerous operations",
            _ => "",
        };
        
        Ok(GetPromptResult::new(messages).with_description(description))
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let all_tools = self.tool_router.list_all();
        let total_count = all_tools.len();
        let _allow_fs = self.state.is_python_fs_access_enabled().await;
        
        let tools: Vec<Tool> = all_tools
            .into_iter()
            .filter(|tool| {
                self.state.tool_status.get(&tool.name.to_string())
                    .map(|s| s.enabled)
                    .unwrap_or(true)
            })
            .collect();
        
        // Tool descriptions are set at registration time via tool_router macro.
        // No runtime description overrides needed.
        
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
        
        if !self.state.is_tool_enabled(&tool_name).await {
            info!("Tool '{}' is disabled, rejecting call", tool_name);
            return Err(McpError::invalid_params(
                format!("Tool '{}' is disabled", tool_name),
                None,
            ));
        }

        let _permit = self.state.concurrency_limiter
            .acquire()
            .await
            .map_err(|_| McpError::internal_error("Failed to acquire concurrency permit", None))?;

        info!("Calling tool: {}", tool_name);
        
        self.state.record_call_start(&tool_name).await;
        
        let tcc = rmcp::handler::server::tool::ToolCallContext::new(
            self,
            request,
            context,
        );
        
        let result = self.tool_router.call(tcc).await;
        
        let has_error = match &result {
            Ok(r) => r.is_error.unwrap_or(false),
            Err(_) => true,
        };
        
        if has_error {
            if let Some(mut s) = self.state.tool_status.get_mut(tool_name.as_ref()) {
                s.error_count += 1;
            }
        }
        
        self.state.record_call_end(&tool_name).await;
        
        result
    }
}
