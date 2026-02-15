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
//!     // Load configuration from all sources (files, env vars, defaults)
//!     let config = Config::load()?;
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
//! ModelMux uses a professional configuration system with multiple sources:
//!
//! 1. **Configuration File** (recommended):
//! ```toml
//! # ~/.config/modelmux/config.toml (Linux)
//! # ~/Library/Application Support/modelmux/config.toml (macOS)
//! # %APPDATA%/modelmux/config.toml (Windows)
//!
//! [server]
//! port = 3000
//! log_level = "info"
//!
//! [llm_provider]
//! provider = "vertex"
//! project_id = "my-gcp-project"
//! location = "us-central1"
//! model_id = "claude-3-5-sonnet@20241022"
//!
//! [auth]
//! service_account_file = "~/.config/modelmux/service-account.json"
//!
//! [streaming]
//! mode = "auto"
//! ```
//!
//! 2. **Environment Variables**:
//! ```bash
//! export MODELMUX_SERVER_PORT=3000
//! export MODELMUX_SERVER_LOG_LEVEL=info
//! export MODELMUX_LLM_PROVIDER_PROJECT_ID=my-gcp-project
//! export MODELMUX_AUTH_SERVICE_ACCOUNT_FILE=/path/to/service-account.json
//! ```
//!
//! 3. **CLI Setup**:
//! ```bash
//! # Interactive configuration setup
//! modelmux config init
//!
//! # Validate configuration
//! modelmux config validate
//!
//! # Show current configuration
//! modelmux config show
//!
//! # Edit configuration
//! modelmux config edit
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
use tracing::info;

use crate::config::{Config, cli::ConfigCli};
use crate::error::Result;
use crate::provider::LlmProviderBackend;
use crate::server::AppState;

/* --- modules --------------------------------------------------------------------------------- */

mod auth;
mod config;
mod converter;
mod error;
mod provider;
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
async fn main() {
    // Load .env file for legacy environment variable support (before any config loading)
    if let Err(e) = dotenvy::dotenv() {
        // Only log at debug level - .env is optional
        if std::path::Path::new(".env").exists() {
            eprintln!("Warning: Could not load .env file: {}", e);
        }
    }

    // Handle CLI arguments before config loading
    if let Some(exit_code) = handle_cli_args().await {
        std::process::exit(exit_code);
    }

    if let Err(e) = run().await {
        // Print error message line by line to ensure proper formatting
        let error_msg = format!("{}", e);
        eprintln!("Error:");
        for line in error_msg.lines() {
            eprintln!("{}", line);
        }
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
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
/// Returns Some(exit_code) if the program should exit, None to continue.
async fn handle_cli_args() -> Option<i32> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        return None; // No arguments, proceed with normal startup
    }

    match args[1].as_str() {
        "--version" | "-V" => {
            println!("modelmux {}", VERSION);
            Some(0)
        }
        "--help" | "-h" => {
            print_help();
            Some(0)
        }
        "config" => handle_config_command(&args[2..]).await,
        "doctor" => {
            let exit_code = run_doctor();
            Some(exit_code)
        }
        "validate" => {
            let exit_code = run_validate();
            Some(exit_code)
        }
        _ => {
            // Unknown command or option - show error and help
            if args[1].starts_with('-') {
                eprintln!("Error: Unknown option: {}", args[1]);
                eprintln!();
                print_help();
                Some(1)
            } else {
                eprintln!("Error: Unknown command: {}", args[1]);
                eprintln!();
                eprintln!("Available commands:");
                eprintln!("  config    - Configuration management");
                eprintln!("  doctor    - Run configuration health check");
                eprintln!("  validate  - Validate configuration");
                eprintln!();
                eprintln!("Available options:");
                eprintln!("  --version, -V  - Show version");
                eprintln!("  --help, -h     - Show help");
                eprintln!();
                eprintln!("Run 'modelmux --help' for more information.");
                Some(1)
            }
        }
    }
}

///
/// Handle config subcommands.
async fn handle_config_command(args: &[String]) -> Option<i32> {
    if args.is_empty() {
        eprintln!("Error: Missing config subcommand");
        eprintln!();
        print_config_help();
        return Some(1);
    }

    let result = match args[0].as_str() {
        "init" => ConfigCli::init(),
        "show" => ConfigCli::show(),
        "validate" => ConfigCli::validate(),
        "edit" => ConfigCli::edit(),
        "--help" | "-h" => {
            print_config_help();
            return Some(0);
        }
        _ => {
            eprintln!("Error: Unknown config subcommand: {}", args[0]);
            eprintln!();
            print_config_help();
            return Some(1);
        }
    };

    match result {
        Ok(_) => Some(0),
        Err(e) => {
            eprintln!("Error: {}", e);
            Some(1)
        }
    }
}

