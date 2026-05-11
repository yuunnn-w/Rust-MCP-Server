use crate::mcp::presets::get_all_presets;
use crate::mcp::state::{ServerState, StatusUpdate};
use crate::utils::system_metrics::SystemMetrics;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, sse::{Event, Sse}},
    Json,
};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Custom API error type with proper HTTP status codes
pub enum ApiError {
    NotFound(String),
    BadRequest(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
        };
        let body = Json(serde_json::json!({
            "success": false,
            "error": message
        }));
        (status, body).into_response()
    }
}

/// Tool status response
#[derive(Debug, Serialize)]
pub struct ToolStatusResponse {
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub call_count: u64,
    pub is_calling: bool,
    pub is_busy: bool,  // New field: true if calling or called within last 5 seconds
    pub is_dangerous: bool,
}

/// All tools response
#[derive(Debug, Serialize)]
pub struct ToolsResponse {
    pub tools: Vec<ToolStatusResponse>,
}

/// Server status response
#[derive(Debug, Serialize)]
pub struct ServerStatusResponse {
    pub current_calls: usize,
    pub max_concurrency: usize,
    pub mcp_running: bool,
}

/// Tool statistics response
#[derive(Debug, Serialize)]
pub struct ToolStatsResponse {
    pub name: String,
    pub total_calls: u64,
    pub recent_calls_15min: usize,
    pub stats_history: Vec<usize>,
    pub recent_call_times: Vec<String>,
    pub avg_duration_ms: f64,
    pub error_rate: f64,
}

/// Configuration response
#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    pub webui_host: String,
    pub webui_port: u16,
    pub mcp_transport: String,
    pub mcp_host: String,
    pub mcp_port: u16,
    pub max_concurrency: usize,
    pub working_dir: String,
    pub log_level: String,
    pub system_prompt: Option<String>,
}

/// Enable/disable tool request
#[derive(Debug, Deserialize)]
pub struct EnableToolRequest {
    pub enabled: bool,
}

/// Python filesystem access toggle request
#[derive(Debug, Deserialize)]
pub struct PythonFsAccessRequest {
    pub enabled: bool,
}

/// Update config request
#[derive(Debug, Deserialize)]
pub struct UpdateConfigRequest {
    pub mcp_transport: Option<String>,
    pub max_concurrency: Option<usize>,
    pub mcp_host: Option<String>,
    pub mcp_port: Option<u16>,
    pub webui_host: Option<String>,
    pub webui_port: Option<u16>,
    pub log_level: Option<String>,
    pub working_dir: Option<String>,
    pub system_prompt: Option<String>,
}

/// Batch enable/disable tools request
#[derive(Debug, Deserialize)]
pub struct BatchEnableToolsRequest {
    pub tools: Vec<String>,
    pub enabled: bool,
}

/// Preset list response item
#[derive(Debug, Serialize)]
pub struct PresetResponse {
    pub name: String,
    pub description: String,
    pub tool_count: usize,
}

/// Get all tools status
pub async fn get_tools(State(state): State<Arc<ServerState>>) -> Json<ToolsResponse> {
    let tool_statuses = state.get_all_tool_statuses().await;
    let tools = tool_statuses
        .into_iter()
        .map(|status| ToolStatusResponse {
            name: status.name,
            description: status.description,
            enabled: status.enabled,
            call_count: status.call_count,
            is_calling: status.is_calling,
            is_busy: status.is_busy,
            is_dangerous: status.is_dangerous,
        })
        .collect();

    Json(ToolsResponse { tools })
}

/// Get server status (all tools)
pub async fn get_status(State(state): State<Arc<ServerState>>) -> Json<ToolsResponse> {
    get_tools(State(state)).await
}

/// Get server runtime status (concurrency, etc.)
pub async fn get_server_status(State(state): State<Arc<ServerState>>) -> Json<ServerStatusResponse> {
    Json(ServerStatusResponse {
        current_calls: state.get_current_calls().await,
        max_concurrency: state.get_max_concurrency().await,
        mcp_running: state.is_mcp_running().await,
    })
}

