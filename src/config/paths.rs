//!
//! Platform-native path resolution for ModelMux configuration.
//!
//! This module provides cross-platform path resolution. ModelMux deliberately
//! uses XDG-style (`~/.config/modelmux/`) locations on both Linux and macOS so
//! users always know where to find their configuration. Windows continues to
//! use the standard Known Folder system.
//!
//! Resolved locations:
//! - Linux/Unix: XDG Base Directory Specification (`~/.config`, `~/.cache`, `~/.local/share`)
//! - macOS: XDG-style under `$HOME` (`~/.config`, `~/.cache`, `~/.local/share`)
//! - Windows: Known Folder system (`%APPDATA%`, `%LOCALAPPDATA%`)
//!
//! Follows Single Responsibility Principle - handles only path resolution concerns.
//!
//! Authors:
//!   Jaro <yarenty@gmail.com>
//!
//! Copyright (c) 2026 SkyCorp

/* --- uses ------------------------------------------------------------------------------------ */

use crate::error::{ProxyError, Result};
#[cfg(not(target_os = "macos"))]
use directories::ProjectDirs;
use std::path::{Path, PathBuf};

/* --- constants ------------------------------------------------------------------------------- */

/// Application name for directory resolution
const APP_NAME: &str = "modelmux";
/// Organization qualifier for directory resolution (used by `ProjectDirs` on
/// Linux/Windows only)
#[cfg(not(target_os = "macos"))]
const ORGANIZATION: &str = "com";
/// Organization name for directory resolution (used by `ProjectDirs` on
/// Linux/Windows only)
#[cfg(not(target_os = "macos"))]
const ORG_NAME: &str = "SkyCorp";

/* --- public functions ------------------------------------------------------------------------ */

/// Get the user configuration directory for ModelMux
///
/// Returns the platform-appropriate configuration directory:
/// - Linux: `~/.config/modelmux/`
/// - macOS: `~/.config/modelmux/` (XDG-style, not `~/Library/Application Support`)
/// - Windows: `%APPDATA%/modelmux/`
///
/// Creates the directory if it doesn't exist.
///
/// # Returns
/// * `Ok(PathBuf)` - Path to user configuration directory
/// * `Err(ProxyError)` - Unable to determine or create config directory
///
/// # Examples
/// ```rust,no_run
/// use modelmux::config::paths::user_config_dir;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let config_dir = user_config_dir()?;
/// let config_file = config_dir.join("config.toml");
/// # Ok(())
/// # }
/// ```
pub fn user_config_dir() -> Result<PathBuf> {
    let config_dir = resolve_user_config_dir()?;
    ensure_directory_exists(&config_dir)?;
    Ok(config_dir)
}

/// Get the user data directory for ModelMux
///
/// Returns the platform-appropriate data directory:
/// - Linux: `~/.local/share/modelmux/`
/// - macOS: `~/.local/share/modelmux/` (XDG-style)
/// - Windows: `%APPDATA%/modelmux/`
///
/// Creates the directory if it doesn't exist.
///
/// # Returns
/// * `Ok(PathBuf)` - Path to user data directory
/// * `Err(ProxyError)` - Unable to determine or create data directory
#[allow(dead_code)]
pub fn user_data_dir() -> Result<PathBuf> {
    let data_dir = resolve_user_data_dir()?;
    ensure_directory_exists(&data_dir)?;
    Ok(data_dir)
}

/// Get the user cache directory for ModelMux
///
/// Returns the platform-appropriate cache directory:
/// - Linux: `~/.cache/modelmux/`
/// - macOS: `~/.cache/modelmux/` (XDG-style)
/// - Windows: `%LOCALAPPDATA%/modelmux/`
///
/// Creates the directory if it doesn't exist.
///
/// # Returns
/// * `Ok(PathBuf)` - Path to user cache directory
/// * `Err(ProxyError)` - Unable to determine or create cache directory
#[allow(dead_code)]
pub fn user_cache_dir() -> Result<PathBuf> {
    let cache_dir = resolve_user_cache_dir()?;
    ensure_directory_exists(&cache_dir)?;
    Ok(cache_dir)
}

/// Get the system configuration directory for ModelMux
///
/// Returns the platform-appropriate system-wide configuration directory:
/// - Linux: `/etc/modelmux/`
/// - macOS: `/etc/modelmux/`
/// - Windows: `%PROGRAMDATA%/modelmux/`
///
/// Note: Does NOT create the directory (requires admin privileges)
///
/// # Returns
/// * `Ok(PathBuf)` - Path to system configuration directory
/// * `Err(ProxyError)` - Unable to determine system config directory
pub fn system_config_dir() -> Result<PathBuf> {
    #[cfg(unix)]
    {
        Ok(PathBuf::from("/etc").join(APP_NAME))
    }

    #[cfg(windows)]
    {
        std::env::var("PROGRAMDATA").map(|path| PathBuf::from(path).join(APP_NAME)).map_err(|_| {
            ProxyError::Config("PROGRAMDATA environment variable not found".to_string())
        })
    }
}

/// Get the default user configuration file path
///
/// Returns the full path to the main user configuration file:
/// - Linux: `~/.config/modelmux/config.toml`
/// - macOS: `~/.config/modelmux/config.toml`
/// - Windows: `%APPDATA%/modelmux/config.toml`
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
/// - Linux: `/etc/modelmux/config.toml`
/// - macOS: `/etc/modelmux/config.toml`
/// - Windows: `%PROGRAMDATA%/modelmux/config.toml`
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
/// - Linux: `~/.config/modelmux/service-account.json`
/// - macOS: `~/.config/modelmux/service-account.json`
/// - Windows: `%APPDATA%/modelmux/service-account.json`
///
/// Creates parent directories if they don't exist.
///
/// # Returns
/// * `Ok(PathBuf)` - Path to service account file
/// * `Err(ProxyError)` - Unable to determine service account file path
pub fn default_service_account_file() -> Result<PathBuf> {
    Ok(user_config_dir()?.join("service-account.json"))
}

