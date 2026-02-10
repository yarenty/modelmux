//!
//! OpenAI to Anthropic format converter for API request translation.
//!
//! Converts OpenAI-compatible chat completion requests to Anthropic/Vertex AI format.
//! Handles message conversion, tool calling, and streaming configuration while
//! maintaining semantic equivalence between the two API formats.
//!
//! Authors:
//!   Jaro <yarenty@gmail.com>
//!
//! Copyright (c) 2026 SkyCorp

/* --- uses ------------------------------------------------------------------------------------ */

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::LogLevel;
use crate::error::{ProxyError, Result};

/* --- helper functions ----------------------------------------------------------------------- */

///
/// Custom serialization helper for tools field.
///
/// Skips serialization when tools is None or empty to avoid sending invalid data to Vertex AI.
///
/// # Arguments
///  * `tools` - optional tools vector
///
/// # Returns
///  * true if field should be skipped (None or empty), false otherwise
fn skip_empty_tools(tools: &Option<Vec<AnthropicTool>>) -> bool {
    match tools {
        None => true,
        Some(vec) => vec.is_empty(),
    }
}

/* --- types ----------------------------------------------------------------------------------- */

///
/// OpenAI chat completion request structure.
///
/// Represents an incoming request in OpenAI's chat completions API format.
/// Contains messages, model configuration, and optional tool definitions.
#[derive(Debug, Deserialize)]
pub struct OpenAiRequest {
    /** the model identifier to use for completion */
    pub model: Option<String>,
    /** conversation messages array */
    pub messages: Vec<OpenAiMessage>,
    /** maximum number of tokens to generate */
    pub max_tokens: Option<u32>,
    /** sampling temperature for response generation */
    pub temperature: Option<f64>,
    /** whether to stream the response */
    pub stream: Option<bool>,
    /** available tools for function calling */
    pub tools: Option<Vec<OpenAiTool>>,
    /** tool choice configuration */
    pub tool_choice: Option<OpenAiToolChoice>,
}

///
/// OpenAI message structure within a chat completion request.
///
/// Represents a single message in the conversation with role-based content
/// and optional tool call information.
#[derive(Debug, Deserialize)]
pub struct OpenAiMessage {
    /** message role: system, user, assistant, or tool */
    pub role: String,
    /** message content, can be string or structured blocks */
    pub content: Option<OpenAiContent>,
    /** tool calls made by the assistant */
    pub tool_calls: Option<Vec<OpenAiToolCall>>,
    /** tool call ID for tool response messages */
    #[serde(rename = "tool_call_id")]
    pub tool_call_id: Option<String>,
}

///
/// OpenAI content union type for flexible message content.
///
/// Supports both simple string content and structured content blocks
/// for multimodal messages including text and images.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum OpenAiContent {
    /** simple string content */
    String(String),
    /** structured content blocks array */
    Array(Vec<OpenAiContentBlock>),
}

///
/// OpenAI structured content block for multimodal messages.
///
/// Represents individual content elements within a message, supporting
/// text and image content types with appropriate metadata.
#[derive(Debug, Deserialize)]
pub struct OpenAiContentBlock {
    /** content block type: text or image_url */
    #[serde(rename = "type")]
    pub block_type: String,
    /** text content for text blocks */
    pub text: Option<String>,
    /** image URL reference for image blocks */
    #[serde(rename = "image_url")]
    pub image_url: Option<ImageUrl>,
}

///
/// Image URL reference structure for image content blocks.
///
/// Contains the URL pointing to the image resource.
#[derive(Debug, Deserialize)]
pub struct ImageUrl {
    /** the image URL */
    pub url: String,
}

///
/// OpenAI tool call structure for function invocations.
///
/// Represents a function call made by the assistant during response generation.
#[derive(Debug, Deserialize)]
pub struct OpenAiToolCall {
    /** unique identifier for this tool call */
    pub id: String,
    /** tool call type, typically "function" */
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub call_type: String,
    /** function call details */
    pub function: OpenAiFunction,
}

///
/// OpenAI function call details within a tool call.
///
/// Contains the function name and arguments for execution.
#[derive(Debug, Deserialize)]
pub struct OpenAiFunction {
    /** function name to call */
    pub name: String,
    /** function arguments as JSON value */
    pub arguments: serde_json::Value,
}

