//!
//! Configuration loading system for ModelMux.
//!
//! This module implements a multi-layered configuration loading system following
//! industry best practices:
//! 1. CLI arguments (highest priority)
//! 2. Environment variables
//! 3. User config file (~/.config/modelmux/config.toml)
//! 4. System config file (/etc/modelmux/config.toml)
//! 5. Built-in defaults (lowest priority)
//!
//! Follows the Builder pattern (Open/Closed Principle) and Single Responsibility
//! Principle - handles only configuration loading concerns.
//!
//! Authors:
//!   Jaro <yarenty@gmail.com>
//!
//! Copyright (c) 2026 SkyCorp

/* --- uses ------------------------------------------------------------------------------------ */

use crate::config::paths;
use crate::config::{AuthConfig, Config, LogLevel, ServerConfig, StreamingConfig, StreamingMode};
use crate::error::{ProxyError, Result};

use std::collections::HashMap;
use std::env;
use std::path::Path;

/* --- types ----------------------------------------------------------------------------------- */

///
/// Configuration loader implementing the Builder pattern.
///
/// Provides a fluent interface for building configuration from multiple sources
/// in the correct precedence order. Each method returns self for chaining.
pub struct ConfigLoader {
    /// Current configuration being built
    config: Config,
    /// Environment variable overrides collected
    env_overrides: HashMap<String, String>,
    /// Whether defaults have been applied
    defaults_applied: bool,
}

/* --- implementations --------------------------------------------------------------------- */

