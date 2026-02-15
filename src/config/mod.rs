//!
//! Professional configuration management for ModelMux.
//!
//! This module provides a clean, industry-standard configuration system using:
//! - Platform-native configuration directories (XDG on Linux, standard paths on macOS/Windows)
//! - TOML format for human-readable configuration files
//! - Multi-layered configuration hierarchy (CLI args > env vars > user config > defaults)
//! - Secure service account file handling
//! - Comprehensive validation and error handling
//!
//! Follows SOLID principles with clear separation of concerns:
//! - `loader.rs` - Configuration loading logic (SRP)
//! - `paths.rs` - Platform-native path resolution (SRP)
//! - `validation.rs` - Configuration validation (SRP)
//! - `cli.rs` - CLI configuration commands (SRP)
//!
//! Authors:
//!   Jaro <yarenty@gmail.com>
//!
//! Copyright (c) 2026 SkyCorp

/* --- modules --------------------------------------------------------------------------------- */

pub mod cli;
pub mod loader;
pub mod paths;
pub mod validation;

/* --- uses ------------------------------------------------------------------------------------ */

use crate::error::{ProxyError, Result};
use crate::provider::{AuthStrategy, LlmProviderBackend, LlmProviderConfig};
use serde::{Deserialize, Serialize};

/* --- types ----------------------------------------------------------------------------------- */

///
/// Main application configuration structure.
///
/// This replaces the old Config struct with TOML-compatible fields
/// and better organization following configuration best practices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// HTTP server configuration
    pub server: ServerConfig,
    /// Authentication configuration
    pub auth: AuthConfig,
    /// Streaming behavior configuration
    pub streaming: StreamingConfig,
    /// Vertex AI provider configuration (optional; env vars used if not set)
    #[serde(default)]
    pub vertex: Option<VertexConfig>,

    /// LLM provider configuration (loaded separately, not serialized)
    #[serde(skip)]
    pub llm_provider: Option<LlmProviderConfig>,
}

///
/// Vertex AI provider configuration.
///
/// Can be set in TOML under `[vertex]` or via environment variables
/// (VERTEX_PROJECT, VERTEX_REGION, etc.). Config file takes precedence over env.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexConfig {
    /// GCP project ID
    #[serde(alias = "project_id")]
    pub project: Option<String>,
    /// Vertex region (e.g. europe-west1)
    #[serde(default)]
    pub region: Option<String>,
    /// Vertex location (often same as region)
    #[serde(default)]
    pub location: Option<String>,
    /// Model publisher (e.g. anthropic)
    #[serde(default)]
    pub publisher: Option<String>,
    /// Model ID (e.g. claude-3-5-sonnet@20241022)
    #[serde(alias = "model_id")]
    pub model: Option<String>,
    /// Full URL override (alternative to region/project/location/publisher/model)
    #[serde(default)]
    pub url: Option<String>,
}

///
/// HTTP server configuration.
///
/// Groups all server-related settings for better organization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// HTTP server port number
    #[serde(default = "default_port")]
    pub port: u16,
    /// Application logging level
    #[serde(default = "default_log_level")]
    pub log_level: LogLevel,
    /// Whether to enable retry logic for quota errors
    #[serde(default = "default_enable_retries")]
    pub enable_retries: bool,
    /// Maximum retry attempts for quota errors
    #[serde(default = "default_max_retry_attempts")]
    pub max_retry_attempts: u32,
}

///
/// Authentication configuration.
///
/// Supports multiple authentication methods with secure defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Path to Google Cloud service account JSON file
    /// Supports tilde expansion (~/.config/modelmux/service-account.json)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_account_file: Option<String>,

    /// Inline service account JSON (for container environments)
    /// Takes precedence over service_account_file if both are provided
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_account_json: Option<String>,

    /// Authentication strategy (for future extensibility)
    #[serde(skip, default = "default_auth_strategy")]
    pub strategy: AuthStrategy,
}

///
/// Streaming configuration.
///
/// Controls how the proxy handles streaming responses for different clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    /// Streaming mode selection
    #[serde(default = "default_streaming_mode")]
    pub mode: StreamingMode,

    /// Buffer size for buffered streaming mode (in bytes)
    #[serde(default = "default_buffer_size")]
    pub buffer_size: usize,

    /// Timeout for streaming chunks (in milliseconds)
    #[serde(default = "default_chunk_timeout")]
    pub chunk_timeout_ms: u64,
}