///
/// OpenAI tool definition for available functions.
///
/// Describes a function that can be called by the model during response generation.
#[derive(Debug, Deserialize)]
pub struct OpenAiTool {
    /** tool type, typically "function" */
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub tool_type: String,
    /** function definition and schema */
    pub function: OpenAiToolFunction,
}

///
/// OpenAI function definition within a tool.
///
/// Contains function metadata and parameter schema for validation.
#[derive(Debug, Deserialize)]
pub struct OpenAiToolFunction {
    /** function name */
    pub name: String,
    /** function description */
    pub description: String,
    /** JSON schema for function parameters */
    pub parameters: serde_json::Value,
}

///
/// OpenAI tool choice configuration.
///
/// Controls how the model should choose which tools to use during generation.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum OpenAiToolChoice {
    /** string choice: "auto", "none", etc. */
    String(String),
    /** object choice with specific function */
    Object(OpenAiToolChoiceObject),
}

///
/// OpenAI tool choice object for specific function selection.
///
/// Allows forcing the model to use a specific function.
#[derive(Debug, Deserialize)]
pub struct OpenAiToolChoiceObject {
    /** choice type */
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub choice_type: String,
    /** specific function to choose */
    pub function: Option<OpenAiToolChoiceFunction>,
}

///
/// OpenAI specific function choice within tool choice object.
///
/// Identifies the exact function to use.
#[derive(Debug, Deserialize)]
pub struct OpenAiToolChoiceFunction {
    /** function name to force */
    pub name: String,
}

///
/// Anthropic chat completion request structure.
///
/// Target format for requests to Anthropic's Claude API via Vertex AI.
/// Contains converted messages and configuration from OpenAI format.
#[derive(Debug, Serialize)]
pub struct AnthropicRequest {
    /** Anthropic API version identifier */
    #[serde(rename = "anthropic_version")]
    pub anthropic_version: String,
    /** conversation messages in Anthropic format */
    pub messages: Vec<AnthropicMessage>,
    /** maximum tokens to generate */
    #[serde(rename = "max_tokens")]
    pub max_tokens: u32,
    /** sampling temperature */
    pub temperature: f64,
    /** whether to stream the response */
    pub stream: bool,
    /** available tools in Anthropic format */
    #[serde(skip_serializing_if = "skip_empty_tools")]
    pub tools: Option<Vec<AnthropicTool>>,
    /** tool choice configuration in Anthropic format */
    #[serde(rename = "tool_choice", skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<AnthropicToolChoice>,
}

///
/// Anthropic message structure for chat conversations.
///
/// Contains role and content blocks in Anthropic's preferred format.
#[derive(Debug, Serialize)]
pub struct AnthropicMessage {
    /** message role: user or assistant */
    pub role: String,
    /** message content as structured blocks */
    pub content: Vec<AnthropicContentBlock>,
}

///
/// Anthropic content block for message content.
///
/// Supports text, tool usage, tool results, and image content types
/// with proper tagging for serialization.
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum AnthropicContentBlock {
    /** text content block */
    #[serde(rename = "text")]
    Text {
        /** the text content */
        text: String,
    },
    /** tool usage block for function calls */
    #[serde(rename = "tool_use")]
    ToolUse {
        /** tool call identifier */
        id: String,
        /** function name */
        name: String,
        /** function input arguments */
        input: serde_json::Value,
    },
    /** tool result block for function responses */
    #[serde(rename = "tool_result")]
    ToolResult {
        /** corresponding tool use identifier */
        #[serde(rename = "tool_use_id")]
        tool_use_id: String,
        /** tool execution result */
        content: AnthropicToolResultContent,
    },
    /** image content block */
    #[serde(rename = "image")]
    Image {
        /** image source information */
        source: ImageSource,
    },
}

///
/// Anthropic tool result content union type.
///
/// Supports both simple string results and structured array results
/// for complex tool responses.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum AnthropicToolResultContent {
    /** simple string result */
    String(String),
    /** structured array result */
    Array(Vec<serde_json::Value>),
}