///
/// Print config command help.
fn print_config_help() {
    println!("ModelMux Configuration Management");
    println!();
    println!("USAGE:");
    println!("    modelmux config <SUBCOMMAND>");
    println!();
    println!("SUBCOMMANDS:");
    println!("    init        Interactive configuration setup");
    println!("    show        Display current configuration");
    println!("    validate    Validate configuration");
    println!("    edit        Edit configuration file in default editor");
    println!("    help        Show this help message");
    println!();
    println!("EXAMPLES:");
    println!("    modelmux config init        # Set up configuration interactively");
    println!("    modelmux config show        # Show current configuration");
    println!("    modelmux config validate    # Check configuration validity");
    println!("    modelmux config edit        # Open config file in editor");
}

///
/// Print help information for the ModelMux CLI.
fn print_help() {
    println!("ModelMux v{}", VERSION);
    println!("High-performance proxy server converting OpenAI API requests to Vertex AI format");
    println!();
    println!("USAGE:");
    println!("    modelmux [COMMAND] [OPTIONS]");
    println!();
    println!("COMMANDS:");
    println!("    config              Configuration management (init, show, validate, edit)");
    println!("    doctor              Check configuration and system health (legacy)");
    println!("    validate            Validate configuration and exit (legacy)");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help          Print help information");
    println!("    -V, --version       Print version information");
    println!();
    println!("CONFIGURATION:");
    println!("    ModelMux uses a modern configuration system with multiple sources:");
    println!("    1. Configuration files (TOML format in standard directories)");
    println!("    2. Environment variables (MODELMUX_* prefix)");
    println!("    3. Built-in defaults");
    println!();
    println!("    Run 'modelmux config init' to set up configuration interactively.");
    println!("    Run 'modelmux config --help' for configuration management help.");
    println!();
    println!("ENVIRONMENT VARIABLES:");
    println!("    PORT                       Server port (default: 3000)");
    println!(
        "    LOG_LEVEL                  Log level: trace, debug, info, warn, error (default: info)"
    );
    println!(
        "    STREAMING_MODE             Streaming mode: auto, non-streaming, standard, buffered (default: auto)"
    );
    println!();
    println!("  Provider / model configuration:");
    println!(
        "    LLM_PROVIDER               Backend: vertex (default) or openai_compatible (future)"
    );
    println!();
    println!("    Either set a single override (ignores provider-specific fields):");
    println!(
        "    LLM_URL                    Full resource URL (no :rawPredict/:streamRawPredict suffix)"
    );
    println!();
    println!("    Or set provider-specific fields. For Vertex (LLM_PROVIDER=vertex):");
    println!(
        "    GCP_SERVICE_ACCOUNT_KEY    Base64-encoded Google Cloud service account key (required)"
    );
    println!("    VERTEX_REGION              e.g. europe-west1");
    println!("    VERTEX_PROJECT             GCP project ID");
    println!("    VERTEX_LOCATION            e.g. europe-west1");
    println!("    VERTEX_PUBLISHER           e.g. anthropic");
    println!("    VERTEX_MODEL_ID            e.g. claude-sonnet-4@20250514");
    println!();
    println!("EXAMPLES:");
    println!("    modelmux                    Start the proxy server");
    println!("    modelmux doctor             Check configuration");
    println!("    modelmux validate           Validate and exit");
    println!();
    println!("For more information, visit: https://github.com/yarenty/modelmux");
}

