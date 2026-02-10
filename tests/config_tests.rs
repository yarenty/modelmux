//! Configuration module tests
//!
//! Tests for configuration loading, validation, and parsing from environment variables.
//!
//! Uses temp-env to safely manage environment variables during tests, automatically
//! restoring them after each test completes.

use base64::Engine;
use modelmux::config::{Config, LogLevel, StreamingMode};
use temp_env::with_vars;

/// Test that required environment variables are validated
#[test]
fn test_missing_required_env_vars() {
    // Skip this test if .env file exists, as dotenv() will load vars from it
    if std::path::Path::new(".env").exists() {
        eprintln!("Skipping test_missing_required_env_vars: .env file exists");
        return;
    }

    // temp-env will unset these vars and restore them after the test
    with_vars(
        vec![
            ("GCP_SERVICE_ACCOUNT_KEY", None::<&str>),
            ("LLM_URL", None::<&str>),
            ("LLM_CHAT_ENDPOINT", None::<&str>),
            ("LLM_MODEL", None::<&str>),
        ],
        || {
            let result = Config::from_env();
            assert!(result.is_err(), "Should fail when required env vars are missing");
            if let Err(e) = result {
                assert!(
                    format!("{}", e).contains("GCP_SERVICE_ACCOUNT_KEY")
                        || format!("{}", e).contains("LLM_URL")
                        || format!("{}", e).contains("LLM_CHAT_ENDPOINT")
                        || format!("{}", e).contains("LLM_MODEL"),
                    "Error should mention missing environment variable"
                );
            }
        },
    );
}

/// Test that default port is used when PORT is not set
#[test]
fn test_default_port() {
    with_vars(
        vec![
            ("GCP_SERVICE_ACCOUNT_KEY", Some(get_test_key_b64().as_str())),
            ("LLM_URL", Some("https://test.example.com/v1/")),
            ("LLM_CHAT_ENDPOINT", Some("test-model:streamRawPredict")),
            ("LLM_MODEL", Some("test-model")),
            ("PORT", None::<&str>),
        ],
        || {
            let config = Config::from_env().expect("Should load config with defaults");
            assert_eq!(config.port, 3000, "Default port should be 3000");
        },
    );
}

/// Test that custom port is parsed correctly
#[test]
fn test_custom_port() {
    with_vars(
        vec![
            ("GCP_SERVICE_ACCOUNT_KEY", Some(get_test_key_b64().as_str())),
            ("LLM_URL", Some("https://test.example.com/v1/")),
            ("LLM_CHAT_ENDPOINT", Some("test-model:streamRawPredict")),
            ("LLM_MODEL", Some("test-model")),
            ("PORT", Some("8080")),
        ],
        || {
            let config = Config::from_env().expect("Should load config");
            assert_eq!(config.port, 8080, "Should use custom port");
        },
    );
}

/// Test that invalid port produces error
#[test]
fn test_invalid_port() {
    with_vars(
        vec![
            ("GCP_SERVICE_ACCOUNT_KEY", Some(get_test_key_b64().as_str())),
            ("LLM_URL", Some("https://test.example.com/v1/")),
            ("LLM_CHAT_ENDPOINT", Some("test-model:streamRawPredict")),
            ("LLM_MODEL", Some("test-model")),
            ("PORT", Some("99999")), // Invalid port number
        ],
        || {
            let result = Config::from_env();
            assert!(result.is_err(), "Should fail with invalid port");
        },
    );
}

/// Test log level parsing
#[test]
fn test_log_level_parsing() {
    let levels = vec!["trace", "debug", "info", "warn", "error"];
    for level in levels {
        with_vars(
            vec![
                ("GCP_SERVICE_ACCOUNT_KEY", Some(get_test_key_b64().as_str())),
                ("LLM_URL", Some("https://test.example.com/v1/")),
                ("LLM_CHAT_ENDPOINT", Some("test-model:streamRawPredict")),
                ("LLM_MODEL", Some("test-model")),
                ("LOG_LEVEL", Some(level)),
            ],
            || {
                let config = Config::from_env().expect("Should load config");
                assert_eq!(
                    format!("{:?}", config.log_level).to_lowercase(),
                    level,
                    "Should parse log level correctly"
                );
            },
        );
    }
}

