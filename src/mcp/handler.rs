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
use tokio::sync::broadcast::error::RecvError;
use tracing::{debug, info};

#[derive(Clone)]
pub struct McpHandler {
    state: Arc<ServerState>,
    tool_router: ToolRouter<Self>,
    tool_list_changed_abort: Arc<std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
    webui_url: String,
}

impl McpHandler {
    pub fn new(state: Arc<ServerState>, config: &AppConfig) -> Self {
        let webui_url = format!("http://{}:{}", config.webui_host, config.webui_port);
        Self {
            state,
            tool_router: Self::tool_router(),
            tool_list_changed_abort: Arc::new(std::sync::Mutex::new(None)),
            webui_url,
        }
    }

    /// Get current working directory from state config
    async fn get_working_dir(&self) -> std::path::PathBuf {
        self.state.config.read().await.working_dir.clone()
    }
}

impl Drop for McpHandler {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.tool_list_changed_abort.lock() {
            if let Some(handle) = guard.take() {
                handle.abort();
            }
        }
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
    #[tool(name = "Glob", description = "List directory contents with enhanced filtering (max depth 10). Supports multi-pattern glob/regex matching, exclude patterns, file type/size/time filters, sort order control, and symlink following. Returns text file char_count and line_count for UTF-8 files. Not restricted to working directory.")]
    async fn dir_list(
        &self,
        params: Parameters<glob::GlobParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        tool_result(glob::dir_list(params, &working_dir).await)
    }

    #[tool(name = "Read", description = "Read a file with format auto-detection. Mode guide: 'auto' (generic text/image auto-detect); 'text' (plain text with line range/offset); 'media' (base64 image for vision models). For DOC/DOCX: 'doc_text' (markdown with headings/tables/formatting), 'doc_with_images' (markdown with images inline at positions), 'doc_images' (extracted images only). For PPT/PPTX: 'ppt_text' (slide text with tables), 'ppt_images' (slides as images; uses LibreOffice if available, falls back to native extraction of embedded images+text per slide). For PDF: 'pdf_text' (extracted text), 'pdf_images' (pages rendered to images via PDFium). For XLS/XLSX: 'text' (sheet tables). For IPYNB: 'text' (cells with outputs). Recommendation: use FileStat first to check document stats (slide/page count, image count, text length), then choose mode accordingly. Image modes return base64-encoded ImageContent. Batch mode via 'paths' parameter. Not restricted to working directory.")]
    async fn file_read(
        &self,
        params: Parameters<read::ReadParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        tool_result(read::file_read(params, &working_dir).await)
    }

    #[tool(name = "Grep", description = "Search pattern in files with enhanced filtering (max depth 10). Supports regex, case-sensitive, whole-word, multiline modes. Searches office documents (DOCX/PPTX/XLSX/PDF/IPYNB) text content. File filtering via include/exclude glob patterns. Not restricted to working directory.")]
    async fn file_search(
        &self,
        params: Parameters<grep::GrepParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        tool_result(grep::file_search(params, &working_dir).await)
    }

    #[tool(name = "Edit", description = "Edit files concurrently. Text modes: string_replace, line_replace, insert, delete, patch. Office: office_insert, office_replace, office_delete, office_insert_image, office_format, office_insert_table. PDF: pdf_delete_page, pdf_insert_image, pdf_insert_text, pdf_replace_text. Can create new files.")]
    async fn file_edit(
        &self,
        params: Parameters<edit::EditParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        tool_result(edit::file_edit(params, &working_dir).await)
    }

    #[tool(name = "Write", description = "Write content to files concurrently (create/append/overwrite). Supports office documents: DOCX (docx_paragraphs or office_markdown), XLSX (xlsx_sheets or office_csv), PPTX (pptx_slides), PDF (office_markdown via LibreOffice), IPYNB (ipynb_cells).")]
    async fn file_write(
        &self,
        params: Parameters<write::WriteParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        tool_result(write::file_write(params, &working_dir).await)
    }

    #[tool(name = "FileOps", description = "Copy, move, delete, or rename one or more files concurrently. Supports dry_run preview and conflict_resolution (skip/overwrite/rename). Accepts a list of operations.")]
    async fn file_ops(
        &self,
        params: Parameters<file_ops::FileOpsParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        tool_result(file_ops::file_ops(params, &working_dir).await)
    }

