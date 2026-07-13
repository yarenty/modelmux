//! URL construction tests for ModelMux.
//!
//! These tests prove that VertexProvider builds the correct Vertex AI endpoint
//! URL for every combination that matters in production:
//!
//!  1. Standard region  -> {region}-aiplatform.googleapis.com
//!  2. global region    -> aiplatform.googleapis.com  (no prefix)
//!  3. Named model inherits all fields from parent [vertex]
//!  4. Multiple named models each get their own model ID
//!  5. Named model overrides publisher
//!  6. Named model has its own explicit url
//!  7. Parent has url override; named model still builds correctly from region
//!  8. Unknown model name returns None (caller falls back to default)
//!  9. Config::build_predict_url_for_model routes by name end-to-end
//! 10. Default model with global region uses correct host (env-var path)

use modelmux::config::{Config, VertexConfig, VertexModelEntry};
use modelmux::provider::VertexProvider;
use temp_env::with_vars;
use tempfile::TempDir;

// ---- helpers ---------------------------------------------------------------

fn isolated_home() -> TempDir {
    TempDir::new().expect("temp dir")
}

fn with_isolated_home<F, V>(vars: V, f: F)
where
    F: FnOnce(),
    V: IntoIterator<Item = (&'static str, Option<String>)>,
{
    let tmp = isolated_home();
    let home = tmp.path().to_string_lossy().to_string();
    let mut all: Vec<(&'static str, Option<String>)> = vec![("HOME", Some(home))];
    all.extend(vars);
    with_vars(all, f);
}

fn test_key_json() -> &'static str {
    r#"{"type":"service_account","project_id":"test-project","private_key_id":"k","private_key":"-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC\n-----END PRIVATE KEY-----\n","client_email":"test@test-project.iam.gserviceaccount.com","client_id":"1","auth_uri":"https://accounts.google.com/o/oauth2/auth","token_uri":"https://oauth2.googleapis.com/token","auth_provider_x509_cert_url":"https://www.googleapis.com/oauth2/v1/certs","client_x509_cert_url":"https://www.googleapis.com/robot/v1/metadata/x509/test%40test-project.iam.gserviceaccount.com"}"#
}

/// Minimal VertexConfig with no named models.
fn base_cfg(region: &str, project: &str, location: &str, publisher: &str, model: &str) -> VertexConfig {
    VertexConfig {
        region:    Some(region.to_string()),
        project:   Some(project.to_string()),
        location:  Some(location.to_string()),
        publisher: Some(publisher.to_string()),
        model:     Some(model.to_string()),
        url:       None,
        models:    vec![],
    }
}

/// Named model entry with only name + model set; all other fields inherit from parent.
fn simple_entry(name: &str, model: &str) -> VertexModelEntry {
    VertexModelEntry {
        name:      name.to_string(),
        model:     model.to_string(),
        region:    None,
        project:   None,
        location:  None,
        publisher: None,
        url:       None,
    }
}

// ---- 1. Standard region ----------------------------------------------------

#[test]
fn test_standard_region_host() {
    let mut cfg = base_cfg("europe-west1", "my-project", "europe-west1", "anthropic", "claude-sonnet@20241022");
    cfg.models = vec![simple_entry("sonnet", "claude-sonnet@20241022")];

    let url = VertexProvider::build_url_for_named_model("sonnet", &cfg, false).unwrap();

    assert_eq!(
        url,
        "https://europe-west1-aiplatform.googleapis.com/v1/projects/my-project/locations/europe-west1/publishers/anthropic/models/claude-sonnet@20241022:rawPredict"
    );
}

// ---- 2. global region ------------------------------------------------------

#[test]
fn test_global_region_host() {
    let mut cfg = base_cfg("global", "my-project", "global", "anthropic", "claude-sonnet-4-6@default");
    cfg.models = vec![simple_entry("sonnet", "claude-sonnet-4-6@default")];

    let url = VertexProvider::build_url_for_named_model("sonnet", &cfg, false).unwrap();

    assert_eq!(
        url,
        "https://aiplatform.googleapis.com/v1/projects/my-project/locations/global/publishers/anthropic/models/claude-sonnet-4-6@default:rawPredict",
        "global region must NOT produce global-aiplatform prefix"
    );
    assert!(
        !url.contains("global-aiplatform"),
        "URL must not contain 'global-aiplatform', got: {}", url
    );
}

// ---- 3. Named model inherits all fields from parent ------------------------

#[test]
fn test_named_model_inherits_parent_fields() {
    let mut cfg = base_cfg("global", "basebox-llm-api", "global", "anthropic", "claude-sonnet-4-6@default");
    cfg.models = vec![simple_entry("claude-opus", "claude-opus-4@default")];

    let url = VertexProvider::build_url_for_named_model("claude-opus", &cfg, false).unwrap();

    assert_eq!(
        url,
        "https://aiplatform.googleapis.com/v1/projects/basebox-llm-api/locations/global/publishers/anthropic/models/claude-opus-4@default:rawPredict"
    );
}

// ---- 4. Multiple named models, each gets their own model ID ----------------

#[test]
fn test_multiple_named_models() {
    let mut cfg = base_cfg("global", "basebox-llm-api", "global", "anthropic", "claude-sonnet-4-6@default");
    cfg.models = vec![
        simple_entry("claude-opus-4.7", "claude-opus-4.7@default"),
        simple_entry("claude-opus-4.8", "claude-opus-4.8@default"),
    ];

    let url_47 = VertexProvider::build_url_for_named_model("claude-opus-4.7", &cfg, false).unwrap();
    let url_48 = VertexProvider::build_url_for_named_model("claude-opus-4.8", &cfg, true).unwrap();

    assert_eq!(
        url_47,
        "https://aiplatform.googleapis.com/v1/projects/basebox-llm-api/locations/global/publishers/anthropic/models/claude-opus-4.7@default:rawPredict",
        "url_47={}", url_47
    );
    assert_eq!(
        url_48,
        "https://aiplatform.googleapis.com/v1/projects/basebox-llm-api/locations/global/publishers/anthropic/models/claude-opus-4.8@default:streamRawPredict",
        "url_48={}", url_48
    );
}

// ---- 5. Named model overrides publisher ------------------------------------

#[test]
fn test_named_model_publisher_override() {
    let mut cfg = base_cfg("europe-west1", "my-project", "europe-west1", "anthropic", "claude-sonnet@20241022");
    cfg.models = vec![VertexModelEntry {
        name:      "gemini".to_string(),
        model:     "gemini-2-flash@001".to_string(),
        publisher: Some("google".to_string()),
        region:    None,
        project:   None,
        location:  None,
        url:       None,
    }];

    let url = VertexProvider::build_url_for_named_model("gemini", &cfg, false).unwrap();

    assert_eq!(
        url,
        "https://europe-west1-aiplatform.googleapis.com/v1/projects/my-project/locations/europe-west1/publishers/google/models/gemini-2-flash@001:rawPredict"
    );
}

// ---- 6. Named model has explicit url ---------------------------------------

#[test]
fn test_named_model_explicit_url() {
    let mut cfg = base_cfg("global", "my-project", "global", "anthropic", "claude-sonnet@default");
    cfg.models = vec![VertexModelEntry {
        name:      "custom".to_string(),
        model:     "ignored-because-url-is-set".to_string(),
        url:       Some("https://custom.example.com/v1/projects/p/locations/l/publishers/pub/models/my-model".to_string()),
        region:    None,
        project:   None,
        location:  None,
        publisher: None,
    }];

    let url = VertexProvider::build_url_for_named_model("custom", &cfg, false).unwrap();

    assert_eq!(
        url,
        "https://custom.example.com/v1/projects/p/locations/l/publishers/pub/models/my-model:rawPredict"
    );
}

// ---- 7. Parent has url override; named model still uses region for host ----
//
// The parent [vertex].url is a convenience override for the *default* model.
// Named model entries always build their URL from structural fields.
// region="global" -> vertex_host("global") = "aiplatform.googleapis.com".
// No string surgery on the parent url.

#[test]
fn test_named_model_ignores_parent_url_string_uses_region() {
    let mut cfg = base_cfg("global", "basebox-llm-api", "global", "anthropic", "claude-sonnet-4-6@default");
    // Parent url points to sonnet -- named model must NOT try to splice into this string
    cfg.url = Some(
        "https://aiplatform.googleapis.com/v1/projects/basebox-llm-api/locations/global/publishers/anthropic/models/claude-sonnet-4-6"
            .to_string(),
    );
    cfg.models = vec![simple_entry("claude-opus", "claude-opus-4@default")];

    let url = VertexProvider::build_url_for_named_model("claude-opus", &cfg, false).unwrap();

    // Must be derived from region="global", not spliced from the parent url string
    assert_eq!(
        url,
        "https://aiplatform.googleapis.com/v1/projects/basebox-llm-api/locations/global/publishers/anthropic/models/claude-opus-4@default:rawPredict"
    );
    // Crucially: the default model id from the parent url must not appear
    assert!(
        !url.contains("claude-sonnet-4-6"),
        "Named model URL must not contain the default model id, got: {}", url
    );
}

// ---- 8. Unknown model name returns None ------------------------------------

#[test]
fn test_unknown_model_returns_none() {
    let cfg = base_cfg("global", "my-project", "global", "anthropic", "claude-sonnet@default");

    let result = VertexProvider::build_url_for_named_model("nonexistent-model", &cfg, false);

    assert!(result.is_none(), "Unknown model name must return None so caller can fall back");
}

// ---- 9. Config::build_predict_url_for_model routes by name end-to-end ------

#[test]
fn test_config_routes_named_model_end_to_end() {
    with_isolated_home(
        vec![
            ("MODELMUX_AUTH_SERVICE_ACCOUNT_JSON", Some(test_key_json().to_string())),
            ("LLM_PROVIDER",      Some("vertex".to_string())),
            ("VERTEX_REGION",     Some("global".to_string())),
            ("VERTEX_PROJECT",    Some("basebox-llm-api".to_string())),
            ("VERTEX_LOCATION",   Some("global".to_string())),
            ("VERTEX_PUBLISHER",  Some("anthropic".to_string())),
            ("VERTEX_MODEL_ID",   Some("claude-sonnet-4-6@default".to_string())),
        ],
        || {
            let mut config = Config::load().expect("config loads");

            // Inject named models into the loaded config
            config.vertex = Some({
                let mut v = base_cfg("global", "basebox-llm-api", "global", "anthropic", "claude-sonnet-4-6@default");
                v.models = vec![
                    simple_entry("claude-opus-4.7", "claude-opus-4.7@default"),
                    simple_entry("claude-opus-4.8", "claude-opus-4.8@default"),
                ];
                v
            });

            let opus_47 = config.build_predict_url_for_model(Some("claude-opus-4.7"), false);
            let opus_48 = config.build_predict_url_for_model(Some("claude-opus-4.8"), true);
            let unknown = config.build_predict_url_for_model(Some("nonexistent"), false);
            let none    = config.build_predict_url_for_model(None, false);

            assert_eq!(
                opus_47,
                "https://aiplatform.googleapis.com/v1/projects/basebox-llm-api/locations/global/publishers/anthropic/models/claude-opus-4.7@default:rawPredict",
                "opus_47={}", opus_47
            );
            assert_eq!(
                opus_48,
                "https://aiplatform.googleapis.com/v1/projects/basebox-llm-api/locations/global/publishers/anthropic/models/claude-opus-4.8@default:streamRawPredict",
                "opus_48={}", opus_48
            );
            // Unknown and None fall back to the default provider URL -- just confirm they don't panic
            assert!(unknown.contains("rawPredict"), "unknown={}", unknown);
            assert!(none.contains("rawPredict"),    "none={}", none);
        },
    );
}

// ---- 10. Default model with global region via env vars ---------------------

#[test]
fn test_default_model_global_region_host() {
    with_isolated_home(
        vec![
            ("MODELMUX_AUTH_SERVICE_ACCOUNT_JSON", Some(test_key_json().to_string())),
            ("LLM_PROVIDER",     Some("vertex".to_string())),
            ("VERTEX_REGION",    Some("global".to_string())),
            ("VERTEX_PROJECT",   Some("basebox-llm-api".to_string())),
            ("VERTEX_LOCATION",  Some("global".to_string())),
            ("VERTEX_PUBLISHER", Some("anthropic".to_string())),
            ("VERTEX_MODEL_ID",  Some("claude-sonnet-4-6@default".to_string())),
        ],
        || {
            let config = Config::load().expect("config loads");
            let url = config.build_predict_url(false);

            assert!(
                url.starts_with("https://aiplatform.googleapis.com/"),
                "global region default URL must use aiplatform.googleapis.com, got: {}", url
            );
            assert!(
                !url.contains("global-aiplatform"),
                "URL must NOT contain 'global-aiplatform', got: {}", url
            );
            assert!(url.contains("basebox-llm-api"), "url={}", url);
            assert!(url.contains("claude-sonnet-4-6@default"), "url={}", url);
            assert!(url.ends_with(":rawPredict"), "url={}", url);
        },
    );
}
