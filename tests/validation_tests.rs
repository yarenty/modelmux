//! Validation tests for ModelMux configuration validation

use modelmux::config::{Config, ValidationSeverity};
use modelmux::provider::{AuthStrategy, LlmProviderConfig, VertexProvider};

fn vertex_config(
    predict_resource_url: &str,
    display_model: &str,
    service_account_key: modelmux::config::ServiceAccountKey,
    port: u16,
) -> Config {
    let vertex = VertexProvider {
        predict_resource_url: predict_resource_url.to_string(),
        display_model: display_model.to_string(),
        auth: AuthStrategy::GcpOAuth2(service_account_key),
    };
    Config {
        llm_provider: LlmProviderConfig::Vertex(vertex),
        port,
        log_level: modelmux::config::LogLevel::Info,
        enable_retries: true,
        max_retry_attempts: 3,
        streaming_mode: modelmux::config::StreamingMode::Auto,
    }
}

/// Test that validation detects empty private key
#[test]
fn test_validation_empty_private_key() {
    use modelmux::config::ServiceAccountKey;

    let key = ServiceAccountKey {
        project_id: "test-project".to_string(),
        private_key_id: "test-key-id".to_string(),
        private_key: "".to_string(),
        client_email: "test@test-project.iam.gserviceaccount.com".to_string(),
        client_id: "123456789".to_string(),
        auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
        token_uri: "https://oauth2.googleapis.com/token".to_string(),
        auth_provider_x509_cert_url: "https://www.googleapis.com/oauth2/v1/certs".to_string(),
        client_x509_cert_url: "https://www.googleapis.com/robot/v1/metadata/x509/test%40test-project.iam.gserviceaccount.com".to_string(),
    };
    let config = vertex_config("https://test.example.com/v1/test-model", "test-model", key, 3000);

    let issues = config.validate();
    assert!(
        issues.iter().any(|i| i.field == "GCP_SERVICE_ACCOUNT_KEY" && i.severity == ValidationSeverity::Error),
        "Should detect empty private key"
    );
}

/// Test that validation detects invalid email format
#[test]
fn test_validation_invalid_email() {
    use modelmux::config::ServiceAccountKey;

    let key = ServiceAccountKey {
        project_id: "test-project".to_string(),
        private_key_id: "test-key-id".to_string(),
        private_key: "-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----\n".to_string(),
        client_email: "invalid-email".to_string(),
        client_id: "123456789".to_string(),
        auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
        token_uri: "https://oauth2.googleapis.com/token".to_string(),
        auth_provider_x509_cert_url: "https://www.googleapis.com/oauth2/v1/certs".to_string(),
        client_x509_cert_url: "https://www.googleapis.com/robot/v1/metadata/x509/test%40test-project.iam.gserviceaccount.com".to_string(),
    };
    let config = vertex_config("https://test.example.com/v1/test-model", "test-model", key, 3000);

    let issues = config.validate();
    assert!(
        issues.iter().any(|i| i.field == "GCP_SERVICE_ACCOUNT_KEY" && i.message.contains("email")),
        "Should detect invalid email format"
    );
}

/// Test that validation detects invalid port
#[test]
fn test_validation_invalid_port() {
    use modelmux::config::ServiceAccountKey;

    let key = ServiceAccountKey {
        project_id: "test-project".to_string(),
        private_key_id: "test-key-id".to_string(),
        private_key: "-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----\n".to_string(),
        client_email: "test@test-project.iam.gserviceaccount.com".to_string(),
        client_id: "123456789".to_string(),
        auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
        token_uri: "https://oauth2.googleapis.com/token".to_string(),
        auth_provider_x509_cert_url: "https://www.googleapis.com/oauth2/v1/certs".to_string(),
        client_x509_cert_url: "https://www.googleapis.com/robot/v1/metadata/x509/test%40test-project.iam.gserviceaccount.com".to_string(),
    };
    let config = vertex_config("https://test.example.com/v1/test-model", "test-model", key, 0);

    let issues = config.validate();
    assert!(
        issues.iter().any(|i| i.field == "PORT" && i.severity == ValidationSeverity::Error),
        "Should detect invalid port"
    );
}