    #[tool(name = "FileStat", description = "Get metadata for one or more files or directories concurrently. Use mode=\"exist\" for lightweight existence check. For regular files: returns is_text, char_count, line_count, encoding (UTF-8), size, permissions, timestamps. For office documents (DOCX/PPTX/PDF/XLSX): additionally returns document_stats with document_type, page/slide/sheet count, embedded image count, and text character count to help decide whether to use text or image reading mode. Use FileStat before Read to choose the optimal mode (e.g., if PDF has many images, use pdf_images; if PPTX has many slides but few images, use ppt_text). Not restricted to working directory.")]
    async fn file_stat(
        &self,
        params: Parameters<file_stat::FileStatParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        tool_result(file_stat::file_stat(params, &working_dir).await)
    }

    #[tool(name = "Git", description = "Run git commands (status, diff, log, branch, show). Supports path filtering and max_count for log. Not restricted to working directory.")]
    async fn git_ops(
        &self,
        params: Parameters<git_ops::GitOpsParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        tool_result(git_ops::git_ops(params, &working_dir).await)
    }

    #[tool(name = "Bash", description = "Execute shell command with optional working_dir, stdin, max_output_chars, and async_mode. Use Monitor tool for async commands.")]
    async fn execute_command(
        &self,
        params: Parameters<bash::ExecuteCommandParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        tool_result(bash::execute_command(params, &working_dir, self.state.clone(), context).await)
    }

    #[tool(name = "SystemInfo", description = "Get system information including processes. Use 'sections' to select sections: system, cpu, memory, disks, network, temperature, processes.")]
    async fn system_info(
        &self,
        params: Parameters<system_info::SystemInfoParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(system_info::system_info(params).await)
    }

    #[tool(name = "ExecutePython", description = "Execute Python code for calculations, data processing, and logic evaluation. Set __result for return value. All Python standard library modules are available.")]
    async fn execute_python(
        &self,
        params: Parameters<execute_python::ExecutePythonParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        let allow_fs = self.state.is_python_fs_access_enabled().await;
        tool_result(execute_python::execute_python(params, &working_dir, allow_fs).await)
    }

    #[tool(name = "Clipboard", description = "Read or write system clipboard content. Supports read_text, write_text, read_image, and clear operations. Optional format parameter (text/html/rtf). Cross-platform.")]
    async fn clipboard(
        &self,
        params: Parameters<clipboard::ClipboardParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(clipboard::clipboard(params).await)
    }

    #[tool(name = "Archive", description = "Create, extract, list, or append ZIP archives. Supports deflate and zstd compression plus AES-256 password encryption. Restricted to working directory.")]
    async fn archive(
        &self,
        params: Parameters<archive::ArchiveParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        tool_result(archive::archive(params, &working_dir).await)
    }

    #[tool(name = "Diff", description = "Compare text, files, or directories. Output formats: unified, side_by_side, summary, inline. Supports git_diff_file, ignore_blank_lines, and configurable context_lines. Compares against HEAD.")]
    async fn diff(
        &self,
        params: Parameters<diff::DiffParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        tool_result(diff::diff(params, &working_dir).await)
    }

    #[tool(name = "NoteStorage", description = "The AI assistant's short-term memory scratchpad. Creates, lists, reads, updates, deletes, searches, and appends notes. Supports export to JSON and import from JSON. Notes are stored only in memory and auto-expire after 30 minutes.")]
    async fn note_storage(
        &self,
        params: Parameters<note_storage::NoteStorageParams>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.clone();
        tool_result(note_storage::note_storage(params, state).await)
    }

    #[tool(name = "Task", description = "Task management with CRUD operations. Use 'operation' parameter: create, list, get, update, delete.")]
    async fn task(
        &self,
        params: Parameters<task::TaskParams>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.state.clone();
        tool_result(task::task(params, state).await)
    }

    #[tool(name = "WebSearch", description = "Search the web via DuckDuckGo with optional region/language filters. Returns results with titles, URLs, and snippets.")]
    async fn web_search(
        &self,
        params: Parameters<web_search::WebSearchParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(web_search::web_search(params).await)
    }

    #[tool(name = "AskUser", description = "Ask the user a question with optional timeout and default_value. Supports multi-choice options via MCP elicitation.")]
    async fn ask_user(
        &self,
        params: Parameters<ask_user::AskUserParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        ask_user::ask_user(params, context).await
    }

