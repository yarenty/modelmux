//!
//! CLI configuration commands for ModelMux.
//!
//! This module provides command-line interface commands for configuration management:
//! - `config init` - Interactive configuration setup
//! - `config show` - Display current configuration
//! - `config validate` - Validate configuration
//! - `config edit` - Edit configuration in default editor
//!
//! Follows Single Responsibility Principle - handles only CLI configuration concerns.
//!
//! Authors:
//!   Jaro <yarenty@gmail.com>
//!
//! Copyright (c) 2026 SkyCorp

/* --- uses ------------------------------------------------------------------------------------ */

use crate::config::paths;
use crate::config::validation::ConfigValidator;
use crate::config::{Config, LogLevel, StreamingMode};
use crate::error::{ProxyError, Result};
use crate::provider::LlmProviderBackend;
use std::fs;
use std::io::{self, Write};
use std::process::Command;

/* --- types ----------------------------------------------------------------------------------- */

///
/// CLI configuration command handler.
///
/// Provides methods for handling all configuration-related CLI commands
/// with user-friendly interfaces and comprehensive error handling.
pub struct ConfigCli;

/* --- implementations --------------------------------------------------------------------- */

impl ConfigCli {
    /// Handle the `config init` command
    ///
    /// Provides an interactive setup wizard that guides users through
    /// configuration setup with intelligent defaults and validation.
    ///
    /// # Returns
    /// * `Ok(())` - Configuration successfully created
    /// * `Err(ProxyError)` - Configuration setup failed
    pub fn init() -> Result<()> {
        println!("🚀 ModelMux Configuration Setup");
        println!("===============================");
        println!();

        // Check if config already exists
        let config_file = paths::user_config_file()?;
        if config_file.exists() {
            println!("⚠️  Configuration file already exists at:");
            println!("   {}", config_file.display());
            println!();

            if !Self::confirm("Do you want to overwrite the existing configuration?")? {
                println!("Configuration setup cancelled.");
                return Ok(());
            }
        }

        // Gather configuration interactively
        let config = Self::gather_config_interactively()?;

        // Create config directory if it doesn't exist
        let config_dir = config_file.parent().unwrap();
        fs::create_dir_all(config_dir).map_err(|e| {
            ProxyError::Config(format!(
                "Failed to create config directory '{}': {}",
                config_dir.display(),
                e
            ))
        })?;

        // Write configuration file
        let config_toml = toml::to_string_pretty(&config)
            .map_err(|e| ProxyError::Config(format!("Failed to serialize configuration: {}", e)))?;

        fs::write(&config_file, config_toml).map_err(|e| {
            ProxyError::Config(format!(
                "Failed to write configuration file '{}': {}",
                config_file.display(),
                e
            ))
        })?;

        println!("✅ Configuration saved to: {}", config_file.display());

        // Handle service account setup if needed
        if let Some(ref sa_file) = config.auth.service_account_file {
            let sa_path = paths::expand_path(sa_file)?;
            if !sa_path.exists() {
                println!();
                println!("📋 Service Account Setup");
                println!("========================");
                println!(
                    "Your configuration references a service account file that doesn't exist yet:"
                );
                println!("   {}", sa_path.display());
                println!();
                println!("To complete setup:");
                println!("1. Download your Google Cloud service account key JSON from:");
                println!("   https://console.cloud.google.com/iam-admin/serviceaccounts");
                println!("2. Save it as: {}", sa_path.display());
                println!("3. Set secure permissions: chmod 600 '{}'", sa_path.display());
                println!("4. Run 'modelmux config validate' to verify setup");
            }
        }

        println!();
        println!("🎉 Configuration setup complete!");
        println!("Run 'modelmux config validate' to verify your configuration.");

        Ok(())
    }

