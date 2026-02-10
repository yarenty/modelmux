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
            ("VERTEX_REGION", None::<&str>),
        ],
        || {
            let result = Config::from_env();
            assert!(result.is_err(), "Should fail when required env vars are missing");
            if let Err(e) = result {
                assert!(
                    format!("{}", e).contains("GCP_SERVICE_ACCOUNT_KEY")
                        || format!("{}", e).contains("Predict URL not configured")
                        || format!("{}", e).contains("VERTEX_")
                        || format!("{}", e).contains("LLM_URL"),
                    "Error should mention missing configuration"
                );
            }
        },
    );
}

fn vertex_test_vars(key_b64: &str) -> Vec<(&'static str, Option<&str>)> {
    vec![
        ("GCP_SERVICE_ACCOUNT_KEY", Some(key_b64)),
        ("LLM_URL", None::<&str>),
        ("LLM_PROVIDER", Some("vertex")),
        ("VERTEX_REGION", Some("test-region")),
        ("VERTEX_PROJECT", Some("test-project")),
        ("VERTEX_LOCATION", Some("test-region")),
        ("VERTEX_PUBLISHER", Some("test-publisher")),
        ("VERTEX_MODEL_ID", Some("test-model")),
    ]
}

/// Test that default port is used when PORT is not set
#[test]
fn test_default_port() {
    let key = get_test_key_b64();
    let mut vars = vertex_test_vars(&key);
    vars.push(("PORT", None::<&str>));
    with_vars(vars, || {
        let config = Config::from_env().expect("Should load config with defaults");
        assert_eq!(config.port, 3000, "Default port should be 3000");
    });
}

/// Test that custom port is parsed correctly
#[test]
fn test_custom_port() {
    let key = get_test_key_b64();
    let mut vars = vertex_test_vars(&key);
    vars.push(("PORT", Some("8080")));
    with_vars(vars, || {
        let config = Config::from_env().expect("Should load config");
        assert_eq!(config.port, 8080, "Should use custom port");
    });
}

/// Test that invalid port produces error
#[test]
fn test_invalid_port() {
    let key = get_test_key_b64();
    let mut vars = vertex_test_vars(&key);
    vars.push(("PORT", Some("99999"))); // Invalid port number
    with_vars(vars, || {
        let result = Config::from_env();
        assert!(result.is_err(), "Should fail with invalid port");
    });
}

/// Test log level parsing
#[test]
fn test_log_level_parsing() {
    let key = get_test_key_b64();
    let levels = vec!["trace", "debug", "info", "warn", "error"];
    for level in levels {
        let mut vars = vertex_test_vars(&key);
        vars.push(("LOG_LEVEL", Some(level)));
        with_vars(vars, || {
            let config = Config::from_env().expect("Should load config");
            assert_eq!(
                format!("{:?}", config.log_level).to_lowercase(),
                level,
                "Should parse log level correctly"
            );
        });
    }
}

/// Test default log level
#[test]
fn test_default_log_level() {
    if std::path::Path::new(".env").exists() {
        eprintln!("Skipping test_default_log_level: .env file exists");
        return;
    }
    let key = get_test_key_b64();
    let mut vars = vertex_test_vars(&key);
    vars.push(("LOG_LEVEL", None::<&str>));
    with_vars(vars, || {
        let config = Config::from_env().expect("Should load config");
        assert_eq!(config.log_level, LogLevel::Info, "Default log level should be Info");
    });
}

/// Test streaming mode parsing
#[test]
fn test_streaming_mode_parsing() {
    let key = get_test_key_b64();
    let modes = vec![
        ("auto", StreamingMode::Auto),
        ("non-streaming", StreamingMode::NonStreaming),
        ("standard", StreamingMode::Standard),
        ("buffered", StreamingMode::Buffered),
    ];
    for (mode_str, expected_mode) in modes {
        let mut vars = vertex_test_vars(&key);
        vars.push(("STREAMING_MODE", Some(mode_str)));
        with_vars(vars, || {
            let config = Config::from_env().expect("Should load config");
            assert_eq!(
                config.streaming_mode, expected_mode,
                "Should parse streaming mode '{}' correctly",
                mode_str
            );
        });
    }
}

/// Test default streaming mode
#[test]
fn test_default_streaming_mode() {
    let key = get_test_key_b64();
    let mut vars = vertex_test_vars(&key);
    vars.push(("STREAMING_MODE", None::<&str>));
    with_vars(vars, || {
        let config = Config::from_env().expect("Should load config");
        assert_eq!(
            config.streaming_mode,
            StreamingMode::Auto,
            "Default streaming mode should be Auto"
        );
    });
}

/// Test retry configuration parsing
#[test]
fn test_retry_config() {
    let key = get_test_key_b64();
    let mut vars = vertex_test_vars(&key);
    vars.push(("ENABLE_RETRIES", Some("false")));
    vars.push(("MAX_RETRY_ATTEMPTS", Some("5")));
    with_vars(vars, || {
        let config = Config::from_env().expect("Should load config");
        assert_eq!(config.enable_retries, false, "Should parse enable_retries");
        assert_eq!(config.max_retry_attempts, 5, "Should parse max_retry_attempts");
    });
}

/// Test default retry configuration
#[test]
fn test_default_retry_config() {
    let key = get_test_key_b64();
    let mut vars = vertex_test_vars(&key);
    vars.push(("ENABLE_RETRIES", None::<&str>));
    vars.push(("MAX_RETRY_ATTEMPTS", None::<&str>));
    with_vars(vars, || {
        let config = Config::from_env().expect("Should load config");
        assert_eq!(config.enable_retries, true, "Default enable_retries should be true");
        assert_eq!(config.max_retry_attempts, 3, "Default max_retry_attempts should be 3");
    });
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

/// Test build_predict_url (Vertex vars)
#[test]
fn test_build_predict_url() {
    let key = get_test_key_b64();
    with_vars(vertex_test_vars(&key), || {
        let config = Config::from_env().expect("Should load config");
        let streaming_url = config.build_predict_url(true);
        assert!(
            streaming_url.contains("streamRawPredict"),
            "Streaming URL should contain streamRawPredict"
        );
        let non_streaming_url = config.build_predict_url(false);
        assert!(
            non_streaming_url.contains("rawPredict"),
            "Non-streaming URL should contain rawPredict"
        );
        assert!(
            config.build_predict_url(false).contains("test-region-aiplatform.googleapis.com"),
            "URL should be built from Vertex vars"
        );
    });
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