/// Test default log level
#[test]
fn test_default_log_level() {
    // Skip this test if .env file exists, as dotenv() will load LOG_LEVEL from it
    // This test verifies default behavior in clean environments (CI)
    if std::path::Path::new(".env").exists() {
        eprintln!("Skipping test_default_log_level: .env file exists (dotenv() will load LOG_LEVEL)");
        return;
    }

    with_vars(
        vec![
            ("GCP_SERVICE_ACCOUNT_KEY", Some(get_test_key_b64().as_str())),
            ("LLM_URL", Some("https://test.example.com/v1/")),
            ("LLM_CHAT_ENDPOINT", Some("test-model:streamRawPredict")),
            ("LLM_MODEL", Some("test-model")),
            ("LOG_LEVEL", None::<&str>), // Unset to test default
        ],
        || {
            let config = Config::from_env().expect("Should load config");
            assert_eq!(config.log_level, LogLevel::Info, "Default log level should be Info");
        },
    );
}

/// Test streaming mode parsing
#[test]
fn test_streaming_mode_parsing() {
    let modes = vec![
        ("auto", StreamingMode::Auto),
        ("non-streaming", StreamingMode::NonStreaming),
        ("standard", StreamingMode::Standard),
        ("buffered", StreamingMode::Buffered),
    ];

    for (mode_str, expected_mode) in modes {
        with_vars(
            vec![
                ("GCP_SERVICE_ACCOUNT_KEY", Some(get_test_key_b64().as_str())),
                ("LLM_URL", Some("https://test.example.com/v1/")),
                ("LLM_CHAT_ENDPOINT", Some("test-model:streamRawPredict")),
                ("LLM_MODEL", Some("test-model")),
                ("STREAMING_MODE", Some(mode_str)),
            ],
            || {
                let config = Config::from_env().expect("Should load config");
                assert_eq!(
                    config.streaming_mode, expected_mode,
                    "Should parse streaming mode '{}' correctly",
                    mode_str
                );
            },
        );
    }
}

/// Test default streaming mode
#[test]
fn test_default_streaming_mode() {
    with_vars(
        vec![
            ("GCP_SERVICE_ACCOUNT_KEY", Some(get_test_key_b64().as_str())),
            ("LLM_URL", Some("https://test.example.com/v1/")),
            ("LLM_CHAT_ENDPOINT", Some("test-model:streamRawPredict")),
            ("LLM_MODEL", Some("test-model")),
            ("STREAMING_MODE", None::<&str>),
        ],
        || {
            let config = Config::from_env().expect("Should load config");
            assert_eq!(
                config.streaming_mode,
                StreamingMode::Auto,
                "Default streaming mode should be Auto"
            );
        },
    );
}

/// Test retry configuration parsing
#[test]
fn test_retry_config() {
    with_vars(
        vec![
            ("GCP_SERVICE_ACCOUNT_KEY", Some(get_test_key_b64().as_str())),
            ("LLM_URL", Some("https://test.example.com/v1/")),
            ("LLM_CHAT_ENDPOINT", Some("test-model:streamRawPredict")),
            ("LLM_MODEL", Some("test-model")),
            ("ENABLE_RETRIES", Some("false")),
            ("MAX_RETRY_ATTEMPTS", Some("5")),
        ],
        || {
            let config = Config::from_env().expect("Should load config");
            assert_eq!(config.enable_retries, false, "Should parse enable_retries");
            assert_eq!(config.max_retry_attempts, 5, "Should parse max_retry_attempts");
        },
    );
}

/// Test default retry configuration
#[test]
fn test_default_retry_config() {
    with_vars(
        vec![
            ("GCP_SERVICE_ACCOUNT_KEY", Some(get_test_key_b64().as_str())),
            ("LLM_URL", Some("https://test.example.com/v1/")),
            ("LLM_CHAT_ENDPOINT", Some("test-model:streamRawPredict")),
            ("LLM_MODEL", Some("test-model")),
            ("ENABLE_RETRIES", None::<&str>),
            ("MAX_RETRY_ATTEMPTS", None::<&str>),
        ],
        || {
            let config = Config::from_env().expect("Should load config");
            assert_eq!(config.enable_retries, true, "Default enable_retries should be true");
            assert_eq!(config.max_retry_attempts, 3, "Default max_retry_attempts should be 3");
        },
    );
}

