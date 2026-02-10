//! Validation tests for ModelMux configuration validation

use modelmux::config::{Config, ValidationSeverity};

/// Test that validation detects empty private key
#[test]
fn test_validation_empty_private_key() {
    use modelmux::config::{LogLevel, ServiceAccountKey, StreamingMode};

    let config = Config {
        llm_url: "https://test.example.com/v1/".to_string(),
        llm_chat_endpoint: "test-model:streamRawPredict".to_string(),
        llm_model: "test-model".to_string(),
        service_account_key: ServiceAccountKey {
            project_id: "test-project".to_string(),
            private_key_id: "test-key-id".to_string(),
            private_key: "".to_string(), // Empty private key
            client_email: "test@test-project.iam.gserviceaccount.com".to_string(),
            client_id: "123456789".to_string(),
            auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
            token_uri: "https://oauth2.googleapis.com/token".to_string(),
            auth_provider_x509_cert_url: "https://www.googleapis.com/oauth2/v1/certs".to_string(),
            client_x509_cert_url: "https://www.googleapis.com/robot/v1/metadata/x509/test%40test-project.iam.gserviceaccount.com".to_string(),
        },
        port: 3000,
        log_level: LogLevel::Info,
        enable_retries: true,
        max_retry_attempts: 3,
        streaming_mode: StreamingMode::Auto,
    };

    let issues = config.validate();
    assert!(
        issues.iter().any(|i| i.field == "GCP_SERVICE_ACCOUNT_KEY" && i.severity == ValidationSeverity::Error),
        "Should detect empty private key"
    );
}

/// Test that validation detects invalid email format
#[test]
fn test_validation_invalid_email() {
    use modelmux::config::{LogLevel, ServiceAccountKey, StreamingMode};

    let config = Config {
        llm_url: "https://test.example.com/v1/".to_string(),
        llm_chat_endpoint: "test-model:streamRawPredict".to_string(),
        llm_model: "test-model".to_string(),
        service_account_key: ServiceAccountKey {
            project_id: "test-project".to_string(),
            private_key_id: "test-key-id".to_string(),
            private_key: "-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----\n".to_string(),
            client_email: "invalid-email".to_string(), // Invalid email
            client_id: "123456789".to_string(),
            auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
            token_uri: "https://oauth2.googleapis.com/token".to_string(),
            auth_provider_x509_cert_url: "https://www.googleapis.com/oauth2/v1/certs".to_string(),
            client_x509_cert_url: "https://www.googleapis.com/robot/v1/metadata/x509/test%40test-project.iam.gserviceaccount.com".to_string(),
        },
        port: 3000,
        log_level: LogLevel::Info,
        enable_retries: true,
        max_retry_attempts: 3,
        streaming_mode: StreamingMode::Auto,
    };

    let issues = config.validate();
    assert!(
        issues.iter().any(|i| i.field == "GCP_SERVICE_ACCOUNT_KEY" && i.message.contains("email")),
        "Should detect invalid email format"
    );
}

/// Test that validation detects invalid port
#[test]
fn test_validation_invalid_port() {
    use modelmux::config::{LogLevel, ServiceAccountKey, StreamingMode};

    let config = Config {
        llm_url: "https://test.example.com/v1/".to_string(),
        llm_chat_endpoint: "test-model:streamRawPredict".to_string(),
        llm_model: "test-model".to_string(),
        service_account_key: ServiceAccountKey {
            project_id: "test-project".to_string(),
            private_key_id: "test-key-id".to_string(),
            private_key: "-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----\n".to_string(),
            client_email: "test@test-project.iam.gserviceaccount.com".to_string(),
            client_id: "123456789".to_string(),
            auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
            token_uri: "https://oauth2.googleapis.com/token".to_string(),
            auth_provider_x509_cert_url: "https://www.googleapis.com/oauth2/v1/certs".to_string(),
            client_x509_cert_url: "https://www.googleapis.com/robot/v1/metadata/x509/test%40test-project.iam.gserviceaccount.com".to_string(),
        },
        port: 0, // Invalid port
        log_level: LogLevel::Info,
        enable_retries: true,
        max_retry_attempts: 3,
        streaming_mode: StreamingMode::Auto,
    };

    let issues = config.validate();
    assert!(
        issues.iter().any(|i| i.field == "PORT" && i.severity == ValidationSeverity::Error),
        "Should detect invalid port"
    );
}

