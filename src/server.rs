//!
//! HTTP server implementation for the Vertex AI to OpenAI proxy.
//!
//! Handles incoming OpenAI-compatible requests and routes them to Vertex AI endpoints.
//! Implements both streaming and non-streaming responses with proper error handling
//! and logging. Follows Dependency Inversion Principle by depending on abstractions.
//!
//! Authors:
//!   Jaro <yarenty@gmail.com>
//!
//! Copyright (c) 2026 SkyCorp

/* --- uses ------------------------------------------------------------------------------------ */

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::sse::Event;
use axum::response::{IntoResponse, Response, Sse};
use reqwest::Client;
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReceiverStream;

use crate::auth::GcpAuthProvider;
use crate::config::Config;
use crate::converter::{AnthropicToOpenAiConverter, OpenAiToAnthropicConverter};
use crate::error::{ProxyError, Result};

/* --- types ----------------------------------------------------------------------------------- */

///
/// Application state containing all dependencies.
///
/// Follows Dependency Inversion Principle by depending on abstractions rather
/// than concrete implementations. Contains all services needed for request processing.
pub struct AppState {
    /** application configuration */
    pub config: Config,
    /** authentication provider for GCP access */
    pub auth_provider: Arc<GcpAuthProvider>,
    /** HTTP client for external requests */
    pub http_client: Client,
    /** converter from OpenAI to Anthropic format */
    pub openai_to_anthropic: OpenAiToAnthropicConverter,
    /** converter from Anthropic to OpenAI format */
    pub anthropic_to_openai: AnthropicToOpenAiConverter,
    /** metrics for monitoring */
    pub metrics: AppMetrics,
}

///
/// Application metrics for monitoring and observability.
///
/// Tracks various operational metrics for monitoring service health.
#[derive(Debug, Default)]
pub struct AppMetrics {
    /** total number of requests processed */
    pub total_requests: AtomicU64,
    /** total number of quota errors encountered */
    pub quota_errors: AtomicU64,
    /** total number of retry attempts made */
    pub retry_attempts: AtomicU64,
    /** total number of successful requests */
    pub successful_requests: AtomicU64,
    /** total number of failed requests */
    pub failed_requests: AtomicU64,
}

///
/// Parameters for processing stream chunks to avoid too many function arguments.
///
/// Groups related parameters for better code organization and maintainability.
struct StreamChunkParams<'a> {
    /** byte chunk from stream */
    chunk: &'a bytes::Bytes,
    /** line buffer for incomplete data */
    buffer: &'a mut String,
    /** application state */
    state: &'a Arc<AppState>,
    /** model identifier */
    model: &'a str,
    /** current tool call state */
    current_tool_call: &'a mut Option<crate::converter::anthropic_to_openai::StreamingToolCall>,
    /** tool calls presence flag */
    has_tool_calls: &'a mut bool,
    /** stop reason from delta */
    stop_reason_from_delta: &'a mut Option<String>,
    /** event sender channel */
    tx: &'a mpsc::Sender<Result<Event>>,
}

/* --- constants ------------------------------------------------------------------------------ */

/** HTTP client timeout in seconds */
const HTTP_CLIENT_TIMEOUT_SECS: u64 = 300;

/** Channel buffer size for streaming responses */
const STREAMING_CHANNEL_BUFFER: usize = 100;

/** Content type header for JSON requests */
const CONTENT_TYPE_JSON: &str = "application/json";

/** Authorization header name */
const AUTHORIZATION_HEADER: &str = "Authorization";

/** Bearer token prefix */
const BEARER_PREFIX: &str = "Bearer ";

/** Base delay in seconds for exponential backoff */
const BASE_RETRY_DELAY_SECS: u64 = 1;

/** Minimum buffer size for text accumulation in buffered streaming */
const MIN_BUFFER_SIZE: usize = 50;

/* --- start of code -------------------------------------------------------------------------- */

impl AppState {
    ///
    /// Create new application state with all dependencies.
    ///
    /// Initializes authentication provider, HTTP client, and format converters
    /// needed for proxy operation.
    ///
    /// # Arguments
    ///  * `config` - application configuration
    ///
    /// # Returns
    ///  * Application state with initialized dependencies
    ///  * `ProxyError` if initialization fails
    pub async fn new(config: Config) -> Result<Self> {
        let auth_provider = Arc::new(GcpAuthProvider::new(&config.service_account_key).await?);
        let http_client = Self::create_http_client()?;
        let openai_to_anthropic = OpenAiToAnthropicConverter::new(config.log_level);
        let anthropic_to_openai = AnthropicToOpenAiConverter::new(config.log_level);
        let metrics = AppMetrics::default();

        Ok(Self {
            config,
            auth_provider,
            http_client,
            openai_to_anthropic,
            anthropic_to_openai,
            metrics,
        })
    }

    ///
    /// Create HTTP client with appropriate timeouts.
    ///
    /// # Returns
    ///  * Configured HTTP client
    ///  * `ProxyError::Http` if client creation fails
    fn create_http_client() -> Result<Client> {
        Client::builder()
            .timeout(Duration::from_secs(HTTP_CLIENT_TIMEOUT_SECS))
            .build()
            .map_err(|e| ProxyError::Http(format!("Failed to create HTTP client: {}", e)))
    }
}

///
/// Handle OpenAI-compatible chat completions endpoint.
///
/// Processes incoming OpenAI format requests, converts them to Anthropic format,
/// forwards to Vertex AI, and converts the response back to OpenAI format.
/// Supports both streaming and non-streaming responses.
///
/// # Arguments
///  * `state` - shared application state
///  * `request` - OpenAI format request JSON
///
/// # Returns
///  * HTTP response with OpenAI format completion or error
pub async fn chat_completions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<Value>,
) -> axum::response::Response {
    state.metrics.total_requests.fetch_add(1, Ordering::Relaxed);

    match process_chat_completion(state.clone(), request, &headers).await {
        Ok(response) => {
            state.metrics.successful_requests.fetch_add(1, Ordering::Relaxed);
            response
        }
        Err(e) => {
            state.metrics.failed_requests.fetch_add(1, Ordering::Relaxed);
            create_error_response(&e)
        }
    }
}