    #[tool(name = "WebFetch", description = "Fetch content from a URL with extract_mode: text (strips HTML), html (raw), or markdown.")]
    async fn web_fetch(
        &self,
        params: Parameters<web_fetch::WebFetchParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(web_fetch::web_fetch(params).await)
    }

    #[tool(name = "NotebookEdit", description = "Read, write, and edit Jupyter .ipynb notebook files. Operations: read, write, add_cell, edit_cell, delete_cell.")]
    async fn notebook_edit(
        &self,
        params: Parameters<notebook_edit::NotebookEditParams>,
    ) -> Result<CallToolResult, McpError> {
        let working_dir = self.get_working_dir().await;
        tool_result(notebook_edit::notebook_edit(params, &working_dir).await)
    }

    #[tool(name = "Monitor", description = "Monitor long-running Bash commands started with async=true. Operations: stream, wait, signal.")]
    async fn monitor(
        &self,
        params: Parameters<monitor::MonitorParams>,
    ) -> Result<CallToolResult, McpError> {
        tool_result(monitor::monitor(params).await)
    }
}

impl ServerHandler for McpHandler {
    fn get_tool(&self, name: &str) -> Option<Tool> {
        self.tool_router.get(name).cloned()
    }

    fn get_info(&self) -> ServerInfo {
        info!("Getting server info for MCP initialization");
        let webui_url = &self.webui_url;
        let mut instructions = format!(
            "A comprehensive MCP server with file operations, calculations, HTTP requests, and system tools. \
            Supports resources (file://) and prompts. \
            Use WebUI at {} to manage tool settings.",
            webui_url
        );
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
        
        // Abort any existing listener task to prevent task leaks on reconnection
        {
            let mut guard = self.tool_list_changed_abort.lock().expect("tool_list_changed_abort lock poisoned");
            if let Some(handle) = guard.take() {
                handle.abort();
            }
        }
        
        let mut rx = self.state.tool_list_changed_tx.subscribe();
        let peer = context.peer.clone();
        let handle = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(_) => { let _ = peer.notify_tool_list_changed().await; }
                    Err(RecvError::Lagged(_)) => { let _ = peer.notify_tool_list_changed().await; }
                    Err(RecvError::Closed) => break,
                }
            }
        });
        
        {
            let mut guard = self.tool_list_changed_abort.lock().expect("tool_list_changed_abort lock poisoned");
            *guard = Some(handle);
        }
        
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

        let tracing_level = match level_str {
            "trace" => tracing::Level::TRACE,
            "debug" => tracing::Level::DEBUG,
            "info" => tracing::Level::INFO,
            "warn" => tracing::Level::WARN,
            "error" => tracing::Level::ERROR,
            _ => tracing::Level::INFO,
        };

        let new_filter = tracing_subscriber::EnvFilter::builder()
            .with_default_directive(tracing_level.into())
            .from_env_lossy()
            .add_directive("hyper=warn".parse().unwrap())
            .add_directive("reqwest=warn".parse().unwrap())
            .add_directive("lopdf=error".parse().unwrap());

        if let Ok(guard) = self.state.log_reload_handle.read() {
            if let Some(handle) = guard.as_ref() {
                if let Err(e) = handle.reload(new_filter) {
                    return Err(McpError::internal_error(format!("Failed to reload log filter: {}", e), None));
                }
            }
        }

        info!("Setting log level to {:?} ({})", level, level_str);
        Ok(())
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        let working_dir = self.get_working_dir().await;
        let uri = "file:///".to_string();
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
            let mut entries = tokio::fs::read_dir(&working_dir)
                .await
                .map_err(|e| McpError::internal_error(format!("Failed to read directory: {}", e), None))?;
            
            let mut content = String::new();
            while let Ok(Some(entry)) = entries.next_entry().await {
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false);
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
            let mut entries = tokio::fs::read_dir(&file_path)
                .await
                .map_err(|e| McpError::internal_error(format!("Failed to read directory: {}", e), None))?;
            
            let mut content = String::new();
            while let Ok(Some(entry)) = entries.next_entry().await {
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false);
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
        
        let _permit = self.state.concurrency_limiter
            .acquire()
            .await
            .map_err(|_| McpError::internal_error("Failed to acquire concurrency permit", None))?;

        if !self.state.is_tool_enabled(&tool_name).await {
            info!("Tool '{}' is disabled, rejecting call", tool_name);
            return Err(McpError::invalid_params(
                format!("Tool '{}' is disabled", tool_name),
                None,
            ));
        }

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
