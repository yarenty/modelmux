//! Conversion of an OpenAI chat request into the Anthropic request sent to Vertex AI.

use modelmux::config::LogLevel;
use modelmux::converter::anthropic_to_openai::AnthropicResponse;
use modelmux::converter::OpenAiToAnthropicConverter;
use modelmux::converter::openai_to_anthropic::OpenAiRequest;
use serde_json::{Value, json};

fn convert(request: Value) -> Value {
    let request: OpenAiRequest = serde_json::from_value(request).expect("valid OpenAI request");
    let converted =
        OpenAiToAnthropicConverter::new(LogLevel::Info).convert(request).expect("conversion");
    serde_json::to_value(converted).expect("serializable")
}

#[test]
fn no_temperature_is_sent_when_the_caller_gave_none() {
    let body = convert(json!({"messages": [{"role": "user", "content": "hi"}]}));
    assert!(body.get("temperature").is_none(), "{body}");
}

#[test]
fn the_callers_temperature_is_passed_on() {
    let body =
        convert(json!({"temperature": 0.2, "messages": [{"role": "user", "content": "hi"}]}));
    assert_eq!(body["temperature"], json!(0.2));
}

#[test]
fn a_response_with_a_block_other_than_text_or_tool_use_is_read() {
    let response: AnthropicResponse = serde_json::from_value(json!({
        "content": [
            {"type": "thinking", "thinking": "", "signature": "abc"},
            {"type": "text", "text": "ok"}
        ],
        "stop_reason": "end_turn"
    }))
    .expect("readable");
    assert_eq!(response.content.len(), 2);
}

#[test]
fn sampling_stop_and_user_fields_are_passed_on() {
    let body = convert(json!({
        "top_p": 0.5,
        "stop": "END",
        "user": "u-1",
        "max_completion_tokens": 1234,
        "messages": [{"role": "user", "content": "hi"}]
    }));
    assert_eq!(body["top_p"], json!(0.5));
    assert_eq!(body["stop_sequences"], json!(["END"]));
    assert_eq!(body["metadata"], json!({"user_id": "u-1"}));
    assert_eq!(body["max_tokens"], json!(1234));

    let body =
        convert(json!({"stop": ["a", "b"], "messages": [{"role": "user", "content": "hi"}]}));
    assert_eq!(body["stop_sequences"], json!(["a", "b"]));
}

#[test]
fn unknown_fields_are_passed_on_and_openai_only_fields_are_dropped() {
    let body = convert(json!({
        "thinking": {"type": "adaptive"},
        "top_k": 5,
        "seed": 7,
        "response_format": {"type": "json_object"},
        "messages": [{"role": "user", "content": "hi"}]
    }));
    assert_eq!(body["thinking"], json!({"type": "adaptive"}));
    assert_eq!(body["top_k"], json!(5));
    assert!(body.get("seed").is_none(), "{body}");
    assert!(body.get("response_format").is_none(), "{body}");
}

#[test]
fn max_tokens_falls_back_to_the_configured_value() {
    let request: OpenAiRequest =
        serde_json::from_value(json!({"messages": [{"role": "user", "content": "hi"}]})).unwrap();
    let converted = OpenAiToAnthropicConverter::new(LogLevel::Info)
        .with_default_max_tokens(32000)
        .convert(request)
        .unwrap();
    assert_eq!(converted.max_tokens, 32000);

    let body = convert(json!({"messages": [{"role": "user", "content": "hi"}]}));
    assert_eq!(body["max_tokens"], json!(8000));
}

#[test]
fn a_messages_body_is_passed_through_with_only_the_vertex_changes() {
    let request = json!({
        "model": "claude-opus",
        "max_tokens": 100,
        "stream": true,
        "thinking": {"type": "adaptive"},
        "messages": [{"role": "user", "content": "hi"}]
    });
    let (body, model, stream) = modelmux::server::prepare_messages_body(request).unwrap();
    assert_eq!(model.as_deref(), Some("claude-opus"));
    assert!(stream);
    assert_eq!(
        body,
        json!({
            "anthropic_version": "vertex-2023-10-16",
            "max_tokens": 100,
            "stream": true,
            "thinking": {"type": "adaptive"},
            "messages": [{"role": "user", "content": "hi"}]
        })
    );

    let (body, _, _) = modelmux::server::prepare_messages_body(
        json!({"anthropic_version": "custom", "messages": []}),
    )
    .unwrap();
    assert_eq!(body["anthropic_version"], json!("custom"));
    assert!(modelmux::server::prepare_messages_body(json!([1])).is_err());
}