///
/// Process chat completion request end-to-end.
///
/// # Arguments
///  * `state` - shared application state
///  * `request` - raw JSON request
///
/// # Returns
///  * HTTP response on success
///  * `ProxyError` on failure
async fn process_chat_completion(
    state: Arc<AppState>,
    mut request: Value,
    headers: &HeaderMap,
) -> Result<axum::response::Response> {
    // Log User-Agent for debugging if present
    if let Some(user_agent) = headers.get("user-agent") {
        if let Ok(ua_str) = user_agent.to_str() {
            tracing::debug!("Client User-Agent: {}", ua_str);
        }
    }

    // Check for goose - it needs special handling
    let is_goose_client = detect_goose_client(headers);

    if is_goose_client {
        // Goose gets non-streaming response wrapped in SSE format
        tracing::debug!("Using goose-compatible mode (non-streaming SSE)");
        let openai_request = parse_openai_request(request)?;
        log_incoming_request(&state, &openai_request);
        return handle_goose_request(state, openai_request).await;
    }

    // Determine streaming behavior based on configuration and client detection
    let (should_force_non_streaming, should_use_buffered_streaming) =
        determine_streaming_behavior(&state.config, headers);

    if should_force_non_streaming {
        // Force non-streaming for problematic clients or configuration
        if let Some(obj) = request.as_object_mut() {
            obj.insert("stream".to_string(), serde_json::Value::Bool(false));
        }
        tracing::debug!("Using non-streaming mode");
    } else if should_use_buffered_streaming {
        tracing::debug!("Using buffered streaming mode");
    } else {
        tracing::debug!("Using standard streaming mode");
    }

    let openai_request = parse_openai_request(request)?;
    log_incoming_request(&state, &openai_request);

    let anthropic_request = convert_to_anthropic(state.clone(), openai_request)?;
    let access_token = get_access_token(state.clone()).await?;
    let vertex_response =
        make_vertex_request_with_retry(state.clone(), &anthropic_request, &access_token).await?;

    if anthropic_request.stream {
        if should_use_buffered_streaming {
            handle_buffered_streaming_response(vertex_response, state).await
        } else {
            handle_streaming_response(vertex_response, state).await
        }
    } else {
        handle_non_streaming_response(vertex_response, state).await
    }
}

///
/// Parse OpenAI request from JSON value.
///
/// # Arguments
///  * `request` - raw JSON request
///
/// # Returns
///  * Parsed OpenAI request structure
///  * `ProxyError::Conversion` if parsing fails
fn parse_openai_request(
    request: Value,
) -> Result<crate::converter::openai_to_anthropic::OpenAiRequest> {
    serde_json::from_value(request)
        .map_err(|e| ProxyError::Conversion(format!("Invalid request format: {}", e)))
}

///
/// Log details about the incoming OpenAI request.
///
/// # Arguments
///  * `state` - application state for logging configuration
///  * `request` - OpenAI request to log
fn log_incoming_request(
    state: &Arc<AppState>,
    request: &crate::converter::openai_to_anthropic::OpenAiRequest,
) {
    state.openai_to_anthropic.debug("=== Incoming OpenAI Request ===");
    state.openai_to_anthropic.debug(&format!("Model: {:?}", request.model));
    state.openai_to_anthropic.debug(&format!("Stream: {:?}", request.stream));
    state.openai_to_anthropic.debug(&format!("Messages: {}", request.messages.len()));

    if let Some(ref tools) = request.tools {
        state.openai_to_anthropic.debug(&format!("Tools provided: {}", tools.len()));
        let tool_names: Vec<String> = tools.iter().map(|t| t.function.name.clone()).collect();
        state.openai_to_anthropic.debug(&format!("Tool names: {}", tool_names.join(", ")));
    }
}

///
/// Convert OpenAI request to Anthropic format.
///
/// # Arguments
///  * `state` - application state with converter
///  * `request` - OpenAI request to convert
///
/// # Returns
///  * Converted Anthropic request
///  * `ProxyError` if conversion fails
fn convert_to_anthropic(
    state: Arc<AppState>,
    request: crate::converter::openai_to_anthropic::OpenAiRequest,
) -> Result<crate::converter::openai_to_anthropic::AnthropicRequest> {
    state.openai_to_anthropic.convert(request)
}

///
/// Get access token for Vertex AI authentication.
///
/// # Arguments
///  * `state` - application state with auth provider
///
/// # Returns
///  * Valid access token
///  * `ProxyError::Auth` if token retrieval fails
async fn get_access_token(state: Arc<AppState>) -> Result<String> {
    state.auth_provider.get_access_token().await
}

///
/// Make HTTP request to Vertex AI endpoint with retry logic for quota errors.
///
/// # Arguments
///  * `state` - application state with HTTP client and config
///  * `anthropic_request` - request to send
///  * `access_token` - authentication token
///
/// # Returns
///  * HTTP response from Vertex AI
///  * `ProxyError::Request` if request fails after all retries
async fn make_vertex_request_with_retry(
    state: Arc<AppState>,
    anthropic_request: &crate::converter::openai_to_anthropic::AnthropicRequest,
    access_token: &str,
) -> Result<reqwest::Response> {
    if !state.config.enable_retries {
        return make_vertex_request(state, anthropic_request, access_token).await;
    }

    let mut attempts = 0;

    loop {
        attempts += 1;
        let response = make_vertex_request(state.clone(), anthropic_request, access_token).await;

        match response {
            Ok(resp) => return Ok(resp),
            Err(ProxyError::Http(msg)) if attempts < state.config.max_retry_attempts => {
                if msg.contains("Rate limit") || msg.contains("Quota exceeded") {
                    state.metrics.quota_errors.fetch_add(1, Ordering::Relaxed);
                    state.metrics.retry_attempts.fetch_add(1, Ordering::Relaxed);

                    let delay_secs = BASE_RETRY_DELAY_SECS * 2_u64.pow(attempts - 1);
                    tracing::warn!(
                        "Quota exceeded, retrying in {} seconds (attempt {}/{}) - Total quota errors: {}, \
             Total retries: {}",
                        delay_secs,
                        attempts,
                        state.config.max_retry_attempts,
                        state.metrics.quota_errors.load(Ordering::Relaxed),
                        state.metrics.retry_attempts.load(Ordering::Relaxed)
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(delay_secs)).await;
                    continue;
                }
                return Err(ProxyError::Http(msg));
            }
            Err(e) => return Err(e),
        }
    }
}

