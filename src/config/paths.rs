//!
//! Platform-native path resolution for ModelMux configuration.
//!
//! This module provides cross-platform path resolution following industry standards:
//! - Linux/Unix: XDG Base Directory Specification (~/.config, ~/.cache, ~/.local/share)
//! - macOS: Standard Application Support directories (~/Library/...)
//! - Windows: Known Folder system (%APPDATA%, %LOCALAPPDATA%)
//!
//! Follows Single Responsibility Principle - handles only path resolution concerns.
//!
//! Authors:
//!   Jaro <yarenty@gmail.com>
//!
//! Copyright (c) 2026 SkyCorp

/* --- uses ------------------------------------------------------------------------------------ */

use crate::error::{ProxyError, Result};
use directories::ProjectDirs;
use std::path::{Path, PathBuf};

/* --- constants ------------------------------------------------------------------------------- */

/// Application name for directory resolution
const APP_NAME: &str = "modelmux";
/// Organization qualifier for directory resolution
const ORGANIZATION: &str = "com";
/// Organization name for directory resolution
const ORG_NAME: &str = "SkyCorp";

/* --- public functions ------------------------------------------------------------------------ */

/// Get the user configuration directory for ModelMux
///
/// Returns the platform-appropriate configuration directory:
/// - Linux: ~/.config/modelmux/
/// - macOS: ~/Library/Application Support/modelmux/
/// - Windows: %APPDATA%/modelmux/
///
/// Creates the directory if it doesn't exist.
///
/// # Returns
/// * `Ok(PathBuf)` - Path to user configuration directory
/// * `Err(ProxyError)` - Unable to determine or create config directory
///
/// # Examples
/// ```rust
/// let config_dir = modelmux::config::paths::user_config_dir()?;
/// let config_file = config_dir.join("config.toml");
/// ```
pub fn user_config_dir() -> Result<PathBuf> {
    let project_dirs = get_project_dirs()?;
    let config_dir = project_dirs.config_dir();

    ensure_directory_exists(config_dir)?;
    Ok(config_dir.to_path_buf())
}

/// Get the user data directory for ModelMux
///
/// Returns the platform-appropriate data directory:
/// - Linux: ~/.local/share/modelmux/
/// - macOS: ~/Library/Application Support/modelmux/
/// - Windows: %APPDATA%/modelmux/
///
/// Creates the directory if it doesn't exist.
///
/// # Returns
/// * `Ok(PathBuf)` - Path to user data directory
/// * `Err(ProxyError)` - Unable to determine or create data directory
pub fn user_data_dir() -> Result<PathBuf> {
    let project_dirs = get_project_dirs()?;
    let data_dir = project_dirs.data_dir();

    ensure_directory_exists(data_dir)?;
    Ok(data_dir.to_path_buf())
}

/// Get the user cache directory for ModelMux
///
/// Returns the platform-appropriate cache directory:
/// - Linux: ~/.cache/modelmux/
/// - macOS: ~/Library/Caches/modelmux/
/// - Windows: %LOCALAPPDATA%/modelmux/
///
/// Creates the directory if it doesn't exist.
///
/// # Returns
/// * `Ok(PathBuf)` - Path to user cache directory
/// * `Err(ProxyError)` - Unable to determine or create cache directory
pub fn user_cache_dir() -> Result<PathBuf> {
    let project_dirs = get_project_dirs()?;
    let cache_dir = project_dirs.cache_dir();

    ensure_directory_exists(cache_dir)?;
    Ok(cache_dir.to_path_buf())
}

/// Get the system configuration directory for ModelMux
///
/// Returns the platform-appropriate system-wide configuration directory:
/// - Linux: /etc/modelmux/
/// - macOS: /Library/Preferences/modelmux/
/// - Windows: %PROGRAMDATA%/modelmux/
///
/// Note: Does NOT create the directory (requires admin privileges)
///
/// # Returns
/// * `Ok(PathBuf)` - Path to system configuration directory
/// * `Err(ProxyError)` - Unable to determine system config directory
pub fn system_config_dir() -> Result<PathBuf> {
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Ok(PathBuf::from("/etc").join(APP_NAME))
    }

    #[cfg(target_os = "macos")]
    {
        Ok(PathBuf::from("/Library/Preferences").join(APP_NAME))
    }

    #[cfg(windows)]
    {
        // On Windows, we use ProgramData for system-wide config
        std::env::var("PROGRAMDATA").map(|path| PathBuf::from(path).join(APP_NAME)).map_err(|_| {
            ProxyError::Config("PROGRAMDATA environment variable not found".to_string())
        })
    }
}

