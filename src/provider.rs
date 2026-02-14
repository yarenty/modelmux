//!
//! LLM provider abstraction for multi-vendor support.
//!
//! Each provider implements [LlmProviderBackend]. Config is driven by `LLM_PROVIDER`;
//! only the matching provider is loaded (Vertex: full URL or VERTEX_* structure;
//! others: provider-specific vars, with stubs ready for future implementation).
//!
//! Authors:
//!   Jaro <yarenty@gmail.com>
//!
//! Copyright (c) 2026 SkyCorp

use std::env;

use crate::config::ServiceAccountKey;
use crate::error::{ProxyError, Result};

/* --- auth strategy --------------------------------------------------------------------------- */

///
/// How to authenticate requests to the LLM backend.
///
/// Each provider returns the strategy it needs; the server uses it to attach
/// the correct headers (e.g. GCP OAuth2 vs Bearer token from env).
#[derive(Debug, Clone)]
pub enum AuthStrategy {
    /// Google Cloud OAuth2 with service account (Vertex AI).
    GcpOAuth2(ServiceAccountKey),
    /// Static Bearer token (e.g. from OPENAI_API_KEY, MISTRAL_API_KEY).
    #[allow(dead_code)]
    BearerToken(String),
}

/* --- provider trait -------------------------------------------------------------------------- */

///
/// Trait that every LLM backend provider must implement.
///
/// Ensures a consistent interface for URL building, display model name,
/// and authentication regardless of vendor.
pub trait LlmProviderBackend: std::fmt::Debug + Send + Sync {
    ///
    /// Provider identifier (e.g. `"vertex"`, `"openai_compatible"`).
    fn id(&self) -> &'static str;

    ///
    /// Build the full request URL for this request (streaming or not).
    ///
    /// Vertex: resource URL + `:streamRawPredict` / `:rawPredict`.
    /// OpenAI-compatible: base URL + path (e.g. `/v1/chat/completions`).
    fn build_request_url(&self, is_streaming: bool) -> String;

    ///
    /// Display name for the model in OpenAI-compatible responses (e.g. `/v1/models`, `choice.model`).
    fn display_model_name(&self) -> &str;

    ///
    /// How to authenticate requests to this backend.
    fn auth_strategy(&self) -> &AuthStrategy;
}

/* --- vertex provider ------------------------------------------------------------------------- */

///
/// Vertex AI provider: supports full URL override or VERTEX_* structure.
#[derive(Debug, Clone)]
pub struct VertexProvider {
    pub predict_resource_url: String,
    pub display_model: String,
    pub auth: AuthStrategy,
}

impl VertexProvider {
    ///
    /// Load Vertex provider from environment.
    ///
    /// Requires `LLM_PROVIDER=vertex` (or unset). URL from `LLM_URL` or from
    /// `VERTEX_REGION`, `VERTEX_PROJECT`, `VERTEX_LOCATION`, `VERTEX_PUBLISHER`, `VERTEX_MODEL_ID`.
    #[allow(dead_code)]
    pub fn from_env() -> Result<Self> {
        let service_account_key = Self::load_service_account_key()?;
        let (predict_resource_url, display_model) = Self::resolve_predict_url_and_model()?;
        let auth = AuthStrategy::GcpOAuth2(service_account_key);

        Ok(Self { predict_resource_url, display_model, auth })
    }

    ///
    /// Load Vertex provider with provided service account key (to avoid circular dependency).
    ///
    /// Requires `LLM_PROVIDER=vertex` (or unset). URL from `LLM_URL` or from
    /// `VERTEX_REGION`, `VERTEX_PROJECT`, `VERTEX_LOCATION`, `VERTEX_PUBLISHER`, `VERTEX_MODEL_ID`.
    pub fn from_env_with_key(service_account_key: ServiceAccountKey) -> Result<Self> {
        let (predict_resource_url, display_model) = Self::resolve_predict_url_and_model()?;
        let auth = AuthStrategy::GcpOAuth2(service_account_key);

        Ok(Self { predict_resource_url, display_model, auth })
    }

    #[allow(dead_code)]
    fn load_service_account_key() -> Result<ServiceAccountKey> {
        crate::config::Config::load_service_account_key_standalone()
    }

    fn resolve_predict_url_and_model() -> Result<(String, String)> {
        if let Ok(url) = env::var("LLM_URL") {
            if !url.trim().is_empty() {
                let resource_url = Self::strip_predict_method_suffix(url.trim());
                let display = Self::get_model_display_name_override()?;
                return Ok((resource_url, display));
            }
        }

        if Self::has_vertex_vars() {
            let resource_url = Self::build_vertex_resource_url()?;
            let display = Self::get_model_display_name_vertex()?;
            return Ok((resource_url, display));
        }

        Err(ProxyError::Config(
            "Vertex URL not configured. Use LLM_URL or set VERTEX_REGION, \
             VERTEX_PROJECT, VERTEX_LOCATION, VERTEX_PUBLISHER, VERTEX_MODEL_ID."
                .to_string(),
        ))
    }

