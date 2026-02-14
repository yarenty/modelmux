//!
//! Configuration validation for ModelMux.
//!
//! This module provides comprehensive validation of configuration values,
//! including authentication setup, provider configuration, network settings,
//! and security requirements. Follows Single Responsibility Principle -
//! handles only configuration validation concerns.
//!
//! Authors:
//!   Jaro <yarenty@gmail.com>
//!
//! Copyright (c) 2026 SkyCorp

/* --- uses ------------------------------------------------------------------------------------ */

use crate::config::paths;
use crate::config::{Config, LogLevel, StreamingMode};
use crate::error::{ProxyError, Result};
use std::path::Path;

/* --- types ----------------------------------------------------------------------------------- */

///
/// Configuration validator implementing comprehensive validation rules.
///
/// Validates all aspects of the configuration including:
/// - Network settings (ports, timeouts)
/// - Authentication configuration
/// - File permissions and accessibility
/// - Provider-specific requirements
/// - Security constraints
pub struct ConfigValidator<'a> {
    /// Configuration to validate
    config: &'a Config,
    /// Validation errors collected during validation
    errors: Vec<String>,
    /// Validation warnings collected during validation
    warnings: Vec<String>,
}

/* --- implementations --------------------------------------------------------------------- */

impl<'a> ConfigValidator<'a> {
    /// Create a new configuration validator
    ///
    /// # Arguments
    /// * `config` - Configuration to validate
    ///
    /// # Returns
    /// * ConfigValidator instance ready for validation
    pub fn new(config: &'a Config) -> Self {
        Self { config, errors: Vec::new(), warnings: Vec::new() }
    }