///
/// Make HTTP request to Vertex AI endpoint.
///
/// # Arguments
///  * `state` - application state with HTTP client and config
///  * `anthropic_request` - request to send
///  * `access_token` - authentication token
///
/// # Returns
///  * HTTP response from Vertex AI
///  * `ProxyError::Request` if request fails
async fn make_vertex_request(
    state: Arc<AppState>,
    anthropic_request: &crate::converter::openai_to_anthropic::AnthropicRequest,
    access_token: &str,
) -> Result<reqwest::Response> {
    let url = state.config.build_vertex_url(anthropic_request.stream);

    let response = state
        .http_client
        .post(&url)
        .header(AUTHORIZATION_HEADER, format!("{}{}", BEARER_PREFIX, access_token))
        .header("Content-Type", CONTENT_TYPE_JSON)
        .json(anthropic_request)
        .send()
        .await
        .map_err(ProxyError::Request)?;

    validate_vertex_response(response).await
}

///
/// Validate that Vertex AI response is successful.
///
/// # Arguments
///  * `response` - HTTP response to validate
///
/// # Returns
///  * `Ok(response)` if response is successful
///  * `ProxyError::Http` if response indicates error
async fn validate_vertex_response(response: reqwest::Response) -> Result<reqwest::Response> {
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());

        // Log the full error for debugging
        tracing::error!("Vertex AI error: {}", error_text);

        // Handle specific error types with appropriate client responses
        let client_error = match status.as_u16() {
            429 => {
                if error_text.contains("Quota exceeded") {
                    tracing::error!(
                        "Quota exceeded for Vertex AI. Consider requesting quota increase: https://cloud.google.com/vertex-ai/docs/generative-ai/quotas-genai"
                    );
                    ProxyError::Http(
            "Rate limit exceeded. Please try again later or contact support for quota increase."
              .to_string(),
          )
                } else {
                    ProxyError::Http("Too many requests. Please try again later.".to_string())
                }
            }
            400 => {
                if error_text.contains("tools: Input should be a valid list") {
                    ProxyError::Conversion("Invalid tools configuration in request.".to_string())
                } else {
                    ProxyError::Http("Bad request format.".to_string())
                }
            }
            401 => ProxyError::Auth(
                "Authentication failed. Please check your API credentials.".to_string(),
            ),
            403 => ProxyError::Auth("Access forbidden. Please check your permissions.".to_string()),
            404 => ProxyError::Http("Model or endpoint not found.".to_string()),
            500..=599 => ProxyError::Http(
                "Vertex AI service is temporarily unavailable. Please try again later.".to_string(),
            ),
            _ => ProxyError::Http(format!("Vertex AI returned error ({}): {}", status, error_text)),
        };

        return Err(client_error);
    }
    Ok(response)
}

///
/// Handle non-streaming response from Vertex AI.
///
/// Converts the complete Anthropic response to OpenAI format and returns it.
///
/// # Arguments
///  * `response` - HTTP response from Vertex AI
///  * `state` - application state with converter
///
/// # Returns
///  * OpenAI format JSON response
///  * `ProxyError` if conversion fails
async fn handle_non_streaming_response(
    response: reqwest::Response,
    state: Arc<AppState>,
) -> Result<Response> {
    state.anthropic_to_openai.debug("=== Non-streaming response ===");

    let anthropic_response: crate::converter::anthropic_to_openai::AnthropicResponse =
        response.json().await.map_err(ProxyError::Request)?;

    log_anthropic_response(&state, &anthropic_response);

    let openai_response =
        state.anthropic_to_openai.convert(anthropic_response, &state.config.llm_model);

    log_openai_response(&state, &openai_response);

    Ok(Json(openai_response).into_response())
}

///
/// Log details about the Anthropic response.
///
/// # Arguments
///  * `state` - application state for logging
///  * `response` - Anthropic response to log
fn log_anthropic_response(
    state: &Arc<AppState>,
    response: &crate::converter::anthropic_to_openai::AnthropicResponse,
) {
    state
        .anthropic_to_openai
        .debug(&format!("Anthropic response stop_reason: {:?}", response.stop_reason));

    let tool_calls_count = response
        .content
        .iter()
        .filter(|c| {
            matches!(
                c,
                crate::converter::anthropic_to_openai::AnthropicContentBlock::ToolUse { .. }
            )
        })
        .count();

    if tool_calls_count > 0 {
        state
            .anthropic_to_openai
            .debug(&format!("Anthropic response contains {} tool call(s)", tool_calls_count));
    }
}

///
/// Log details about the outgoing OpenAI response.
///
/// # Arguments
///  * `state` - application state for logging
///  * `response` - OpenAI response to log
fn log_openai_response(
    state: &Arc<AppState>,
    response: &crate::converter::anthropic_to_openai::OpenAiResponse,
) {
    state.anthropic_to_openai.debug("=== Outgoing OpenAI Response ===");
    state
        .anthropic_to_openai
        .debug(&format!("Finish reason: {}", response.choices[0].finish_reason));

    if let Some(ref tool_calls) = response.choices[0].message.tool_calls {
        state.anthropic_to_openai.debug(&format!("Tool calls in response: {}", tool_calls.len()));
    }
}