    fn strip_predict_method_suffix(s: &str) -> String {
        let s = s.trim();
        if s.ends_with(":streamRawPredict") {
            s.trim_end_matches(":streamRawPredict").to_string()
        } else if s.ends_with(":rawPredict") {
            s.trim_end_matches(":rawPredict").to_string()
        } else {
            s.to_string()
        }
    }

    fn has_vertex_vars() -> bool {
        env::var("VERTEX_REGION").is_ok()
            && env::var("VERTEX_PROJECT").is_ok()
            && env::var("VERTEX_LOCATION").is_ok()
            && env::var("VERTEX_PUBLISHER").is_ok()
            && env::var("VERTEX_MODEL_ID").is_ok()
    }

    fn build_vertex_resource_url() -> Result<String> {
        let region = env::var("VERTEX_REGION")
            .map_err(|_| ProxyError::Config("VERTEX_REGION is required.".to_string()))?;
        let project = env::var("VERTEX_PROJECT")
            .map_err(|_| ProxyError::Config("VERTEX_PROJECT is required.".to_string()))?;
        let location = env::var("VERTEX_LOCATION")
            .map_err(|_| ProxyError::Config("VERTEX_LOCATION is required.".to_string()))?;
        let publisher = env::var("VERTEX_PUBLISHER")
            .map_err(|_| ProxyError::Config("VERTEX_PUBLISHER is required.".to_string()))?;
        let model_id = env::var("VERTEX_MODEL_ID")
            .map_err(|_| ProxyError::Config("VERTEX_MODEL_ID is required.".to_string()))?;
        Ok(format!(
            "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/{}/models/{}",
            region.trim(),
            project.trim(),
            location.trim(),
            publisher.trim(),
            model_id.trim(),
        ))
    }

    fn get_model_display_name_override() -> Result<String> {
        if let Ok(name) = env::var("LLM_MODEL_DISPLAY_NAME") {
            if !name.trim().is_empty() {
                return Ok(name.trim().to_string());
            }
        }
        if let Ok(name) = env::var("LLM_MODEL") {
            if !name.trim().is_empty() {
                return Ok(name.trim().to_string());
            }
        }
        if let Ok(url) = env::var("LLM_URL") {
            let segment = url.trim().rsplit('/').next().unwrap_or("");
            let display = segment.split('@').next().unwrap_or(segment).to_string();
            if !display.is_empty() {
                return Ok(display);
            }
        }
        Err(ProxyError::Config("With LLM_URL set LLM_MODEL or LLM_MODEL_DISPLAY_NAME.".to_string()))
    }

    fn get_model_display_name_vertex() -> Result<String> {
        if let Ok(name) = env::var("LLM_MODEL_DISPLAY_NAME") {
            if !name.trim().is_empty() {
                return Ok(name.trim().to_string());
            }
        }
        if let Ok(name) = env::var("LLM_MODEL") {
            if !name.trim().is_empty() {
                return Ok(name.trim().to_string());
            }
        }
        if let Ok(id) = env::var("VERTEX_MODEL_ID") {
            let display = id.trim().split('@').next().unwrap_or(id.trim()).to_string();
            if !display.is_empty() {
                return Ok(display);
            }
        }
        Err(ProxyError::Config(
            "Set LLM_MODEL, LLM_MODEL_DISPLAY_NAME, or VERTEX_MODEL_ID for display name."
                .to_string(),
        ))
    }
}

impl LlmProviderBackend for VertexProvider {
    fn id(&self) -> &'static str {
        "vertex"
    }

    fn build_request_url(&self, is_streaming: bool) -> String {
        let method = if is_streaming { "streamRawPredict" } else { "rawPredict" };
        format!("{}:{}", self.predict_resource_url, method)
    }

    fn display_model_name(&self) -> &str {
        &self.display_model
    }

    fn auth_strategy(&self) -> &AuthStrategy {
        &self.auth
    }
}

/* --- openai-compatible provider (stub) ------------------------------------------------------- */

///
/// OpenAI-compatible providers (Mistral, Cloudflare, custom /v1/chat/completions endpoints).
///
/// Template for future implementation: base URL + path + Bearer token.
#[derive(Debug, Clone)]
pub struct OpenAiCompatibleProvider {
    _base_url: String,
    _chat_path: String,
    _display_model: String,
    auth: AuthStrategy,
}