/// Get specific tool statistics
pub async fn get_tool_stats(
    State(state): State<Arc<ServerState>>,
    Path(name): Path<String>,
) -> Result<Json<ToolStatsResponse>, ApiError> {
    let status = state
        .get_tool_status(&name)
        .await
        .ok_or_else(|| ApiError::NotFound(format!("Tool '{}' not found", name)))?;

    let recent_calls_15min = status.get_recent_calls_count(15).await;
    let stats_history = status.get_stats(120, 5).await; // 120 minutes, 5 minute intervals
    let recent_call_times = status.get_recent_call_times(10).await;

    let avg_duration_ms = {
        let durations = status.call_durations.read().await;
        if durations.is_empty() {
            0.0
        } else {
            let total: u64 = durations.iter().copied().fold(0u64, |acc, x| acc.saturating_add(x));
            total as f64 / durations.len() as f64
        }
    };
    let error_rate = if status.call_count == 0 {
        0.0
    } else {
        status.error_count as f64 / status.call_count as f64 * 100.0
    };

    Ok(Json(ToolStatsResponse {
        name: status.name,
        total_calls: status.call_count,
        recent_calls_15min,
        stats_history,
        recent_call_times,
        avg_duration_ms,
        error_rate,
    }))
}

/// Enable or disable a tool
pub async fn enable_tool(
    State(state): State<Arc<ServerState>>,
    Path(name): Path<String>,
    Json(request): Json<EnableToolRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .set_tool_enabled(&name, request.enabled)
        .await
        .map_err(|e| {
            if e.contains("not found") {
                ApiError::NotFound(e)
            } else {
                ApiError::BadRequest(e)
            }
        })?;

    Ok(Json(serde_json::json!({
        "success": true,
        "tool": name,
        "enabled": request.enabled
    })))
}

/// Get python filesystem access status
pub async fn get_python_fs_access(State(state): State<Arc<ServerState>>) -> Json<serde_json::Value> {
    let enabled = state.is_python_fs_access_enabled().await;
    Json(serde_json::json!({
        "success": true,
        "enabled": enabled
    }))
}

/// Enable or disable python filesystem access
pub async fn set_python_fs_access(
    State(state): State<Arc<ServerState>>,
    Json(request): Json<PythonFsAccessRequest>,
) -> Json<serde_json::Value> {
    state.set_python_fs_access_enabled(request.enabled).await;
    Json(serde_json::json!({
        "success": true,
        "enabled": request.enabled
    }))
}

/// Get current configuration
pub async fn get_config(State(state): State<Arc<ServerState>>) -> Json<ConfigResponse> {
    let config = state.config.read().await;
    let system_prompt = state.get_system_prompt_sync();
    Json(ConfigResponse {
        webui_host: config.webui_host.clone(),
        webui_port: config.webui_port,
        mcp_transport: config.mcp_transport.clone(),
        mcp_host: config.mcp_host.clone(),
        mcp_port: config.mcp_port,
        max_concurrency: config.max_concurrency,
        working_dir: config.working_dir.to_string_lossy().to_string(),
        log_level: config.log_level.clone(),
        system_prompt,
    })
}