    /// Handle the `config show` command
    ///
    /// Displays the current configuration in a readable format,
    /// showing the effective configuration after merging all sources.
    ///
    /// # Returns
    /// * `Ok(())` - Configuration displayed successfully
    /// * `Err(ProxyError)` - Failed to load or display configuration
    pub fn show() -> Result<()> {
        println!("📋 Current ModelMux Configuration");
        println!("=================================");
        println!();

        // Load current configuration
        let config = Config::load()?;

        // Display configuration sections
        println!("Server Configuration:");
        println!("  Port: {}", config.server.port);
        println!("  Log Level: {:?}", config.server.log_level);
        println!("  Enable Retries: {}", config.server.enable_retries);
        println!("  Max Retry Attempts: {}", config.server.max_retry_attempts);
        println!();

        println!("LLM Provider Configuration:");
        if let Some(ref provider) = config.llm_provider {
            println!("  Provider: {}", provider.id());
            println!("  Model: {}", provider.display_model_name());
            println!("  Request URL: {}", provider.build_request_url(false));
        } else {
            println!("  Provider: Not loaded (will be detected from environment)");
        }
        println!();

        println!("Authentication Configuration:");
        println!("  Strategy: {:?}", config.auth.strategy);
        if let Some(ref file) = config.auth.service_account_file {
            println!("  Service Account File: {}", file);

            // Check if file exists
            match paths::expand_path(file) {
                Ok(path) => {
                    if path.exists() {
                        println!("    Status: ✅ File exists");
                    } else {
                        println!("    Status: ❌ File not found");
                    }
                }
                Err(_) => {
                    println!("    Status: ❌ Invalid path");
                }
            }
        }
        if config.auth.service_account_json.is_some() {
            println!("  Service Account JSON: ✅ Inline JSON configured");
        }
        println!();

        println!("Streaming Configuration:");
        println!("  Streaming mode: {:?}", config.streaming.mode);

        if let Some(ref provider) = config.llm_provider {
            println!("  LLM Provider: {}", provider.id());
        } else {
            println!("  LLM Provider: Not loaded");
        }
        println!("  Buffer Size: {} bytes", config.streaming.buffer_size);
        println!("  Chunk Timeout: {}ms", config.streaming.chunk_timeout_ms);
        println!();

        // Show configuration file locations
        println!("Configuration Sources:");
        let config_paths = paths::config_file_paths();
        for (i, path) in config_paths.iter().enumerate() {
            let priority = match i {
                0 => "highest priority",
                n if n == config_paths.len() - 1 => "lowest priority",
                _ => "medium priority",
            };

            let status = if path.exists() { "✅ exists" } else { "❌ not found" };
            println!("  {} ({}): {}", path.display(), priority, status);
        }

        Ok(())
    }

    /// Handle the `config validate` command
    ///
    /// Performs comprehensive validation of the current configuration
    /// and provides detailed feedback about any issues found.
    ///
    /// # Returns
    /// * `Ok(())` - Configuration is valid
    /// * `Err(ProxyError)` - Configuration validation failed
    pub fn validate() -> Result<()> {
        println!("🔍 Validating ModelMux Configuration");
        println!("====================================");
        println!();

        // Load configuration
        print!("Loading configuration... ");
        io::stdout().flush().unwrap();

        let config = match Config::load() {
            Ok(config) => {
                println!("✅ Loaded");
                config
            }
            Err(e) => {
                println!("❌ Failed");
                println!();
                println!("Configuration loading failed:");
                println!("{}", e);
                return Err(e);
            }
        };

        // Validate configuration
        print!("Validating configuration... ");
        io::stdout().flush().unwrap();

        let validation_result = ConfigValidator::new(&config).validate();

        match validation_result {
            Ok(()) => {
                println!("✅ Valid");
                println!();
                println!("🎉 Configuration validation passed!");
                println!("Your ModelMux configuration is ready to use.");
            }
            Err(e) => {
                println!("❌ Invalid");
                println!();
                println!("Configuration validation failed:");
                println!("{}", e);
                return Err(e);
            }
        }

        // Additional checks
        println!();
        println!("Additional Checks:");

        // Test service account loading
        print!("Testing service account access... ");
        io::stdout().flush().unwrap();

        match config.load_service_account_key() {
            Ok(_) => {
                println!("✅ Service account loaded successfully");
            }
            Err(e) => {
                println!("❌ Service account loading failed");
                println!("   Error: {}", e);
            }
        }

        // Check network port availability (basic check)
        print!("Checking port availability... ");
        io::stdout().flush().unwrap();

        match std::net::TcpListener::bind(format!("127.0.0.1:{}", config.server.port)) {
            Ok(_) => {
                println!("✅ Port {} appears to be available", config.server.port);
            }
            Err(_) => {
                println!("⚠️  Port {} may be in use", config.server.port);
                println!("   This might be okay if another ModelMux instance is running");
            }
        }

        Ok(())
    }

