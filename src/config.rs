//!
//! Configuration management for the Vertex AI to OpenAI proxy server.
//!
//! Handles loading configuration from environment variables with sensible defaults.
//! Follows Single Responsibility Principle - manages all configuration concerns.
//!
//! Authors:
//!   Jaro <yarenty@gmail.com>
//!
//! Copyright (c) 2026 SkyCorp

/* --- uses ------------------------------------------------------------------------------------ */

use std::env;

use serde::{Deserialize, Serialize};

use crate::error::{ProxyError, Result};

/* --- types ----------------------------------------------------------------------------------- */

///
/// Application configuration structure.
///
/// Contains all runtime configuration options loaded from environment variables.
/// Provides methods for loading and validating configuration settings.
#[derive(Debug, Clone)]
pub struct Config {
    /** the base URL for Vertex AI API endpoints */
    pub llm_url: String,
    /** the specific chat endpoint path for LLM requests */
    pub llm_chat_endpoint: String,
    /** the LLM model identifier */
    pub llm_model: String,
    /** Google Cloud service account credentials */
    pub service_account_key: ServiceAccountKey,
    /** HTTP server port number */
    pub port: u16,
    /** application logging level */
    pub log_level: LogLevel,
    /** whether to enable retry logic for quota errors */
    pub enable_retries: bool,
    /** maximum retry attempts for quota errors */
    pub max_retry_attempts: u32,
    /** streaming mode configuration */
    pub streaming_mode: StreamingMode,
}

///
/// Streaming mode configuration.
///
/// Controls how the proxy handles streaming responses for different clients.
/// This is to support current version in:
/// - RustRover - where streaming need to be buffered in bigger chunks (hopefully it is temporary)
/// - Goose - where it says "streaming=true", but in reality it expects whole text/tool calls as
///   single ones!
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamingMode {
    /** Auto-detect client and choose appropriate streaming mode */
    Auto,
    /** Force all requests to use non-streaming responses */
    NonStreaming,
    /** Use standard word-by-word streaming */
    Standard,
    /** Use buffered streaming for better client compatibility */
    Buffered,
}

///
/// Google Cloud service account key structure.
///
/// Contains all fields required for OAuth2 authentication with Google Cloud Platform.
#[derive(Debug, Clone)]
pub struct ServiceAccountKey {
    /** Google Cloud project identifier */
    pub project_id: String,
    /** unique identifier for the private key */
    pub private_key_id: String,
    /** PEM-encoded private key for signing */
    pub private_key: String,
    /** service account email address */
    pub client_email: String,
    /** OAuth2 client identifier */
    pub client_id: String,
    /** OAuth2 authorization URI */
    pub auth_uri: String,
    /** OAuth2 token endpoint URI */
    pub token_uri: String,
    /** X.509 certificate URL for auth provider */
    pub auth_provider_x509_cert_url: String,
    /** X.509 certificate URL for this client */
    pub client_x509_cert_url: String,
}

///
/// Logging level enumeration.
///
/// Defines available log levels with helper methods for level checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

///
/// Service account key JSON structure for deserialization.
///
/// Internal structure used when parsing the base64-encoded service account key.
#[derive(Debug, Deserialize, Serialize)]
struct ServiceAccountKeyJson {
    #[serde(rename = "type")]
    /** the key type, should be "service_account" */
    key_type: String,
    /** Google Cloud project identifier */
    project_id: String,
    /** unique identifier for the private key */
    private_key_id: String,
    /** PEM-encoded private key for signing */
    private_key: String,
    /** service account email address */
    client_email: String,
    /** OAuth2 client identifier */
    client_id: String,
    /** OAuth2 authorization URI */
    auth_uri: String,
    /** OAuth2 token endpoint URI */
    token_uri: String,
    /** X.509 certificate URL for auth provider */
    auth_provider_x509_cert_url: String,
    /** X.509 certificate URL for this client */
    client_x509_cert_url: String,
}

