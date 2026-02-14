//! Configuration tests for ModelMux

use modelmux::config::{Config, LogLevel, StreamingMode};
use temp_env::with_vars;

/// Helper function to get JSON service account key
fn get_test_key_json() -> &'static str {
    r#"{"type":"service_account","project_id":"test-project","private_key_id":"test","private_key":"-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC\n-----END PRIVATE KEY-----\n","client_email":"test@test-project.iam.gserviceaccount.com","client_id":"123456789","auth_uri":"https://accounts.google.com/o/oauth2/auth","token_uri":"https://oauth2.googleapis.com/token","auth_provider_x509_cert_url":"https://www.googleapis.com/oauth2/v1/certs","client_x509_cert_url":"https://www.googleapis.com/robot/v1/metadata/x509/test%40test-project.iam.gserviceaccount.com"}"#
}

/// Test that configuration loads with minimal required environment variables
#[test]
fn test_config_load_with_required_vars() {
    with_vars(
        vec![
            ("MODELMUX_AUTH_SERVICE_ACCOUNT_JSON", Some(get_test_key_json())),
            ("LLM_PROVIDER", Some("vertex")),
            ("VERTEX_PROJECT", Some("test-project")),
            ("VERTEX_LOCATION", Some("us-central1")),
            ("VERTEX_MODEL_ID", Some("claude-3-5-sonnet@20241022")),
        ],
        || {
            let config = Config::load().expect("Should load config with required vars");
            assert_eq!(config.server.port, 3000); // Default port
            assert_eq!(config.server.log_level, LogLevel::Info); // Default log level
            assert_eq!(config.streaming.mode, StreamingMode::Auto); // Default streaming mode
        },
    );
}

/// Test that default port is used when not specified
#[test]
fn test_default_port() {
    with_vars(
        vec![
            ("MODELMUX_AUTH_SERVICE_ACCOUNT_JSON", Some(get_test_key_json())),
            ("LLM_PROVIDER", Some("vertex")),
            ("VERTEX_PROJECT", Some("test-project")),
            ("VERTEX_LOCATION", Some("us-central1")),
            ("VERTEX_MODEL_ID", Some("claude-3-5-sonnet@20241022")),
        ],
        || {
            let config = Config::load().expect("Should load config with defaults");
            assert_eq!(config.server.port, 3000, "Default port should be 3000");
        },
    );
}

/// Test that custom port is parsed correctly
#[test]
fn test_custom_port() {
    with_vars(
        vec![
            ("MODELMUX_AUTH_SERVICE_ACCOUNT_JSON", Some(get_test_key_json())),
            ("MODELMUX_SERVER_PORT", Some("8080")),
            ("LLM_PROVIDER", Some("vertex")),
            ("VERTEX_PROJECT", Some("test-project")),
            ("VERTEX_LOCATION", Some("us-central1")),
            ("VERTEX_MODEL_ID", Some("claude-3-5-sonnet@20241022")),
        ],
        || {
            let config = Config::load().expect("Should load config");
            assert_eq!(config.server.port, 8080, "Should use custom port");
        },
    );
}

