mod config;
mod mcp;
mod utils;
mod web;

use crate::config::AppConfig;
use crate::mcp::state::ServerState;

use tracing::{error, info, Level};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    let config = AppConfig::parse_args();

    // Initialize logging
    init_logging(&config.log_level);

    info!("Starting Rust MCP Server v{}", env!("CARGO_PKG_VERSION"));
    info!("Configuration: {:?}", config);

    // Create shared state
    let state = ServerState::new(config.clone());

    // Start WebUI server if not disabled
    let _web_handle = if !config.disable_webui {
        let state = state.clone();
        let bind_addr = config.webui_bind_addr();
        
        info!("Starting WebUI server on http://{}", bind_addr);
        
        Some(tokio::spawn(async move {
            if let Err(e) = web::start_web_server(state, bind_addr).await {
                error!("WebUI server error: {}", e);
            }
        }))
    } else {
        info!("WebUI is disabled");
        None
    };

    // Start MCP server
    let mcp_handle = {
        let state = state.clone();
        let config = config.clone();
        
        tokio::spawn(async move {
            // Initialize tools
            let tools = mcp::tools::get_all_tools();
            let tool_count = tools.len();
            state.init_tools(tools).await;
            
            info!("Starting MCP server with {} tools", tool_count);
            
            // Start MCP service
            if let Err(e) = mcp::start_server(state, config).await {
                error!("MCP server error: {}", e);
            }
        })
    };

    // Wait for shutdown signal or Ctrl+C
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal (Ctrl+C)");
        }
        result = mcp_handle => {
            if let Err(e) = result {
                error!("MCP server task error: {}", e);
            }
        }
    }

    // Graceful shutdown
    info!("Shutting down...");
    
    // Clean up
    state.set_mcp_running(false).await;

    // Give tasks time to clean up
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    info!("Shutdown complete");
    Ok(())
}

/// Initialize logging with the specified level
fn init_logging(log_level: &str) {
    let level = match log_level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    let env_filter = EnvFilter::builder()
        .with_default_directive(level.into())
        .from_env_lossy()
        .add_directive("hyper=warn".parse().unwrap())
        .add_directive("reqwest=warn".parse().unwrap());

    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_level(true)
        .with_thread_ids(false)
        .with_thread_names(false);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}
