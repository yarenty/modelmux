//! # ModelMux - Vertex AI to OpenAI Proxy Server
//!
//! A high-performance proxy server that converts OpenAI-compatible API requests
//! to Vertex AI (Anthropic Claude) format. Built with Rust following SOLID principles
//! for type safety, performance, and reliability.
//!
//! ## Features
//!
//! - **OpenAI-compatible API**: Drop-in replacement for OpenAI API endpoints
//! - **Tool/Function Calling**: Full support for OpenAI tool calling format
//! - **Streaming Support**: Server-Sent Events (SSE) streaming responses
//! - **Smart Client Detection**: Automatically adjusts streaming behavior based on client capabilities
//! - **Error Handling**: Comprehensive error handling with proper Result types
//! - **Type Safety**: Leverages Rust's type system for compile-time safety
//! - **Performance**: Async/await with Tokio for high concurrency
//! - **Configurable Logging**: Structured logging with tracing
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use modelmux::{Config, create_app};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Load configuration from environment
//!     let config = Config::from_env()?;
//!
//!     // Create the application
//!     let app = create_app(config).await?;
//!
//!     // Start the server
//!     let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
//!     axum::serve(listener, app).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Configuration
//!
//! Configure the server using environment variables:
//!
//! ```bash
//! # Required: Base64-encoded Google Cloud service account key
//! export GCP_SERVICE_ACCOUNT_KEY="your-base64-encoded-key-here"
//!
//! # Required: Vertex AI configuration
//! export LLM_URL="https://europe-west1-aiplatform.googleapis.com/v1/projects/..."
//! export LLM_CHAT_ENDPOINT="your-chat-endpoint"
//! export LLM_MODEL="claude-sonnet-4"
//!
//! # Optional: Server configuration
//! export PORT=3000
//! export LOG_LEVEL=info
//! export STREAMING_MODE=auto  # auto, non-streaming, standard, buffered
//! ```
//!
//! ## API Usage
//!
//! The server provides OpenAI-compatible endpoints:
//!
//! ```bash
//! curl -X POST http://localhost:3000/v1/chat/completions \
//!   -H "Content-Type: application/json" \
//!   -d '{
//!     "model": "claude-sonnet-4",
//!     "messages": [{"role": "user", "content": "Hello!"}],
//!     "stream": false
//!   }'
//! ```
//!
//! ## License
//!
//! Licensed under either of Apache License, Version 2.0 or MIT license at your option.
//!
//! Authors: Jaro <yarenty@gmail.com>
//!
//! Copyright (c) 2024 SkyCorp
//!

/* --- uses ------------------------------------------------------------------------------------ */

use std::env;
use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{Level, info};

use crate::config::Config;
use crate::error::Result;
use crate::server::AppState;

/* --- modules --------------------------------------------------------------------------------- */

mod auth;
mod config;
mod converter;
mod error;
mod server;

/* --- constants ------------------------------------------------------------------------------ */

/** the version as defined in cargo.toml */
const VERSION: &str = env!("CARGO_PKG_VERSION");

/* --- start of code -------------------------------------------------------------------------- */

///
/// Main application entry point for the ModelMux Vertex AI to OpenAI proxy server.
///
/// Initializes logging, loads configuration from environment variables,
/// creates the application state, and starts the HTTP server with proper
/// routing and middleware.
///
/// # Returns
///  * `Ok(())` on successful server shutdown
///  * `ProxyError` if initialization or server startup fails
#[tokio::main]
async fn main() -> Result<()> {
    // Handle CLI arguments before config loading
    handle_cli_args();

    let config = initialize_config()?;
    initialize_logging(&config);

    let app_state = create_app_state(config.clone()).await?;
    let app = create_router(app_state);

    start_server(&config, app).await
}

