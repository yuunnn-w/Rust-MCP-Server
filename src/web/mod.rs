pub mod handlers;

use crate::mcp::state::ServerState;
use axum::{
    body::Body,
    http::{header, HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use md5::{Digest, Md5};
use rust_embed::RustEmbed;
use std::sync::Arc;
use tracing::info;

#[derive(RustEmbed)]
#[folder = "src/web/static"]
struct StaticFiles;

/// Create the Axum router
pub fn create_router(state: Arc<ServerState>) -> Router {
    Router::new()
        // API routes
        .route("/api/tools", get(handlers::get_tools))
        .route("/api/status", get(handlers::get_status))
        .route("/api/server-status", get(handlers::get_server_status))
        .route("/api/tool/{name}/stats", get(handlers::get_tool_stats))
        .route("/api/tool/{name}/detail", get(handlers::get_tool_detail))
        .route("/api/tool/{name}/enable", post(handlers::enable_tool))
        .route("/api/tools/batch-enable", post(handlers::batch_enable_tools))
        .route("/api/tool-presets", get(handlers::get_tool_presets))
        .route("/api/tool-presets/current", get(handlers::get_current_preset))
        .route("/api/tool-presets/apply/{name}", post(handlers::apply_tool_preset))
        .route("/api/python-fs-access", get(handlers::get_python_fs_access).post(handlers::set_python_fs_access))
        .route("/api/config", get(handlers::get_config).put(handlers::update_config))
        .route("/api/mcp/start", post(handlers::start_mcp))
        .route("/api/mcp/stop", post(handlers::stop_mcp))
        .route("/api/mcp/restart", post(handlers::restart_mcp))
        .route("/api/events", get(handlers::sse_handler))
        .route("/api/search", get(handlers::search_tools))
        .route("/api/system-metrics", get(handlers::get_system_metrics))
        .route("/api/version", get(handlers::get_version))
        // Static files
        .route("/", get(index_handler))
        .route("/{*path}", get(static_handler))
        .with_state(state)
}

/// Generate an ETag for the given file data
fn generate_etag(data: &[u8]) -> String {
    let mut hasher = Md5::new();
    hasher.update(data);
    format!("\"{:x}\"", hasher.finalize())
}

/// Build a response for a static file with proper caching headers
fn build_static_response(data: &[u8], path: &str, headers: &HeaderMap) -> Response {
    let mime_type = mime_guess::from_path(path).first_or_octet_stream();
    let etag = generate_etag(data);

    // Check If-None-Match for 304 Not Modified
    if let Some(if_none_match) = headers.get(header::IF_NONE_MATCH) {
        if if_none_match.as_bytes() == etag.as_bytes() {
            return Response::builder()
                .status(StatusCode::NOT_MODIFIED)
                .header(header::ETAG, etag)
                .body(Body::empty())
                .unwrap_or_else(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response());
        }
    }

    let cache_control = if path == "index.html" {
        "no-cache"
    } else {
        "public, max-age=86400"
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime_type.as_ref())
        .header(header::CACHE_CONTROL, cache_control)
        .header(header::ETAG, etag)
        .body(Body::from(data.to_vec()))
        .unwrap_or_else(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response())
}

/// Handle index route
async fn index_handler(headers: HeaderMap) -> impl IntoResponse {
    match StaticFiles::get("index.html") {
        Some(content) => build_static_response(&content.data, "index.html", &headers),
        None => (StatusCode::NOT_FOUND, "Not Found").into_response(),
    }
}

/// Handle static files
async fn static_handler(uri: Uri, headers: HeaderMap) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    
    // Default to index.html for root
    let path = if path.is_empty() { "index.html" } else { path };
    
    match StaticFiles::get(path) {
        Some(content) => build_static_response(&content.data, path, &headers),
        None => {
            // Don't serve index.html for unknown API routes — return proper 404
            if path.starts_with("api/") {
                return (StatusCode::NOT_FOUND, "Not Found").into_response();
            }
            
            // Try to serve index.html for SPA routing
            if let Some(content) = StaticFiles::get("index.html") {
                build_static_response(&content.data, "index.html", &headers)
            } else {
                (StatusCode::NOT_FOUND, "Not Found").into_response()
            }
        }
    }
}

/// Start the web server
pub async fn start_web_server(
    state: Arc<ServerState>,
    bind_addr: String,
) -> anyhow::Result<()> {
    let app = create_router(state);
    
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    info!("WebUI server listening on http://{}", bind_addr);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}