/// Get the default user configuration file path
///
/// Returns the full path to the main user configuration file:
/// - Linux: ~/.config/modelmux/config.toml
/// - macOS: ~/Library/Application Support/modelmux/config.toml
/// - Windows: %APPDATA%/modelmux/config.toml
///
/// Creates parent directories if they don't exist.
///
/// # Returns
/// * `Ok(PathBuf)` - Path to user configuration file
/// * `Err(ProxyError)` - Unable to determine config file path
pub fn user_config_file() -> Result<PathBuf> {
    Ok(user_config_dir()?.join("config.toml"))
}

/// Get the system configuration file path
///
/// Returns the full path to the system-wide configuration file:
/// - Linux: /etc/modelmux/config.toml
/// - macOS: /Library/Preferences/modelmux/config.toml
/// - Windows: %PROGRAMDATA%/modelmux/config.toml
///
/// # Returns
/// * `Ok(PathBuf)` - Path to system configuration file
/// * `Err(ProxyError)` - Unable to determine system config file path
pub fn system_config_file() -> Result<PathBuf> {
    Ok(system_config_dir()?.join("config.toml"))
}

/// Get the default service account file path
///
/// Returns the recommended path for storing the Google Cloud service account key:
/// - Linux: ~/.config/modelmux/service-account.json
/// - macOS: ~/Library/Application Support/modelmux/service-account.json
/// - Windows: %APPDATA%/modelmux/service-account.json
///
/// Creates parent directories if they don't exist.
///
/// # Returns
/// * `Ok(PathBuf)` - Path to service account file
/// * `Err(ProxyError)` - Unable to determine service account file path
pub fn default_service_account_file() -> Result<PathBuf> {
    Ok(user_config_dir()?.join("service-account.json"))
}

/// Expand tilde (~) in file paths
///
/// Supports tilde expansion for user home directory references.
/// Also handles Windows-style environment variable expansion.
///
/// # Arguments
/// * `path` - Path string that may contain ~ or environment variables
///
/// # Returns
/// * `Ok(PathBuf)` - Expanded absolute path
/// * `Err(ProxyError)` - Path expansion failed
///
/// # Examples
/// ```rust
/// let expanded = expand_path("~/.config/modelmux/config.toml")?;
/// let expanded = expand_path("$HOME/.config/modelmux/config.toml")?;
/// ```
pub fn expand_path<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
    let path_str = path.as_ref().to_string_lossy();

    // Handle tilde expansion
    if path_str.starts_with("~/") {
        if let Some(dirs) = directories::UserDirs::new() {
            let expanded = dirs.home_dir().join(&path_str[2..]);
            return Ok(expanded);
        } else {
            return Err(ProxyError::Config(
                "Unable to determine user home directory for tilde expansion".to_string(),
            ));
        }
    }

    // Handle environment variable expansion (Unix-style)
    if path_str.contains('$') {
        let expanded = shellexpand::full(&path_str).map_err(|e| {
            ProxyError::Config(format!(
                "Failed to expand environment variables in path '{}': {}",
                path_str, e
            ))
        })?;
        return Ok(PathBuf::from(expanded.as_ref()));
    }

    // Return as-is if no expansion needed
    Ok(path.as_ref().to_path_buf())
}

/// Check if a configuration file exists and is readable
///
/// Verifies that the specified configuration file:
/// 1. Exists on the filesystem
/// 2. Is a regular file (not a directory)
/// 3. Has read permissions for the current user
///
/// # Arguments
/// * `path` - Path to configuration file to check
///
/// # Returns
/// * `Ok(())` - File exists and is readable
/// * `Err(ProxyError)` - File doesn't exist, isn't readable, or is invalid
pub fn validate_config_file<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();

    if !path.exists() {
        return Err(ProxyError::Config(format!(
            "Configuration file '{}' does not exist",
            path.display()
        )));
    }

    if !path.is_file() {
        return Err(ProxyError::Config(format!(
            "Configuration path '{}' exists but is not a regular file",
            path.display()
        )));
    }

    // Test readability by attempting to open
    std::fs::File::open(path).map_err(|e| {
        ProxyError::Config(format!(
            "Configuration file '{}' exists but cannot be read: {}\n\
             \n\
             Please check file permissions. The file should be readable by the current user.\n\
             You can fix this with: chmod 644 '{}'",
            path.display(),
            e,
            path.display()
        ))
    })?;

    Ok(())
}

