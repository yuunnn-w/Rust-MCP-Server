use crate::mcp::state::ServerState;

use axum::{
    extract::{Path, State},
    response::sse::{Event, Sse},
    Json,
};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::broadcast;

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
        // Get the actual status from the state to check is_busy
        let is_busy = if let Some(s) = state.tool_status.get(&status.name) {
            let status_ref = s.value();
            // Clone to avoid holding the lock
            let last_end = *status_ref.last_call_end.read().await;
            let is_calling = status_ref.is_calling;
            
            if is_calling {
                true
            } else if let Some(end_time) = last_end {
                Instant::now().duration_since(end_time) < std::time::Duration::from_secs(5)
            } else {
                false
            }
        } else {
            status.is_calling
        };
        
        tools.push(ToolStatusResponse {
            name: status.name.clone(),
            description: status.description.clone(),
            enabled: status.enabled,
            call_count: status.call_count,
            is_calling: status.is_calling,
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
) -> Result<Json<ToolStatsResponse>, String> {
    let status = state
        .get_tool_status(&name)
        .ok_or_else(|| format!("Tool '{}' not found", name))?;

    let recent_calls_15min = status.get_recent_calls_count(15).await;
    let stats_history = status.get_stats(120, 5).await; // 120 minutes, 5 minute intervals
    let recent_call_times = status.get_recent_call_times(10).await;

    Ok(Json(ToolStatsResponse {
        name: status.name,
        total_calls: status.call_count,
        recent_calls_15min,
        stats_history,
        recent_call_times,
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
        .await
        .map_err(|e| e)?;

    Ok(Json(serde_json::json!({
        "success": true,
        "tool": name,
        "enabled": request.enabled
    })))
}

/// Get current configuration
pub async fn get_config(State(state): State<Arc<ServerState>>) -> Json<ConfigResponse> {
    Json(ConfigResponse {
        webui_host: state.config.webui_host.clone(),
        webui_port: state.config.webui_port,
        mcp_transport: state.config.mcp_transport.clone(),
        mcp_host: state.config.mcp_host.clone(),
        mcp_port: state.config.mcp_port,
        max_concurrency: state.config.max_concurrency,
        working_dir: state.config.working_dir.to_string_lossy().to_string(),
        log_level: state.config.log_level.clone(),
    })
}

/// Update configuration
pub async fn update_config(
    State(state): State<Arc<ServerState>>,
    Json(request): Json<UpdateConfigRequest>,
) -> Result<Json<serde_json::Value>, String> {
    let mut changes = Vec::new();
    let mut restart_required = false;

    // Validate and update max_concurrency
    if let Some(max_concurrency) = request.max_concurrency {
        if max_concurrency == 0 || max_concurrency > 1000 {
            return Err("max_concurrency must be between 1 and 1000".to_string());
        }
        state.set_max_concurrency(max_concurrency).await;
        changes.push(format!("max_concurrency: {}", max_concurrency));
    }

    // Transport change requires restart
    if let Some(mcp_transport) = request.mcp_transport {
        if matches!(mcp_transport.as_str(), "http" | "sse") {
            changes.push(format!("mcp_transport: {}", mcp_transport));
            restart_required = true;
        } else {
            return Err("mcp_transport must be one of: http, sse".to_string());
        }
    }

    // These changes require restart
    if let Some(mcp_host) = request.mcp_host {
        if !mcp_host.is_empty() {
            // Note: In a real implementation, you'd persist this to config file
            // For now, we just track the change
            changes.push(format!("mcp_host: {}", mcp_host));
            restart_required = true;
        }
    }

    if let Some(mcp_port) = request.mcp_port {
        if mcp_port > 0 {
            changes.push(format!("mcp_port: {}", mcp_port));
            restart_required = true;
        }
    }

    if let Some(webui_host) = request.webui_host {
        if !webui_host.is_empty() {
            changes.push(format!("webui_host: {}", webui_host));
            restart_required = true;
        }
    }

    if let Some(webui_port) = request.webui_port {
        if webui_port > 0 {
            changes.push(format!("webui_port: {}", webui_port));
            restart_required = true;
        }
    }

    if let Some(log_level) = request.log_level {
        if matches!(log_level.as_str(), "trace" | "debug" | "info" | "warn" | "error") {
            changes.push(format!("log_level: {}", log_level));
            restart_required = true;
        } else {
            return Err("log_level must be one of: trace, debug, info, warn, error".to_string());
        }
    }

    if let Some(working_dir) = request.working_dir {
        if !working_dir.is_empty() {
            changes.push(format!("working_dir: {}", working_dir));
            restart_required = true;
        }
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

/// Start MCP service
pub async fn start_mcp(State(state): State<Arc<ServerState>>) -> Json<serde_json::Value> {
    state.set_mcp_running(true).await;
    Json(serde_json::json!({
        "success": true,
        "message": "MCP service started"
    }))
}

/// Stop MCP service
pub async fn stop_mcp(State(state): State<Arc<ServerState>>) -> Json<serde_json::Value> {
    state.set_mcp_running(false).await;
    Json(serde_json::json!({
        "success": true,
        "message": "MCP service stopped"
    }))
}

/// Restart MCP service
pub async fn restart_mcp(State(state): State<Arc<ServerState>>) -> Json<serde_json::Value> {
    state.set_mcp_running(false).await;
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    state.set_mcp_running(true).await;
    
    Json(serde_json::json!({
        "success": true,
        "message": "MCP service restarted"
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
        "dir_list" => "Usage: Provide a 'path' parameter to list directory contents.\nExample: {\"path\": \"/home/user\"}".to_string(),
        "file_read" => "Usage: Read text file content with line range.\nParameters: 'path', optional 'start_line' (default: 0), optional 'end_line' (default: 100)\nNote: Content is limited to 10KB. If exceeded, last line will be truncated.\nReturns: File content, total line count, and hints for remaining content.\nExample: {\"path\": \"/home/user/file.txt\", \"start_line\": 0, \"end_line\": 100}".to_string(),
        "file_search" => "Usage: Search for keyword in file or directory.\nParameters: 'path' (file or directory), 'keyword'\nNote: Only searches text files (UTF-8). Binary files are skipped.\nFor directories: searches recursively up to depth 3.\nReturns: Matching file paths with line numbers, search statistics, and skipped directories if any.\nExample: {\"path\": \"/home/user/src\", \"keyword\": \"TODO\"}".to_string(),
        "file_write" => "Usage: Write content to a file.\nParameters: 'path', 'content', 'mode' (new/append/overwrite)\nExample: {\"path\": \"test.txt\", \"content\": \"Hello\", \"mode\": \"new\"}".to_string(),
        "file_copy" => "Usage: Copy a file.\nParameters: 'source', 'destination'\nExample: {\"source\": \"file1.txt\", \"destination\": \"file2.txt\"}".to_string(),
        "file_move" => "Usage: Move a file.\nParameters: 'source', 'destination'\nExample: {\"source\": \"old.txt\", \"destination\": \"new.txt\"}".to_string(),
        "file_delete" => "Usage: Delete a file.\nParameter: 'path'\nExample: {\"path\": \"file.txt\"}".to_string(),
        "file_rename" => "Usage: Rename a file.\nParameters: 'path', 'new_name'\nExample: {\"path\": \"old.txt\", \"new_name\": \"new.txt\"}".to_string(),
        "calculator" => "Usage: Calculate mathematical expressions.\nParameter: 'expression'\nSupports: +, -, *, /, ^, sqrt, sin, cos, tan, log, ln, abs, pi, e\nExample: {\"expression\": \"2 + 3 * 4\"}".to_string(),
        "http_request" => "Usage: Make HTTP requests.\nParameters: 'url', 'method' (GET/POST), optional 'headers', 'body'\nExample: {\"url\": \"https://api.example.com\", \"method\": \"GET\"}".to_string(),
        "datetime" => "Usage: Get current date and time.\nNo parameters required.\nExample: {}".to_string(),
        "image_read" => "Usage: Read an image file and return base64 encoded data.\nParameter: 'path'\nExample: {\"path\": \"image.png\"}".to_string(),
        "execute_command" => "Usage: Execute a shell command (disabled by default).\nParameters: 'command', 'working_dir', optional 'timeout'\nExample: {\"command\": \"ls -la\", \"working_dir\": \"/home/user\"}".to_string(),
        "process_list" => "Usage: List system processes.\nNo parameters required.\nExample: {}".to_string(),
        "base64_encode" => "Usage: Encode string to base64.\nParameter: 'input'\nExample: {\"input\": \"Hello, World!\"}".to_string(),
        "base64_decode" => "Usage: Decode base64 string.\nParameter: 'input'\nExample: {\"input\": \"SGVsbG8sIFdvcmxkIQ==\"}".to_string(),
        "hash_compute" => "Usage: Compute hash of string or file.\nParameters: 'input', 'algorithm' (MD5/SHA1/SHA256)\nFor files, prefix path with 'file:'\nExample: {\"input\": \"hello\", \"algorithm\": \"SHA256\"}".to_string(),
        "system_info" => "Usage: Get system information.\nNo parameters required.\nExample: {}".to_string(),
        _ => "No usage information available.".to_string(),
    }
}