/// Test that invalid port causes error
#[test]
fn test_invalid_port() {
    with_vars(
        vec![
            ("MODELMUX_AUTH_SERVICE_ACCOUNT_JSON", Some(get_test_key_json())),
            ("MODELMUX_SERVER_PORT", Some("99999")), // Invalid port number
            ("LLM_PROVIDER", Some("vertex")),
            ("VERTEX_PROJECT", Some("test-project")),
            ("VERTEX_LOCATION", Some("us-central1")),
            ("VERTEX_MODEL_ID", Some("claude-3-5-sonnet@20241022")),
        ],
        || {
            let result = Config::load();
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
                ("MODELMUX_AUTH_SERVICE_ACCOUNT_JSON", Some(get_test_key_json())),
                ("MODELMUX_SERVER_LOG_LEVEL", Some(level)),
                ("LLM_PROVIDER", Some("vertex")),
                ("VERTEX_PROJECT", Some("test-project")),
                ("VERTEX_LOCATION", Some("us-central1")),
                ("VERTEX_MODEL_ID", Some("claude-3-5-sonnet@20241022")),
            ],
            || {
                let config = Config::load().expect("Should load config");
                assert_eq!(
                    format!("{:?}", config.server.log_level).to_lowercase(),
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
    with_vars(
        vec![
            ("MODELMUX_AUTH_SERVICE_ACCOUNT_JSON", Some(get_test_key_json())),
            ("LLM_PROVIDER", Some("vertex")),
            ("VERTEX_PROJECT", Some("test-project")),
            ("VERTEX_LOCATION", Some("us-central1")),
            ("VERTEX_MODEL_ID", Some("claude-3-5-sonnet@20241022")),
        ],
        || {
            let config = Config::load().expect("Should load config");
            assert_eq!(config.server.log_level, LogLevel::Info, "Default log level should be Info");
        },
    );
}

/// Test streaming mode parsing
#[test]
fn test_streaming_mode_parsing() {
    let modes = vec![
        ("auto", StreamingMode::Auto),
        ("never", StreamingMode::Never),
        ("standard", StreamingMode::Standard),
        ("buffered", StreamingMode::Buffered),
        ("always", StreamingMode::Always),
    ];
    for (mode_str, expected_mode) in modes {
        with_vars(
            vec![
                ("MODELMUX_AUTH_SERVICE_ACCOUNT_JSON", Some(get_test_key_json())),
                ("MODELMUX_STREAMING_MODE", Some(mode_str)),
                ("LLM_PROVIDER", Some("vertex")),
                ("VERTEX_PROJECT", Some("test-project")),
                ("VERTEX_LOCATION", Some("us-central1")),
                ("VERTEX_MODEL_ID", Some("claude-3-5-sonnet@20241022")),
            ],
            || {
                let config = Config::load().expect("Should load config");
                assert_eq!(
                    config.streaming.mode, expected_mode,
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
            ("MODELMUX_AUTH_SERVICE_ACCOUNT_JSON", Some(get_test_key_json())),
            ("LLM_PROVIDER", Some("vertex")),
            ("VERTEX_PROJECT", Some("test-project")),
            ("VERTEX_LOCATION", Some("us-central1")),
            ("VERTEX_MODEL_ID", Some("claude-3-5-sonnet@20241022")),
        ],
        || {
            let config = Config::load().expect("Should load config");
            assert_eq!(
                config.streaming.mode,
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
            ("MODELMUX_AUTH_SERVICE_ACCOUNT_JSON", Some(get_test_key_json())),
            ("MODELMUX_SERVER_ENABLE_RETRIES", Some("false")),
            ("MODELMUX_SERVER_MAX_RETRY_ATTEMPTS", Some("5")),
            ("LLM_PROVIDER", Some("vertex")),
            ("VERTEX_PROJECT", Some("test-project")),
            ("VERTEX_LOCATION", Some("us-central1")),
            ("VERTEX_MODEL_ID", Some("claude-3-5-sonnet@20241022")),
        ],
        || {
            let config = Config::load().expect("Should load config");
            assert_eq!(config.server.enable_retries, false, "Should parse enable_retries");
            assert_eq!(config.server.max_retry_attempts, 5, "Should parse max_retry_attempts");
        },
    );
}

/// Test default retry configuration
#[test]
fn test_default_retry_config() {
    with_vars(
        vec![
            ("MODELMUX_AUTH_SERVICE_ACCOUNT_JSON", Some(get_test_key_json())),
            ("LLM_PROVIDER", Some("vertex")),
            ("VERTEX_PROJECT", Some("test-project")),
            ("VERTEX_LOCATION", Some("us-central1")),
            ("VERTEX_MODEL_ID", Some("claude-3-5-sonnet@20241022")),
        ],
        || {
            let config = Config::load().expect("Should load config");
            assert_eq!(config.server.enable_retries, true, "Default enable_retries should be true");
            assert_eq!(
                config.server.max_retry_attempts, 3,
                "Default max_retry_attempts should be 3"
            );
        },
    );
}

/// Test that config fails without required auth configuration
#[test]
fn test_config_fails_without_auth() {
    with_vars(
        vec![
            ("LLM_PROVIDER", Some("vertex")),
            ("VERTEX_PROJECT", Some("test-project")),
            ("VERTEX_LOCATION", Some("us-central1")),
            ("VERTEX_MODEL_ID", Some("claude-3-5-sonnet@20241022")),
        ],
        || {
            let result = Config::load();
            assert!(result.is_err(), "Should fail when auth configuration is missing");
        },
    );
}

/// Test StreamingMode::from_str function
#[test]
fn test_streaming_mode_from_str() {
    assert_eq!(StreamingMode::from_str("auto").unwrap(), StreamingMode::Auto);
    assert_eq!(StreamingMode::from_str("AUTO").unwrap(), StreamingMode::Auto); // Case insensitive
    assert_eq!(StreamingMode::from_str("never").unwrap(), StreamingMode::Never);
    assert_eq!(StreamingMode::from_str("standard").unwrap(), StreamingMode::Standard);
    assert_eq!(StreamingMode::from_str("buffered").unwrap(), StreamingMode::Buffered);
    assert_eq!(StreamingMode::from_str("always").unwrap(), StreamingMode::Always);
    assert!(StreamingMode::from_str("unknown").is_err()); // Should fail for invalid input
}

/// Test LogLevel::from_str function
#[test]
fn test_log_level_from_str() {
    assert_eq!(LogLevel::from_str("trace").unwrap(), LogLevel::Trace);
    assert_eq!(LogLevel::from_str("TRACE").unwrap(), LogLevel::Trace); // Case insensitive
    assert_eq!(LogLevel::from_str("debug").unwrap(), LogLevel::Debug);
    assert_eq!(LogLevel::from_str("info").unwrap(), LogLevel::Info);
    assert_eq!(LogLevel::from_str("warn").unwrap(), LogLevel::Warn);
    assert_eq!(LogLevel::from_str("error").unwrap(), LogLevel::Error);
    assert!(LogLevel::from_str("unknown").is_err()); // Should fail for invalid input
}

/// Test build_predict_url functionality
#[test]
fn test_build_predict_url() {
    with_vars(
        vec![
            ("MODELMUX_AUTH_SERVICE_ACCOUNT_JSON", Some(get_test_key_json())),
            ("LLM_PROVIDER", Some("vertex")),
            ("VERTEX_PROJECT", Some("test-project")),
            ("VERTEX_LOCATION", Some("us-central1")),
            ("VERTEX_MODEL_ID", Some("claude-3-5-sonnet@20241022")),
        ],
        || {
            let config = Config::load().expect("Should load config");
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
                !non_streaming_url.contains("streamRawPredict"),
                "Non-streaming URL should not contain streamRawPredict"
            );
        },
    );
}