/// Get all possible configuration file paths in precedence order
///
/// Returns configuration file paths in the order they should be checked:
/// 1. User configuration file (~/.config/modelmux/config.toml)
/// 2. System configuration file (/etc/modelmux/config.toml)
///
/// # Returns
/// * Vector of PathBuf in precedence order (highest to lowest priority)
pub fn config_file_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // User config has highest priority
    if let Ok(user_config) = user_config_file() {
        paths.push(user_config);
    }

    // System config has lowest priority
    if let Ok(system_config) = system_config_file() {
        paths.push(system_config);
    }

    paths
}

/* --- private functions ----------------------------------------------------------------------- */

/// Get ProjectDirs instance for ModelMux
fn get_project_dirs() -> Result<ProjectDirs> {
    ProjectDirs::from(ORGANIZATION, ORG_NAME, APP_NAME).ok_or_else(|| {
        ProxyError::Config(
            "Unable to determine user directories. This may indicate:\n\
             1. No valid home directory found\n\
             2. Platform-specific directory resolution failed\n\
             3. Insufficient permissions to access user directories\n\
             \n\
             Please ensure your user account has a valid home directory."
                .to_string(),
        )
    })
}

/// Ensure a directory exists, creating it if necessary
fn ensure_directory_exists<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();

    if path.exists() {
        if !path.is_dir() {
            return Err(ProxyError::Config(format!(
                "Path '{}' exists but is not a directory",
                path.display()
            )));
        }
        return Ok(());
    }

    // Create directory and all parent directories
    std::fs::create_dir_all(path).map_err(|e| {
        ProxyError::Config(format!(
            "Failed to create configuration directory '{}': {}\n\
             \n\
             Please ensure:\n\
             1. You have write permissions to the parent directory\n\
             2. There's sufficient disk space\n\
             3. No conflicting files exist in the path",
            path.display(),
            e
        ))
    })?;

    Ok(())
}

/* --- tests ----------------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_user_config_dir_creation() {
        let config_dir = user_config_dir().expect("Should get user config directory");
        assert!(config_dir.exists(), "Config directory should be created");
        assert!(config_dir.is_dir(), "Config path should be a directory");
    }

    #[test]
    fn test_user_config_file_path() {
        let config_file = user_config_file().expect("Should get config file path");
        assert!(config_file.file_name().unwrap() == "config.toml");
        assert!(config_file.parent().unwrap().exists(), "Parent directory should exist");
    }

    #[test]
    fn test_tilde_expansion() {
        let expanded = expand_path("~/test/path").expect("Should expand tilde");
        assert!(!expanded.to_string_lossy().contains('~'), "Tilde should be expanded");

        // Test already absolute path
        let absolute = expand_path("/absolute/path").expect("Should handle absolute path");
        assert_eq!(absolute, PathBuf::from("/absolute/path"));
    }

    #[test]
    fn test_validate_config_file() {
        // Test with non-existent file
        let result = validate_config_file("/non/existent/file.toml");
        assert!(result.is_err());

        // Test with existing file
        let temp_dir = TempDir::new().unwrap();
        let temp_file = temp_dir.path().join("test.toml");
        fs::write(&temp_file, "test content").unwrap();

        let result = validate_config_file(&temp_file);
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_file_paths_order() {
        let paths = config_file_paths();
        assert!(!paths.is_empty(), "Should return at least one config path");

        // User config should come before system config
        if paths.len() > 1 {
            let user_path = &paths[0];
            let system_path = &paths[1];

            assert!(
                user_path.to_string_lossy().contains("config"),
                "First path should be user config"
            );

            #[cfg(unix)]
            assert!(system_path.starts_with("/etc") || system_path.starts_with("/Library"));
        }
    }

    #[test]
    fn test_default_service_account_file() {
        let sa_file = default_service_account_file().expect("Should get service account path");
        assert!(sa_file.file_name().unwrap() == "service-account.json");
        assert!(sa_file.parent().unwrap().exists(), "Parent directory should exist");
    }
}