    /// Handle the `config edit` command
    ///
    /// Opens the user configuration file in the default editor for manual editing.
    /// Creates a new configuration file if none exists.
    ///
    /// # Returns
    /// * `Ok(())` - Editor opened successfully
    /// * `Err(ProxyError)` - Failed to open editor or create config file
    pub fn edit() -> Result<()> {
        let config_file = paths::user_config_file()?;

        // Create config file if it doesn't exist
        if !config_file.exists() {
            println!("Configuration file doesn't exist. Creating example configuration...");

            let config_dir = config_file.parent().unwrap();
            fs::create_dir_all(config_dir).map_err(|e| {
                ProxyError::Config(format!(
                    "Failed to create config directory '{}': {}",
                    config_dir.display(),
                    e
                ))
            })?;

            let example_config = Config::example_toml();
            fs::write(&config_file, example_config).map_err(|e| {
                ProxyError::Config(format!("Failed to create example configuration: {}", e))
            })?;
        }

        // Determine editor to use
        let editor =
            std::env::var("EDITOR").or_else(|_| std::env::var("VISUAL")).unwrap_or_else(|_| {
                // Default editors by platform
                if cfg!(target_os = "windows") {
                    "notepad".to_string()
                } else if cfg!(target_os = "macos") {
                    "open -e".to_string()
                } else {
                    "nano".to_string()
                }
            });

        println!("Opening configuration file in editor: {}", editor);
        println!("File: {}", config_file.display());
        println!();

        // Split editor command if it contains spaces (like "open -e")
        let editor_parts: Vec<&str> = editor.split_whitespace().collect();
        let (editor_cmd, editor_args) = if editor_parts.len() > 1 {
            (editor_parts[0], &editor_parts[1..])
        } else {
            (editor_parts[0], &[] as &[&str])
        };

        // Launch editor
        let mut command = Command::new(editor_cmd);
        command.args(editor_args);
        command.arg(&config_file);

        let status = command.status().map_err(|e| {
            ProxyError::Config(format!(
                "Failed to launch editor '{}': {}\n\
                 \n\
                 You can also edit the configuration file manually:\n\
                 {}\n\
                 \n\
                 Or set the EDITOR environment variable to your preferred editor.",
                editor,
                e,
                config_file.display()
            ))
        })?;

        if status.success() {
            println!("Editor closed successfully.");
            println!("Run 'modelmux config validate' to check your changes.");
        } else {
            println!("Editor exited with an error. Please check the configuration manually.");
        }

        Ok(())
    }

    /* --- private helper methods ---------------------------------------------------------- */

    /// Gather configuration through interactive prompts
    fn gather_config_interactively() -> Result<Config> {
        let mut config = Config::default();

        // Server configuration
        println!("📡 Server Configuration");
        println!("======================");

        config.server.port = Self::prompt_number("HTTP server port", config.server.port, 1, 65535)?;

        config.server.log_level = Self::prompt_log_level(
            "Logging level (trace/debug/info/warn/error)",
            config.server.log_level,
        )?;

        config.server.enable_retries = Self::prompt_bool(
            "Enable automatic retries for rate limits",
            config.server.enable_retries,
        )?;

        if config.server.enable_retries {
            config.server.max_retry_attempts = Self::prompt_number(
                "Maximum retry attempts",
                config.server.max_retry_attempts,
                1,
                10,
            )?;
        }

        println!();

        // Provider configuration note
        println!("🤖 LLM Provider Configuration");
        println!("=============================");
        println!("Provider configuration is handled via environment variables.");
        println!("Current provider will be detected automatically from:");
        println!("  LLM_PROVIDER=vertex (default)");
        println!("  VERTEX_PROJECT, VERTEX_LOCATION, VERTEX_MODEL_ID, etc.");
        println!("  or LLM_URL for direct URL override");
        println!();
        println!("For detailed provider setup, see the documentation or run:");
        println!("  modelmux --help");
        println!();

        // Authentication configuration
        println!("🔐 Authentication Configuration");
        println!("===============================");

        let use_file =
            Self::prompt_bool("Use service account file (recommended for development)", true)?;

        if use_file {
            let default_sa_file =
                paths::default_service_account_file()?.to_string_lossy().to_string();

            let sa_file = Self::prompt_string_with_default(
                "Service account file path (supports ~ expansion)",
                "",
                &default_sa_file,
            )?;

            config.auth.service_account_file = Some(sa_file);
        } else {
            println!("You'll need to set service_account_json in the config file manually.");
            config.auth.service_account_json = None;
        }

        println!();

        // Streaming configuration
        println!("📡 Streaming Configuration");
        println!("==========================");

        config.streaming.mode = Self::prompt_streaming_mode(
            "Streaming mode (auto/never/standard/buffered/always)",
            config.streaming.mode,
        )?;

        if config.streaming.mode != StreamingMode::Never {
            config.streaming.buffer_size = Self::prompt_number(
                "Buffer size in bytes",
                config.streaming.buffer_size,
                1024,
                10 * 1024 * 1024,
            )?;

            config.streaming.chunk_timeout_ms = Self::prompt_number(
                "Chunk timeout in milliseconds",
                config.streaming.chunk_timeout_ms,
                100,
                60000,
            )?;
        }

        Ok(config)
    }