///
/// Streaming mode configuration.
///
/// Controls how the proxy handles streaming responses for different clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StreamingMode {
    /// Auto-detect client and choose appropriate streaming mode
    Auto,
    /// Force all requests to use non-streaming responses
    Never,
    /// Use standard word-by-word streaming
    Standard,
    /// Use buffered streaming for better client compatibility
    Buffered,
    /// Always stream, regardless of client
    Always,
}

///
/// Logging level enumeration.
///
/// Defines available log levels compatible with tracing crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    #[serde(alias = "trace")]
    Trace,
    #[serde(alias = "debug")]
    Debug,
    #[serde(alias = "info")]
    Info,
    #[serde(alias = "warn")]
    Warn,
    #[serde(alias = "error")]
    Error,
}

///
/// Google Cloud service account key structure.
///
/// Contains all fields required for OAuth2 authentication with Google Cloud Platform.
/// This structure matches the standard GCP service account JSON format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceAccountKey {
    /// Type of account (should be "service_account")
    #[serde(rename = "type")]
    pub account_type: String,
    /// Google Cloud project identifier
    pub project_id: String,
    /// Unique identifier for the private key
    pub private_key_id: String,
    /// PEM-encoded private key for signing
    pub private_key: String,
    /// Service account email address
    pub client_email: String,
    /// OAuth2 client identifier
    pub client_id: String,
    /// OAuth2 authorization URI
    pub auth_uri: String,
    /// OAuth2 token endpoint URI
    pub token_uri: String,
    /// X.509 certificate URL for auth provider
    pub auth_provider_x509_cert_url: String,
    /// X.509 certificate URL for this client
    pub client_x509_cert_url: String,
    /// Universe domain (optional, for Workload Identity Federation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub universe_domain: Option<String>,
}

/* --- defaults -------------------------------------------------------------------------------- */

/// Default HTTP port
fn default_port() -> u16 {
    3000
}

/// Default logging level
fn default_log_level() -> LogLevel {
    LogLevel::Info
}

/// Default retry behavior
fn default_enable_retries() -> bool {
    true
}

/// Default maximum retry attempts
fn default_max_retry_attempts() -> u32 {
    3
}

/// Default authentication strategy
pub fn default_auth_strategy() -> AuthStrategy {
    // Use GcpOAuth2 with a placeholder key that will be replaced during loading
    use crate::config::ServiceAccountKey;
    let placeholder_key = ServiceAccountKey {
        account_type: "service_account".to_string(),
        project_id: "placeholder".to_string(),
        private_key_id: "placeholder".to_string(),
        private_key: "placeholder".to_string(),
        client_email: "placeholder@placeholder.com".to_string(),
        client_id: "placeholder".to_string(),
        auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
        token_uri: "https://oauth2.googleapis.com/token".to_string(),
        auth_provider_x509_cert_url: "https://www.googleapis.com/oauth2/v1/certs".to_string(),
        client_x509_cert_url: "".to_string(),
        universe_domain: None,
    };
    AuthStrategy::GcpOAuth2(placeholder_key)
}

/// Default streaming mode
fn default_streaming_mode() -> StreamingMode {
    StreamingMode::Auto
}

/// Default streaming buffer size (64KB)
fn default_buffer_size() -> usize {
    65536
}

/// Default chunk timeout (5 seconds)
fn default_chunk_timeout() -> u64 {
    5000
}

/* --- implementations --------------------------------------------------------------------- */

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            auth: AuthConfig::default(),
            streaming: StreamingConfig::default(),
            vertex: None,
            // Provider will be loaded separately
            llm_provider: None,
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            log_level: default_log_level(),
            enable_retries: default_enable_retries(),
            max_retry_attempts: default_max_retry_attempts(),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            service_account_file: None,
            service_account_json: None,
            strategy: default_auth_strategy(),
        }
    }
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            mode: default_streaming_mode(),
            buffer_size: default_buffer_size(),
            chunk_timeout_ms: default_chunk_timeout(),
        }
    }
}

