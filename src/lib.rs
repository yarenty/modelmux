//! # ModelMux - Vertex AI to OpenAI Proxy Library
//!
//! This crate provides a high-performance proxy server that converts OpenAI-compatible
//! API requests to Vertex AI (Anthropic Claude) format. While primarily designed as a
//! binary application, this library exposes its core functionality for programmatic use.
//!
//! ## Library Usage
//!
//! ```rust,no_run
//! use modelmux::{Config, create_app};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Load configuration
//!     let config = Config::from_env()?;
//!
//!     // Create the application
//!     let app = create_app(config).await?;
//!
//!     // Start server
//!     let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
//!     axum::serve(listener, app).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Modules
//!
//! - [`config`] - Configuration management and environment variable handling
//! - [`provider`] - LLM backend abstraction ([`LlmProviderBackend`]); Vertex and OpenAI-compatible (stub)
//! - [`auth`] - Request auth (GCP OAuth2 or Bearer token)
//! - [`server`] - HTTP server setup and route handlers
//! - [`converter`] - Format conversion between OpenAI and Anthropic formats
//! - [`error`] - Error types and handling

pub mod auth;
pub mod config;
pub mod converter;
pub mod error;
pub mod provider;
pub mod server;

// Re-export commonly used types
pub use config::{Config, ValidationIssue, ValidationSeverity};
pub use error::ProxyError;

/// Creates a new ModelMux application with the given configuration.
///
/// This is a convenience function that sets up the full application stack
/// including authentication, routing, and middleware.
///
/// # Arguments
///
/// * `config` - Application configuration
///
/// # Returns
///
/// Returns an Axum Router that can be served directly.
///
/// # Errors
///
/// Returns a `ProxyError` if authentication setup fails or other
/// initialization issues occur.
///
/// # Examples
///
/// ```rust,no_run
/// use modelmux::{Config, create_app};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = Config::from_env()?;
///     let app = create_app(config).await?;
///
///     let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
///     axum::serve(listener, app).await?;
///     Ok(())
/// }
/// ```
pub async fn create_app(config: Config) -> Result<axum::Router, ProxyError> {
    use axum::Router;
    use axum::routing::{get, post};
    use std::sync::Arc;
    use tower_http::cors::CorsLayer;
    use tower_http::trace::TraceLayer;

    let app_state = Arc::new(server::AppState::new(config).await?);

    Ok(Router::new()
        .route("/v1/chat/completions", post(server::chat_completions))
        .route("/v1/models", get(server::models))
        .route("/health", get(server::health))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(app_state))
}