/// Test that validation detects warnings for non-HTTPS URLs
#[test]
fn test_validation_http_url_warning() {
    use modelmux::config::ServiceAccountKey;

    let key = ServiceAccountKey {
            project_id: "test-project".to_string(),
            private_key_id: "test-key-id".to_string(),
            private_key: "-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----\n".to_string(),
            client_email: "test@test-project.iam.gserviceaccount.com".to_string(),
            client_id: "123456789".to_string(),
            auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
            token_uri: "https://oauth2.googleapis.com/token".to_string(),
            auth_provider_x509_cert_url: "https://www.googleapis.com/oauth2/v1/certs".to_string(),
            client_x509_cert_url: "https://www.googleapis.com/robot/v1/metadata/x509/test%40test-project.iam.gserviceaccount.com".to_string(),
    };
    let config = vertex_config("http://test.example.com/v1/test-model", "test-model", key, 3000);

    let issues = config.validate();
    assert!(
        issues.iter().any(|i| i.field == "request_url" && i.severity == ValidationSeverity::Warning && i.message.contains("HTTPS")),
        "Should warn about non-HTTPS URL"
    );
}

/// Test that validation detects high retry attempts warning
#[test]
fn test_validation_high_retry_warning() {
    use modelmux::config::ServiceAccountKey;

    let key = ServiceAccountKey {
        project_id: "test-project".to_string(),
        private_key_id: "test-key-id".to_string(),
        private_key: "-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----\n".to_string(),
        client_email: "test@test-project.iam.gserviceaccount.com".to_string(),
        client_id: "123456789".to_string(),
        auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
        token_uri: "https://oauth2.googleapis.com/token".to_string(),
        auth_provider_x509_cert_url: "https://www.googleapis.com/oauth2/v1/certs".to_string(),
        client_x509_cert_url: "https://www.googleapis.com/robot/v1/metadata/x509/test%40test-project.iam.gserviceaccount.com".to_string(),
    };
    let vertex = VertexProvider {
        predict_resource_url: "https://test.example.com/v1/test-model".to_string(),
        display_model: "test-model".to_string(),
        auth: AuthStrategy::GcpOAuth2(key),
    };
    let config = Config {
        llm_provider: LlmProviderConfig::Vertex(vertex),
        port: 3000,
        log_level: modelmux::config::LogLevel::Info,
        enable_retries: true,
        max_retry_attempts: 20,
        streaming_mode: modelmux::config::StreamingMode::Auto,
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
    use modelmux::config::ServiceAccountKey;

    let key = ServiceAccountKey {
        project_id: "test-project".to_string(),
        private_key_id: "test-key-id".to_string(),
        private_key: "-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC\n-----END PRIVATE KEY-----\n".to_string(),
        client_email: "test@test-project.iam.gserviceaccount.com".to_string(),
        client_id: "123456789".to_string(),
        auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
        token_uri: "https://oauth2.googleapis.com/token".to_string(),
        auth_provider_x509_cert_url: "https://www.googleapis.com/oauth2/v1/certs".to_string(),
        client_x509_cert_url: "https://www.googleapis.com/robot/v1/metadata/x509/test%40test-project.iam.gserviceaccount.com".to_string(),
    };
    let vertex = VertexProvider {
        predict_resource_url: "https://europe-west1-aiplatform.googleapis.com/v1/projects/test/locations/europe-west1/publishers/anthropic/models/claude-sonnet-4".to_string(),
        display_model: "claude-sonnet-4".to_string(),
        auth: AuthStrategy::GcpOAuth2(key),
    };
    let config = Config {
        llm_provider: LlmProviderConfig::Vertex(vertex),
        port: 3000,
        log_level: modelmux::config::LogLevel::Info,
        enable_retries: true,
        max_retry_attempts: 3,
        streaming_mode: modelmux::config::StreamingMode::Auto,
    };

    let issues = config.validate();
    let errors: Vec<_> = issues.iter().filter(|i| i.severity == ValidationSeverity::Error).collect();
    assert_eq!(errors.len(), 0, "Valid config should have no errors");
}