/* --- start of code -------------------------------------------------------------------------- */

impl From<&str> for StreamingMode {
    ///
    /// Convert string representation to StreamingMode enum.
    ///
    /// Case-insensitive conversion with Auto as the default fallback.
    ///
    /// # Arguments
    ///  * `s` - string representation of streaming mode
    ///
    /// # Returns
    ///  * Corresponding StreamingMode enum value
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "auto" => StreamingMode::Auto,
            "non-streaming" | "nonstreaming" | "none" => StreamingMode::NonStreaming,
            "standard" | "std" => StreamingMode::Standard,
            "buffered" | "buffer" => StreamingMode::Buffered,
            _ => StreamingMode::Auto,
        }
    }
}

impl LogLevel {
    ///
    /// Check if trace-level logging is enabled.
    ///
    /// Returns true for Trace and Debug levels, which enable detailed logging
    /// of tool calls and API interactions.
    ///
    /// # Returns
    ///  * `true` if trace logging should be enabled
    ///  * `false` otherwise
    pub fn is_trace_enabled(self) -> bool {
        matches!(self, LogLevel::Trace | LogLevel::Debug)
    }
}

impl From<&str> for LogLevel {
    ///
    /// Convert string representation to LogLevel enum.
    ///
    /// Case-insensitive conversion with Info as the default fallback.
    ///
    /// # Arguments
    ///  * `s` - string representation of log level
    ///
    /// # Returns
    ///  * Corresponding LogLevel enum value
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "trace" => LogLevel::Trace,
            "debug" => LogLevel::Debug,
            "info" => LogLevel::Info,
            "warn" => LogLevel::Warn,
            "error" => LogLevel::Error,
            _ => LogLevel::Info,
        }
    }
}

impl Config {
    ///
    /// Load configuration from environment variables.
    ///
    /// Attempts to load .env file if present, then reads configuration from
    /// environment variables with sensible defaults. Follows Open/Closed
    /// Principle - can be extended without modification.
    ///
    /// # Returns
    ///  * Configuration object with all settings loaded
    ///  * `ProxyError::Config` if required variables are missing or invalid
    pub fn from_env() -> Result<Self> {
        let _ = dotenvy::dotenv();

        let service_account_key = Self::load_service_account_key()?;
        let llm_url = Self::get_llm_url()?;
        let llm_chat_endpoint = Self::get_llm_chat_endpoint()?;
        let llm_model = Self::get_llm_model()?;
        let port = Self::get_port()?;
        let log_level = Self::get_log_level();
        let enable_retries = Self::get_enable_retries();
        let max_retry_attempts = Self::get_max_retry_attempts();
        let streaming_mode = Self::get_streaming_mode();

        Ok(Config {
            llm_url,
            llm_chat_endpoint,
            llm_model,
            service_account_key,
            port,
            log_level,
            enable_retries,
            max_retry_attempts,
            streaming_mode,
        })
    }

    ///
    /// Build the full Vertex AI URL for a request.
    ///
    /// Constructs the complete URL by combining the base URL with the chat endpoint,
    /// adjusting the endpoint based on whether streaming is requested.
    ///
    /// # Arguments
    ///  * `is_streaming` - whether this is for a streaming request
    ///
    /// # Returns
    ///  * Complete URL string for the Vertex AI endpoint
    pub fn build_vertex_url(&self, is_streaming: bool) -> String {
        let endpoint = if is_streaming { "streamRawPredict" } else { "rawPredict" };

        let chat_endpoint = self
            .llm_chat_endpoint
            .replace(":streamRawPredict", &format!(":{}", endpoint))
            .replace(":rawPredict", &format!(":{}", endpoint));

        format!("{}{}", self.llm_url, chat_endpoint)
    }