///
/// Run diagnostic check of the server configuration and environment (legacy command).
///
/// This command helps users verify their configuration is correct by loading
/// and validating all settings, then providing detailed feedback about any
/// issues found.
fn run_doctor() -> i32 {
    println!("⚠️  The 'doctor' command is deprecated. Use 'modelmux config validate' instead.");
    println!();
    println!("ModelMux Doctor - Configuration Health Check");
    println!("==========================================");
    println!();

    // Check if configuration files exist
    let config_paths = crate::config::paths::config_file_paths();
    let mut found_config = false;

    println!("Configuration file locations:");
    for (i, path) in config_paths.iter().enumerate() {
        let priority = match i {
            0 => "user config",
            _ => "system config",
        };

        if path.exists() {
            println!("  ✓ {} found: {}", priority, path.display());
            found_config = true;
        } else {
            println!("  ✗ {} not found: {}", priority, path.display());
        }
    }

    if !found_config {
        println!("  ℹ️  No configuration files found, using defaults and environment variables");
    }
    println!();

    // Try to load configuration
    println!("Testing configuration loading:");
    match Config::load() {
        Ok(config) => {
            println!("✓ Configuration loaded successfully");
            println!();

            // Run validation checks
            println!("Running validation checks:");
            match config.validate() {
                Ok(_) => {
                    println!("✓ No validation issues found");
                    println!();
                    println!("[SUCCESS] Configuration looks good! You're ready to run ModelMux.");
                    println!();
                    println!("Configuration summary:");
                    println!("  Server port: {}", config.server.port);
                    println!("  Log level: {:?}", config.server.log_level);
                    if let Some(ref provider) = config.llm_provider {
                        println!("  LLM provider: {}", provider.id());
                    } else {
                        println!("  LLM provider: Not loaded");
                    }
                    println!("  Streaming mode: {:?}", config.streaming.mode);

                    if let Some(ref file) = config.auth.service_account_file {
                        match crate::config::paths::expand_path(file) {
                            Ok(path) => {
                                if path.exists() {
                                    println!("  Service account file: ✓ {}", path.display());
                                } else {
                                    println!(
                                        "  Service account file: ✗ {} (not found)",
                                        path.display()
                                    );
                                }
                            }
                            Err(_) => {
                                println!("  Service account file: ✗ {} (invalid path)", file);
                            }
                        }
                    } else if config.auth.service_account_json.is_some() {
                        println!("  Service account: ✓ Inline JSON configured");
                    } else {
                        println!("  Service account: ✗ Not configured");
                    }

                    0
                }
                Err(e) => {
                    println!("✗ Configuration validation failed:");
                    println!("{}", e);
                    println!();
                    println!(
                        "[ERROR] Configuration has errors. Please fix them before running ModelMux."
                    );
                    1
                }
            }
        }
        Err(e) => {
            println!("✗ Configuration loading failed: {}", e);
            println!();
            println!("Suggestions:");
            println!("   • Run 'modelmux config init' to set up configuration interactively");
            println!("   • Use 'modelmux config validate' for detailed validation");
            println!("   • Check 'modelmux config show' to see current configuration");
            1
        }
    }
}

///
/// Run the validate command to validate configuration and exit.
///
/// Returns exit code 0 if valid, 1 if invalid.
fn run_validate() -> i32 {
    println!("⚠️  The 'validate' command is deprecated. Use 'modelmux config validate' instead.");
    println!();

    match Config::load() {
        Ok(config) => match config.validate() {
            Ok(_) => {
                println!("✅ Configuration is valid");
                0
            }
            Err(e) => {
                println!("❌ Configuration validation failed:");
                println!("{}", e);
                1
            }
        },
        Err(e) => {
            println!("❌ Failed to load configuration:");
            println!("{}", e);
            1
        }
    }
}

///
/// Initialize configuration from environment variables.
///
/// # Returns
///  * Configuration object loaded from environment
///  * `ProxyError::Config` if required environment variables are missing
fn initialize_config() -> Result<Config> {
    Config::load()
}

///
/// Initialize logging with the specified log level.
///
/// Sets up tracing subscriber with appropriate log level based on configuration.
///
/// # Arguments
///  * `config` - application configuration containing log level settings
/// Initialize logging based on configuration settings.
fn initialize_logging(config: &Config) {
    let level = config.server.log_level.to_tracing_level();

    tracing_subscriber::fmt().with_max_level(level).with_target(false).init();
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
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.server.port))
        .await
        .map_err(|e| {
        let error_msg = format!("Failed to bind to port {}: {}", config.server.port, e);

        // Check if it's an "Address already in use" error and provide helpful suggestions
        let error_str = e.to_string();
        if error_str.contains("Address already in use")
            || error_str.contains("address already in use")
        {
            let suggestions = format!(
                "{}\n\n\
                    Port {} is already in use. Here are some solutions:\n\n\
                    1. Close the other instance:\n\
                       • Find the process using port {}:\n\
                         lsof -i :{}\n\
                       • Kill the process:\n\
                         kill -9 <PID>\n\n\
                    2. Use killport (if installed):\n\
                       killport {}\n\n\
                    3. Change the port:\n\
                       export PORT=3001\n\
                       modelmux\n\n\
                    Run 'modelmux doctor' for more help.",
                error_msg,
                config.server.port,
                config.server.port,
                config.server.port,
                config.server.port
            );
            crate::error::ProxyError::Http(suggestions)
        } else {
            crate::error::ProxyError::Http(format!(
                "{}\n\n\
                    To fix this:\n\
                    • Check if the port is valid (1-65535)\n\
                    • Ensure you have permission to bind to the port\n\
                    • Try a different port: export PORT=3001\n\n\
                    Run 'modelmux doctor' for more help.",
                error_msg
            ))
        }
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
    info!("ModelMux v{} running on port {}", VERSION, config.server.port);
    info!("Proxy supports tool/function calling for file creation and editing");
    info!("OpenAI-compatible endpoint: http://localhost:{}/v1", config.server.port);

    if matches!(
        config.server.log_level,
        crate::config::LogLevel::Trace | crate::config::LogLevel::Debug
    ) {
        info!(
            "[TRACE] Trace logging is ENABLED (LOG_LEVEL={:?}) - tool calls and interactions will \
       be logged",
            config.server.log_level
        );
    }
}