    /// Prompt for a string value
    fn prompt_string(prompt: &str, current: &str) -> Result<String> {
        loop {
            if current.is_empty() {
                print!("{}: ", prompt);
            } else {
                print!("{} [{}]: ", prompt, current);
            }
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .map_err(|e| ProxyError::Config(format!("Failed to read input: {}", e)))?;

            let input = input.trim();
            if input.is_empty() && !current.is_empty() {
                return Ok(current.to_string());
            } else if !input.is_empty() {
                return Ok(input.to_string());
            }

            println!("Please enter a value.");
        }
    }

    /// Prompt for a string value with a specific default
    fn prompt_string_with_default(prompt: &str, current: &str, default: &str) -> Result<String> {
        let display_current = if current.is_empty() { default } else { current };
        print!("{} [{}]: ", prompt, display_current);
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| ProxyError::Config(format!("Failed to read input: {}", e)))?;

        let input = input.trim();
        if input.is_empty() { Ok(display_current.to_string()) } else { Ok(input.to_string()) }
    }

    /// Prompt for a numeric value within range
    fn prompt_number<T>(prompt: &str, current: T, min: T, max: T) -> Result<T>
    where
        T: std::fmt::Display + std::str::FromStr + PartialOrd + Copy,
        <T as std::str::FromStr>::Err: std::fmt::Display,
    {
        loop {
            print!("{} ({}-{}) [{}]: ", prompt, min, max, current);
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .map_err(|e| ProxyError::Config(format!("Failed to read input: {}", e)))?;

            let input = input.trim();
            if input.is_empty() {
                return Ok(current);
            }

            match input.parse::<T>() {
                Ok(value) => {
                    if value >= min && value <= max {
                        return Ok(value);
                    } else {
                        println!("Value must be between {} and {}.", min, max);
                    }
                }
                Err(e) => {
                    println!("Invalid number: {}", e);
                }
            }
        }
    }

    /// Prompt for a boolean value
    fn prompt_bool(prompt: &str, default: bool) -> Result<bool> {
        loop {
            let default_str = if default { "Y/n" } else { "y/N" };
            print!("{} ({}): ", prompt, default_str);
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .map_err(|e| ProxyError::Config(format!("Failed to read input: {}", e)))?;

            let input = input.trim().to_lowercase();
            match input.as_str() {
                "" => return Ok(default),
                "y" | "yes" | "true" | "1" => return Ok(true),
                "n" | "no" | "false" | "0" => return Ok(false),
                _ => println!("Please enter y/yes or n/no."),
            }
        }
    }

    /// Prompt for log level
    fn prompt_log_level(prompt: &str, default: LogLevel) -> Result<LogLevel> {
        loop {
            print!("{} [{:?}]: ", prompt, default);
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .map_err(|e| ProxyError::Config(format!("Failed to read input: {}", e)))?;

            let input = input.trim();
            if input.is_empty() {
                return Ok(default);
            }

            match LogLevel::from_str(input) {
                Ok(level) => return Ok(level),
                Err(_) => {
                    println!("Invalid log level. Valid options: trace, debug, info, warn, error");
                }
            }
        }
    }

    /// Prompt for streaming mode
    fn prompt_streaming_mode(prompt: &str, default: StreamingMode) -> Result<StreamingMode> {
        loop {
            print!("{} [{:?}]: ", prompt, default);
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .map_err(|e| ProxyError::Config(format!("Failed to read input: {}", e)))?;

            let input = input.trim();
            if input.is_empty() {
                return Ok(default);
            }

            match StreamingMode::from_str(input) {
                Ok(mode) => return Ok(mode),
                Err(_) => {
                    println!(
                        "Invalid streaming mode. Valid options: auto, never, standard, buffered, always"
                    );
                }
            }
        }
    }

    /// Prompt for confirmation
    fn confirm(message: &str) -> Result<bool> {
        Self::prompt_bool(message, false)
    }
}

/* --- tests ------------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Most CLI tests would require mocking stdin/stdout,
    // which is complex. Here we test the parts we can test easily.

    #[test]
    fn test_config_cli_exists() {
        // Just ensure the struct can be created
        let _cli = ConfigCli;
    }

    // Integration tests would go here, but they'd need:
    // - Temporary directories
    // - Mocked stdin/stdout
    // - Environment setup
    // These are better handled as separate integration tests
}
