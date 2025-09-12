//! Google Gemini LLM provider implementation

use async_trait::async_trait;
use anyhow::{Result, Context};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use futures::{Stream, StreamExt};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::llm::{
    LlmProvider, LlmResult, ChatRequest, ProviderResponse, ProviderEvent,
    TokenUsage, ContentBlock, MessageRole, Message,
    errors::LlmError,
};

const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Gemini provider configuration
pub struct GeminiConfig {
    pub api_key: String,
    pub model: String,
}

/// Gemini LLM provider
pub struct GeminiProvider {
    client: Client,
    config: GeminiConfig,
}

impl GeminiProvider {
    /// Create a new Gemini provider
    pub fn new(config: GeminiConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .context("Failed to create HTTP client")?;
        
        Ok(Self { client, config })
    }
    
    /// Convert messages to Gemini format
    fn convert_messages(&self, messages: &[Message]) -> Vec<GeminiMessage> {
        messages.iter().map(|msg| {
            let role = match msg.role {
                MessageRole::System => "user", // Gemini doesn't have system role
                MessageRole::User => "user",
                MessageRole::Assistant => "model",
                MessageRole::Tool => "user",
            };
            
            let parts = msg.content.iter().map(|block| {
                match block {
                    ContentBlock::Text { text } => GeminiPart::Text { text: text.clone() },
                    ContentBlock::Image { image } => GeminiPart::InlineData {
                        inline_data: InlineData {
                            mime_type: image.media_type.clone(),
                            data: image.data.clone(),
                        }
                    },
                    _ => GeminiPart::Text { text: "".to_string() },
                }
            }).collect();
            
            GeminiMessage {
                role: role.to_string(),
                parts,
            }
        }).collect()
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    async fn chat_completion(&self, request: ChatRequest) -> LlmResult<ProviderResponse> {
        let url = format!(
            "{}/models/{}:generateContent?key={}",
            GEMINI_API_BASE,
            self.config.model,
            self.config.api_key
        );
        
        let gemini_request = GeminiRequest {
            contents: self.convert_messages(&request.messages),
            generation_config: Some(GenerationConfig {
                temperature: request.temperature,
                max_output_tokens: request.max_tokens,
                stop_sequences: request.stop,
                ..Default::default()
            }),
            safety_settings: None,
        };
        
        let response = self.client
            .post(&url)
            .json(&gemini_request)
            .send()
            .await
            .context("Failed to send request to Gemini")?;
        
        if response.status() != StatusCode::OK {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(format!("Gemini API error: {}", error_text)));
        }
        
        let gemini_response: GeminiResponse = response.json().await
            .context("Failed to parse Gemini response")?;
        
        // Extract content from response
        let content = gemini_response.candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .and_then(|p| match p {
                GeminiPart::Text { text } => Some(text.clone()),
                _ => None,
            })
            .unwrap_or_default();
        
        // Extract usage metadata
        let usage = gemini_response.usage_metadata.map(|meta| TokenUsage {
            input_tokens: meta.prompt_token_count as u32,
            output_tokens: meta.candidates_token_count as u32,
            total_tokens: meta.total_token_count as u32,
        }).unwrap_or_default();
        
        Ok(ProviderResponse {
            content,
            tool_calls: vec![],
            usage,
            finish_reason: None,
            metadata: std::collections::HashMap::new(),
        })
    }
    
    async fn chat_completion_stream(
        &self,
        request: ChatRequest,
    ) -> LlmResult<Pin<Box<dyn Stream<Item = LlmResult<ProviderEvent>> + Send>>> {
        let url = format!(
            "{}/models/{}:streamGenerateContent?key={}",
            GEMINI_API_BASE,
            self.config.model,
            self.config.api_key
        );
        
        let gemini_request = GeminiRequest {
            contents: self.convert_messages(&request.messages),
            generation_config: Some(GenerationConfig {
                temperature: request.temperature,
                max_output_tokens: request.max_tokens,
                stop_sequences: request.stop,
                ..Default::default()
            }),
            safety_settings: None,
        };
        
        let response = self.client
            .post(&url)
            .json(&gemini_request)
            .send()
            .await
            .context("Failed to send streaming request to Gemini")?;
        
        if response.status() != StatusCode::OK {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(format!("Gemini API error: {}", error_text)));
        }
        
        let (tx, rx) = mpsc::unbounded_channel();
        
        // Process stream in background
        tokio::spawn(async move {
            let mut stream = response.bytes_stream();
            
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        // Parse streaming response
                        if let Ok(text) = std::str::from_utf8(&bytes) {
                            // Gemini uses data: prefix for SSE
                            for line in text.lines() {
                                if let Some(json_str) = line.strip_prefix("data: ") {
                                    if let Ok(response) = serde_json::from_str::<GeminiResponse>(json_str) {
                                        if let Some(candidate) = response.candidates.first() {
                                            if let Some(part) = candidate.content.parts.first() {
                                                if let GeminiPart::Text { text } = part {
                                                    let _ = tx.send(Ok(ProviderEvent::ContentDelta {
                                                        delta: text.clone(),
                                                    }));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(LlmError::StreamError(e.to_string())));
                        break;
                    }
                }
            }
            
            // Send completion event
            let _ = tx.send(Ok(ProviderEvent::ContentStop));
        });
        
        Ok(Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(rx)))
    }
    
    fn name(&self) -> &str {
        "gemini"
    }
    
    fn model(&self) -> &str {
        &self.config.model
    }
    
    fn validate_config(&self) -> LlmResult<()> {
        if self.config.api_key.is_empty() {
            return Err(LlmError::ConfigError("Gemini API key is required".to_string()));
        }
        Ok(())
    }
}

// Gemini API types
#[derive(Debug, Serialize, Deserialize)]
struct GeminiRequest {
    contents: Vec<GeminiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    safety_settings: Option<Vec<SafetySetting>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiMessage {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum GeminiPart {
    Text { text: String },
    InlineData { inline_data: InlineData },
}

#[derive(Debug, Serialize, Deserialize)]
struct InlineData {
    mime_type: String,
    data: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SafetySetting {
    category: String,
    threshold: String,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: GeminiContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
    role: String,
}

#[derive(Debug, Deserialize)]
struct UsageMetadata {
    prompt_token_count: i64,
    candidates_token_count: i64,
    total_token_count: i64,
}