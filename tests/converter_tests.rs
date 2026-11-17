//! Conversion of an OpenAI chat request into the Anthropic request sent to Vertex AI.

use modelmux::config::LogLevel;
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
