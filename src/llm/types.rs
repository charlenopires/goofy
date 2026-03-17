//! Common types for LLM providers

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Role of a message in the conversation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// Content block types for messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text { text: String },
    Image { image: ImageContent },
    ToolUse { id: String, name: String, input: serde_json::Value },
    ToolResult { tool_call_id: String, content: String },
}

/// Image content for multimodal messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageContent {
    pub data: String, // base64 encoded
    pub media_type: String, // e.g., "image/jpeg"
}

/// A message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: MessageRole,
    pub content: Vec<ContentBlock>,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Message {
    pub fn new_text(role: MessageRole, text: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role,
            content: vec![ContentBlock::Text { text }],
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }
    
    pub fn new_system(text: String) -> Self {
        Self::new_text(MessageRole::System, text)
    }
    
    pub fn new_user(text: String) -> Self {
        Self::new_text(MessageRole::User, text)
    }
    
    pub fn new_assistant(text: String) -> Self {
        Self::new_text(MessageRole::Assistant, text)
    }
    
    /// Alias for `timestamp` to match component expectations
    pub fn created_at(&self) -> DateTime<Utc> {
        self.timestamp
    }

    pub fn get_text_content(&self) -> Option<String> {
        self.content.iter()
            .filter_map(|block| match block {
                ContentBlock::Text { text } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
            .into()
    }
}

/// Tool definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value, // JSON schema
}

/// Token usage information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

impl TokenUsage {
    pub fn add(&mut self, other: &TokenUsage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.total_tokens += other.total_tokens;
    }
}

/// Finish reason for a completion
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ContentFilter,
    ToolCalls,
    Error,
}

/// Response from an LLM provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub usage: TokenUsage,
    pub finish_reason: Option<FinishReason>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Tool call from the model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Events emitted during streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProviderEvent {
    ContentStart,
    ContentDelta { delta: String },
    ContentStop,
    ToolUseStart { tool_call: ToolCall },
    ToolUseDelta { delta: String },
    ToolUseStop,
    Error { error: String },
    Done { response: ProviderResponse },
}

/// Configuration for an LLM provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider_type: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub stream: bool,
    pub tools: Vec<Tool>,
    pub extra_headers: HashMap<String, String>,
    pub extra_body: HashMap<String, serde_json::Value>,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            provider_type: "openai".to_string(),
            api_key: None,
            base_url: None,
            model: "gpt-4".to_string(),
            max_tokens: Some(4096),
            temperature: Some(0.7),
            top_p: None,
            stream: true,
            tools: Vec::new(),
            extra_headers: HashMap::new(),
            extra_body: HashMap::new(),
        }
    }
}

/// Chat completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<Message>,
    pub tools: Vec<Tool>,
    pub system_message: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub stream: bool,
    pub metadata: HashMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
}

impl Default for ChatRequest {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            tools: Vec::new(),
            system_message: None,
            max_tokens: None,
            temperature: None,
            top_p: None,
            stream: false,
            metadata: HashMap::new(),
            tool_choice: None,
            stop: None,
        }
    }
}