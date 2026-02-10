//!
//! Error handling for the Vertex AI to OpenAI proxy server.
//!
//! Defines all error types used throughout the application using thiserror
//! for ergonomic error handling. Follows Rust best practices for error design.
//!
//! Authors:
//!   Jaro <yarenty@gmail.com>
//!
//! Copyright (c) 2026 SkyCorp

/* --- uses ------------------------------------------------------------------------------------ */

use thiserror::Error;

/* --- types ----------------------------------------------------------------------------------- */

///
/// Application error types following Rust best practices.
///
/// Covers all possible error conditions that can occur during proxy operation.
/// Uses thiserror for automatic Display and Error trait implementations.
#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Conversion error: {0}")]
    Conversion(String),
}

/* --- start of code -------------------------------------------------------------------------- */

/// Result type alias for cleaner error handling throughout the application
pub type Result<T> = std::result::Result<T, ProxyError>;
