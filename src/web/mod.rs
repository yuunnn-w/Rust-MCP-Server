pub mod handlers;

use crate::mcp::state::ServerState;
use axum::{
    body::Body,
    http::{header, StatusCode, Uri},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Router,
};
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
        .route("/api/config", get(handlers::get_config).put(handlers::update_config))
        .route("/api/mcp/start", post(handlers::start_mcp))
        .route("/api/mcp/stop", post(handlers::stop_mcp))
        .route("/api/mcp/restart", post(handlers::restart_mcp))
        .route("/api/events", get(handlers::sse_handler))
        .route("/api/search", get(handlers::search_tools))
        // Static files
        .route("/", get(index_handler))
        .route("/{*path}", get(static_handler))
        .with_state(state)
}

/// Handle index route
async fn index_handler() -> impl IntoResponse {
    match StaticFiles::get("index.html") {
        Some(content) => Html(content.data).into_response(),
        None => (StatusCode::NOT_FOUND, "Not Found").into_response(),
    }
}

/// Handle static files
async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    
    // Default to index.html for root
    let path = if path.is_empty() { "index.html" } else { path };
    
    match StaticFiles::get(path) {
        Some(content) => {
            let mime_type = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .header(header::CONTENT_TYPE, mime_type.as_ref())
                .body(Body::from(content.data))
                .unwrap()
        }
        None => {
            // Try to serve index.html for SPA routing
            if let Some(content) = StaticFiles::get("index.html") {
                Html(content.data).into_response()
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