/// Test LogLevel::is_trace_enabled
#[test]
fn test_log_level_trace_enabled() {
    assert!(LogLevel::Trace.is_trace_enabled(), "Trace should enable trace logging");
    assert!(LogLevel::Debug.is_trace_enabled(), "Debug should enable trace logging");
    assert!(!LogLevel::Info.is_trace_enabled(), "Info should not enable trace logging");
    assert!(!LogLevel::Warn.is_trace_enabled(), "Warn should not enable trace logging");
    assert!(!LogLevel::Error.is_trace_enabled(), "Error should not enable trace logging");
}

/// Test StreamingMode::from string conversion
#[test]
fn test_streaming_mode_from_str() {
    assert_eq!(StreamingMode::from("auto"), StreamingMode::Auto);
    assert_eq!(StreamingMode::from("AUTO"), StreamingMode::Auto); // Case insensitive
    assert_eq!(StreamingMode::from("non-streaming"), StreamingMode::NonStreaming);
    assert_eq!(StreamingMode::from("nonstreaming"), StreamingMode::NonStreaming);
    assert_eq!(StreamingMode::from("none"), StreamingMode::NonStreaming);
    assert_eq!(StreamingMode::from("standard"), StreamingMode::Standard);
    assert_eq!(StreamingMode::from("std"), StreamingMode::Standard);
    assert_eq!(StreamingMode::from("buffered"), StreamingMode::Buffered);
    assert_eq!(StreamingMode::from("buffer"), StreamingMode::Buffered);
    assert_eq!(StreamingMode::from("unknown"), StreamingMode::Auto); // Default
}

/// Test LogLevel::from string conversion
#[test]
fn test_log_level_from_str() {
    assert_eq!(LogLevel::from("trace"), LogLevel::Trace);
    assert_eq!(LogLevel::from("TRACE"), LogLevel::Trace); // Case insensitive
    assert_eq!(LogLevel::from("debug"), LogLevel::Debug);
    assert_eq!(LogLevel::from("info"), LogLevel::Info);
    assert_eq!(LogLevel::from("warn"), LogLevel::Warn);
    assert_eq!(LogLevel::from("error"), LogLevel::Error);
    assert_eq!(LogLevel::from("unknown"), LogLevel::Info); // Default
}

/// Test build_vertex_url method
#[test]
fn test_build_vertex_url() {
    with_vars(
        vec![
            ("GCP_SERVICE_ACCOUNT_KEY", Some(get_test_key_b64().as_str())),
            ("LLM_URL", Some("https://test.example.com/v1/")),
            ("LLM_CHAT_ENDPOINT", Some("test-model:streamRawPredict")),
            ("LLM_MODEL", Some("test-model")),
        ],
        || {
            let config = Config::from_env().expect("Should load config");

            // Test streaming URL
            let streaming_url = config.build_vertex_url(true);
            assert!(
                streaming_url.contains("streamRawPredict"),
                "Streaming URL should contain streamRawPredict"
            );

            // Test non-streaming URL
            let non_streaming_url = config.build_vertex_url(false);
            assert!(
                non_streaming_url.contains("rawPredict"),
                "Non-streaming URL should contain rawPredict"
            );
        },
    );
}

/// Helper function to get base64-encoded test service account key
fn get_test_key_b64() -> String {
    let minimal_key_json = r#"{
        "type": "service_account",
        "project_id": "test-project",
        "private_key_id": "test-key-id",
        "private_key": "-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC\n-----END PRIVATE KEY-----\n",
        "client_email": "test@test-project.iam.gserviceaccount.com",
        "client_id": "123456789",
        "auth_uri": "https://accounts.google.com/o/oauth2/auth",
        "token_uri": "https://oauth2.googleapis.com/token",
        "auth_provider_x509_cert_url": "https://www.googleapis.com/oauth2/v1/certs",
        "client_x509_cert_url": "https://www.googleapis.com/robot/v1/metadata/x509/test%40test-project.iam.gserviceaccount.com"
    }"#;
    base64::engine::general_purpose::STANDARD.encode(minimal_key_json)
}