    /// Perform comprehensive configuration validation
    ///
    /// Validates all configuration aspects and returns detailed error information
    /// if validation fails. Collects all validation issues before returning.
    ///
    /// # Returns
    /// * `Ok(())` - Configuration is valid
    /// * `Err(ProxyError)` - Configuration validation failed with detailed errors
    pub fn validate(mut self) -> Result<()> {
        // Validate each configuration section
        self.validate_server_config();
        self.validate_auth_config();
        self.validate_streaming_config();
        self.validate_security_requirements();

        // Report warnings
        for warning in &self.warnings {
            tracing::warn!("Configuration warning: {}", warning);
        }

        // Check if there were any validation errors
        if !self.errors.is_empty() {
            let error_msg = format!(
                "Configuration validation failed with {} error(s):\n\n{}\n\
                 \n\
                 Please fix these issues and try again.\n\
                 Run 'modelmux config init' for interactive configuration setup.",
                self.errors.len(),
                self.errors
                    .iter()
                    .enumerate()
                    .map(|(i, e)| format!("{}. {}", i + 1, e))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            return Err(ProxyError::Config(error_msg));
        }

        tracing::info!("Configuration validation passed");
        if !self.warnings.is_empty() {
            tracing::info!("Configuration has {} warning(s) but is valid", self.warnings.len());
        }

        Ok(())
    }

    /* --- private validation methods ------------------------------------------------------ */

    /// Validate server configuration
    fn validate_server_config(&mut self) {
        let server = &self.config.server;

        // Validate port range
        if server.port == 0 {
            self.add_error(format!(
                "Invalid server port {}: must be between 1 and 65535",
                server.port
            ));
        }

        // Warn about privileged ports
        if server.port < 1024 {
            self.add_warning(format!(
                "Server port {} requires root/administrator privileges",
                server.port
            ));
        }

        // Warn about common conflicting ports
        match server.port {
            80 | 443 => {
                self.add_warning(format!(
                    "Port {} is commonly used by web servers and may conflict",
                    server.port
                ));
            }
            22 => {
                self.add_warning("Port 22 is used by SSH and may conflict".to_string());
            }
            25 | 587 | 465 => {
                self.add_warning(format!(
                    "Port {} is used by mail servers and may conflict",
                    server.port
                ));
            }
            _ => {}
        }

        // Validate retry attempts
        if server.max_retry_attempts > 10 {
            self.add_warning(format!(
                "High retry count ({}): may cause long delays on failures",
                server.max_retry_attempts
            ));
        }

        // Log level validation is implicit (enum ensures validity)
        tracing::debug!("Server config validation completed");
    }

    /// Validate authentication configuration
    fn validate_auth_config(&mut self) {
        let auth = &self.config.auth;

        // Must have either service account file or inline JSON
        let has_file = auth.service_account_file.is_some();
        let has_json = auth.service_account_json.is_some();

        if !has_file && !has_json {
            self.add_error(
                "No service account configuration found. Please set either:\n\
                 - auth.service_account_file = \"/path/to/service-account.json\"\n\
                 - auth.service_account_json = \"{ ... }\" (inline JSON)"
                    .to_string(),
            );
            return; // Can't validate further without auth config
        }

        // Validate service account file if specified
        if let Some(ref file_path) = auth.service_account_file {
            self.validate_service_account_file(file_path);
        }

        // Validate inline JSON if specified
        if let Some(ref json_str) = auth.service_account_json {
            self.validate_service_account_json(json_str);
        }

        // Warn if both are specified
        if has_file && has_json {
            self.add_warning(
                "Both service_account_file and service_account_json are specified. \
                 service_account_json will take precedence."
                    .to_string(),
            );
        }

        tracing::debug!("Auth config validation completed");
    }

    /// Validate service account file configuration
    fn validate_service_account_file(&mut self, file_path: &str) {
        // Expand path (handle ~, environment variables)
        let expanded_path = match paths::expand_path(file_path) {
            Ok(path) => path,
            Err(e) => {
                self.add_error(format!(
                    "Failed to expand service account file path '{}': {}",
                    file_path, e
                ));
                return;
            }
        };

        // Check if file exists
        if !expanded_path.exists() {
            self.add_error(format!(
                "Service account file not found: '{}'\n\
                 \n\
                 To fix this:\n\
                 1. Download your Google Cloud service account key JSON\n\
                 2. Save it to the specified path\n\
                 3. Ensure the file is readable\n\
                 \n\
                 Example:\n\
                   mkdir -p ~/.config/modelmux\n\
                   cp /path/to/downloaded-key.json ~/.config/modelmux/service-account.json\n\
                   chmod 600 ~/.config/modelmux/service-account.json",
                expanded_path.display()
            ));
            return;
        }

        // Check if it's a regular file
        if !expanded_path.is_file() {
            self.add_error(format!(
                "Service account path exists but is not a regular file: '{}'",
                expanded_path.display()
            ));
            return;
        }

        // Validate file permissions
        self.validate_file_permissions(&expanded_path);

        // Try to read and parse the file
        match std::fs::read_to_string(&expanded_path) {
            Ok(contents) => {
                self.validate_service_account_json(&contents);
            }
            Err(e) => {
                self.add_error(format!(
                    "Cannot read service account file '{}': {}\n\
                     Please check file permissions.",
                    expanded_path.display(),
                    e
                ));
            }
        }
    }

    /// Validate inline service account JSON
    fn validate_service_account_json(&mut self, json_str: &str) {
        // Try to parse as JSON
        let service_account: serde_json::Value = match serde_json::from_str(json_str) {
            Ok(value) => value,
            Err(e) => {
                self.add_error(format!(
                    "Invalid service account JSON: {}\n\
                     Please ensure the JSON is properly formatted.",
                    e
                ));
                return;
            }
        };

        // Validate required fields for Google Cloud service account
        let required_fields = [
            "type",
            "project_id",
            "private_key_id",
            "private_key",
            "client_email",
            "client_id",
            "auth_uri",
            "token_uri",
        ];

        for field in &required_fields {
            if !service_account.get(field).and_then(|v| v.as_str()).map_or(false, |s| !s.is_empty())
            {
                self.add_error(format!(
                    "Service account JSON missing or empty required field: '{}'",
                    field
                ));
            }
        }

        // Validate specific field formats
        if let Some(account_type) = service_account.get("type").and_then(|v| v.as_str()) {
            if account_type != "service_account" {
                self.add_error(format!(
                    "Invalid service account type: '{}'. Expected 'service_account'",
                    account_type
                ));
            }
        }

        if let Some(client_email) = service_account.get("client_email").and_then(|v| v.as_str()) {
            if !client_email.contains('@') || !client_email.contains("gserviceaccount.com") {
                self.add_warning(format!(
                    "Service account email '{}' doesn't look like a Google service account email",
                    client_email
                ));
            }
        }

        if let Some(private_key) = service_account.get("private_key").and_then(|v| v.as_str()) {
            if !private_key.starts_with("-----BEGIN PRIVATE KEY-----") {
                self.add_error("Private key doesn't appear to be in valid PEM format".to_string());
            }
        }
    }

    /// Validate file permissions for security
    fn validate_file_permissions(&mut self, path: &Path) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if let Ok(metadata) = std::fs::metadata(path) {
                let permissions = metadata.permissions();
                let mode = permissions.mode();

                // Check if file is readable by group or others (security risk)
                if mode & 0o044 != 0 {
                    self.add_warning(format!(
                        "Service account file '{}' is readable by group/others (permissions: {:o}). \
                         Consider restricting permissions: chmod 600 '{}'",
                        path.display(), mode & 0o777, path.display()
                    ));
                }

                // Check if file is writable by group or others (security risk)
                if mode & 0o022 != 0 {
                    self.add_warning(format!(
                        "Service account file '{}' is writable by group/others (permissions: {:o}). \
                         Consider restricting permissions: chmod 600 '{}'",
                        path.display(), mode & 0o777, path.display()
                    ));
                }
            }
        }