/// Test that validation detects warnings for non-HTTPS URLs
#[test]
fn test_validation_http_url_warning() {
    use modelmux::config::{LogLevel, ServiceAccountKey, StreamingMode};

    let config = Config {
        llm_url: "http://test.example.com/v1/".to_string(), // HTTP instead of HTTPS
        llm_chat_endpoint: "test-model:streamRawPredict".to_string(),
        llm_model: "test-model".to_string(),
        service_account_key: ServiceAccountKey {
            project_id: "test-project".to_string(),
            private_key_id: "test-key-id".to_string(),
            private_key: "-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----\n".to_string(),
            client_email: "test@test-project.iam.gserviceaccount.com".to_string(),
            client_id: "123456789".to_string(),
            auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
            token_uri: "https://oauth2.googleapis.com/token".to_string(),
            auth_provider_x509_cert_url: "https://www.googleapis.com/oauth2/v1/certs".to_string(),
            client_x509_cert_url: "https://www.googleapis.com/robot/v1/metadata/x509/test%40test-project.iam.gserviceaccount.com".to_string(),
        },
        port: 3000,
        log_level: LogLevel::Info,
        enable_retries: true,
        max_retry_attempts: 3,
        streaming_mode: StreamingMode::Auto,
    };

    let issues = config.validate();
    assert!(
        issues.iter().any(|i| i.field == "LLM_URL" && i.severity == ValidationSeverity::Warning && i.message.contains("HTTPS")),
        "Should warn about non-HTTPS URL"
    );
}

/// Test that validation detects high retry attempts warning
#[test]
fn test_validation_high_retry_warning() {
    use modelmux::config::{LogLevel, ServiceAccountKey, StreamingMode};

    let config = Config {
        llm_url: "https://test.example.com/v1/".to_string(),
        llm_chat_endpoint: "test-model:streamRawPredict".to_string(),
        llm_model: "test-model".to_string(),
        service_account_key: ServiceAccountKey {
            project_id: "test-project".to_string(),
            private_key_id: "test-key-id".to_string(),
            private_key: "-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----\n".to_string(),
            client_email: "test@test-project.iam.gserviceaccount.com".to_string(),
            client_id: "123456789".to_string(),
            auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
            token_uri: "https://oauth2.googleapis.com/token".to_string(),
            auth_provider_x509_cert_url: "https://www.googleapis.com/oauth2/v1/certs".to_string(),
            client_x509_cert_url: "https://www.googleapis.com/robot/v1/metadata/x509/test%40test-project.iam.gserviceaccount.com".to_string(),
        },
        port: 3000,
        log_level: LogLevel::Info,
        enable_retries: true,
        max_retry_attempts: 20, // Very high
        streaming_mode: StreamingMode::Auto,
    };

    let issues = config.validate();
    assert!(
        issues.iter().any(|i| i.field == "MAX_RETRY_ATTEMPTS" && i.severity == ValidationSeverity::Warning),
        "Should warn about high retry attempts"
    );
}

/// Test that valid configuration has no errors
#[test]
fn test_validation_valid_config() {
    use modelmux::config::{LogLevel, ServiceAccountKey, StreamingMode};

    let config = Config {
        llm_url: "https://europe-west1-aiplatform.googleapis.com/v1/projects/test/locations/europe-west1/publishers/".to_string(),
        llm_chat_endpoint: "claude-sonnet-4:streamRawPredict".to_string(),
        llm_model: "claude-sonnet-4".to_string(),
        service_account_key: ServiceAccountKey {
            project_id: "test-project".to_string(),
            private_key_id: "test-key-id".to_string(),
            private_key: "-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC\n-----END PRIVATE KEY-----\n".to_string(),
            client_email: "test@test-project.iam.gserviceaccount.com".to_string(),
            client_id: "123456789".to_string(),
            auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
            token_uri: "https://oauth2.googleapis.com/token".to_string(),
            auth_provider_x509_cert_url: "https://www.googleapis.com/oauth2/v1/certs".to_string(),
            client_x509_cert_url: "https://www.googleapis.com/robot/v1/metadata/x509/test%40test-project.iam.gserviceaccount.com".to_string(),
        },
        port: 3000,
        log_level: LogLevel::Info,
        enable_retries: true,
        max_retry_attempts: 3,
        streaming_mode: StreamingMode::Auto,
    };

    let issues = config.validate();
    let errors: Vec<_> = issues.iter().filter(|i| i.severity == ValidationSeverity::Error).collect();
    assert_eq!(errors.len(), 0, "Valid config should have no errors");
}