/// Update configuration
pub async fn update_config(
    State(state): State<Arc<ServerState>>,
    Json(request): Json<UpdateConfigRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Validate all parameters before acquiring the lock
    if let Some(max_concurrency) = request.max_concurrency {
        if max_concurrency == 0 || max_concurrency > 1000 {
            return Err(ApiError::BadRequest("max_concurrency must be between 1 and 1000".to_string()));
        }
    }

    if let Some(ref mcp_transport) = request.mcp_transport {
        if !matches!(mcp_transport.as_str(), "http" | "sse") {
            return Err(ApiError::BadRequest("mcp_transport must be one of: http, sse".to_string()));
        }
    }

    if let Some(ref log_level) = request.log_level {
        if !matches!(log_level.as_str(), "trace" | "debug" | "info" | "warn" | "error") {
            return Err(ApiError::BadRequest("log_level must be one of: trace, debug, info, warn, error".to_string()));
        }
    }

    let mut changes = Vec::new();
    let mut restart_required = false;
    let mut max_concurrency_updated = None;

    // Validate working_dir outside the lock to avoid blocking I/O while holding it
    let validated_working_dir = if let Some(ref working_dir) = request.working_dir {
        if !working_dir.is_empty() {
            let path = std::path::PathBuf::from(working_dir);
            if !path.exists() {
                return Err(ApiError::BadRequest(format!("working_dir does not exist: {}", working_dir)));
            }
            if !path.is_dir() {
                return Err(ApiError::BadRequest(format!("working_dir is not a directory: {}", working_dir)));
            }
            Some(path)
        } else {
            None
        }
    } else {
        None
    };

    // Acquire write lock once for all modifications
    let mut config = state.config.write().await;

    if let Some(max_concurrency) = request.max_concurrency {
        config.max_concurrency = max_concurrency;
        changes.push(format!("max_concurrency: {}", max_concurrency));
        max_concurrency_updated = Some(max_concurrency);
    }

    if let Some(mcp_transport) = request.mcp_transport {
        config.mcp_transport = mcp_transport.clone();
        changes.push(format!("mcp_transport: {}", mcp_transport));
        restart_required = true;
    }

    if let Some(mcp_host) = request.mcp_host {
        if !mcp_host.is_empty() {
            config.mcp_host = mcp_host.clone();
            changes.push(format!("mcp_host: {}", mcp_host));
            restart_required = true;
        }
    }

    if let Some(mcp_port) = request.mcp_port {
        if mcp_port > 0 {
            config.mcp_port = mcp_port;
            changes.push(format!("mcp_port: {}", mcp_port));
            restart_required = true;
        }
    }

    if let Some(webui_host) = request.webui_host {
        if !webui_host.is_empty() {
            config.webui_host = webui_host.clone();
            changes.push(format!("webui_host: {}", webui_host));
            restart_required = true;
        }
    }

    if let Some(webui_port) = request.webui_port {
        if webui_port > 0 {
            config.webui_port = webui_port;
            changes.push(format!("webui_port: {}", webui_port));
            restart_required = true;
        }
    }

    if let Some(log_level) = request.log_level {
        config.log_level = log_level.clone();
        changes.push(format!("log_level: {}", log_level));
        restart_required = true;
    }

    if let Some(path) = validated_working_dir {
            let display = path.to_string_lossy().to_string();
            changes.push(format!("working_dir: {}", display));
            config.working_dir = path;
            restart_required = true;
    }

    let mut system_prompt_updated = None;
    if let Some(system_prompt) = request.system_prompt {
        config.system_prompt = Some(system_prompt.clone());
        system_prompt_updated = Some(system_prompt.clone());
        changes.push(format!("system_prompt: {}", system_prompt));
    }

    drop(config); // Release lock before side effects

    // Apply side effects outside the lock
    if let Some(prompt) = system_prompt_updated {
        state.set_system_prompt(Some(prompt));
    }

    if let Some(max_concurrency) = max_concurrency_updated {
        state.set_max_concurrency(max_concurrency).await;
    }

    let message = if restart_required {
        "Configuration updated. Restart server to apply all changes."
    } else {
        "Configuration updated."
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "message": message,
        "changes": changes,
        "restart_required": restart_required
    })))
}

/// Start MCP service (updates status flag only)
pub async fn start_mcp(State(state): State<Arc<ServerState>>) -> Json<serde_json::Value> {
    tracing::info!("Received request to start MCP service");
    state.set_mcp_running(true).await;
    tracing::info!("MCP service status set to running");
    Json(serde_json::json!({
        "success": true,
        "message": "MCP service status set to running. Note: full restart requires process manager."
    }))
}

