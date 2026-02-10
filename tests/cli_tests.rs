//! CLI tests for ModelMux binary
//!
//! Tests command-line interface functionality including --version and --help flags.
//! These tests ensure the binary works correctly for Homebrew deployment.
//!
//! Note: These tests use `cargo run` which requires the project to be built.
//! For Homebrew, the binary will be installed and these tests verify the CLI works.

use std::process::Command;
use std::str;

/// Get the path to the modelmux binary
/// In CI/Homebrew, this would be the installed binary path
/// For local testing, we use cargo run
fn get_binary_command() -> Command {
    // Try to use the built binary first, fall back to cargo run
    if std::path::Path::new("target/release/modelmux").exists() {
        let cmd = Command::new("target/release/modelmux");
        cmd
    } else if std::path::Path::new("target/debug/modelmux").exists() {
        let cmd = Command::new("target/debug/modelmux");
        cmd
    } else {
        // Fall back to cargo run for development
        let mut cmd = Command::new("cargo");
        cmd.args(&["run", "--bin", "modelmux", "--"]);
        cmd
    }
}

/// Test that --version flag works and outputs correct version format
#[test]
fn test_version_flag() {
    let mut cmd = get_binary_command();
    cmd.arg("--version");
    
    let output = cmd.output().expect("Failed to execute command");

    assert!(output.status.success(), "Version command should succeed");
    let stdout = str::from_utf8(&output.stdout).expect("Invalid UTF-8");
    assert!(
        stdout.contains("modelmux"),
        "Version output should contain 'modelmux', got: {}",
        stdout
    );
    // Version should be in format "modelmux X.Y.Z"
    assert!(
        stdout.matches(char::is_numeric).count() > 0,
        "Version output should contain version number, got: {}",
        stdout
    );
}

/// Test that -V flag works (short version)
#[test]
fn test_version_flag_short() {
    let mut cmd = get_binary_command();
    cmd.arg("-V");
    
    let output = cmd.output().expect("Failed to execute command");

    assert!(output.status.success(), "Version command should succeed");
    let stdout = str::from_utf8(&output.stdout).expect("Invalid UTF-8");
    assert!(
        stdout.contains("modelmux"),
        "Version output should contain 'modelmux', got: {}",
        stdout
    );
}

/// Test that --help flag works and shows usage information
#[test]
fn test_help_flag() {
    let mut cmd = get_binary_command();
    cmd.arg("--help");
    
    let output = cmd.output().expect("Failed to execute command");

    assert!(output.status.success(), "Help command should succeed");
    let stdout = str::from_utf8(&output.stdout).expect("Invalid UTF-8");
    assert!(
        stdout.contains("USAGE"),
        "Help output should contain 'USAGE', got: {}",
        stdout
    );
    assert!(
        stdout.contains("OPTIONS"),
        "Help output should contain 'OPTIONS', got: {}",
        stdout
    );
    assert!(
        stdout.contains("ENVIRONMENT VARIABLES"),
        "Help output should contain 'ENVIRONMENT VARIABLES', got: {}",
        stdout
    );
}

/// Test that -h flag works (short help)
#[test]
fn test_help_flag_short() {
    let mut cmd = get_binary_command();
    cmd.arg("-h");
    
    let output = cmd.output().expect("Failed to execute command");

    assert!(output.status.success(), "Help command should succeed");
    let stdout = str::from_utf8(&output.stdout).expect("Invalid UTF-8");
    assert!(
        stdout.contains("USAGE"),
        "Help output should contain 'USAGE', got: {}",
        stdout
    );
}