    ///
    /// Load and decode the Google Cloud service account key.
    ///
    /// Reads the base64-encoded service account key from environment variable
    /// and decodes it into a usable structure.
    ///
    /// # Returns
    ///  * Decoded service account key structure
    ///  * `ProxyError::Config` if key is missing, invalid, or malformed
    fn load_service_account_key() -> Result<ServiceAccountKey> {
        let service_account_key_b64 = env::var("GCP_SERVICE_ACCOUNT_KEY").map_err(|_| {
            ProxyError::Config(
                "GCP_SERVICE_ACCOUNT_KEY environment variable is not set.\n\
         \n\
         To fix this:\n\
           1. Get your Google Cloud service account key JSON file\n\
           2. Encode it to base64: cat key.json | base64\n\
           3. Set the environment variable:\n\
              export GCP_SERVICE_ACCOUNT_KEY=\"your-base64-encoded-key\"\n\
           4. Or add it to a .env file:\n\
              GCP_SERVICE_ACCOUNT_KEY=\"your-base64-encoded-key\"\n\
         \n\
         Run 'modelmux doctor' for more help."
                    .to_string(),
            )
        })?;

        Self::decode_service_account_key(&service_account_key_b64)
    }

    ///
    /// Get the LLM base URL from environment.
    ///
    /// # Returns
    ///  * LLM base URL string
    ///  * `ProxyError::Config` if LLM_URL is not set
    fn get_llm_url() -> Result<String> {
        env::var("LLM_URL").map_err(|_| {
            ProxyError::Config(
                "LLM_URL environment variable is not set.\n\
         \n\
         To fix this:\n\
           1. Get your Vertex AI endpoint URL from Google Cloud Console\n\
           2. Format: https://REGION-aiplatform.googleapis.com/v1/projects/PROJECT/locations/LOCATION/publishers/\n\
           3. Set the environment variable:\n\
              export LLM_URL=\"https://your-region-aiplatform.googleapis.com/v1/projects/your-project/locations/your-location/publishers/\"\n\
           4. Or add it to a .env file\n\
         \n\
         Run 'modelmux doctor' for more help."
                    .to_string(),
            )
        })
    }

    ///
    /// Get the LLM chat endpoint from environment.
    ///
    /// # Returns
    ///  * LLM chat endpoint string
    ///  * `ProxyError::Config` if LLM_CHAT_ENDPOINT is not set
    fn get_llm_chat_endpoint() -> Result<String> {
        env::var("LLM_CHAT_ENDPOINT").map_err(|_| {
            ProxyError::Config(
                "LLM_CHAT_ENDPOINT environment variable is not set.\n\
         \n\
         To fix this:\n\
           1. Format: MODEL_NAME:streamRawPredict or MODEL_NAME:rawPredict\n\
           2. Example: claude-sonnet-4:streamRawPredict\n\
           3. Set the environment variable:\n\
              export LLM_CHAT_ENDPOINT=\"your-model:streamRawPredict\"\n\
           4. Or add it to a .env file\n\
         \n\
         Run 'modelmux doctor' for more help."
                    .to_string(),
            )
        })
    }

    ///
    /// Get the LLM model identifier from environment.
    ///
    /// # Returns
    ///  * LLM model identifier string
    ///  * `ProxyError::Config` if LLM_MODEL is not set
    fn get_llm_model() -> Result<String> {
        env::var("LLM_MODEL").map_err(|_| {
            ProxyError::Config(
                "LLM_MODEL environment variable is not set.\n\
         \n\
         To fix this:\n\
           1. Set the model identifier (e.g., claude-sonnet-4)\n\
           2. Set the environment variable:\n\
              export LLM_MODEL=\"claude-sonnet-4\"\n\
           3. Or add it to a .env file\n\
         \n\
         Run 'modelmux doctor' for more help."
                    .to_string(),
            )
        })
    }