///
/// Handle command line arguments like --version and --help before config loading.
///
/// This ensures these commands work even without proper configuration.
fn handle_cli_args() {
    let args: Vec<String> = env::args().collect();

    for arg in &args[1..] {
        match arg.as_str() {
            "--version" | "-V" => {
                println!("modelmux {}", VERSION);
                std::process::exit(0);
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => {}
        }
    }
}

///
/// Print help information for the ModelMux CLI.
fn print_help() {
    println!("ModelMux v{}", VERSION);
    println!("High-performance proxy server converting OpenAI API requests to Vertex AI format");
    println!();
    println!("USAGE:");
    println!("    modelmux [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help       Print help information");
    println!("    -V, --version    Print version information");
    println!();
    println!("ENVIRONMENT VARIABLES:");
    println!(
        "    GCP_SERVICE_ACCOUNT_KEY    Base64-encoded Google Cloud service account key (required)"
    );
    println!("    LLM_URL                    Vertex AI base URL (required)");
    println!("    LLM_CHAT_ENDPOINT         Chat endpoint path (required)");
    println!("    LLM_MODEL                 Model identifier (required)");
    println!("    PORT                      Server port (default: 3000)");
    println!(
        "    LOG_LEVEL                 Log level: trace, debug, info, warn, error (default: info)"
    );
    println!(
        "    STREAMING_MODE            Streaming mode: auto, non-streaming, standard, buffered (default: auto)"
    );
    println!();
    println!("For more information, visit: https://github.com/yarenty/modelmux");
}

///
/// Initialize configuration from environment variables.
///
/// # Returns
///  * Configuration object loaded from environment
///  * `ProxyError::Config` if required environment variables are missing
fn initialize_config() -> Result<Config> {
    Config::from_env()
}

///
/// Initialize logging with the specified log level.
///
/// Sets up tracing subscriber with appropriate log level based on configuration.
///
/// # Arguments
///  * `config` - application configuration containing log level settings
fn initialize_logging(config: &Config) {
    let log_level = match config.log_level {
        config::LogLevel::Trace => Level::TRACE,
        config::LogLevel::Debug => Level::DEBUG,
        config::LogLevel::Info => Level::INFO,
        config::LogLevel::Warn => Level::WARN,
        config::LogLevel::Error => Level::ERROR,
    };

    tracing_subscriber::fmt().with_max_level(log_level).with_target(false).init();
}

///
/// Create application state with all required dependencies.
///
/// Initializes authentication providers, HTTP clients, and converters
/// needed for the proxy functionality.
///
/// # Arguments
///  * `config` - application configuration
///
/// # Returns
///  * Application state wrapped in Arc for sharing across handlers
///  * `ProxyError` if state initialization fails
async fn create_app_state(config: Config) -> Result<Arc<AppState>> {
    let app_state = Arc::new(AppState::new(config.clone()).await?);
    Ok(app_state)
}

///
/// Create the Axum router with all routes and middleware.
///
/// Sets up endpoints for chat completions, models listing, and health checks
/// with proper CORS and tracing middleware.
///
/// # Arguments
///  * `app_state` - shared application state
///
/// # Returns
///  * Configured Axum router ready for serving
fn create_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(server::chat_completions))
        .route("/v1/models", get(server::models))
        .route("/health", get(server::health))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(app_state)
}

///
/// Start the HTTP server and log startup information.
///
/// Binds to the configured port and starts serving requests. Logs important
/// information about the server configuration and available endpoints.
///
/// # Arguments
///  * `config` - application configuration
///  * `app` - configured Axum application
///
/// # Returns
///  * `Ok(())` when server shuts down gracefully
///  * `ProxyError::Http` if server binding or startup fails
async fn start_server(config: &Config, app: Router) -> Result<()> {
    let listener =
        tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await.map_err(|e| {
            crate::error::ProxyError::Http(format!("Failed to bind to port {}: {}", config.port, e))
        })?;

    log_startup_info(config);

    axum::serve(listener, app)
        .await
        .map_err(|e| crate::error::ProxyError::Http(format!("Server error: {}", e)))?;

    Ok(())
}

///
/// Log startup information and configuration details.
///
/// Provides useful information about the running server including port,
/// supported features, and trace logging status.
///
/// # Arguments
///  * `config` - application configuration
fn log_startup_info(config: &Config) {
    info!("ModelMux v{} running on port {}", VERSION, config.port);
    info!("Proxy supports tool/function calling for file creation and editing");
    info!("OpenAI-compatible endpoint: http://localhost:{}/v1", config.port);

    if config.log_level.is_trace_enabled() {
        info!(
            "[TRACE] Trace logging is ENABLED (AISRV_LOG_LEVEL={:?}) - tool calls and interactions will \
       be logged",
            config.log_level
        );
    }
}
