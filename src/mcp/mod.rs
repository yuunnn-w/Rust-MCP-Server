pub mod handler;
pub mod state;
pub mod tools;

pub use state::ServerState;

use crate::config::AppConfig;
use crate::mcp::handler::McpHandler as Handler;
use axum::http::Method;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};

/// Build Streamable HTTP server config from AppConfig
/// http mode: stateless + json response
/// sse mode: stateful + sse response
fn build_streamable_config(config: &AppConfig, ct: tokio_util::sync::CancellationToken) -> StreamableHttpServerConfig {
    let is_http_mode = config.mcp_transport == "http";
    let stateful_mode = !is_http_mode; // http mode = stateless, sse mode = stateful
    let json_response = is_http_mode;  // http mode = json, sse mode = sse
    
    info!(
        "MCP server config: transport={}, stateful_mode={}, json_response={}",
        config.mcp_transport, stateful_mode, json_response
    );
    
    StreamableHttpServerConfig {
        stateful_mode,
        json_response,
        cancellation_token: ct,
        ..Default::default()
    }
}

/// Start MCP server with HTTP transport
pub async fn start_http_server(
    state: Arc<ServerState>,
    config: AppConfig,
) -> anyhow::Result<()> {
    info!(
        "Starting MCP server with HTTP transport on {}",
        config.mcp_bind_addr()
    );

    let bind_addr: SocketAddr = config.mcp_bind_addr().parse()?;
    let ct = tokio_util::sync::CancellationToken::new();

    // Clone state for the closure
    let state_for_closure = state.clone();
    let config_for_closure = config.clone();
    
    // Create the service factory with proper config
    let service = StreamableHttpService::new(
        move || {
            let handler = Handler::new(state_for_closure.clone(), &config_for_closure);
            handler.init_tools();
            Ok(handler)
        },
        LocalSessionManager::default().into(),
        build_streamable_config(&config, ct.child_token()),
    );

    // Create CORS layer to allow cross-origin requests
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers(Any);

    // Create router with MCP endpoint
    let app = axum::Router::new()
        .nest_service("/mcp", service)
        .layer(cors);

    // Start server
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    
    state.set_mcp_running(true).await;
    info!("MCP HTTP server listening on http://{}/mcp", bind_addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            tokio::signal::ctrl_c().await.ok();
            ct.cancel();
        })
        .await?;

    state.set_mcp_running(false).await;
    info!("MCP HTTP server stopped");

    Ok(())
}

/// Start MCP server with SSE transport (HTTP with SSE support)
pub async fn start_sse_server(
    state: Arc<ServerState>,
    config: AppConfig,
) -> anyhow::Result<()> {
    info!(
        "Starting MCP server with SSE transport on {}",
        config.mcp_bind_addr()
    );

    let bind_addr: SocketAddr = config.mcp_bind_addr().parse()?;
    let ct = tokio_util::sync::CancellationToken::new();

    // Clone state for the closure
    let state_for_closure = state.clone();
    let config_for_closure = config.clone();
    
    // Create the service factory - SSE uses the same StreamableHttpService
    let service = StreamableHttpService::new(
        move || {
            let handler = Handler::new(state_for_closure.clone(), &config_for_closure);
            handler.init_tools();
            Ok(handler)
        },
        LocalSessionManager::default().into(),
        build_streamable_config(&config, ct.child_token()),
    );

    // Create CORS layer to allow cross-origin requests
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers(Any);

    // Create router with MCP endpoint
    let app = axum::Router::new()
        .nest_service("/mcp", service)
        .layer(cors);

    // Start server
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    
    state.set_mcp_running(true).await;
    info!("MCP SSE server listening on http://{}/mcp", bind_addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            tokio::signal::ctrl_c().await.ok();
            ct.cancel();
        })
        .await?;

    state.set_mcp_running(false).await;
    info!("MCP SSE server stopped");

    Ok(())
}

/// Start MCP server with the configured transport
pub async fn start_server(
    state: Arc<ServerState>,
    config: AppConfig,
) -> anyhow::Result<()> {
    match config.mcp_transport.as_str() {
        "http" => start_http_server(state, config).await,
        "sse" => start_sse_server(state, config).await,
        _ => {
            error!("Unsupported transport: {}. Use 'http' or 'sse'", config.mcp_transport);
            Err(anyhow::anyhow!(
                "Unsupported transport: {}. Use 'http' or 'sse'",
                config.mcp_transport
            ))
        }
    }
}