/// Get legacy macOS user configuration file locations.
///
/// Returns paths that were used by previous releases on macOS. These are kept
/// only so the loader can transparently pick up an existing configuration and
/// warn the user to migrate to `~/.config/modelmux/`.
///
/// Returned in priority order (most-recent legacy location first).
#[cfg(target_os = "macos")]
pub fn legacy_macos_user_config_files() -> Vec<PathBuf> {
    legacy_macos_user_config_dirs()
        .into_iter()
        .map(|dir| dir.join("config.toml"))
        .collect()
}

/// Get legacy macOS user configuration directories.
///
/// These directories were produced by `directories::ProjectDirs` in older
/// releases. The loader checks these before reporting "no config found" so
/// existing users keep working after the move to `~/.config/modelmux/`.
#[cfg(target_os = "macos")]
pub fn legacy_macos_user_config_dirs() -> Vec<PathBuf> {
    let Ok(home) = home_dir() else {
        return Vec::new();
    };
    let app_support = home.join("Library").join("Application Support");
    vec![app_support.join("com.SkyCorp.modelmux"), app_support.join(APP_NAME)]
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
/// ```rust,no_run
/// use modelmux::config::paths::expand_path;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let expanded = expand_path("~/.config/modelmux/config.toml")?;
/// let expanded = expand_path("$HOME/.config/modelmux/config.toml")?;
/// # Ok(())
/// # }
/// ```
pub fn expand_path<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
    let path_str = path.as_ref().to_string_lossy();

    // Handle tilde expansion
    if let Some(rest) = path_str.strip_prefix("~/") {
        if let Some(dirs) = directories::UserDirs::new() {
            let expanded = dirs.home_dir().join(rest);
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
/// 1. User configuration file (`~/.config/modelmux/config.toml`)
/// 2. Legacy macOS user config (only on macOS, only as a migration fallback)
/// 3. System configuration file (`/etc/modelmux/config.toml`)
///
/// # Returns
/// * Vector of PathBuf in precedence order (highest to lowest priority)
pub fn config_file_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(user_config) = user_config_file() {
        paths.push(user_config);
    }

    #[cfg(target_os = "macos")]
    {
        for legacy in legacy_macos_user_config_files() {
            paths.push(legacy);
        }
    }

    if let Ok(system_config) = system_config_file() {
        paths.push(system_config);
    }

    paths
}

/* --- private functions ----------------------------------------------------------------------- */

/// Resolve the user configuration directory for the current platform.
fn resolve_user_config_dir() -> Result<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        Ok(home_dir()?.join(".config").join(APP_NAME))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(get_project_dirs()?.config_dir().to_path_buf())
    }
}

/// Resolve the user data directory for the current platform.
fn resolve_user_data_dir() -> Result<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        Ok(home_dir()?.join(".local").join("share").join(APP_NAME))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(get_project_dirs()?.data_dir().to_path_buf())
    }
}

/// Resolve the user cache directory for the current platform.
fn resolve_user_cache_dir() -> Result<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        Ok(home_dir()?.join(".cache").join(APP_NAME))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(get_project_dirs()?.cache_dir().to_path_buf())
    }
}

/// Resolve the current user's home directory.
fn home_dir() -> Result<PathBuf> {
    directories::UserDirs::new().map(|d| d.home_dir().to_path_buf()).ok_or_else(|| {
        ProxyError::Config(
            "Unable to determine user home directory. This may indicate:\n\
             1. No valid home directory found\n\
             2. Platform-specific directory resolution failed\n\
             3. Insufficient permissions to access user directories\n\
             \n\
             Please ensure your user account has a valid home directory."
                .to_string(),
        )
    })
}

/// Get `ProjectDirs` instance for ModelMux (Linux/Windows only)
#[cfg(not(target_os = "macos"))]
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
            let system_path = paths.last().expect("paths is non-empty");

            assert!(
                user_path.to_string_lossy().contains("config"),
                "First path should be user config"
            );

            #[cfg(unix)]
            assert!(
                system_path.starts_with("/etc"),
                "System path should live under /etc on Unix-like systems, got: {}",
                system_path.display()
            );
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_user_config_dir_uses_xdg_on_macos() {
        let config_dir = user_config_dir().expect("Should get user config directory");
        let config_str = config_dir.to_string_lossy();
        assert!(
            config_str.contains("/.config/modelmux"),
            "macOS user config should be ~/.config/modelmux, got: {}",
            config_str
        );
        assert!(
            !config_str.contains("Library/Application Support"),
            "macOS user config must no longer live under Library/Application Support, got: {}",
            config_str
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_legacy_macos_paths_are_listed() {
        let legacy = legacy_macos_user_config_files();
        assert!(
            legacy.iter().any(|p| p.to_string_lossy().contains("com.SkyCorp.modelmux")),
            "Legacy macOS lookup must include com.SkyCorp.modelmux/config.toml"
        );
    }

    #[test]
    fn test_default_service_account_file() {
        let sa_file = default_service_account_file().expect("Should get service account path");
        assert!(sa_file.file_name().unwrap() == "service-account.json");
        assert!(sa_file.parent().unwrap().exists(), "Parent directory should exist");
    }
}