impl ConfigLoader {
    /// Create a new configuration loader
    ///
    /// Initializes an empty loader ready to build configuration from multiple sources.
    ///
    /// # Returns
    /// * ConfigLoader instance ready for configuration building
    ///
    /// # Examples
    /// ```rust,no_run
    /// use modelmux::config::loader::ConfigLoader;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ConfigLoader::new()
    ///     .with_defaults()
    ///     .with_user_config()?
    ///     .with_env_vars()?
    ///     .build_base()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new() -> Self {
        Self { config: Config::default(), env_overrides: HashMap::new(), defaults_applied: false }
    }

    /// Apply built-in default values
    ///
    /// Sets sensible defaults for all configuration values. This should
    /// typically be called first in the configuration loading chain.
    ///
    /// # Returns
    /// * Self for method chaining
    pub fn with_defaults(mut self) -> Self {
        self.config = Config::default();
        self.defaults_applied = true;
        self
    }

    /// Load system-wide configuration file
    ///
    /// Attempts to load configuration from the system config file:
    /// - Linux: /etc/modelmux/config.toml
    /// - macOS: /Library/Preferences/modelmux/config.toml
    /// - Windows: %PROGRAMDATA%/modelmux/config.toml
    ///
    /// If the file doesn't exist, this is not considered an error.
    ///
    /// # Returns
    /// * `Ok(Self)` - System config loaded or skipped (file not found)
    /// * `Err(ProxyError)` - System config exists but failed to load
    pub fn with_system_config(mut self) -> Result<Self> {
        let system_config_path = paths::system_config_file()?;

        if system_config_path.exists() {
            tracing::debug!("Loading system config from: {}", system_config_path.display());
            self.load_config_file(&system_config_path)?;
        } else {
            tracing::debug!("System config not found at: {}", system_config_path.display());
        }

        Ok(self)
    }

    /// Load user configuration file
    ///
    /// Attempts to load configuration from the user config file:
    /// - Linux: ~/.config/modelmux/config.toml
    /// - macOS: ~/Library/Application Support/modelmux/config.toml
    /// - Windows: %APPDATA%/modelmux/config.toml
    ///
    /// If the file doesn't exist, this is not considered an error.
    ///
    /// # Returns
    /// * `Ok(Self)` - User config loaded or skipped (file not found)
    /// * `Err(ProxyError)` - User config exists but failed to load
    pub fn with_user_config(mut self) -> Result<Self> {
        let user_config_path = paths::user_config_file()?;

        if user_config_path.exists() {
            tracing::debug!("Loading user config from: {}", user_config_path.display());
            self.load_config_file(&user_config_path)?;
        } else {
            tracing::debug!("User config not found at: {}", user_config_path.display());
        }

        Ok(self)
    }

    /// Load configuration from specific file path
    ///
    /// Loads configuration from a custom file path. Useful for testing
    /// or when users want to specify a custom config location.
    ///
    /// # Arguments
    /// * `path` - Path to configuration file to load
    ///
    /// # Returns
    /// * `Ok(Self)` - Config loaded successfully
    /// * `Err(ProxyError)` - Failed to load or parse config file
    #[allow(dead_code)]
    pub fn with_config_file<P: AsRef<Path>>(mut self, path: P) -> Result<Self> {
        let path = path.as_ref();
        tracing::debug!("Loading custom config from: {}", path.display());
        self.load_config_file(path)?;
        Ok(self)
    }

    /// Apply environment variable overrides
    ///
    /// Loads configuration values from environment variables using the
    /// MODELMUX_ prefix. Environment variables take precedence over config files.
    ///
    /// Supported environment variables:
    /// - MODELMUX_SERVER_PORT
    /// - MODELMUX_SERVER_LOG_LEVEL
    /// - MODELMUX_AUTH_SERVICE_ACCOUNT_FILE
    /// - MODELMUX_LLM_PROVIDER_PROJECT_ID
    /// - ... and more
    ///
    /// # Returns
    /// * `Ok(Self)` - Environment variables applied
    /// * `Err(ProxyError)` - Invalid environment variable values
    pub fn with_env_vars(mut self) -> Result<Self> {
        tracing::debug!("Loading configuration from environment variables");

        // Collect all MODELMUX_ environment variables
        for (key, value) in env::vars() {
            if key.starts_with("MODELMUX_") {
                self.env_overrides.insert(key, value);
            }
        }

        // Apply environment variable overrides
        self.apply_env_overrides()?;

        Ok(self)
    }

    /// Build the final configuration
    ///
    /// Validates the final configuration and returns it. This should be
    /// called last in the configuration loading chain.
    ///
    /// # Returns
    /// * `Ok(Config)` - Valid, fully-loaded configuration
    /// * `Err(ProxyError)` - Configuration validation failed
    #[allow(dead_code)]
    pub fn build(self) -> Result<Config> {
        if !self.defaults_applied {
            return Err(ProxyError::Config(
                "Configuration loader must call with_defaults() before build()".to_string(),
            ));
        }

        // Validate the final configuration
        self.config.validate()?;

        tracing::info!("Configuration loaded successfully");
        tracing::debug!(
            "Final config: server.port={}, server.log_level={:?}, streaming.mode={:?}",
            self.config.server.port,
            self.config.server.log_level,
            self.config.streaming.mode
        );

        Ok(self.config)
    }

    /// Build configuration without provider validation (for use with external provider loading)
    ///
    /// This method builds the configuration but skips provider validation,
    /// allowing the provider to be loaded separately using the existing system.
    ///
    /// # Returns
    /// * `Ok(Config)` - Configuration with basic validation
    /// * `Err(ProxyError)` - Configuration loading failed
    pub fn build_base(self) -> Result<Config> {
        if !self.defaults_applied {
            return Err(ProxyError::Config(
                "Configuration loader must call with_defaults() before build()".to_string(),
            ));
        }

        tracing::info!("Base configuration loaded successfully");
        tracing::debug!(
            "Base config: server.port={}, server.log_level={:?}, streaming.mode={:?}",
            self.config.server.port,
            self.config.server.log_level,
            self.config.streaming.mode
        );

        Ok(self.config)
    }

    /* --- private methods ----------------------------------------------------------------- */

    /// Load and merge configuration from a TOML file
    fn load_config_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();

        // Validate file exists and is readable
        paths::validate_config_file(path)?;

        // Read file contents
        let contents = std::fs::read_to_string(path).map_err(|e| {
            ProxyError::Config(format!(
                "Failed to read configuration file '{}': {}",
                path.display(),
                e
            ))
        })?;

        // Parse TOML
        let file_config: Config = toml::from_str(&contents).map_err(|e| {
            ProxyError::Config(format!(
                "Failed to parse TOML configuration file '{}': {}\n\
                 \n\
                 Please check the syntax of your configuration file.\n\
                 Common issues:\n\
                 1. Missing quotes around string values\n\
                 2. Invalid TOML syntax\n\
                 3. Incorrect section names or field names\n\
                 \n\
                 You can validate your TOML syntax at: https://www.toml-lint.com/\n\
                 \n\
                 Run 'modelmux config validate' for more details.",
                path.display(),
                e
            ))
        })?;

        // Merge configuration (file config overrides current config)
        self.merge_config(file_config);

        tracing::debug!("Successfully loaded config from: {}", path.display());
        Ok(())
    }

    /// Merge another config into the current config
    fn merge_config(&mut self, other: Config) {
        // Merge server config
        self.merge_server_config(other.server);

        // Merge LLM provider config if present
        if other.llm_provider.is_some() {
            self.config.llm_provider = other.llm_provider;
        }

        // Merge vertex config if present
        if other.vertex.is_some() {
            self.config.vertex = other.vertex;
        }

        // Merge auth config
        self.merge_auth_config(other.auth);

        // Merge streaming config
        self.merge_streaming_config(other.streaming);
    }

    /// Merge server configuration
    fn merge_server_config(&mut self, other: ServerConfig) {
        // Only override if the value is explicitly set (not default)
        if other.port != ServerConfig::default().port {
            self.config.server.port = other.port;
        }

        // For enums, we need to check if they're different from default
        // Since we can't easily detect "explicitly set", we always merge
        self.config.server.log_level = other.log_level;
        self.config.server.enable_retries = other.enable_retries;

        if other.max_retry_attempts != ServerConfig::default().max_retry_attempts {
            self.config.server.max_retry_attempts = other.max_retry_attempts;
        }
    }

    /// Merge authentication configuration
    fn merge_auth_config(&mut self, other: AuthConfig) {
        if other.service_account_file.is_some() {
            self.config.auth.service_account_file = other.service_account_file;
        }

        if other.service_account_json.is_some() {
            self.config.auth.service_account_json = other.service_account_json;
        }

        // Always merge strategy
        self.config.auth.strategy = other.strategy;
    }

    /// Merge streaming configuration
    fn merge_streaming_config(&mut self, other: StreamingConfig) {
        self.config.streaming.mode = other.mode;

        if other.buffer_size != StreamingConfig::default().buffer_size {
            self.config.streaming.buffer_size = other.buffer_size;
        }

        if other.chunk_timeout_ms != StreamingConfig::default().chunk_timeout_ms {
            self.config.streaming.chunk_timeout_ms = other.chunk_timeout_ms;
        }
    }

    /// Apply environment variable overrides to current configuration
    fn apply_env_overrides(&mut self) -> Result<()> {
        for (key, value) in &self.env_overrides {
            match key.as_str() {
                // Server configuration
                "MODELMUX_SERVER_PORT" => {
                    self.config.server.port = value.parse().map_err(|e| {
                        ProxyError::Config(format!(
                            "Invalid MODELMUX_SERVER_PORT value '{}': {}\n\
                             Port must be a number between 1 and 65535.",
                            value, e
                        ))
                    })?;
                }
                "MODELMUX_SERVER_LOG_LEVEL" => {
                    self.config.server.log_level = LogLevel::from_str(value)?;
                }
                "MODELMUX_SERVER_ENABLE_RETRIES" => {
                    self.config.server.enable_retries = parse_bool_env(value, key)?;
                }
                "MODELMUX_SERVER_MAX_RETRY_ATTEMPTS" => {
                    self.config.server.max_retry_attempts = value.parse().map_err(|e| {
                        ProxyError::Config(format!(
                            "Invalid MODELMUX_SERVER_MAX_RETRY_ATTEMPTS value '{}': {}",
                            value, e
                        ))
                    })?;
                }

                // Authentication configuration
                "MODELMUX_AUTH_SERVICE_ACCOUNT_FILE" => {
                    self.config.auth.service_account_file = Some(value.clone());
                }
                "MODELMUX_AUTH_SERVICE_ACCOUNT_JSON" => {
                    self.config.auth.service_account_json = Some(value.clone());
                }

                // Streaming configuration
                "MODELMUX_STREAMING_MODE" => {
                    self.config.streaming.mode = StreamingMode::from_str(value)?;
                }
                "MODELMUX_STREAMING_BUFFER_SIZE" => {
                    self.config.streaming.buffer_size = value.parse().map_err(|e| {
                        ProxyError::Config(format!(
                            "Invalid MODELMUX_STREAMING_BUFFER_SIZE value '{}': {}",
                            value, e
                        ))
                    })?;
                }
                "MODELMUX_STREAMING_CHUNK_TIMEOUT_MS" => {
                    self.config.streaming.chunk_timeout_ms = value.parse().map_err(|e| {
                        ProxyError::Config(format!(
                            "Invalid MODELMUX_STREAMING_CHUNK_TIMEOUT_MS value '{}': {}",
                            value, e
                        ))
                    })?;
                }

                // LLM Provider configuration (delegate to provider)
                key if key.starts_with("MODELMUX_LLM_PROVIDER_") => {
                    // Let the LlmProviderConfig handle its own env vars
                    // This will be handled when LlmProviderConfig::from_env() is called
                    tracing::debug!("LLM provider env var will be handled by provider: {}", key);
                }

                // Legacy environment variables for backward compatibility
                "GCP_SERVICE_ACCOUNT_KEY" => {
                    tracing::warn!(
                        "GCP_SERVICE_ACCOUNT_KEY is deprecated. Please use MODELMUX_AUTH_SERVICE_ACCOUNT_JSON or config file."
                    );
                    self.config.auth.service_account_json = Some(value.clone());
                }
                "PORT" => {
                    tracing::warn!(
                        "PORT environment variable is deprecated. Please use MODELMUX_SERVER_PORT."
                    );
                    self.config.server.port = value.parse().map_err(|e| {
                        ProxyError::Config(format!("Invalid PORT value '{}': {}", value, e))
                    })?;
                }

                // Unknown environment variable
                _ => {
                    tracing::debug!("Ignoring unknown environment variable: {}", key);
                }
            }
        }

        Ok(())
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

/* --- utility functions ------------------------------------------------------------------- */

/// Parse boolean value from environment variable
fn parse_bool_env(value: &str, var_name: &str) -> Result<bool> {
    match value.to_lowercase().as_str() {
        "true" | "yes" | "1" | "on" | "enabled" => Ok(true),
        "false" | "no" | "0" | "off" | "disabled" => Ok(false),
        _ => Err(ProxyError::Config(format!(
            "Invalid boolean value for {}: '{}'\n\
             Valid values: true/false, yes/no, 1/0, on/off, enabled/disabled",
            var_name, value
        ))),
    }
}

/* --- tests ------------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_config_loader_defaults() {
        let config =
            ConfigLoader::new().with_defaults().build_base().expect("Should build with defaults");

        assert_eq!(config.server.port, 3000);
        assert!(matches!(config.server.log_level, LogLevel::Info));
        assert!(config.server.enable_retries);
        assert_eq!(config.server.max_retry_attempts, 3);
    }

    #[test]
    fn test_config_loader_with_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("config.toml");

        let config_content = r#"
[server]
port = 8080
log_level = "debug"

[auth]
service_account_json = '{"type":"service_account","project_id":"test","private_key_id":"test","private_key":"-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----","client_email":"test@test.com","client_id":"test","auth_uri":"https://accounts.google.com/o/oauth2/auth","token_uri":"https://oauth2.googleapis.com/token","auth_provider_x509_cert_url":"https://www.googleapis.com/oauth2/v1/certs","client_x509_cert_url":"test"}'

[streaming]
mode = "standard"
"#;

        fs::write(&config_file, config_content).unwrap();

        let config = ConfigLoader::new()
            .with_defaults()
            .with_config_file(&config_file)
            .expect("Should create loader")
            .build_base()
            .expect("Should load custom config file");

        assert_eq!(config.server.port, 8080);
        assert!(matches!(config.server.log_level, LogLevel::Debug));
        assert!(matches!(config.streaming.mode, StreamingMode::Standard));
    }

    #[test]
    fn test_env_var_overrides() {
        temp_env::with_vars(
            [
                ("MODELMUX_SERVER_PORT", Some("9090")),
                ("MODELMUX_SERVER_LOG_LEVEL", Some("error")),
                ("MODELMUX_STREAMING_MODE", Some("never")),
                (
                    "MODELMUX_AUTH_SERVICE_ACCOUNT_JSON",
                    Some(
                        r#"{"type":"service_account","project_id":"test","private_key_id":"test","private_key":"-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----","client_email":"test@test.com","client_id":"test","auth_uri":"https://accounts.google.com/o/oauth2/auth","token_uri":"https://oauth2.googleapis.com/token","auth_provider_x509_cert_url":"https://www.googleapis.com/oauth2/v1/certs","client_x509_cert_url":"test"}"#,
                    ),
                ),
            ],
            || {
                let config = ConfigLoader::new()
                    .with_defaults()
                    .with_env_vars()
                    .expect("Should apply env vars")
                    .build_base()
                    .expect("Should build with env vars");

                assert_eq!(config.server.port, 9090);
                assert!(matches!(config.server.log_level, LogLevel::Error));
                assert!(matches!(config.streaming.mode, StreamingMode::Never));
            },
        );
    }

    #[test]
    fn test_precedence_order() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("config.toml");

        // Config file sets port to 7070
        let config_content = r#"
[server]
port = 7070

[auth]
service_account_json = '{"type":"service_account","project_id":"test","private_key_id":"test","private_key":"-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----","client_email":"test@test.com","client_id":"test","auth_uri":"https://accounts.google.com/o/oauth2/auth","token_uri":"https://oauth2.googleapis.com/token","auth_provider_x509_cert_url":"https://www.googleapis.com/oauth2/v1/certs","client_x509_cert_url":"test"}'

[streaming]
mode = "auto"
"#;
        fs::write(&config_file, config_content).unwrap();

        // Environment variable should override config file
        temp_env::with_vars([("MODELMUX_SERVER_PORT", Some("8080"))], || {
            let config = ConfigLoader::new()
                .with_defaults()
                .with_config_file(&config_file)
                .expect("Should create loader")
                .with_env_vars()
                .expect("Should apply env vars")
                .build_base()
                .expect("Should build with precedence");

            // Env var should win over config file
            assert_eq!(config.server.port, 8080);
        });
    }

    #[test]
    fn test_invalid_toml_error() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("config.toml");

        // Invalid TOML content
        let invalid_content = r#"
[server
port = 8080
"#;
        fs::write(&config_file, invalid_content).unwrap();

        let result = ConfigLoader::new()
            .with_defaults()
            .with_config_file(&config_file)
            .and_then(|loader| loader.build_base());

        assert!(result.is_err());
        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("Failed to parse TOML"));
    }

    #[test]
    fn test_boolean_env_parsing() {
        assert!(parse_bool_env("true", "TEST").unwrap());
        assert!(parse_bool_env("yes", "TEST").unwrap());
        assert!(parse_bool_env("1", "TEST").unwrap());
        assert!(parse_bool_env("on", "TEST").unwrap());
        assert!(parse_bool_env("enabled", "TEST").unwrap());

        assert!(!parse_bool_env("false", "TEST").unwrap());
        assert!(!parse_bool_env("no", "TEST").unwrap());
        assert!(!parse_bool_env("0", "TEST").unwrap());
        assert!(!parse_bool_env("off", "TEST").unwrap());
        assert!(!parse_bool_env("disabled", "TEST").unwrap());

        assert!(parse_bool_env("invalid", "TEST").is_err());
    }
}