impl Config {
    /// Load configuration from the standard hierarchy:
    /// 1. CLI arguments (highest priority)
    /// 2. Environment variables
    /// 3. User config file (~/.config/modelmux/config.toml)
    /// 4. System config file (/etc/modelmux/config.toml)
    /// 5. Built-in defaults (lowest priority)
    ///
    /// # Returns
    /// * `Ok(Config)` - Successfully loaded configuration
    /// * `Err(ProxyError)` - Configuration loading or validation failed
    ///
    /// # Examples
    /// ```rust,no_run
    /// use modelmux::config::Config;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Config::load()?;
    /// println!("Server will run on port {}", config.server.port);
    /// # Ok(())
    /// # }
    /// ```
    pub fn load() -> Result<Self> {
        // First load using the new system for most settings
        let mut base_config = loader::ConfigLoader::new()
            .with_defaults()
            .with_system_config()?
            .with_user_config()?
            .with_env_vars()?
            .build_base()?;

        // Load service account key from auth config to avoid circular dependency
        let service_account_key = Self::load_service_account_key_from_auth(&base_config.auth)?;

        // Then load provider config (from vertex config, env vars, or .env)
        base_config.llm_provider = Some(LlmProviderConfig::from_config_or_env_with_key(
            service_account_key,
            base_config.vertex.as_ref(),
        )?);

        Ok(base_config)
    }

    /// Get the build URL for API requests
    pub fn build_predict_url(&self, is_streaming: bool) -> String {
        self.llm_provider
            .as_ref()
            .map(|p| p.build_request_url(is_streaming))
            .unwrap_or_else(|| "http://localhost:3000/unknown".to_string())
    }

    /// Display model name for OpenAI-compatible API responses
    pub fn llm_model(&self) -> &str {
        self.llm_provider.as_ref().map(|p| p.display_model_name()).unwrap_or("unknown")
    }

    /// Legacy method for loading service account key (for backward compatibility)
    #[allow(dead_code)]
    pub fn load_service_account_key_standalone() -> Result<ServiceAccountKey> {
        // Load auth config directly to avoid circular dependency
        let auth_config =
            loader::ConfigLoader::new().with_defaults().with_env_vars()?.build_base()?.auth;
        Self::load_service_account_key_from_auth(&auth_config)
    }

    /// Load service account key from provided auth config (to avoid circular dependency)
    pub fn load_service_account_key_from_auth(auth: &AuthConfig) -> Result<ServiceAccountKey> {
        if let Some(ref json_str) = auth.service_account_json {
            // Load from inline JSON
            serde_json::from_str(json_str).map_err(|e| {
                ProxyError::Config(format!(
                    "Failed to parse inline service account JSON: {}\n\
                     \n\
                     The JSON appears to be malformed. Please verify:\n\
                     1. All required fields are present\n\
                     2. JSON syntax is valid\n\
                     3. No extra commas or missing quotes\n\
                     \n\
                     Run 'modelmux config validate' for more details.",
                    e
                ))
            })
        } else if let Some(ref file_path) = auth.service_account_file {
            // Load from file
            let expanded_path = paths::expand_path(file_path)?;
            let file_contents = std::fs::read_to_string(&expanded_path).map_err(|e| {
                ProxyError::Config(format!(
                    "Failed to read service account file '{}': {}\n\
                     \n\
                     To fix this:\n\
                     1. Verify the file exists and is readable\n\
                     2. Check file permissions (should be 600 or similar)\n\
                     3. Ensure the path is correct\n\
                     \n\
                     Example:\n\
                       ls -la '{}'\n\
                       chmod 600 '{}'",
                    expanded_path.display(),
                    e,
                    expanded_path.display(),
                    expanded_path.display()
                ))
            })?;

            serde_json::from_str(&file_contents).map_err(|e| {
                ProxyError::Config(format!(
                    "Failed to parse service account file '{}': {}\n\
                     \n\
                     The file appears to contain invalid JSON. Please verify:\n\
                     1. The file was downloaded correctly from Google Cloud\n\
                     2. No extra characters or modifications were made\n\
                     3. The file is a valid service account key JSON\n\
                     \n\
                     Run 'modelmux config validate' for more details.",
                    expanded_path.display(),
                    e
                ))
            })
        } else {
            Err(ProxyError::Config(
                "No service account configuration found.\n\
                 \n\
                 Please configure either:\n\
                 1. auth.service_account_file = \"/path/to/service-account.json\"\n\
                 2. auth.service_account_json = \"{ ... }\" (inline JSON)\n\
                 \n\
                 Run 'modelmux config init' for interactive setup."
                    .to_string(),
            ))
        }
    }

    /// Validate the current configuration
    ///
    /// Performs comprehensive validation of all configuration values,
    /// including authentication setup, provider configuration, and
    /// network settings.
    ///
    /// # Returns
    /// * `Ok(())` - Configuration is valid
    /// * `Err(ProxyError)` - Configuration validation failed with details
    pub fn validate(&self) -> Result<()> {
        validation::ConfigValidator::new(self).validate()
    }