impl OpenAiCompatibleProvider {
    ///
    /// Build from explicit values (for when from_env is implemented).
    #[allow(dead_code)]
    pub fn new(
        base_url: String,
        chat_path: String,
        display_model: String,
        auth: AuthStrategy,
    ) -> Self {
        Self { _base_url: base_url, _chat_path: chat_path, _display_model: display_model, auth }
    }

    ///
    /// Load from env. Currently returns an error (not yet implemented).
    pub fn from_env() -> Result<Self> {
        let _ = env::var("OPENAI_BASE_URL").map_err(|_| {
            ProxyError::Config(
                "openai_compatible provider not yet implemented. \
                 Set OPENAI_BASE_URL, OPENAI_CHAT_PATH, model and API key when supported."
                    .to_string(),
            )
        })?;
        Err(ProxyError::Config(
            "LLM_PROVIDER=openai_compatible is not yet implemented. Use vertex for now."
                .to_string(),
        ))
    }
}

impl LlmProviderBackend for OpenAiCompatibleProvider {
    fn id(&self) -> &'static str {
        "openai_compatible"
    }

    fn build_request_url(&self, is_streaming: bool) -> String {
        let _ = is_streaming;
        format!("{}{}", self._base_url.trim_end_matches('/'), self._chat_path)
    }

    fn display_model_name(&self) -> &str {
        &self._display_model
    }

    fn auth_strategy(&self) -> &AuthStrategy {
        &self.auth
    }
}

/* --- provider config enum -------------------------------------------------------------------- */

///
/// Enum of all supported LLM provider configs.
///
/// Config is selected by `LLM_PROVIDER`; only one variant is loaded.
#[derive(Debug, Clone)]
pub enum LlmProviderConfig {
    Vertex(VertexProvider),
    OpenAiCompatible(OpenAiCompatibleProvider),
}

impl LlmProviderConfig {
    ///
    /// Load the provider config from environment based on `LLM_PROVIDER`.
    ///
    /// Defaults to `vertex` when unset. Supported: `vertex`, `openai_compatible` (stub).
    #[allow(dead_code)]
    pub fn from_env() -> Result<Self> {
        let id = env::var("LLM_PROVIDER").unwrap_or_else(|_| "vertex".to_string());
        let id = id.trim().to_lowercase();
        match id.as_str() {
            "vertex" => VertexProvider::from_env().map(Self::Vertex),
            "openai_compatible" | "openai" | "mistral" | "cloudflare" => {
                OpenAiCompatibleProvider::from_env().map(Self::OpenAiCompatible)
            }
            _ => Err(ProxyError::Config(format!(
                "Unknown LLM_PROVIDER: '{}'. Supported: vertex, openai_compatible",
                id
            ))),
        }
    }

    ///
    /// Load the provider config with provided service account key (to avoid circular dependency).
    ///
    /// Defaults to `vertex` when unset. Supported: `vertex`, `openai_compatible` (stub).
    pub fn from_env_with_key(service_account_key: ServiceAccountKey) -> Result<Self> {
        let id = env::var("LLM_PROVIDER").unwrap_or_else(|_| "vertex".to_string());
        let id = id.trim().to_lowercase();
        match id.as_str() {
            "vertex" => VertexProvider::from_env_with_key(service_account_key).map(Self::Vertex),
            "openai_compatible" | "openai" | "mistral" | "cloudflare" => {
                OpenAiCompatibleProvider::from_env().map(Self::OpenAiCompatible)
            }
            _ => Err(ProxyError::Config(format!(
                "Unknown LLM_PROVIDER: '{}'. Supported: vertex, openai_compatible",
                id
            ))),
        }
    }
}

impl LlmProviderBackend for LlmProviderConfig {
    fn id(&self) -> &'static str {
        match self {
            Self::Vertex(p) => p.id(),
            Self::OpenAiCompatible(p) => p.id(),
        }
    }

    fn build_request_url(&self, is_streaming: bool) -> String {
        match self {
            Self::Vertex(p) => p.build_request_url(is_streaming),
            Self::OpenAiCompatible(p) => p.build_request_url(is_streaming),
        }
    }

    fn display_model_name(&self) -> &str {
        match self {
            Self::Vertex(p) => p.display_model_name(),
            Self::OpenAiCompatible(p) => p.display_model_name(),
        }
    }

    fn auth_strategy(&self) -> &AuthStrategy {
        match self {
            Self::Vertex(p) => p.auth_strategy(),
            Self::OpenAiCompatible(p) => p.auth_strategy(),
        }
    }
}
