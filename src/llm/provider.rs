//! Provider trait and factory for LLM providers

use async_trait::async_trait;
use std::pin::Pin;
use futures::Stream;
use crate::llm::{
    types::{ChatRequest, ProviderResponse, ProviderEvent, ProviderConfig},
    errors::{LlmError, LlmResult},
    openai::OpenAIProvider,
    anthropic::AnthropicProvider,
    ollama::OllamaProvider,
    azure::AzureProvider,
    gemini::{GeminiProvider, GeminiConfig},
};

/// Trait for LLM providers
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send a chat completion request and get a response
    async fn chat_completion(&self, request: ChatRequest) -> LlmResult<ProviderResponse>;
    
    /// Send a chat completion request and get a stream of events
    async fn chat_completion_stream(
        &self,
        request: ChatRequest,
    ) -> LlmResult<Pin<Box<dyn Stream<Item = LlmResult<ProviderEvent>> + Send>>>;
    
    /// Get the provider name
    fn name(&self) -> &str;
    
    /// Get the model name
    fn model(&self) -> &str;
    
    /// Validate the configuration
    fn validate_config(&self) -> LlmResult<()>;
}

/// Factory for creating LLM providers
pub struct ProviderFactory;

impl ProviderFactory {
    /// Create a new provider from configuration
    pub fn create_provider(config: ProviderConfig) -> LlmResult<Box<dyn LlmProvider>> {
        match config.provider_type.as_str() {
            "openai" => {
                let provider = OpenAIProvider::new(config)?;
                Ok(Box::new(provider))
            }
            "anthropic" => {
                let provider = AnthropicProvider::new(config)?;
                Ok(Box::new(provider))
            }
            "ollama" => {
                let provider = OllamaProvider::new(config)?;
                Ok(Box::new(provider))
            }
            "azure" => {
                let provider = AzureProvider::from_config(config)?;
                Ok(Box::new(provider))
            }
            "gemini" => {
                let api_key = config.api_key.ok_or_else(|| 
                    LlmError::ConfigError("Gemini API key is required".to_string()))?;
                let gemini_config = GeminiConfig {
                    api_key,
                    model: config.model,
                };
                let provider = GeminiProvider::new(gemini_config)
                    .map_err(|e| LlmError::ConfigError(e.to_string()))?;
                Ok(Box::new(provider))
            }
            _ => Err(LlmError::ConfigError(format!(
                "Unsupported provider type: {}",
                config.provider_type
            ))),
        }
    }
    
    /// Get available provider types
    pub fn available_providers() -> Vec<&'static str> {
        vec!["openai", "anthropic", "ollama", "azure", "gemini"]
    }
}

/// Provider client options for flexible configuration
#[derive(Debug, Clone)]
pub struct ProviderClientOptions {
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub timeout_seconds: u64,
    pub user_agent: String,
}

impl Default for ProviderClientOptions {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay_ms: 1000,
            timeout_seconds: 300,
            user_agent: "ClaudeContextTerminal/1.0".to_string(),
        }
    }
}

/// Utility functions for provider implementations
pub mod utils {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;
    use rand::Rng;
    
    /// Exponential backoff with jitter
    pub async fn exponential_backoff_with_jitter(attempt: u32, base_delay_ms: u64) {
        use rand::Rng;
        let jitter: f64 = {
            let mut rng = rand::thread_rng();
            rng.gen_range(0.0..=1.0)
        };
        let delay_ms = (base_delay_ms as f64 * 2.0_f64.powi(attempt as i32) * (1.0 + jitter)) as u64;
        let delay = Duration::from_millis(delay_ms.min(30000)); // Cap at 30 seconds
        sleep(delay).await;
    }
    
    /// Check if an error is retryable
    pub fn is_retryable_error(error: &LlmError) -> bool {
        match error {
            LlmError::RateLimitError(_) => true,
            LlmError::HttpError(e) => {
                e.status().map_or(false, |status| {
                    status.is_server_error() || status == 429 || status == 408
                })
            }
            LlmError::TimeoutError(_) => true,
            _ => false,
        }
    }
    
    /// Sanitize content for safe display
    pub fn sanitize_content(content: &str) -> String {
        content
            .chars()
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
            .collect()
    }
    
    /// Extract error message from HTTP response
    pub async fn extract_error_message(response: reqwest::Response) -> String {
        let status = response.status();
        match response.text().await {
            Ok(text) => {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(error) = json.get("error") {
                        if let Some(message) = error.get("message") {
                            return format!("{}: {}", status, message.as_str().unwrap_or("Unknown error"));
                        }
                    }
                }
                format!("{}: {}", status, text)
            }
            Err(_) => format!("{}: Failed to read error response", status),
        }
    }
}