    /// Load service account key from the configured source
    ///
    /// Loads the Google Cloud service account key from either:
    /// 1. Inline JSON (auth.service_account_json)
    /// 2. File path (auth.service_account_file)
    ///
    /// # Returns
    /// * `Ok(ServiceAccountKey)` - Successfully loaded and parsed key
    /// * `Err(ProxyError)` - Key loading or parsing failed
    pub fn load_service_account_key(&self) -> Result<ServiceAccountKey> {
        Self::load_service_account_key_from_auth(&self.auth)
    }

    /// Get configuration file example as TOML string
    ///
    /// Returns a well-documented example configuration file that users
    /// can use as a starting point for their own configuration.
    pub fn example_toml() -> &'static str {
        r#"# ModelMux Configuration
# This file should be placed at:
#   Linux/Unix: ~/.config/modelmux/config.toml
#   macOS: ~/Library/Application Support/modelmux/config.toml
#   Windows: %APPDATA%/modelmux/config.toml

[server]
# HTTP server port (default: 3000)
port = 3000

# Logging level: trace, debug, info, warn, error (default: info)
log_level = "info"

# Enable automatic retries for quota/rate limit errors (default: true)
enable_retries = true

# Maximum number of retry attempts (default: 3)
max_retry_attempts = 3

[auth]
# Path to Google Cloud service account JSON file (recommended)
# Supports tilde (~) expansion
service_account_file = "~/.config/modelmux/service-account.json"

# Alternative: Inline service account JSON (for containers)
# service_account_json = '{"type": "service_account", ...}'

[streaming]
# Streaming mode: auto, never, standard, buffered, always (default: auto)
# - auto: detect client and choose appropriate mode
# - never: disable streaming for all clients
# - standard: word-by-word streaming
# - buffered: chunk streaming for better compatibility
# - always: force streaming for all clients
mode = "auto"

# Buffer size for buffered streaming in bytes (default: 65536)
buffer_size = 65536

# Timeout for streaming chunks in milliseconds (default: 5000)
chunk_timeout_ms = 5000

# Vertex AI provider (optional - can also use env vars or .env)
[vertex]
project = "your-gcp-project"
region = "europe-west1"
location = "europe-west1"
publisher = "anthropic"
model = "claude-3-5-sonnet@20241022"
# Or use full URL override instead:
# url = "https://europe-west1-aiplatform.googleapis.com/v1/projects/MY_PROJECT/locations/europe-west1/publishers/anthropic/models/claude-3-5-sonnet@20241022"

# Alternative: use environment variables (including from .env file):
# LLM_PROVIDER=vertex
# VERTEX_PROJECT=your-gcp-project
# VERTEX_REGION=europe-west1
# VERTEX_LOCATION=europe-west1
# VERTEX_PUBLISHER=anthropic
# VERTEX_MODEL_ID=claude-3-5-sonnet@20241022
"#
    }
}

impl LogLevel {
    /// Convert to tracing::Level for logging setup
    pub fn to_tracing_level(&self) -> tracing::Level {
        match self {
            LogLevel::Trace => tracing::Level::TRACE,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Error => tracing::Level::ERROR,
        }
    }

    /// Check if trace-level logging is enabled (backward compatibility)
    pub fn is_trace_enabled(self) -> bool {
        matches!(self, LogLevel::Trace | LogLevel::Debug)
    }

    /// Parse from string (case-insensitive)
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err(ProxyError::Config(format!(
                "Invalid log level '{}'. Valid levels are: trace, debug, info, warn, error",
                s
            ))),
        }
    }
}

impl StreamingMode {
    /// Parse from string (case-insensitive)
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(StreamingMode::Auto),
            "never" | "false" | "no" => Ok(StreamingMode::Never),
            "standard" | "normal" => Ok(StreamingMode::Standard),
            "buffered" | "buffer" => Ok(StreamingMode::Buffered),
            "always" | "true" | "yes" => Ok(StreamingMode::Always),
            _ => Err(ProxyError::Config(format!(
                "Invalid streaming mode '{}'. Valid modes are: auto, never, standard, buffered, always",
                s
            ))),
        }
    }

    /// Check if this mode supports streaming
    #[allow(dead_code)]
    pub fn is_streaming(&self) -> bool {
        !matches!(self, StreamingMode::Never)
    }

    /// Check if this mode should auto-detect client behavior
    #[allow(dead_code)]
    pub fn is_auto_detect(&self) -> bool {
        matches!(self, StreamingMode::Auto)
    }

    /// Convert to legacy NonStreaming variant (backward compatibility)
    #[allow(dead_code)]
    pub fn is_non_streaming(&self) -> bool {
        matches!(self, StreamingMode::Never)
    }
}