    ///
    /// Get the server port from environment or use default.
    ///
    /// # Returns
    ///  * Port number as u16
    ///  * `ProxyError::Config` if port value is invalid
    fn get_port() -> Result<u16> {
        env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .map_err(|e| {
                ProxyError::Config(format!(
                    "Invalid PORT value: {}\n\
         \n\
         PORT must be a number between 1 and 65535.\n\
            Example: export PORT=3000\n\
         \n\
         Run 'modelmux doctor' for more help.",
                    e
                ))
            })
    }

    ///
    /// Get the log level from environment or use default.
    ///
    /// # Returns
    ///  * LogLevel enum value
    fn get_log_level() -> LogLevel {
        let log_level_str = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
        LogLevel::from(log_level_str.as_str())
    }

    ///
    /// Get retry enablement from environment or use default.
    ///
    /// # Returns
    ///  * Whether retries are enabled
    fn get_enable_retries() -> bool {
        env::var("ENABLE_RETRIES").unwrap_or_else(|_| "true".to_string()).parse().unwrap_or(true)
    }

    ///
    /// Get maximum retry attempts from environment or use default.
    ///
    /// # Returns
    ///  * Maximum retry attempts
    fn get_max_retry_attempts() -> u32 {
        env::var("MAX_RETRY_ATTEMPTS").unwrap_or_else(|_| "3".to_string()).parse().unwrap_or(3)
    }

    ///
    /// Get streaming mode from environment or use default.
    ///
    /// # Returns
    ///  * StreamingMode enum value
    fn get_streaming_mode() -> StreamingMode {
        let mode_str = env::var("STREAMING_MODE").unwrap_or_else(|_| "auto".to_string());
        StreamingMode::from(mode_str.as_str())
    }

    ///
    /// Decode base64-encoded service account key into structured format.
    ///
    /// Takes a base64-encoded JSON string and converts it into a ServiceAccountKey
    /// structure for use with OAuth2 authentication.
    ///
    /// # Arguments
    ///  * `key_b64` - base64-encoded service account key JSON
    ///
    /// # Returns
    ///  * Decoded and structured service account key
    ///  * `ProxyError::Config` if decoding or parsing fails
    fn decode_service_account_key(key_b64: &str) -> Result<ServiceAccountKey> {
        use base64::Engine;

        let decoded = base64::engine::general_purpose::STANDARD.decode(key_b64).map_err(|e| {
            ProxyError::Config(format!(
                "Failed to decode base64 service account key: {}\n\
         \n\
         To fix this:\n\
           1. Ensure your key is properly base64-encoded\n\
           2. Encode your JSON key file: cat key.json | base64\n\
           3. Verify the encoded string doesn't have newlines or spaces\n\
         \n\
         Run 'modelmux doctor' for more help.",
                e
            ))
        })?;

        let key_json: ServiceAccountKeyJson = serde_json::from_slice(&decoded).map_err(|e| {
            ProxyError::Config(format!(
                "Failed to parse service account key JSON: {}\n\
         \n\
         To fix this:\n\
           1. Verify your service account key is valid JSON\n\
           2. Ensure it contains all required fields:\n\
              - type, project_id, private_key_id, private_key\n\
              - client_email, client_id, auth_uri, token_uri\n\
           3. Download a fresh key from Google Cloud Console if needed\n\
         \n\
         Run 'modelmux doctor' for more help.",
                e
            ))
        })?;

        Ok(ServiceAccountKey {
            project_id: key_json.project_id,
            private_key_id: key_json.private_key_id,
            private_key: key_json.private_key,
            client_email: key_json.client_email,
            client_id: key_json.client_id,
            auth_uri: key_json.auth_uri,
            token_uri: key_json.token_uri,
            auth_provider_x509_cert_url: key_json.auth_provider_x509_cert_url,
            client_x509_cert_url: key_json.client_x509_cert_url,
        })
    }

