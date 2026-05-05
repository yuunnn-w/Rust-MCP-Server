pub mod handler;
pub mod presets;
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
use sysinfo::Networks;
use tracing::info;

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
    
    let base_config = StreamableHttpServerConfig::default()
        .with_stateful_mode(stateful_mode)
        .with_json_response(json_response)
        .with_cancellation_token(ct);
    
    // User explicitly disables allowed hosts check
    if config.disable_allowed_hosts {
        info!("allowed_hosts check is disabled via --disable-allowed-hosts");
        return base_config.disable_allowed_hosts();
    }
    
    // User explicitly specifies allowed hosts
    if let Some(ref custom_hosts) = config.allowed_hosts {
        info!("Using custom allowed_hosts: {:?}", custom_hosts);
        return base_config.with_allowed_hosts(custom_hosts.clone());
    }
    
    // Auto-detect allowed_hosts based on bind configuration
    let mut allowed_hosts = vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "::1".to_string(),
    ];
    
    if config.mcp_host == "0.0.0.0" {
        // 0.0.0.0 listens on all interfaces; we need to discover actual local IPs
        // because clients will send Host headers with the specific IP they connect to.
        info!("mcp_host is 0.0.0.0, auto-detecting network interface IPs for allowed_hosts");
        let networks = Networks::new_with_refreshed_list();
        for (_, data) in &networks {
            for ip_network in data.ip_networks() {
                let ip_str = ip_network.addr.to_string();
                if !allowed_hosts.contains(&ip_str) {
                    allowed_hosts.push(ip_str);
                }
            }
        }
    } else {
        // Specific bind address: add it and its port-qualified form
        if !config.mcp_host.is_empty() && !allowed_hosts.contains(&config.mcp_host) {
            allowed_hosts.push(config.mcp_host.clone());
        }
        let mcp_with_port = format!("{}:{}", config.mcp_host, config.mcp_port);
        if !allowed_hosts.contains(&mcp_with_port) {
            allowed_hosts.push(mcp_with_port);
        }
        if !config.webui_host.is_empty() 
            && config.webui_host != config.mcp_host 
            && !allowed_hosts.contains(&config.webui_host) {
            allowed_hosts.push(config.webui_host.clone());
        }
    }
    
    info!("allowed_hosts: {:?}", allowed_hosts);
    base_config.with_allowed_hosts(allowed_hosts)
}

/// Start MCP server with the configured transport
pub async fn start_server(
    state: Arc<ServerState>,
    config: AppConfig,
) -> anyhow::Result<()> {
    let transport_name = config.mcp_transport.clone();
    info!(
        "Starting MCP server with {} transport on {}",
        transport_name,
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
    info!("MCP {} server listening on http://{}/mcp", transport_name, bind_addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            tokio::signal::ctrl_c().await.ok();
            ct.cancel();
        })
        .await?;

    state.set_mcp_running(false).await;
    info!("MCP {} server stopped", transport_name);

    Ok(())
}
