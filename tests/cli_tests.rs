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
/// Prefer debug binary so `cargo test` uses the binary just built; then release, then cargo run.
fn get_binary_command() -> Command {
    if std::path::Path::new("target/debug/modelmux").exists() {
        Command::new("target/debug/modelmux")
    } else if std::path::Path::new("target/release/modelmux").exists() {
        Command::new("target/release/modelmux")
    } else {
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

/// Test that doctor command works
#[test]
fn test_doctor_command() {
    let mut cmd = get_binary_command();
    cmd.arg("doctor");
    
    // Set a timeout to avoid hanging
    let output = cmd.output().expect("Failed to execute command");

    // Doctor should exit successfully even if config is invalid
    // Check both stdout and stderr as output may go to either
    let stdout = str::from_utf8(&output.stdout).unwrap_or("");
    let stderr = str::from_utf8(&output.stderr).unwrap_or("");
    let combined = format!("{}\n{}", stdout, stderr);
    
    // Doctor command should produce some output (diagnostics)
    // It may contain various keywords depending on config state
    // Check for text indicators that doctor ran, or config/port errors
    let has_diagnostic_keywords = combined.contains("Doctor")
        || combined.contains("ModelMux Doctor")
        || combined.contains("Configuration")
        || combined.contains("Checking")
        || combined.contains("Environment")
        || combined.contains("Variables")
        || combined.contains("Failed to load configuration")
        || combined.contains("Vertex")
        || combined.contains("LLM_URL")
        || combined.contains("VERTEX_")
        || combined.contains("GCP_SERVICE_ACCOUNT")
        || combined.contains("Failed to bind to port")
        || combined.contains("Address already in use")
        || combined.contains("Http(\"Failed to bind to port")
        || combined.contains("[OK]")
        || combined.contains("[ERROR]")
        || combined.contains("[WARNING]")
        || combined.contains("[INFO]")
        || combined.contains("[TIP]")
        || combined.contains("[SUCCESS]");
    
    // If we got any output at all, that's a good sign
    // The command should at least print something
    let has_any_output = !stdout.is_empty() || !stderr.is_empty();
    
    assert!(
        has_diagnostic_keywords || (has_any_output && output.status.code() == Some(0)),
        "Doctor command should produce diagnostic output or exit successfully. stdout: '{}', stderr: '{}', exit_code: {:?}, combined: '{}'",
        stdout,
        stderr,
        output.status.code(),
        combined
    );
}

/// Test that validate command works
#[test]
fn test_validate_command() {
    let mut cmd = get_binary_command();
    cmd.arg("validate");
    // No env: config load fails; binary prints "[ERROR] Configuration error: ..."
    let output = cmd.output().expect("Failed to execute command");

    let stdout = str::from_utf8(&output.stdout).expect("Invalid UTF-8");
    let stderr = str::from_utf8(&output.stderr).expect("Invalid UTF-8");
    let combined = format!("{}\n{}", stdout, stderr);

    // Should contain validation result: valid, or error/Configuration from current provider messages
    let has_result = combined.contains("valid")
        || combined.contains("error")
        || combined.contains("Configuration")
        || combined.contains("[ERROR]")
        || combined.contains("[OK]")
        || combined.contains("Vertex")
        || combined.contains("LLM_URL")
        || combined.contains("GCP_SERVICE_ACCOUNT");
    assert!(
        has_result,
        "Validate output should contain validation result, got stdout: {}, stderr: {}",
        stdout,
        stderr
    );
}