        #[cfg(not(unix))]
        {
            // On non-Unix systems, we can't easily check detailed permissions
            // but we can at least check basic read/write access
            if let Err(e) = std::fs::File::open(path) {
                self.add_error(format!(
                    "Cannot open service account file '{}': {}",
                    path.display(),
                    e
                ));
            }
        }
    }

    /// Validate streaming configuration
    fn validate_streaming_config(&mut self) {
        let streaming = &self.config.streaming;

        // Validate buffer size
        if streaming.buffer_size == 0 {
            self.add_error("Streaming buffer size cannot be zero".to_string());
        } else if streaming.buffer_size < 1024 {
            self.add_warning(format!(
                "Small streaming buffer size ({} bytes) may impact performance",
                streaming.buffer_size
            ));
        } else if streaming.buffer_size > 10 * 1024 * 1024 {
            self.add_warning(format!(
                "Large streaming buffer size ({} bytes) may consume excessive memory",
                streaming.buffer_size
            ));
        }

        // Validate chunk timeout
        if streaming.chunk_timeout_ms == 0 {
            self.add_error("Streaming chunk timeout cannot be zero".to_string());
        } else if streaming.chunk_timeout_ms < 100 {
            self.add_warning(format!(
                "Very short chunk timeout ({}ms) may cause premature timeouts",
                streaming.chunk_timeout_ms
            ));
        } else if streaming.chunk_timeout_ms > 60000 {
            self.add_warning(format!(
                "Long chunk timeout ({}ms) may cause poor user experience",
                streaming.chunk_timeout_ms
            ));
        }

        // Mode-specific validations
        match streaming.mode {
            StreamingMode::Never => {
                if streaming.buffer_size > 1024 * 1024 {
                    self.add_warning(
                        "Large buffer size not needed when streaming is disabled".to_string(),
                    );
                }
            }
            StreamingMode::Buffered => {
                if streaming.buffer_size < 4096 {
                    self.add_warning(
                        "Small buffer size may reduce effectiveness of buffered streaming"
                            .to_string(),
                    );
                }
            }
            _ => {} // Other modes are fine
        }

        tracing::debug!("Streaming config validation completed");
    }

    /// Validate security requirements
    fn validate_security_requirements(&mut self) {
        // Check for development/testing configurations that shouldn't be used in production
        if self.config.server.log_level == LogLevel::Trace {
            self.add_warning(
                "Trace log level enabled: may log sensitive information in production".to_string(),
            );
        }

        // Check for insecure configurations
        if !self.config.server.enable_retries {
            self.add_warning(
                "Retries are disabled: may impact reliability in production".to_string(),
            );
        }

        tracing::debug!("Security validation completed");
    }

    /// Add a validation error
    fn add_error(&mut self, error: String) {
        tracing::debug!("Validation error: {}", error);
        self.errors.push(error);
    }

    /// Add a validation warning
    fn add_warning(&mut self, warning: String) {
        tracing::debug!("Validation warning: {}", warning);
        self.warnings.push(warning);
    }
}

/* --- utility functions ------------------------------------------------------------------- */

/// Validate a single configuration value and return detailed error
///
/// This is a utility function for validating individual config values
/// outside of the full configuration context.
///
/// # Arguments
/// * `value` - The value to validate
/// * `field_name` - Name of the field for error messages
/// * `validator` - Closure that performs the validation
///
/// # Returns
/// * `Ok(())` - Value is valid
/// * `Err(ProxyError)` - Validation failed
#[allow(dead_code)]
pub fn validate_field<T, F>(value: &T, field_name: &str, validator: F) -> Result<()>
where
    F: FnOnce(&T) -> Result<()>,
{
    validator(value).map_err(|e| ProxyError::Config(format!("Invalid {}: {}", field_name, e)))
}