///
/// Handle streaming response from Vertex AI.
///
/// Sets up a streaming pipeline to convert Anthropic SSE events to OpenAI format
/// and streams them back to the client.
///
/// # Arguments
///  * `response` - streaming HTTP response from Vertex AI
///  * `state` - application state with converter
///
/// # Returns
///  * Server-Sent Events response stream
///  * `ProxyError` if streaming setup fails
async fn handle_streaming_response(
    response: reqwest::Response,
    state: Arc<AppState>,
) -> Result<Response> {
    state.anthropic_to_openai.debug("=== Streaming response ===");

    let (tx, rx) = mpsc::channel::<Result<Event>>(STREAMING_CHANNEL_BUFFER);
    let state_clone = state.clone();
    let model = state.config.llm_model.clone();

    tokio::spawn(async move {
        process_streaming_events(response, state_clone, model, tx).await;
    });

    Ok(Sse::new(ReceiverStream::new(rx)).into_response())
}

///
/// Process streaming events from Vertex AI and convert to OpenAI format.
///
/// # Arguments
///  * `response` - streaming HTTP response
///  * `state` - application state
///  * `model` - model identifier
///  * `tx` - channel sender for streaming events
async fn process_streaming_events(
    response: reqwest::Response,
    state: Arc<AppState>,
    model: String,
    tx: mpsc::Sender<Result<Event>>,
) {
    let mut stream = response.bytes_stream();
    let mut current_tool_call: Option<crate::converter::anthropic_to_openai::StreamingToolCall> =
        None;
    let mut has_tool_calls = false;
    let mut stop_reason_from_delta: Option<String> = None;
    let mut buffer = String::new();

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                let params = StreamChunkParams {
                    chunk: &chunk,
                    buffer: &mut buffer,
                    state: &state,
                    model: &model,
                    current_tool_call: &mut current_tool_call,
                    has_tool_calls: &mut has_tool_calls,
                    stop_reason_from_delta: &mut stop_reason_from_delta,
                    tx: &tx,
                };

                if let Err(e) = process_stream_chunk(params).await {
                    tracing::error!("Stream processing error: {}", e);
                    break;
                }
            }
            Err(e) => {
                tracing::error!("Stream chunk error: {}", e);
                break;
            }
        }
    }

    send_stream_done(&tx).await;
}

///
/// Determine streaming behavior based on configuration and client detection.
///
/// Uses the configuration's streaming mode setting and client detection
/// to decide how to handle streaming responses.
///
/// # Arguments
///  * `config` - application configuration
///  * `headers` - HTTP request headers
///
/// # Returns
///  * Tuple of (should_force_non_streaming, should_use_buffered_streaming)
fn determine_streaming_behavior(
    config: &crate::config::Config,
    headers: &HeaderMap,
) -> (bool, bool) {
    use crate::config::StreamingMode;

    match config.streaming_mode {
        StreamingMode::NonStreaming => (true, false),
        StreamingMode::Standard => (false, false),
        StreamingMode::Buffered => (false, true),
        StreamingMode::Auto => {
            let should_force_non_streaming = detect_problematic_client(headers);
            let should_use_buffered_streaming =
                !should_force_non_streaming && detect_buffered_streaming_client(headers);
            (should_force_non_streaming, should_use_buffered_streaming)
        }
    }
}

///
/// Detect problematic clients that don't handle Server-Sent Events properly.
///
/// Many IDE integrations and CLI tools expect JSON responses instead of SSE streams,
/// even when they set stream=true. This function identifies such clients.
///
/// # Arguments
///  * `headers` - HTTP request headers
///
/// # Returns
///  * `true` if the client should use non-streaming responses
fn detect_goose_client(headers: &HeaderMap) -> bool {
    // Check for goose using organization header (it doesn't send User-Agent)
    if let Some(org) = headers.get("openai-organization") {
        if let Ok(org_str) = org.to_str() {
            if org_str.to_lowercase().contains("basebox") {
                return true;
            }
        }
    }

    // Check for goose using project header
    if let Some(project) = headers.get("openai-project") {
        if let Ok(project_str) = project.to_str() {
            if project_str.to_lowercase().contains("gui") {
                return true;
            }
        }
    }

    false
}

fn detect_problematic_client(headers: &HeaderMap) -> bool {
    // Keep only clients that truly can't handle SSE

    if let Some(user_agent) = headers.get("user-agent") {
        if let Ok(user_agent_str) = user_agent.to_str() {
            let ua = user_agent_str.to_lowercase();

            // JetBrains IDEs moved to buffered streaming - they need SSE but with larger chunks
            // Keep only pure CLI tools here that truly can't handle SSE

            // Detect CLI tools that truly can't handle SSE
            if ua.contains("goose")
                || ua.contains("curl")
                || ua.contains("wget")
                || ua.contains("httpie")
                || ua.contains("python-requests")
            {
                return true;
            }

            // Detect other known problematic clients
            if ua.contains("postman") || ua.contains("insomnia") || ua.contains("thunderclient") {
                return true;
            }
        }
    }

    // Check Accept header - clients that don't accept text/event-stream probably can't handle SSE
    if let Some(accept) = headers.get("accept") {
        if let Ok(accept_str) = accept.to_str() {
            if !accept_str.contains("text/event-stream") && !accept_str.contains("*/*") {
                return true;
            }
        }
    }

    false
}

///
/// Detect clients that can handle SSE but prefer buffered streaming.
///
/// Some clients can handle Server-Sent Events but get overwhelmed by
/// word-by-word streaming. These clients benefit from buffered chunks.
///
/// # Arguments
///  * `headers` - HTTP request headers
///
/// # Returns
///  * `true` if the client should use buffered streaming
fn detect_buffered_streaming_client(headers: &HeaderMap) -> bool {
    if let Some(user_agent) = headers.get("user-agent") {
        if let Ok(user_agent_str) = user_agent.to_str() {
            let ua = user_agent_str.to_lowercase();

            // Clients that can handle SSE but prefer larger chunks
            if ua.contains("chrome")
                || ua.contains("firefox")
                || ua.contains("safari")
                || ua.contains("edge")
                || ua.contains("vscode")
                || ua.contains("visual studio code")
                || ua.contains("intellij")
                || ua.contains("rustrover")
                || ua.contains("jetbrains")
                || ua.contains("pycharm")
                || ua.contains("clion")
                || ua.contains("webstorm")
                || ua.contains("phpstorm")
            {
                return true;
            }
        }
    }

    false
}

