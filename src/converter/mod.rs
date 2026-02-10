//!
//! Format conversion modules for OpenAI and Anthropic API compatibility.
//!
//! Handles bidirectional conversion between OpenAI and Anthropic/Vertex AI formats.
//! Each converter follows Single Responsibility Principle and focuses on a specific
//! conversion direction.
//!
//! Authors:
//!   Jaro <yarenty@gmail.com>
//!
//! Copyright (c) 2026 SkyCorp

/* --- modules --------------------------------------------------------------------------------- */

pub mod anthropic_to_openai;
pub mod openai_to_anthropic;

/* --- start of code -------------------------------------------------------------------------- */

pub use anthropic_to_openai::AnthropicToOpenAiConverter;
pub use openai_to_anthropic::OpenAiToAnthropicConverter;