/// Stop MCP service (updates status flag only)
pub async fn stop_mcp(State(state): State<Arc<ServerState>>) -> Json<serde_json::Value> {
    tracing::info!("Received request to stop MCP service");
    state.set_mcp_running(false).await;
    tracing::info!("MCP service status set to stopped");
    Json(serde_json::json!({
        "success": true,
        "message": "MCP service status set to stopped. Note: full shutdown requires process manager."
    }))
}

/// Restart MCP service signal (toggles status flag only, not a full process restart)
pub async fn restart_mcp(State(state): State<Arc<ServerState>>) -> Json<serde_json::Value> {
    tracing::info!("Received request to restart MCP service");
    state.set_mcp_running(false).await;
    tracing::info!("MCP service stopping for restart...");
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    state.set_mcp_running(true).await;
    tracing::info!("MCP service restarted successfully");
    
    Json(serde_json::json!({
        "success": true,
        "message": "MCP service status restarted. Note: for a full restart, please use your process manager."
    }))
}

/// SSE endpoint for real-time status updates
pub async fn sse_handler(
    State(state): State<Arc<ServerState>>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {

    
    let mut rx = state.subscribe_status();
    
    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(update) => {
                    let update = match update {
                        StatusUpdate::ToolCallCount { tool, count, is_calling, .. } => {
                            let is_busy = if let Some(s) = state.tool_status.get(&tool) {
                                let last_end = *s.last_call_end.read().await;
                                if is_calling {
                                    true
                                } else if let Some(end_time) = last_end {
                                    end_time.elapsed() < std::time::Duration::from_secs(5)
                                } else {
                                    false
                                }
                            } else {
                                false
                            };
                            StatusUpdate::ToolCallCount {
                                tool,
                                count,
                                is_calling,
                                is_busy,
                            }
                        }
                        other => other,
                    };
                    let json = serde_json::to_string(&update).unwrap_or_else(|e| {
                        tracing::error!("Failed to serialize StatusUpdate: {}", e);
                        String::new()
                    });
                    if json.is_empty() { continue; }
                    yield Ok(Event::default().data(json));
                }
                Err(broadcast::error::RecvError::Closed) => break,
                Err(_) => continue,
            }
        }
    };

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(30))
            .text("keep-alive"),
    )
}