///
/// Handle streaming response with buffering for better client compatibility.
///
/// Buffers small text chunks and sends them in larger batches to reduce
/// the chattiness of word-by-word streaming while maintaining responsiveness.
///
/// # Arguments
///  * `response` - streaming HTTP response from Vertex AI
///  * `state` - application state
///
/// # Returns
///  * Server-sent events response with buffered chunks
///  * `ProxyError` if processing fails
async fn handle_buffered_streaming_response(
    response: reqwest::Response,
    state: Arc<AppState>,
) -> Result<Response> {
    state.anthropic_to_openai.debug("=== Buffered streaming response ===");

    let (tx, rx) = mpsc::channel::<Result<Event>>(STREAMING_CHANNEL_BUFFER);
    let state_clone = state.clone();
    let model = state.config.llm_model.clone();

    tokio::spawn(async move {
        process_buffered_streaming_events(response, state_clone, model, tx).await;
    });

    Ok(Sse::new(ReceiverStream::new(rx)).into_response())
}

///
/// Process streaming events with buffering for text content.
///
/// Accumulates small text chunks and sends them in larger batches,
/// while immediately forwarding tool calls and control messages.
///
/// # Arguments
///  * `response` - streaming HTTP response
///  * `state` - application state
///  * `model` - model identifier
///  * `tx` - channel sender for streaming events
async fn process_buffered_streaming_events(
    response: reqwest::Response,
    state: Arc<AppState>,
    model: String,
    tx: mpsc::Sender<Result<Event>>,
) {
    let mut stream = response.bytes_stream();
    let mut current_tool_call: Option<crate::converter::anthropic_to_openai::StreamingToolCall> =
        None;
    let mut has_tool_calls = false;
    let mut stop_reason_from_delta: Option<String> = None;
    let mut buffer = String::new();
    let mut text_accumulator = String::new();

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                if let Err(e) = process_buffered_stream_chunk(
                    &chunk,
                    &mut buffer,
                    &state,
                    &model,
                    &mut current_tool_call,
                    &mut has_tool_calls,
                    &mut stop_reason_from_delta,
                    &mut text_accumulator,
                    &tx,
                )
                .await
                {
                    tracing::error!("Buffered stream processing error: {}", e);
                    break;
                }
            }
            Err(e) => {
                tracing::error!("Stream chunk error: {}", e);
                break;
            }
        }
    }

    // Send any remaining buffered text
    if !text_accumulator.is_empty() {
        send_buffered_text(&text_accumulator, &model, &state, &tx).await;
    }

    send_stream_done(&tx).await;
}

///
/// Process a single stream chunk with text buffering.
///
/// Similar to normal chunk processing but accumulates text content
/// and sends it in larger batches for better client compatibility.
async fn process_buffered_stream_chunk(
    chunk: &bytes::Bytes,
    buffer: &mut String,
    state: &Arc<AppState>,
    model: &str,
    current_tool_call: &mut Option<crate::converter::anthropic_to_openai::StreamingToolCall>,
    has_tool_calls: &mut bool,
    stop_reason_from_delta: &mut Option<String>,
    text_accumulator: &mut String,
    tx: &mpsc::Sender<Result<Event>>,
) -> Result<()> {
    let chunk_str = String::from_utf8_lossy(chunk);
    let new_content = format!("{}{}", buffer, chunk_str);

    let (lines_to_process, new_buffer) = split_sse_lines(&new_content);
    *buffer = new_buffer;

    for line in lines_to_process {
        if let Some(data) = extract_sse_data(line) {
            if data == "[DONE]" {
                // Send any remaining buffered text before DONE
                if !text_accumulator.is_empty() {
                    send_buffered_text(text_accumulator, model, state, tx).await;
                    text_accumulator.clear();
                }
                send_sse_event(tx, "[DONE]").await;
                continue;
            }

            process_buffered_sse_event(
                data,
                state,
                model,
                current_tool_call,
                has_tool_calls,
                stop_reason_from_delta,
                text_accumulator,
                tx,
            )
            .await;
        }
    }

    Ok(())
}

