use crate::mcp::state::ServerState;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, sse::{Event, Sse}},
    Json,
};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Instant;
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
}

/// Get all tools status
pub async fn get_tools(State(state): State<Arc<ServerState>>) -> Json<ToolsResponse> {
    let tool_statuses = state.get_all_tool_statuses();
    let mut tools = Vec::new();
    
    for status in tool_statuses {
        // Read is_calling and is_busy consistently from the same source
        let (is_calling, is_busy) = if let Some(s) = state.tool_status.get(&status.name) {
            let status_ref = s.value();
            let last_end = *status_ref.last_call_end.read().await;
            let calling = status_ref.is_calling;
            
            let busy = if calling {
                true
            } else if let Some(end_time) = last_end {
                Instant::now().duration_since(end_time) < std::time::Duration::from_secs(5)
            } else {
                false
            };
            (calling, busy)
        } else {
            (status.is_calling, status.is_calling)
        };
        
        tools.push(ToolStatusResponse {
            name: status.name.clone(),
            description: status.description.clone(),
            enabled: status.enabled,
            call_count: status.call_count,
            is_calling,
            is_busy,
            is_dangerous: status.is_dangerous,
        });
    }

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
        .ok_or_else(|| ApiError::NotFound(format!("Tool '{}' not found", name)))?;

    let recent_calls_15min = status.get_recent_calls_count(15).await;
    let stats_history = status.get_stats(120, 5).await; // 120 minutes, 5 minute intervals
    let recent_call_times = status.get_recent_call_times(10).await;

    let avg_duration_ms = {
        let durations = status.call_durations.read().await;
        if durations.is_empty() {
            0.0
        } else {
            durations.iter().sum::<u64>() as f64 / durations.len() as f64
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
) -> Result<Json<serde_json::Value>, String> {
    state
        .set_tool_enabled(&name, request.enabled)
        .await?;

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
    Json(ConfigResponse {
        webui_host: config.webui_host.clone(),
        webui_port: config.webui_port,
        mcp_transport: config.mcp_transport.clone(),
        mcp_host: config.mcp_host.clone(),
        mcp_port: config.mcp_port,
        max_concurrency: config.max_concurrency,
        working_dir: config.working_dir.to_string_lossy().to_string(),
        log_level: config.log_level.clone(),
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

    if let Some(working_dir) = request.working_dir {
        if !working_dir.is_empty() {
            config.working_dir = std::path::PathBuf::from(&working_dir);
            changes.push(format!("working_dir: {}", working_dir));
            restart_required = true;
        }
    }

    drop(config); // Release lock before side effects

    // Apply side effects outside the lock
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
                    let json = serde_json::to_string(&update).unwrap_or_default();
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
) -> Result<Json<ToolDetailResponse>, String> {
    let status = state
        .get_tool_status(&name)
        .ok_or_else(|| format!("Tool '{}' not found", name))?;

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
        "dir_list" => r#"Usage: List directory contents with filtering and brief mode (max depth 5). Returns char_count and line_count for UTF-8 text files. Not restricted to working directory.
Parameters: path, optional max_depth (default: 2, max: 5), optional include_hidden, optional pattern (glob e.g. *.rs), optional brief (default: true), optional sort_by (name/type/size/modified), optional flatten (default: false)
Example: {"path": "/home/user", "pattern": "*.rs", "brief": true}"#.to_string(),
        "file_read" => r#"Usage: Read one or more text files concurrently with line numbers and range support. Not restricted to working directory.
Parameters: files (list of file items), each with path, optional start_line (default: 0), optional end_line (default: 500), optional offset_chars, optional max_chars (default: 15000), optional line_numbers (default: true), optional highlight_line (1-based)
Example: {"files": [{"path": "a.txt", "start_line": 0, "end_line": 100}, {"path": "b.txt", "start_line": 0, "end_line": 50}]}"#.to_string(),
        "file_search" => r#"Usage: Search for keyword and return matching content fragments with context (max depth 5). Not restricted to working directory.
Parameters: path, keyword, optional file_pattern (glob), optional use_regex (default: false), optional max_results (default: 20), optional context_lines (default: 3), optional brief (default: false), optional output_format (detailed/compact/location, default: detailed)
Example: {"path": "/home/user/src", "keyword": "TODO", "context_lines": 3}"#.to_string(),
        "file_edit" => r#"Usage: Edit one or more files concurrently using string_replace, line_replace, insert, delete, or patch mode. string_replace, line_replace, and insert can create new files if they do not exist.
Parameters: operations (list of operations), each with path, mode, and mode-specific args.
string_replace: path, old_string, new_string, optional occurrence (1=first default, 0=all). Creates new file if not exists and new_string is provided.
line_replace: path, start_line, end_line, new_string. Creates new file if not exists and new_string is provided.
insert: path, start_line, new_string. Creates new file if not exists and new_string is provided.
delete: path, start_line, end_line
patch: path, patch (unified diff string)
Example: {"operations": [{"path": "main.rs", "mode": "string_replace", "old_string": "fn old()", "new_string": "fn new()"}, {"path": "new.rs", "mode": "insert", "new_string": "fn main() {}"}]}"#.to_string(),
        "file_write" => r#"Usage: Write content to one or more files concurrently.
Parameters: files (list of file items), each with path, content, optional mode (new/append/overwrite, default: new)
Example: {"files": [{"path": "test.txt", "content": "Hello", "mode": "new"}, {"path": "log.txt", "content": "Line", "mode": "append"}]}"#.to_string(),
        "file_ops" => r#"Usage: Copy, move, delete, or rename one or more files concurrently.
Parameters: operations (list of operations), each with action (copy/move/delete/rename), source, optional target, optional overwrite (default: false)
Example: {"operations": [{"action": "copy", "source": "a.txt", "target": "b.txt"}, {"action": "delete", "source": "file.txt"}]}"#.to_string(),
        "file_stat" => r#"Usage: Get metadata for one or more files or directories concurrently. Not restricted to working directory.
Parameters: paths (list of paths)
Returns: name, size, file_type, readable, writable, modified/created/accessed. For UTF-8 text files, also includes is_text, char_count, line_count, encoding
Example: {"paths": ["src/main.rs", "Cargo.toml"]}"#.to_string(),
        "path_exists" => r#"Usage: Check if a path exists and get its type. Not restricted to working directory.
Parameters: path
Returns: exists (bool), path_type (file/dir/symlink/none)
Example: {"path": "src/main.rs"}"#.to_string(),
        "json_query" => r#"Usage: Query a JSON file using JSON Pointer syntax. Not restricted to working directory.
Parameters: path, query (JSON Pointer like /data/0/name), optional max_chars (default: 15000)
Example: {"path": "config.json", "query": "/database/host"}"#.to_string(),
        "git_ops" => r#"Usage: Run git commands in a repository. Not restricted to working directory.
Parameters: action (status/diff/log/branch/show), optional repo_path (default: working_dir), optional options (array of extra args)
Example: {"action": "status"} | {"action": "log", "options": ["--oneline", "-n", "10"]}"#.to_string(),
        "calculator" => r#"Usage: Calculate mathematical expressions.
Parameter: expression
Supports: +, -, *, /, ^, sqrt, sin, cos, tan, log, ln, abs, pi, e
Example: {"expression": "2 + 3 * 4"}"#.to_string(),
        "http_request" => r#"Usage: Make HTTP requests with optional JSON extraction and response limiting.
Parameters: url, method (GET/POST), optional headers, body, optional extract_json_path (e.g. /data/0/name), optional include_response_headers (default: false), optional max_response_chars (default: 15000)
Example: {"url": "https://api.example.com", "method": "GET"}"#.to_string(),
        "datetime" => r#"Usage: Get current date and time.
No parameters required.
Example: {}"#.to_string(),
        "image_read" => r#"Usage: Read an image file and return base64 data or metadata only. Not restricted to working directory.
Parameters: path, optional mode (full/metadata, default: full)
Example: {"path": "image.png", "mode": "metadata"}"#.to_string(),
        "execute_command" => r#"Usage: Execute a shell command (disabled by default).
Parameters: command, optional cwd, optional timeout, optional shell (cmd/powershell/pwsh on Windows; sh/bash/zsh on Unix)
Example: {"command": "ls -la", "cwd": "/home/user"}"#.to_string(),
        "process_list" => r#"Usage: List system processes.
No parameters required.
Example: {}"#.to_string(),
        "base64_codec" => r#"Usage: Encode or decode base64 strings.
Parameters: operation (encode/decode), input
Example: {"operation": "encode", "input": "Hello, World!"}"#.to_string(),
        "hash_compute" => r#"Usage: Compute hash of string or file. Not restricted to working directory.
Parameters: input, algorithm (MD5/SHA1/SHA256)
For files, prefix path with file:
Example: {"input": "hello", "algorithm": "SHA256"}"#.to_string(),
        "system_info" => r#"Usage: Get system information.
No parameters required.
Example: {}"#.to_string(),
        "env_get" => r#"Usage: Get the value of an environment variable.
Parameters: name
Example: {"name": "PATH"}"#.to_string(),
        "execute_python" => r#"Usage: Execute Python code with filesystem access (dangerous).
Parameters: code (Python code), optional timeout_ms (default: 5000, max: 30000)
Set the variable __result to return a value. If not set, the last line is automatically evaluated as an expression.
The global variable __working_dir contains the server working directory.
Example: {"code": "import math\n__result = math.pi * 2"}"#.to_string(),
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
    let metrics = state.collect_metrics();
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