/// Search tools
pub async fn search_tools(
    State(state): State<Arc<ServerState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<Vec<String>> {
    let query = params.get("q").map(|s| s.to_lowercase()).unwrap_or_default();
    
    let matching: Vec<String> = state
        .tool_status
        .iter()
        .filter(|entry| {
            let name = entry.key().to_lowercase();
            let desc = entry.value().description.to_lowercase();
            name.contains(&query) || desc.contains(&query)
        })
        .map(|entry| entry.key().clone())
        .collect();

    Json(matching)
}

/// Tool detail response with description and usage
#[derive(Debug, Serialize)]
pub struct ToolDetailResponse {
    pub name: String,
    pub description: String,
    pub usage: String,
    pub enabled: bool,
    pub is_dangerous: bool,
}

/// Get tool details with usage information
pub async fn get_tool_detail(
    State(state): State<Arc<ServerState>>,
    Path(name): Path<String>,
) -> Result<Json<ToolDetailResponse>, ApiError> {
    let status = state
        .get_tool_status(&name)
        .await
        .ok_or_else(|| ApiError::NotFound(format!("Tool '{}' not found", name)))?;

    // Generate usage information based on tool name
    let usage = generate_tool_usage(&name);

    Ok(Json(ToolDetailResponse {
        name: status.name,
        description: status.description,
        usage,
        enabled: status.enabled,
        is_dangerous: status.is_dangerous,
    }))
}

/// Generate usage information for a tool
fn generate_tool_usage(tool_name: &str) -> String {
    match tool_name {
        "Glob" => r#"Usage: List directory contents with enhanced filtering (max depth 10). Supports multi-pattern glob/regex matching, exclude patterns, file type/size/time filters, sort order control, and symlink following. Returns text file char_count and line_count for UTF-8 files. Not restricted to working directory.
Parameters: path, optional max_depth (default: 2, max: 10), optional include_hidden, optional pattern (glob e.g. *.rs), optional exclude_patterns, optional pattern_mode (glob/regex/literal), optional file_types (file/dir/symlink), optional min_size/max_size, optional sort_by (name/modified/size/file_type), optional flatten (default: false), optional follow_symlinks (default: false)
Example: {"path": "/home/user", "pattern": "*.rs"} | {"path": "/home/user", "pattern_mode": "regex", "pattern": "test_.*\\.rs"}"#.to_string(),
        "Read" => r#"Usage: Read a file with format auto-detection. Supports text files (line numbers/highlight/offset), PDF text extraction, DOCX/PPTX/XLSX/IPYNB parsing, and image metadata (dimensions/type/size). Use mode="text" to force text, "metadata" for image info, "pdf_text" for PDF text. Batch mode via paths parameter. Not restricted to working directory.
Parameters: path (single file), optional paths (list for batch), optional mode (auto/text/metadata/pdf_text), optional start_line (default: 0), optional end_line (default: 500), optional offset_chars, optional max_chars (default: 15000), optional line_numbers (default: true), optional highlight_line (1-based)
Example: {"path": "a.txt", "start_line": 0, "end_line": 100} | {"paths": ["a.txt", "b.txt"]}"#.to_string(),
        "Grep" => r#"Usage: Search pattern in files with enhanced filtering (max depth 10). Supports regex, case-sensitive, whole-word, multiline modes. Searches office documents (DOCX/PPTX/XLSX/PDF/IPYNB) text content. File filtering via include/exclude glob patterns. Not restricted to working directory.
Parameters: path, keyword, optional file_pattern (glob), optional include_patterns/exclude_patterns, optional use_regex (default: false), optional case_sensitive (default: true), optional whole_word, optional multiline, optional max_results (default: 20), optional context_lines (default: 3), optional brief (default: false), optional output_format (detailed/compact/location, default: detailed)
Example: {"path": "/home/user/src", "keyword": "TODO", "use_regex": true, "context_lines": 3}"#.to_string(),
        "Edit" => r#"Usage: Edit one or more files concurrently using string_replace, line_replace, insert, delete, or patch mode. Supports office formats (DOCX/PPTX/XLSX) via string_replace. PPTX editing may lose templates. XLSX editing may lose formulas. Can create new files with string_replace/line_replace/insert.
Parameters: operations (list of operations), each with path, mode, and mode-specific args.
string_replace: path, old_string, new_string, optional occurrence (1=first default, 0=all). Creates new file if not exists.
line_replace: path, start_line, end_line, new_string. Creates new file if not exists.
insert: path, start_line, new_string. Creates new file if not exists.
delete: path, start_line, end_line
patch: path, patch (unified diff string)
Example: {"operations": [{"path": "main.rs", "mode": "string_replace", "old_string": "fn old()", "new_string": "fn new()"}, {"path": "new.rs", "mode": "insert", "new_string": "fn main() {}"}]}"#.to_string(),
        "Write" => r#"Usage: Write content to one or more files concurrently (create/append/overwrite). Supports creating office documents: DOCX (docx_paragraphs), XLSX (xlsx_sheets), PPTX (pptx_slides), IPYNB (ipynb_cells). Accepts a list of file items.
Parameters: files (list of file items), each with path, content, optional mode (create/append/overwrite, default: create)
Example: {"files": [{"path": "test.txt", "content": "Hello", "mode": "create"}, {"path": "log.txt", "content": "Line", "mode": "append"}]}"#.to_string(),
        "FileOps" => r#"Usage: Copy, move, delete, or rename one or more files concurrently. Supports dry_run preview and conflict_resolution (skip/overwrite/rename).
Parameters: operations (list of operations), each with action (copy/move/delete/rename), source, optional target, optional overwrite (default: false), optional dry_run, optional conflict_resolution
Example: {"operations": [{"action": "copy", "source": "a.txt", "target": "b.txt"}, {"action": "delete", "source": "file.txt"}]}"#.to_string(),
        "FileStat" => r#"Usage: Get metadata for one or more files or directories concurrently. Use mode="exist" for lightweight existence check. Returns is_text, char_count, line_count, and encoding for UTF-8 files. Not restricted to working directory.
Parameters: paths (list of paths), optional mode (metadata/exist, default: metadata)
Returns (metadata mode): name, size, file_type, readable, writable, modified/created/accessed, encoding info, is_text, char_count, line_count
Returns (exist mode): path, exists (bool), file_type (file/dir/symlink/none)
Example: {"paths": ["src/main.rs"]} | {"paths": ["config.json"], "mode": "exist"}"#.to_string(),
        "Git" => r#"Usage: Run git commands (status, diff, log, branch, show). Supports path filtering and max_count for log. Not restricted to working directory.
Parameters: action (status/diff/log/branch/show), optional repo_path (default: working_dir), optional options (array of extra args), optional path (for log filtering), optional max_count
Example: {"action": "status"} | {"action": "log", "options": ["--oneline"], "max_count": 10}"#.to_string(),
        "HttpRequest" => r#"Usage: Make HTTP requests with optional JSON extraction and response limiting.
Parameters: url, method (GET/POST), optional headers, body, optional extract_json_path (e.g. /data/0/name), optional include_response_headers (default: false), optional max_response_chars (default: 15000)
Example: {"url": "https://api.example.com", "method": "GET"}"#.to_string(),
        "Bash" => r#"Usage: Execute a shell command with optional working_dir, stdin, max_output_chars, and async_mode (disabled by default). Use Monitor tool for async commands.
Parameters: command, optional cwd, optional timeout, optional shell (cmd/powershell/pwsh on Windows; sh/bash/zsh on Unix), optional shell_path (custom executable path), optional shell_arg (custom argument, e.g., -Command, /C), optional stdin, optional max_output_chars, optional async_mode (default: false)
Example: {"command": "ls -la", "cwd": "/home/user"} | {"command": "npm run build", "async_mode": true}"#.to_string(),
        "SystemInfo" => r#"Usage: Get comprehensive system information including optional process listing. Use 'sections' to select sections.
Parameters: optional sections (list: "system", "cpu", "memory", "disks", "network", "temperature", "processes")
Example: {} | {"sections": ["cpu", "memory", "processes"]}"#.to_string(),
        "ExecutePython" => r#"Usage: Execute Python code for calculations, data processing, and logic evaluation. All Python standard library modules available.
Parameters: code (Python code), optional timeout_ms (default: 5000, max: 30000)
Set the variable __result to return a value. If not set, the last line is automatically evaluated as an expression.
Example: {"code": "import math\n__result = math.pi * 2"}"#.to_string(),
        "Clipboard" => r#"Usage: Read or write system clipboard content. Supports read_text, write_text, read_image, and clear operations. Optional format (text/html/rtf). Cross-platform.
Parameters: operation (read_text/write_text/read_image/clear), optional text (required for write_text), optional format
Example: {"operation": "read_text"} | {"operation": "write_text", "text": "Hello"} | {"operation": "clear"}"#.to_string(),
        "Archive" => r#"Usage: Create, extract, list, or append ZIP archives. Supports deflate and zstd compression. Restricted to working directory.
Parameters: operation (create/extract/list/append), archive_path, optional source_paths (for create/append), optional destination (for extract), optional compression_level 1-9 (default: 6)
Example: {"operation": "create", "archive_path": "backup.zip", "source_paths": ["src", "Cargo.toml"]} | {"operation": "extract", "archive_path": "backup.zip", "destination": "./extracted"}"#.to_string(),
        "Diff" => r#"Usage: Compare text, files, or directories. Output formats: unified, side_by_side, summary, inline. Supports ignore_blank_lines, context_lines, and git HEAD comparison.
Parameters: operation (compare_text/compare_files/directory_diff/git_diff_file), optional old_text/new_text (for compare_text), optional old_path/new_path (for compare_files/directory_diff), optional file_path (for git_diff_file), optional output_format (unified/side_by_side/summary/inline, default: unified), optional context_lines (default: 3), optional ignore_whitespace (default: false), optional ignore_blank_lines, optional word_level (default: true), optional max_output_lines (default: 500)
Example: {"operation": "compare_text", "old_text": "foo\nbar", "new_text": "foo\nbaz"} | {"operation": "git_diff_file", "file_path": "src/main.rs"}"#.to_string(),
        "NoteStorage" => r#"Usage: The AI assistant's short-term memory scratchpad. Creates, lists, reads, updates, deletes, searches, and appends notes. Supports export to JSON and import from JSON. Notes auto-expire after 30 minutes of inactivity.
Parameters: operation (create/list/read/update/delete/search/append/export/import), optional id (for read/update/delete/append), optional title/content/tags/category (for create/update), optional tag_filter/category (for list), optional query (for search), optional append_content (for append), optional notes_data (for import)
Max 100 notes, max 50,000 chars per note.
Example: {"operation": "create", "title": "User prefers dark mode", "content": "...", "tags": ["preference"], "category": "user_prefs"} | {"operation": "search", "query": "preference"}"#.to_string(),
        "Task" => r#"Usage: Task management with CRUD operations. Use 'operation' parameter.
Parameters: operation (create/list/get/update/delete), optional title (max 200 chars, required for create), optional description (max 5000 chars), optional priority (low/medium/high, default: medium), optional tags (max 5, each max 50 chars), optional id (required for get/update/delete), optional status (pending/in_progress/completed), optional status_filter/priority_filter/tag_filter (for list), optional sort_by (created/priority/status)
create: requires title, returns task with id and status "pending"
list: returns { tasks: [...], total_count }
update: requires id, returns updated task
delete: requires id, returns { deleted: true, id }
Example: {"operation": "create", "title": "Implement login", "priority": "high", "tags": ["backend"]} | {"operation": "list", "status_filter": "pending"}"#.to_string(),
        "WebSearch" => r#"Usage: Search the web via DuckDuckGo with optional region/language filters. Returns results with titles, URLs, and snippets.
Parameters: query (required, max 500 chars), optional num_results (1-20, default: 10), optional region (e.g. us-en, de-de), optional language
Output: { results: [{ title, url, snippet }], query, total_results }
Example: {"query": "Rust programming language", "num_results": 5}"#.to_string(),
        "AskUser" => r#"Usage: Ask the user a question with optional timeout and default_value. Supports multi-choice options via MCP elicitation.
Parameters: question (required, max 1000 chars), optional options (list of strings, max 10), optional timeout_sec (default: 120, max: 600), optional default_value
Output: { question, response, selected_option (if options provided) }
Example: {"question": "Which file should I edit?", "options": ["src/main.rs", "src/lib.rs"]}"#.to_string(),
        "WebFetch" => r#"Usage: Fetch content from a URL with extract_mode: text (strips HTML), html (raw), or markdown.
Parameters: url (required), optional max_chars (default: 50000, max: 100000), optional extract_mode (text/html/markdown, default: text)
Output: { url, title, text_content, content_length, encoding }
Example: {"url": "https://example.com", "max_chars": 10000}"#.to_string(),
        "NotebookEdit" => r#"Usage: Read, write, and edit Jupyter .ipynb notebook files. Supports add_cell, edit_cell, delete_cell operations.
Parameters: path, operation (read/write/add_cell/edit_cell/delete_cell), optional cells (for write), optional cell_id/cell_index (for edit_cell/delete_cell), optional new_cell (for add_cell), optional content (for edit_cell), optional cell_type (code/markdown)
Example: {"path": "notebook.ipynb", "operation": "read"} | {"path": "notebook.ipynb", "operation": "add_cell", "new_cell": {"cell_type": "code", "source": "print('hello')"}}"#.to_string(),
        "Monitor" => r#"Usage: Monitor long-running Bash commands started with async=true. Operations: stream, wait, signal.
Parameters: operation (stream/wait/signal), optional shell_id (required for all operations), optional signal (SIGTERM/SIGKILL/SIGINT, for signal operation)
stream: returns real-time output chunks as they become available
wait: blocks until command completes, returns final output and exit code
signal: sends a signal to the running process
Example: {"operation": "stream", "shell_id": "abc123"} | {"operation": "signal", "shell_id": "abc123", "signal": "SIGTERM"}"#.to_string(),
        _ => "No usage information available.".to_string(),
    }
}

/// System metrics response
#[derive(Debug, Serialize)]
pub struct SystemMetricsResponse {
    pub cpu_percent: f32,
    pub memory_total: u64,
    pub memory_used: u64,
    pub memory_percent: f32,
    pub cpu_cores: usize,
    pub uptime_seconds: u64,
    pub load_average: [f64; 3],
    pub process_count: usize,
}

/// Get current system metrics
pub async fn get_system_metrics(State(state): State<Arc<ServerState>>) -> Json<SystemMetricsResponse> {
    let metrics = state.collect_metrics().await.unwrap_or_else(|e| {
        tracing::error!("Failed to collect system metrics: {}", e);
        SystemMetrics {
            cpu_percent: 0.0,
            memory_total: 0,
            memory_used: 0,
            memory_percent: 0.0,
            cpu_cores: 0,
            uptime_seconds: 0,
            load_average: [0.0; 3],
            process_count: 0,
        }
    });
    Json(SystemMetricsResponse {
        cpu_percent: metrics.cpu_percent,
        memory_total: metrics.memory_total,
        memory_used: metrics.memory_used,
        memory_percent: metrics.memory_percent,
        cpu_cores: metrics.cpu_cores,
        uptime_seconds: metrics.uptime_seconds,
        load_average: metrics.load_average,
        process_count: metrics.process_count,
    })
}

/// Version information response
#[derive(Debug, Serialize)]
pub struct VersionResponse {
    pub name: String,
    pub version: String,
    pub description: String,
    pub authors: String,
    pub repository: String,
    pub license: String,
}

/// Get server version information
pub async fn get_version() -> Json<VersionResponse> {
    Json(VersionResponse {
        name: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: env!("CARGO_PKG_DESCRIPTION").to_string(),
        authors: env!("CARGO_PKG_AUTHORS").to_string(),
        repository: env!("CARGO_PKG_REPOSITORY").to_string(),
        license: env!("CARGO_PKG_LICENSE").to_string(),
    })
}

/// Get all tool presets
pub async fn get_tool_presets() -> Json<Vec<PresetResponse>> {
    let presets = get_all_presets()
        .into_iter()
        .map(|p| PresetResponse {
            name: p.name,
            description: p.description,
            tool_count: p.tools_enabled.len(),
        })
        .collect();
    Json(presets)
}

/// Apply a tool preset
pub async fn apply_tool_preset(
    State(state): State<Arc<ServerState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.apply_preset(&name).await.map_err(|e| ApiError::NotFound(e))?;
    Ok(Json(serde_json::json!({
        "success": true,
        "preset": name
    })))
}

/// Get current active preset
pub async fn get_current_preset(State(state): State<Arc<ServerState>>) -> Json<serde_json::Value> {
    let preset = state.get_current_preset().await;
    Json(serde_json::json!({
        "success": true,
        "preset": preset
    }))
}

/// Batch enable/disable tools
pub async fn batch_enable_tools(
    State(state): State<Arc<ServerState>>,
    Json(request): Json<BatchEnableToolsRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut changed = Vec::new();
    let mut failed = Vec::new();
    for tool_name in &request.tools {
        match state.set_tool_enabled(tool_name, request.enabled).await {
            Ok(()) => changed.push(tool_name.clone()),
            Err(e) => failed.push((tool_name.clone(), e)),
        }
    }
    Ok(Json(serde_json::json!({
        "success": true,
        "enabled": request.enabled,
        "changed": changed,
        "failed": failed
    })))
}