///
/// Image source information for Anthropic image blocks.
///
/// Contains metadata about image resources.
#[derive(Debug, Serialize)]
pub struct ImageSource {
    /** source type identifier */
    #[serde(rename = "type")]
    pub source_type: String,
    /** image URL */
    pub url: String,
}

///
/// Anthropic tool definition for function calling.
///
/// Describes available functions in Anthropic's format.
#[derive(Debug, Serialize)]
pub struct AnthropicTool {
    /** function name */
    pub name: String,
    /** function description */
    pub description: String,
    /** function input schema */
    #[serde(rename = "input_schema")]
    pub input_schema: serde_json::Value,
}

///
/// Anthropic tool choice configuration.
///
/// Controls tool selection behavior in Anthropic format.
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum AnthropicToolChoice {
    /** automatic tool selection */
    #[serde(rename = "auto")]
    Auto,
    /** force specific tool usage */
    #[serde(rename = "tool")]
    Tool {
        /** tool name to force */
        name: String,
    },
}

///
/// Converter from OpenAI format to Anthropic format.
///
/// Follows Single Responsibility Principle - handles only format conversion
/// from OpenAI chat completions to Anthropic message format.
pub struct OpenAiToAnthropicConverter {
    /** logging level for debug output */
    log_level: LogLevel,
}

/* --- constants ------------------------------------------------------------------------------ */

/** Anthropic API version to use for requests */
const ANTHROPIC_VERSION: &str = "vertex-2023-10-16";

/** Default maximum tokens if not specified */
const DEFAULT_MAX_TOKENS: u32 = 8000;

/** Default temperature if not specified */
const DEFAULT_TEMPERATURE: f64 = 0.9;

/* --- start of code -------------------------------------------------------------------------- */

impl OpenAiToAnthropicConverter {
    ///
    /// Create a new OpenAI to Anthropic converter.
    ///
    /// # Arguments
    ///  * `log_level` - logging level for debug output
    ///
    /// # Returns
    ///  * New converter instance
    pub fn new(log_level: LogLevel) -> Self {
        Self { log_level }
    }

    ///
    /// Convert OpenAI request to Anthropic request format.
    ///
    /// Transforms the entire request structure including messages, tools, and
    /// configuration parameters. Handles system messages, tool calls, and
    /// multimodal content appropriately.
    ///
    /// # Arguments
    ///  * `request` - OpenAI format request to convert
    ///
    /// # Returns
    ///  * Converted Anthropic format request
    ///  * `ProxyError::Conversion` if conversion fails
    pub fn convert(&self, request: OpenAiRequest) -> Result<AnthropicRequest> {
        self.debug(&format!(
            "Converting {} message(s) from OpenAI to Anthropic format",
            request.messages.len()
        ));

        let mut anthropic_messages = Vec::new();
        let mut pending_tool_results = Vec::new();
        let mut last_assistant_message: Option<&'_ OpenAiMessage> = None;
        let mut system_messages = Vec::new();

        self.process_messages(
            &request.messages,
            &mut anthropic_messages,
            &mut pending_tool_results,
            &mut last_assistant_message,
            &mut system_messages,
        )?;

        self.handle_remaining_tool_results(
            &mut anthropic_messages,
            &mut pending_tool_results,
            last_assistant_message,
        )?;

        self.prepend_system_messages(&mut anthropic_messages, system_messages);

        let tools = self.convert_tools(request.tools);
        let tool_choice = self.convert_tool_choice(request.tool_choice);

        let anthropic_request = AnthropicRequest {
            anthropic_version: ANTHROPIC_VERSION.to_string(),
            messages: anthropic_messages,
            max_tokens: request.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
            temperature: request.temperature.unwrap_or(DEFAULT_TEMPERATURE),
            stream: request.stream.unwrap_or(false),
            tools,
            tool_choice,
        };

        self.debug(&format!(
            "Converted Anthropic request with {} messages",
            anthropic_request.messages.len()
        ));

        Ok(anthropic_request)
    }