///
/// Process SSE event with text buffering logic.
///
/// Accumulates text content and forwards other events immediately.
async fn process_buffered_sse_event(
    data: &str,
    state: &Arc<AppState>,
    model: &str,
    current_tool_call: &mut Option<crate::converter::anthropic_to_openai::StreamingToolCall>,
    has_tool_calls: &mut bool,
    stop_reason_from_delta: &mut Option<String>,
    text_accumulator: &mut String,
    tx: &mpsc::Sender<Result<Event>>,
) {
    match serde_json::from_str::<crate::converter::anthropic_to_openai::AnthropicStreamEvent>(data)
    {
        Ok(event) => {
            if let Some(chunk) = state.anthropic_to_openai.convert_stream_event(
                &event,
                model,
                current_tool_call,
                has_tool_calls,
                stop_reason_from_delta,
            ) {
                // Check if this is a text chunk that should be buffered
                if let Some(content) =
                    chunk.choices.get(0).and_then(|choice| choice.delta.content.as_ref())
                {
                    // Accumulate text content
                    text_accumulator.push_str(content);

                    // Send buffered text if it's large enough or if we hit certain punctuation
                    if text_accumulator.len() >= MIN_BUFFER_SIZE
                        || content.contains('.')
                        || content.contains('!')
                        || content.contains('?')
                        || content.contains('\n')
                    {
                        send_buffered_text(text_accumulator, model, state, tx).await;
                        text_accumulator.clear();
                    }
                } else {
                    // Non-text chunks (tool calls, finish_reason, etc.) are sent immediately
                    // But first flush any accumulated text
                    if !text_accumulator.is_empty() {
                        send_buffered_text(text_accumulator, model, state, tx).await;
                        text_accumulator.clear();
                    }

                    // Send the non-text chunk
                    match serde_json::to_string(&chunk) {
                        Ok(json) => {
                            send_sse_event(tx, &json).await;
                        }
                        Err(e) => {
                            tracing::error!("Failed to serialize chunk: {}", e);
                        }
                    }
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to parse stream event: {} - data: {}", e, data);
        }
    }
}

///
/// Send accumulated text as a single chunk.
async fn send_buffered_text(
    text: &str,
    model: &str,
    state: &Arc<AppState>,
    tx: &mpsc::Sender<Result<Event>>,
) {
    if let Some(chunk) = state.anthropic_to_openai.create_text_chunk(text, model) {
        match serde_json::to_string(&chunk) {
            Ok(json) => {
                send_sse_event(tx, &json).await;
            }
            Err(e) => {
                tracing::error!("Failed to serialize buffered text chunk: {}", e);
            }
        }
    }
}

///
/// Handle goose requests with non-streaming response in SSE format.
///
/// Goose expects SSE but can't handle chunked responses properly.
/// This sends the complete response as a single SSE event.
async fn handle_goose_request(
    state: Arc<AppState>,
    openai_request: crate::converter::openai_to_anthropic::OpenAiRequest,
) -> Result<axum::response::Response> {
    // Convert to Anthropic format
    let anthropic_request = state.openai_to_anthropic.convert(openai_request)?;

    // Get access token
    let access_token = get_access_token(state.clone()).await?;

    // Make non-streaming request to Vertex AI
    let mut anthropic_request_non_streaming = anthropic_request;
    anthropic_request_non_streaming.stream = false;

    let vertex_response = make_vertex_request_with_retry(
        state.clone(),
        &anthropic_request_non_streaming,
        &access_token,
    )
    .await?;

    // Get the complete response
    let anthropic_response: crate::converter::anthropic_to_openai::AnthropicResponse =
        vertex_response.json().await.map_err(ProxyError::Request)?;

    // Convert to OpenAI format
    let openai_response =
        state.anthropic_to_openai.convert(anthropic_response, &state.config.llm_model);

    // Create SSE response with complete content
    let (tx, rx) = mpsc::channel::<Result<Event>>(STREAMING_CHANNEL_BUFFER);

    tokio::spawn(async move {
        // Send the complete response as SSE chunks
        if let Some(choice) = openai_response.choices.first() {
            // Handle text content if present
            if let Some(content) = &choice.message.content {
                let chunk = crate::converter::anthropic_to_openai::OpenAiStreamChunk {
                    id: openai_response.id.clone(),
                    object: "chat.completion.chunk".to_string(),
                    created: openai_response.created,
                    model: openai_response.model.clone(),
                    choices: vec![crate::converter::anthropic_to_openai::OpenAiStreamChoice {
                        index: 0,
                        delta: crate::converter::anthropic_to_openai::OpenAiStreamDelta {
                            content: Some(content.clone()),
                            tool_calls: None,
                        },
                        finish_reason: None,
                    }],
                };

                if let Ok(json) = serde_json::to_string(&chunk) {
                    let _ = tx.send(Ok(Event::default().data(json))).await;
                }
            }

            // Handle tool calls if present
            if let Some(tool_calls) = &choice.message.tool_calls {
                for (index, tool_call) in tool_calls.iter().enumerate() {
                    let tool_chunk = crate::converter::anthropic_to_openai::OpenAiStreamChunk {
                        id: openai_response.id.clone(),
                        object: "chat.completion.chunk".to_string(),
                        created: openai_response.created,
                        model: openai_response.model.clone(),
                        choices: vec![crate::converter::anthropic_to_openai::OpenAiStreamChoice {
                            index: 0,
                            delta: crate::converter::anthropic_to_openai::OpenAiStreamDelta {
                                content: None,
                                tool_calls: Some(vec![
                  crate::converter::anthropic_to_openai::OpenAiStreamToolCall {
                    index:     index as u32,
                    id:        Some(tool_call.id.clone()),
                    call_type: Some(tool_call.call_type.clone()),
                    function:  Some(
                      crate::converter::anthropic_to_openai::OpenAiStreamFunctionCall {
                        name:      Some(tool_call.function.name.clone()),
                        arguments: Some(tool_call.function.arguments.clone()),
                      },
                    ),
                  },
                ]),
                            },
                            finish_reason: None,
                        }],
                    };

                    if let Ok(json) = serde_json::to_string(&tool_chunk) {
                        let _ = tx.send(Ok(Event::default().data(json))).await;
                    }
                }
            }

            // Send finish chunk
            let finish_chunk = crate::converter::anthropic_to_openai::OpenAiStreamChunk {
                id: openai_response.id,
                object: "chat.completion.chunk".to_string(),
                created: openai_response.created,
                model: openai_response.model,
                choices: vec![crate::converter::anthropic_to_openai::OpenAiStreamChoice {
                    index: 0,
                    delta: crate::converter::anthropic_to_openai::OpenAiStreamDelta {
                        content: None,
                        tool_calls: None,
                    },
                    finish_reason: Some(choice.finish_reason.clone()),
                }],
            };

            if let Ok(json) = serde_json::to_string(&finish_chunk) {
                let _ = tx.send(Ok(Event::default().data(json))).await;
            }
        }

        // Send [DONE]
        let _ = tx.send(Ok(Event::default().data("[DONE]"))).await;
    });

    Ok(Sse::new(ReceiverStream::new(rx)).into_response())
}

///
/// Process a single stream chunk and extract SSE events.
///
/// # Arguments
///  * `params` - grouped parameters for stream chunk processing
///
/// # Returns
///  * `Ok(())` on successful processing
///  * `ProxyError` on processing failure
async fn process_stream_chunk(params: StreamChunkParams<'_>) -> Result<()> {
    let chunk_str = String::from_utf8_lossy(params.chunk);
    let new_content = format!("{}{}", params.buffer, chunk_str);

    let (lines_to_process, new_buffer) = split_sse_lines(&new_content);
    *params.buffer = new_buffer;

    for line in lines_to_process {
        if let Some(data) = extract_sse_data(line) {
            if data == "[DONE]" {
                send_sse_event(params.tx, "[DONE]").await;
                continue;
            }

            process_sse_event(
                data,
                params.state,
                params.model,
                params.current_tool_call,
                params.has_tool_calls,
                params.stop_reason_from_delta,
                params.tx,
            )
            .await;
        }
    }

    Ok(())
}

///
/// Split content into complete SSE lines and remaining buffer.
///
/// # Arguments
///  * `content` - content to split
///
/// # Returns
///  * Tuple of (complete lines, remaining buffer)
fn split_sse_lines(content: &str) -> (Vec<&str>, String) {
    let mut lines_to_process = Vec::new();
    let mut new_buffer = String::new();

    let ends_with_newline = content.ends_with('\n');
    let all_lines: Vec<&str> = content.lines().collect();
    let line_count = all_lines.len();

    for (i, line) in all_lines.into_iter().enumerate() {
        let is_last = i == line_count - 1;
        if is_last && !ends_with_newline {
            new_buffer = line.to_string();
        } else {
            lines_to_process.push(line);
        }
    }

    (lines_to_process, new_buffer)
}

///
/// Extract data from SSE line if it's a data event.
///
/// # Arguments
///  * `line` - SSE line to process
///
/// # Returns
///  * Some(data) if line contains data event, None otherwise
fn extract_sse_data(line: &str) -> Option<&str> {
    line.strip_prefix("data: ")
}

///
/// Process a single SSE event and convert to OpenAI format.
///
/// # Arguments
///  * `data` - SSE event data
///  * `state` - application state
///  * `model` - model identifier
///  * `current_tool_call` - current tool call state
///  * `has_tool_calls` - tool calls presence flag
///  * `stop_reason_from_delta` - stop reason from delta
///  * `tx` - event sender channel
async fn process_sse_event(
    data: &str,
    state: &Arc<AppState>,
    model: &str,
    current_tool_call: &mut Option<crate::converter::anthropic_to_openai::StreamingToolCall>,
    has_tool_calls: &mut bool,
    stop_reason_from_delta: &mut Option<String>,
    tx: &mpsc::Sender<Result<Event>>,
) {
    match serde_json::from_str::<crate::converter::anthropic_to_openai::AnthropicStreamEvent>(data)
    {
        Ok(event) => {
            if let Some(chunk) = state.anthropic_to_openai.convert_stream_event(
                &event,
                model,
                current_tool_call,
                has_tool_calls,
                stop_reason_from_delta,
            ) {
                match serde_json::to_string(&chunk) {
                    Ok(json) => {
                        send_sse_event(tx, &json).await;
                    }
                    Err(e) => {
                        tracing::error!("Failed to serialize chunk: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to parse stream event: {} - data: {}", e, data);
        }
    }
}

///
/// Send an SSE event through the channel.
///
/// # Arguments
///  * `tx` - event sender channel
///  * `data` - event data to send
async fn send_sse_event(tx: &mpsc::Sender<Result<Event>>, data: &str) {
    let _ = tx.send(Ok(Event::default().data(data))).await;
}

///
/// Send the final [DONE] event to complete the stream.
///
/// # Arguments
///  * `tx` - event sender channel
async fn send_stream_done(tx: &mpsc::Sender<Result<Event>>) {
    let _ = tx.send(Ok(Event::default().data("[DONE]"))).await;
}

///
/// Create an error response for client errors.
///
/// # Arguments
///  * `error` - error to convert to HTTP response
///
/// # Returns
///  * HTTP error response with JSON error details
fn create_error_response(error: &ProxyError) -> axum::response::Response {
    let (status_code, error_type) = match error {
        ProxyError::Config(_) | ProxyError::Conversion(_) => {
            (axum::http::StatusCode::BAD_REQUEST, "invalid_request_error")
        }
        ProxyError::Auth(_) => (axum::http::StatusCode::UNAUTHORIZED, "authentication_error"),
        ProxyError::Http(msg) if msg.contains("Rate limit") || msg.contains("Quota exceeded") => {
            (axum::http::StatusCode::TOO_MANY_REQUESTS, "rate_limit_error")
        }
        ProxyError::Http(msg) if msg.contains("temporarily unavailable") => {
            (axum::http::StatusCode::SERVICE_UNAVAILABLE, "service_unavailable")
        }
        _ => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
    };

    let error_response = json!({
      "error": {
        "message": error.to_string(),
        "type": error_type,
        "code": status_code.as_u16()
      }
    });

    (status_code, Json(error_response)).into_response()
}

///
/// Handle models listing endpoint for OpenAI compatibility.
///
/// Returns a list of available models in OpenAI format.
///
/// # Arguments
///  * `state` - shared application state
///
/// # Returns
///  * JSON response with model list
pub async fn models(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({
      "object": "list",
      "data": [{
        "id": state.config.llm_model,
        "object": "model",
        "created": chrono::Utc::now().timestamp_millis(),
        "owned_by": "anthropic"
      }]
    }))
}

///
/// Handle health check endpoint.
///
/// Returns a simple health status for service monitoring with basic metrics.
///
/// # Arguments
///  * `state` - shared application state with metrics
///
/// # Returns
///  * JSON response with health status and metrics
pub async fn health(State(state): State<Arc<AppState>>) -> Json<Value> {
    let total_requests = state.metrics.total_requests.load(Ordering::Relaxed);
    let quota_errors = state.metrics.quota_errors.load(Ordering::Relaxed);
    let retry_attempts = state.metrics.retry_attempts.load(Ordering::Relaxed);
    let successful_requests = state.metrics.successful_requests.load(Ordering::Relaxed);
    let failed_requests = state.metrics.failed_requests.load(Ordering::Relaxed);

    Json(json!({
      "status": "ok",
      "metrics": {
        "total_requests": total_requests,
        "successful_requests": successful_requests,
        "failed_requests": failed_requests,
        "quota_errors": quota_errors,
        "retry_attempts": retry_attempts,
        "success_rate": if total_requests > 0 {
          (successful_requests as f64 / total_requests as f64 * 100.0).round()
        } else {
          100.0
        }
      }
    }))
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    #[test]
    fn test_detect_buffered_streaming_client_rustrover() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "user-agent",
            HeaderValue::from_static("RustRover/2024.1 Build #RR-241.14494.158"),
        );

        assert!(detect_buffered_streaming_client(&headers));
    }

    #[test]
    fn test_detect_buffered_streaming_client_intellij() {
        let mut headers = HeaderMap::new();
        headers.insert("user-agent", HeaderValue::from_static("IntelliJ IDEA/2024.1"));

        assert!(detect_buffered_streaming_client(&headers));
    }

    #[test]
    fn test_detect_problematic_client_goose() {
        let mut headers = HeaderMap::new();
        headers.insert("user-agent", HeaderValue::from_static("goose/1.0.0"));

        assert!(detect_problematic_client(&headers));
    }

    #[test]
    fn test_detect_problematic_client_curl() {
        let mut headers = HeaderMap::new();
        headers.insert("user-agent", HeaderValue::from_static("curl/7.68.0"));

        assert!(detect_problematic_client(&headers));
    }

    #[test]
    fn test_detect_problematic_client_no_sse_accept() {
        let mut headers = HeaderMap::new();
        headers.insert("user-agent", HeaderValue::from_static("CustomClient/1.0"));
        headers.insert("accept", HeaderValue::from_static("application/json"));

        assert!(detect_problematic_client(&headers));
    }

    #[test]
    fn test_detect_buffered_streaming_client_chrome() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "user-agent",
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
         Chrome/91.0.4472.124 Safari/537.36",
            ),
        );

        assert!(detect_buffered_streaming_client(&headers));
    }

    #[test]
    fn test_detect_buffered_streaming_client_vscode() {
        let mut headers = HeaderMap::new();
        headers.insert("user-agent", HeaderValue::from_static("Visual Studio Code 1.85.0"));

        assert!(detect_buffered_streaming_client(&headers));
    }

    #[test]
    fn test_normal_client_not_problematic() {
        let mut headers = HeaderMap::new();
        headers.insert("user-agent", HeaderValue::from_static("OpenAI-Client/1.0"));
        headers.insert("accept", HeaderValue::from_static("text/event-stream, application/json"));

        assert!(!detect_problematic_client(&headers));
        assert!(!detect_buffered_streaming_client(&headers));
    }

    #[test]
    fn test_determine_streaming_behavior_auto_mode() {
        use crate::config::{Config, LogLevel, ServiceAccountKey, StreamingMode};

        let config = Config {
            llm_url: "test".to_string(),
            llm_chat_endpoint: "test".to_string(),
            llm_model: "test".to_string(),
            service_account_key: ServiceAccountKey {
                project_id: "test".to_string(),
                private_key_id: "test".to_string(),
                private_key: "test".to_string(),
                client_email: "test".to_string(),
                client_id: "test".to_string(),
                auth_uri: "test".to_string(),
                token_uri: "test".to_string(),
                auth_provider_x509_cert_url: "test".to_string(),
                client_x509_cert_url: "test".to_string(),
            },
            port: 3000,
            log_level: LogLevel::Info,
            enable_retries: true,
            max_retry_attempts: 3,
            streaming_mode: StreamingMode::Auto,
        };

        // Test with CLI client that can't handle SSE (goose)
        let mut headers = HeaderMap::new();
        headers.insert("user-agent", HeaderValue::from_static("goose/1.0.0"));
        let (force_non_streaming, use_buffered) = determine_streaming_behavior(&config, &headers);
        assert!(force_non_streaming);
        assert!(!use_buffered);

        // Test with browser (should use buffered streaming)
        let mut headers = HeaderMap::new();
        headers.insert("user-agent", HeaderValue::from_static("Mozilla/5.0 Chrome/91.0"));
        headers.insert("accept", HeaderValue::from_static("text/event-stream"));
        let (force_non_streaming, use_buffered) = determine_streaming_behavior(&config, &headers);
        assert!(!force_non_streaming);
        assert!(use_buffered);

        // Test with truly problematic client (should force non-streaming)
        let mut headers = HeaderMap::new();
        headers.insert("user-agent", HeaderValue::from_static("curl/7.68.0"));
        let (force_non_streaming, use_buffered) = determine_streaming_behavior(&config, &headers);
        assert!(force_non_streaming);
        assert!(!use_buffered);
    }

    #[test]
    fn test_determine_streaming_behavior_non_streaming_mode() {
        use crate::config::{Config, LogLevel, ServiceAccountKey, StreamingMode};

        let config = Config {
            llm_url: "test".to_string(),
            llm_chat_endpoint: "test".to_string(),
            llm_model: "test".to_string(),
            service_account_key: ServiceAccountKey {
                project_id: "test".to_string(),
                private_key_id: "test".to_string(),
                private_key: "test".to_string(),
                client_email: "test".to_string(),
                client_id: "test".to_string(),
                auth_uri: "test".to_string(),
                token_uri: "test".to_string(),
                auth_provider_x509_cert_url: "test".to_string(),
                client_x509_cert_url: "test".to_string(),
            },
            port: 3000,
            log_level: LogLevel::Info,
            enable_retries: true,
            max_retry_attempts: 3,
            streaming_mode: StreamingMode::NonStreaming,
        };

        let headers = HeaderMap::new();
        let (force_non_streaming, use_buffered) = determine_streaming_behavior(&config, &headers);
        assert!(force_non_streaming);
        assert!(!use_buffered);
    }
}