    ///
    /// Validate configuration and return detailed validation results.
    ///
    /// Checks all configuration values for correctness and provides helpful
    /// suggestions for any issues found.
    ///
    /// # Returns
    ///  * Vector of validation issues (empty if all valid)
    pub fn validate(&self) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Validate service account key structure
        if self.service_account_key.private_key.is_empty() {
            issues.push(ValidationIssue {
                field: "GCP_SERVICE_ACCOUNT_KEY".to_string(),
                severity: ValidationSeverity::Error,
                message: "Service account key private_key is empty".to_string(),
                suggestion: Some("Ensure your service account key JSON contains a valid private_key field".to_string()),
            });
        }

        if !self.service_account_key.client_email.contains('@') {
            issues.push(ValidationIssue {
                field: "GCP_SERVICE_ACCOUNT_KEY".to_string(),
                severity: ValidationSeverity::Error,
                message: format!("Invalid client_email format: {}", self.service_account_key.client_email),
                suggestion: Some("Service account email should be in format: name@project-id.iam.gserviceaccount.com".to_string()),
            });
        }

        // Validate URLs
        if !self.llm_url.starts_with("https://") {
            issues.push(ValidationIssue {
                field: "LLM_URL".to_string(),
                severity: ValidationSeverity::Warning,
                message: format!("LLM_URL should use HTTPS: {}", self.llm_url),
                suggestion: Some("Use https:// for secure connections".to_string()),
            });
        }

        if !self.llm_url.contains("aiplatform.googleapis.com") {
            issues.push(ValidationIssue {
                field: "LLM_URL".to_string(),
                severity: ValidationSeverity::Warning,
                message: format!("LLM_URL doesn't appear to be a Vertex AI URL: {}", self.llm_url),
                suggestion: Some("Verify this is the correct Vertex AI endpoint URL".to_string()),
            });
        }

        // Validate endpoint format
        if !self.llm_chat_endpoint.contains(':') {
            issues.push(ValidationIssue {
                field: "LLM_CHAT_ENDPOINT".to_string(),
                severity: ValidationSeverity::Warning,
                message: format!("LLM_CHAT_ENDPOINT should include model name and method: {}", self.llm_chat_endpoint),
                suggestion: Some("Format should be: model-name:streamRawPredict or model-name:rawPredict".to_string()),
            });
        }

        // Validate port range
        // Note: port is u16, so max value is 65535 (enforced by type system)
        if self.port == 0 {
            issues.push(ValidationIssue {
                field: "PORT".to_string(),
                severity: ValidationSeverity::Error,
                message: "Port cannot be 0".to_string(),
                suggestion: Some("Use a valid port number between 1 and 65535".to_string()),
            });
        }

        // Validate retry configuration
        if self.max_retry_attempts == 0 && self.enable_retries {
            issues.push(ValidationIssue {
                field: "MAX_RETRY_ATTEMPTS".to_string(),
                severity: ValidationSeverity::Warning,
                message: "Retries enabled but max_retry_attempts is 0".to_string(),
                suggestion: Some("Set MAX_RETRY_ATTEMPTS to a value > 0 or disable retries".to_string()),
            });
        }

        if self.max_retry_attempts > 10 {
            issues.push(ValidationIssue {
                field: "MAX_RETRY_ATTEMPTS".to_string(),
                severity: ValidationSeverity::Warning,
                message: format!("MAX_RETRY_ATTEMPTS ({}) is very high", self.max_retry_attempts),
                suggestion: Some("Consider using a lower value (3-5) to avoid excessive retries".to_string()),
            });
        }

        issues
    }
}

///
/// Configuration validation issue.
///
/// Represents a single validation problem found during configuration check.
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    /// Configuration field name
    pub field: String,
    /// Severity of the issue
    pub severity: ValidationSeverity,
    /// Description of the issue
    pub message: String,
    /// Optional suggestion for fixing the issue
    pub suggestion: Option<String>,
}

///
/// Severity level for validation issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationSeverity {
    /// Error - configuration is invalid and will cause failures
    Error,
    /// Warning - configuration may work but has potential issues
    Warning,
    /// Info - informational note about configuration
    Info,
}