    ///
    /// Process all messages in the OpenAI request.
    ///
    /// Iterates through messages and converts them based on role type,
    /// managing tool calls and results properly.
    ///
    /// # Arguments
    ///  * `messages` - OpenAI messages to process
    ///  * `anthropic_messages` - output Anthropic messages
    ///  * `pending_tool_results` - accumulated tool results
    ///  * `last_assistant_message` - reference to last assistant message
    ///  * `system_messages` - accumulated system messages
    ///
    /// # Returns
    ///  * `Ok(())` on successful processing
    ///  * `ProxyError::Conversion` if message conversion fails
    fn process_messages<'a>(
        &self,
        messages: &'a [OpenAiMessage],
        anthropic_messages: &mut Vec<AnthropicMessage>,
        pending_tool_results: &mut Vec<(String, AnthropicToolResultContent)>,
        last_assistant_message: &mut Option<&'a OpenAiMessage>,
        system_messages: &mut Vec<String>,
    ) -> Result<()> {
        for msg in messages {
            self.debug(&format!("Processing message with role: {}", msg.role));

            match msg.role.as_str() {
                "system" => {
                    self.process_system_message(msg, system_messages);
                }
                "assistant" => {
                    self.process_assistant_message(
                        msg,
                        anthropic_messages,
                        pending_tool_results,
                        last_assistant_message,
                    )?;
                }
                "tool" => {
                    self.process_tool_message(msg, pending_tool_results);
                }
                "user" => {
                    self.process_user_message(
                        msg,
                        anthropic_messages,
                        pending_tool_results,
                        *last_assistant_message,
                    )?;
                }
                _ => {
                    return Err(ProxyError::Conversion(format!(
                        "Unknown message role: {}",
                        msg.role
                    )));
                }
            }
        }
        Ok(())
    }

    ///
    /// Process a system message by extracting its content.
    ///
    /// # Arguments
    ///  * `msg` - system message to process
    ///  * `system_messages` - collection to add system content to
    fn process_system_message(&self, msg: &OpenAiMessage, system_messages: &mut Vec<String>) {
        if let Some(OpenAiContent::String(content)) = &msg.content {
            system_messages.push(content.clone());
        }
    }

    ///
    /// Process an assistant message with optional tool calls.
    ///
    /// # Arguments
    ///  * `msg` - assistant message to process
    ///  * `anthropic_messages` - output Anthropic messages
    ///  * `pending_tool_results` - accumulated tool results
    ///  * `last_assistant_message` - reference to last assistant message
    ///
    /// # Returns
    ///  * `Ok(())` on successful processing
    ///  * `ProxyError::Conversion` if conversion fails
    fn process_assistant_message<'a>(
        &self,
        msg: &'a OpenAiMessage,
        anthropic_messages: &mut Vec<AnthropicMessage>,
        pending_tool_results: &mut Vec<(String, AnthropicToolResultContent)>,
        last_assistant_message: &mut Option<&'a OpenAiMessage>,
    ) -> Result<()> {
        if last_assistant_message.is_some() && !pending_tool_results.is_empty() {
            self.attach_tool_results(anthropic_messages, pending_tool_results)?;
        }

        let anthropic_msg = self.convert_assistant_message(msg)?;
        anthropic_messages.push(anthropic_msg);
        *last_assistant_message = Some(msg);
        Ok(())
    }

    ///
    /// Process a tool message by collecting its result.
    ///
    /// # Arguments
    ///  * `msg` - tool message to process
    ///  * `pending_tool_results` - collection to add tool result to
    fn process_tool_message(
        &self,
        msg: &OpenAiMessage,
        pending_tool_results: &mut Vec<(String, AnthropicToolResultContent)>,
    ) {
        if let Some(tool_call_id) = &msg.tool_call_id {
            let content = self.convert_tool_result_content(&msg.content);
            pending_tool_results.push((tool_call_id.clone(), content));
            self.debug(&format!("Collected tool result for tool_call_id: {}", tool_call_id));
        }
    }

    ///
    /// Process a user message and attach any pending tool results.
    ///
    /// # Arguments
    ///  * `msg` - user message to process
    ///  * `anthropic_messages` - output Anthropic messages
    ///  * `pending_tool_results` - accumulated tool results
    ///  * `last_assistant_message` - optional reference to last assistant message
    ///
    /// # Returns
    ///  * `Ok(())` on successful processing
    ///  * `ProxyError::Conversion` if conversion fails
    fn process_user_message<'a>(
        &self,
        msg: &'a OpenAiMessage,
        anthropic_messages: &mut Vec<AnthropicMessage>,
        pending_tool_results: &mut Vec<(String, AnthropicToolResultContent)>,
        last_assistant_message: Option<&'a OpenAiMessage>,
    ) -> Result<()> {
        if last_assistant_message.is_some() && !pending_tool_results.is_empty() {
            self.debug(&format!(
                "Attaching {} tool result(s) before user message",
                pending_tool_results.len()
            ));
            self.attach_tool_results(anthropic_messages, pending_tool_results)?;
        }

        let anthropic_msg = self.convert_user_message(msg)?;
        anthropic_messages.push(anthropic_msg);
        Ok(())
    }

    ///
    /// Convert tool result content from OpenAI to Anthropic format.
    ///
    /// # Arguments
    ///  * `content` - OpenAI message content to convert
    ///
    /// # Returns
    ///  * Converted tool result content
    fn convert_tool_result_content(
        &self,
        content: &Option<OpenAiContent>,
    ) -> AnthropicToolResultContent {
        match content {
            Some(OpenAiContent::String(s)) => AnthropicToolResultContent::String(s.clone()),
            Some(OpenAiContent::Array(arr)) => {
                let mut json_blocks = Vec::new();
                for block in arr {
                    match block.block_type.as_str() {
                        "text" => {
                            if let Some(text) = &block.text {
                                json_blocks.push(json!({ "type": "text", "text": text }));
                            }
                        }
                        "image_url" => {
                            if let Some(img) = &block.image_url {
                                json_blocks.push(
                                    json!({ "type": "image_url", "image_url": { "url": img.url } }),
                                );
                            }
                        }
                        _ => {}
                    }
                }
                AnthropicToolResultContent::Array(json_blocks)
            }
            None => AnthropicToolResultContent::String(String::new()),
        }
    }

    ///
    /// Handle any remaining tool results after processing all messages.
    ///
    /// # Arguments
    ///  * `anthropic_messages` - output Anthropic messages
    ///  * `pending_tool_results` - accumulated tool results
    ///  * `last_assistant_message` - optional reference to last assistant message
    ///
    /// # Returns
    ///  * `Ok(())` on successful processing
    ///  * `ProxyError::Conversion` if attachment fails
    fn handle_remaining_tool_results(
        &self,
        anthropic_messages: &mut Vec<AnthropicMessage>,
        pending_tool_results: &mut Vec<(String, AnthropicToolResultContent)>,
        last_assistant_message: Option<&OpenAiMessage>,
    ) -> Result<()> {
        if last_assistant_message.is_some() && !pending_tool_results.is_empty() {
            self.attach_tool_results(anthropic_messages, pending_tool_results)?;
        }
        Ok(())
    }

    ///
    /// Prepend system messages to the first user message.
    ///
    /// # Arguments
    ///  * `anthropic_messages` - output Anthropic messages to modify
    ///  * `system_messages` - system messages to prepend
    fn prepend_system_messages(
        &self,
        anthropic_messages: &mut [AnthropicMessage],
        system_messages: Vec<String>,
    ) {
        if !system_messages.is_empty() && !anthropic_messages.is_empty() {
            let system_text = system_messages.join("\n\n");
            if let Some(first_user_msg) = anthropic_messages.iter_mut().find(|m| m.role == "user") {
                self.prepend_system_text(first_user_msg, &system_text);
            }
        }
    }

    ///
    /// Convert OpenAI tools to Anthropic format.
    ///
    /// # Arguments
    ///  * `tools` - optional OpenAI tools to convert
    ///
    /// # Returns
    ///  * Converted Anthropic tools or None
    fn convert_tools(&self, tools: Option<Vec<OpenAiTool>>) -> Option<Vec<AnthropicTool>> {
        tools.map(|tools| {
            self.debug(&format!(
                "Converting {} tool(s) from OpenAI to Anthropic format",
                tools.len()
            ));
            tools
                .into_iter()
                .map(|tool| AnthropicTool {
                    name: tool.function.name,
                    description: tool.function.description,
                    input_schema: tool.function.parameters,
                })
                .collect()
        })
    }

    ///
    /// Convert OpenAI tool choice to Anthropic format.
    ///
    /// # Arguments
    ///  * `tool_choice` - optional OpenAI tool choice to convert
    ///
    /// # Returns
    ///  * Converted Anthropic tool choice or None
    fn convert_tool_choice(
        &self,
        tool_choice: Option<OpenAiToolChoice>,
    ) -> Option<AnthropicToolChoice> {
        tool_choice.and_then(|choice| {
            self.debug(&format!("Tool choice: {:?}", choice));
            match choice {
                OpenAiToolChoice::String(s) if s == "auto" => Some(AnthropicToolChoice::Auto),
                OpenAiToolChoice::String(s) if s == "none" => {
                    self.debug("Tool choice 'none' not supported by Anthropic, omitting");
                    None
                }
                OpenAiToolChoice::Object(obj) => {
                    if let Some(function) = obj.function {
                        self.debug(&format!("Forced tool choice: {}", function.name));
                        Some(AnthropicToolChoice::Tool { name: function.name })
                    } else {
                        None
                    }
                }
                _ => None,
            }
        })
    }

    ///
    /// Convert an OpenAI assistant message to Anthropic format.
    ///
    /// Handles both text content and tool calls within the message.
    ///
    /// # Arguments
    ///  * `msg` - OpenAI assistant message to convert
    ///
    /// # Returns
    ///  * Converted Anthropic message
    ///  * `ProxyError::Conversion` if conversion fails
    fn convert_assistant_message(&self, msg: &OpenAiMessage) -> Result<AnthropicMessage> {
        let mut content = Vec::new();

        self.add_text_content(&mut content, &msg.content);
        self.add_tool_calls(&mut content, &msg.tool_calls)?;

        if content.is_empty() {
            content.push(AnthropicContentBlock::Text { text: String::new() });
        }

        Ok(AnthropicMessage { role: "assistant".to_string(), content })
    }

    ///
    /// Add text content from OpenAI message to Anthropic content blocks.
    ///
    /// # Arguments
    ///  * `content` - content blocks to add to
    ///  * `openai_content` - OpenAI content to extract text from
    fn add_text_content(
        &self,
        content: &mut Vec<AnthropicContentBlock>,
        openai_content: &Option<OpenAiContent>,
    ) {
        match openai_content {
            Some(OpenAiContent::String(text)) if !text.is_empty() => {
                content.push(AnthropicContentBlock::Text { text: text.clone() });
            }
            Some(OpenAiContent::Array(blocks)) => {
                for block in blocks {
                    if block.block_type == "text" {
                        if let Some(text) = &block.text {
                            content.push(AnthropicContentBlock::Text { text: text.clone() });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    ///
    /// Add tool calls from OpenAI message to Anthropic content blocks.
    ///
    /// # Arguments
    ///  * `content` - content blocks to add to
    ///  * `tool_calls` - OpenAI tool calls to convert
    ///
    /// # Returns
    ///  * `Ok(())` on successful addition
    ///  * `ProxyError::Conversion` if tool call conversion fails
    fn add_tool_calls(
        &self,
        content: &mut Vec<AnthropicContentBlock>,
        tool_calls: &Option<Vec<OpenAiToolCall>>,
    ) -> Result<()> {
        if let Some(tool_calls) = tool_calls {
            self.debug(&format!(
                "Converting {} tool call(s) from assistant message",
                tool_calls.len()
            ));
            for tool_call in tool_calls {
                let args = self.parse_tool_arguments(&tool_call.function.arguments);
                content.push(AnthropicContentBlock::ToolUse {
                    id: tool_call.id.clone(),
                    name: tool_call.function.name.clone(),
                    input: args,
                });
            }
        }
        Ok(())
    }

    ///
    /// Parse tool call arguments from JSON value.
    ///
    /// # Arguments
    ///  * `arguments` - JSON arguments value
    ///
    /// # Returns
    ///  * Parsed JSON value for tool input
    fn parse_tool_arguments(&self, arguments: &serde_json::Value) -> serde_json::Value {
        match arguments {
            serde_json::Value::String(s) => {
                serde_json::from_str(s).unwrap_or_else(|_| arguments.clone())
            }
            _ => arguments.clone(),
        }
    }

    ///
    /// Convert an OpenAI user message to Anthropic format.
    ///
    /// Handles text, image, and multimodal content appropriately.
    ///
    /// # Arguments
    ///  * `msg` - OpenAI user message to convert
    ///
    /// # Returns
    ///  * Converted Anthropic message
    ///  * `ProxyError::Conversion` if conversion fails
    fn convert_user_message(&self, msg: &OpenAiMessage) -> Result<AnthropicMessage> {
        let content = match &msg.content {
            Some(OpenAiContent::String(text)) => {
                vec![AnthropicContentBlock::Text { text: text.clone() }]
            }
            Some(OpenAiContent::Array(blocks)) => self.convert_content_blocks(blocks),
            None => vec![AnthropicContentBlock::Text { text: String::new() }],
        };

        Ok(AnthropicMessage { role: "user".to_string(), content })
    }

    ///
    /// Convert OpenAI content blocks to Anthropic content blocks.
    ///
    /// # Arguments
    ///  * `blocks` - OpenAI content blocks to convert
    ///
    /// # Returns
    ///  * Converted Anthropic content blocks
    fn convert_content_blocks(&self, blocks: &[OpenAiContentBlock]) -> Vec<AnthropicContentBlock> {
        blocks
            .iter()
            .filter_map(|block| match block.block_type.as_str() {
                "text" => {
                    block.text.as_ref().map(|t| AnthropicContentBlock::Text { text: t.clone() })
                }
                "image_url" => block.image_url.as_ref().map(|img| AnthropicContentBlock::Image {
                    source: ImageSource { source_type: "url".to_string(), url: img.url.clone() },
                }),
                _ => None,
            })
            .collect()
    }

    ///
    /// Attach pending tool results to the conversation.
    ///
    /// Creates a user message containing tool result blocks and adds it
    /// to the conversation after the last assistant message.
    ///
    /// # Arguments
    ///  * `anthropic_messages` - messages to add tool results to
    ///  * `pending_tool_results` - tool results to attach
    ///
    /// # Returns
    ///  * `Ok(())` on successful attachment
    ///  * `ProxyError::Conversion` if attachment fails
    fn attach_tool_results(
        &self,
        anthropic_messages: &mut Vec<AnthropicMessage>,
        pending_tool_results: &mut Vec<(String, AnthropicToolResultContent)>,
    ) -> Result<()> {
        if let Some(last_msg) = anthropic_messages.last() {
            if last_msg.role == "assistant" {
                let tool_results: Vec<AnthropicContentBlock> = pending_tool_results
                    .drain(..)
                    .map(|(tool_use_id, content)| AnthropicContentBlock::ToolResult {
                        tool_use_id,
                        content,
                    })
                    .collect();

                self.debug(&format!(
                    "Adding tool results user message with {} result(s)",
                    tool_results.len()
                ));

                anthropic_messages
                    .push(AnthropicMessage { role: "user".to_string(), content: tool_results });
            } else {
                self.debug("WARNING: Last message is not assistant, cannot attach tool results");
            }
        }
        Ok(())
    }

    ///
    /// Prepend system text to the first text block of a message.
    ///
    /// Either modifies the first existing text block or inserts a new
    /// text block at the beginning with the system content.
    ///
    /// # Arguments
    ///  * `msg` - message to prepend system text to
    ///  * `system_text` - system text to prepend
    fn prepend_system_text(&self, msg: &mut AnthropicMessage, system_text: &str) {
        if let Some(first_text_block) =
            msg.content.iter_mut().find(|c| matches!(c, AnthropicContentBlock::Text { .. }))
        {
            if let AnthropicContentBlock::Text { text } = first_text_block {
                *text = format!("{}\n\n{}", system_text, text);
            }
        } else {
            msg.content.insert(0, AnthropicContentBlock::Text { text: system_text.to_string() });
        }
    }

    ///
    /// Log debug message if trace logging is enabled.
    ///
    /// # Arguments
    ///  * `msg` - debug message to log
    pub(crate) fn debug(&self, msg: &str) {
        if self.log_level.is_trace_enabled() {
            tracing::debug!("[TRACE] {}", msg);
        }
    }
}