/* --- tests ------------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AuthConfig, Config, ServerConfig, StreamingConfig, default_auth_strategy};
    use std::fs;
    use tempfile::TempDir;

    fn create_test_config() -> Config {
        Config {
            server: ServerConfig {
                port: 3000,
                log_level: LogLevel::Info,
                enable_retries: true,
                max_retry_attempts: 3,
            },
            auth: AuthConfig {
                service_account_file: None,
                service_account_json: Some(r#"{"type":"service_account","project_id":"test","private_key_id":"123","private_key":"-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----","client_email":"test@test.gserviceaccount.com","client_id":"123","auth_uri":"https://accounts.google.com/o/oauth2/auth","token_uri":"https://oauth2.googleapis.com/token"}"#.to_string()),
                strategy: default_auth_strategy(),
            },
            streaming: StreamingConfig {
                mode: StreamingMode::Auto,
                buffer_size: 65536,
                chunk_timeout_ms: 5000,
            },
            llm_provider: None, // Provider is loaded separately
        }
    }

    #[test]
    fn test_valid_config_passes_validation() {
        let config = create_test_config();
        let result = ConfigValidator::new(&config).validate();
        assert!(result.is_ok(), "Valid config should pass validation");
    }

    #[test]
    fn test_invalid_port_fails_validation() {
        let mut config = create_test_config();
        config.server.port = 0;

        let result = ConfigValidator::new(&config).validate();
        assert!(result.is_err());
        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("Invalid server port 0"));
    }

    #[test]
    fn test_missing_auth_fails_validation() {
        let mut config = create_test_config();
        config.auth.service_account_file = None;
        config.auth.service_account_json = None;

        let result = ConfigValidator::new(&config).validate();
        assert!(result.is_err());
        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("No service account configuration"));
    }

    #[test]
    fn test_invalid_json_fails_validation() {
        let mut config = create_test_config();
        config.auth.service_account_json = Some("invalid json".to_string());

        let result = ConfigValidator::new(&config).validate();
        assert!(result.is_err());
        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("Invalid service account JSON"));
    }

    #[test]
    fn test_service_account_file_validation() {
        let temp_dir = TempDir::new().unwrap();
        let service_account_file = temp_dir.path().join("service-account.json");

        let valid_json = r#"{"type":"service_account","project_id":"test","private_key_id":"123","private_key":"-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----","client_email":"test@test.gserviceaccount.com","client_id":"123","auth_uri":"https://accounts.google.com/o/oauth2/auth","token_uri":"https://oauth2.googleapis.com/token"}"#;
        fs::write(&service_account_file, valid_json).unwrap();

        let mut config = create_test_config();
        config.auth.service_account_file = Some(service_account_file.to_string_lossy().to_string());
        config.auth.service_account_json = None;

        let result = ConfigValidator::new(&config).validate();
        assert!(result.is_ok(), "Valid service account file should pass validation");
    }

    #[test]
    fn test_zero_buffer_size_fails_validation() {
        let mut config = create_test_config();
        config.streaming.buffer_size = 0;

        let result = ConfigValidator::new(&config).validate();
        assert!(result.is_err());
        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("buffer size cannot be zero"));
    }

    #[test]
    fn test_privileged_port_warning() {
        let mut config = create_test_config();
        config.server.port = 80;

        // For this test, we need to capture warnings somehow
        // Since warnings don't fail validation, we'll check the result is Ok
        // In a real implementation, you might want to return warnings separately
        let result = ConfigValidator::new(&config).validate();
        assert!(result.is_ok(), "Config with privileged port should still be valid");
    }

    #[test]
    fn test_validate_field_utility() {
        let port = 8080u16;
        let result = validate_field(&port, "port", |p| {
            if *p == 0 { Err(ProxyError::Config("cannot be zero".to_string())) } else { Ok(()) }
        });
        assert!(result.is_ok());

        let bad_port = 0u16;
        let result = validate_field(&bad_port, "port", |p| {
            if *p == 0 { Err(ProxyError::Config("cannot be zero".to_string())) } else { Ok(()) }
        });
        assert!(result.is_err());
    }
}
