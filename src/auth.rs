//!
//! Google Cloud Platform authentication provider for Vertex AI access.
//!
//! Handles OAuth2 authentication with Google Cloud Platform using service account
//! credentials. Follows Single Responsibility Principle - only handles authentication.
//!
//! Authors:
//!   Jaro <yarenty@gmail.com>
//!
//! Copyright (c) 2026 SkyCorp

/* --- uses ------------------------------------------------------------------------------------ */

use std::sync::Arc;

use hyper_util::client::legacy::connect::HttpConnector;
use tokio::sync::Mutex;
use yup_oauth2::authenticator::Authenticator;
use yup_oauth2::{ServiceAccountAuthenticator, ServiceAccountKey as OAuthKey, hyper_rustls};

use crate::config::ServiceAccountKey;
use crate::error::{ProxyError, Result};

/* --- types ----------------------------------------------------------------------------------- */

///
/// Google Cloud Platform authentication provider.
///
/// Manages OAuth2 authentication flow for accessing Vertex AI services using
/// service account credentials. Handles token generation and refresh automatically.
pub struct GcpAuthProvider {
    /** the OAuth2 authenticator instance for token management */
    authenticator: Arc<Mutex<ServiceAccountAuth>>,
}

/* --- constants ------------------------------------------------------------------------------ */

/** Google Cloud Platform scope for accessing cloud services */
const CLOUD_PLATFORM_SCOPE: &str = "https://www.googleapis.com/auth/cloud-platform";

/* --- start of code -------------------------------------------------------------------------- */

// Type alias for the authenticator type returned by ServiceAccountAuthenticator::builder().build()
type ServiceAccountAuth = Authenticator<hyper_rustls::HttpsConnector<HttpConnector>>;

impl GcpAuthProvider {
    ///
    /// Create a new GCP authentication provider.
    ///
    /// Initializes the OAuth2 authenticator with the provided service account
    /// credentials. The authenticator will automatically handle token refresh
    /// when needed.
    ///
    /// # Arguments
    ///  * `service_account_key` - Google Cloud service account credentials
    ///
    /// # Returns
    ///  * New authentication provider instance
    ///  * `ProxyError::Auth` if authenticator creation fails
    pub async fn new(service_account_key: &ServiceAccountKey) -> Result<Self> {
        let oauth_key = Self::convert_service_account_key(service_account_key);
        let authenticator = Self::create_authenticator(oauth_key).await?;

        Ok(Self { authenticator: Arc::new(Mutex::new(authenticator)) })
    }

    ///
    /// Get a valid access token for Google Cloud Platform.
    ///
    /// Retrieves a fresh access token, automatically refreshing if the current
    /// token has expired. The token can be used for authenticating requests
    /// to Vertex AI services.
    ///
    /// # Returns
    ///  * Valid access token string
    ///  * `ProxyError::Auth` if token retrieval fails
    pub async fn get_access_token(&self) -> Result<String> {
        let scopes = &[CLOUD_PLATFORM_SCOPE];
        let guard = self.authenticator.lock().await;

        let token = guard
            .token(scopes)
            .await
            .map_err(|e| ProxyError::Auth(format!("Failed to get access token: {}", e)))?;

        // AccessToken has a token() method that returns Option<&str>
        token
            .token()
            .ok_or_else(|| ProxyError::Auth("Access token is missing from response".to_string()))
            .map(|s| s.to_string())
    }

    ///
    /// Convert internal service account key to OAuth2 library format.
    ///
    /// Transforms our configuration structure into the format expected by
    /// the yup-oauth2 library for service account authentication.
    ///
    /// # Arguments
    ///  * `service_account_key` - internal service account key structure
    ///
    /// # Returns
    ///  * OAuth2 library service account key structure
    fn convert_service_account_key(service_account_key: &ServiceAccountKey) -> OAuthKey {
        OAuthKey {
            key_type: Some("service_account".to_string()),
            project_id: Some(service_account_key.project_id.clone()),
            private_key_id: Some(service_account_key.private_key_id.clone()),
            private_key: service_account_key.private_key.clone(),
            client_email: service_account_key.client_email.clone(),
            client_id: Some(service_account_key.client_id.clone()),
            auth_uri: Some(service_account_key.auth_uri.clone()),
            token_uri: service_account_key.token_uri.clone(),
            auth_provider_x509_cert_url: Some(
                service_account_key.auth_provider_x509_cert_url.clone(),
            ),
            client_x509_cert_url: Some(service_account_key.client_x509_cert_url.clone()),
        }
    }

    ///
    /// Create the OAuth2 authenticator instance.
    ///
    /// Builds and configures the authenticator using the provided OAuth2 key.
    /// This handles the low-level OAuth2 flow setup.
    ///
    /// # Arguments
    ///  * `oauth_key` - OAuth2 service account key
    ///
    /// # Returns
    ///  * Configured authenticator instance
    ///  * `ProxyError::Auth` if authenticator creation fails
    async fn create_authenticator(oauth_key: OAuthKey) -> Result<ServiceAccountAuth> {
        ServiceAccountAuthenticator::builder(oauth_key)
            .build()
            .await
            .map_err(|e| ProxyError::Auth(format!("Failed to create authenticator: {}", e)))
    }
}
