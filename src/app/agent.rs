//! AI agent abstraction for handling conversations

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::{
    llm::{LlmProvider, ChatRequest, ProviderResponse, Message, MessageRole, tools::ToolManager},
    app::AppEvent,
};

/// An AI agent that manages conversations with an LLM provider
pub struct Agent {
    provider: Arc<dyn LlmProvider>,
    tool_manager: Arc<ToolManager>,
    event_tx: mpsc::UnboundedSender<AppEvent>,
    session_id: String,
}

impl Agent {
    /// Create a new agent
    pub fn new(
        provider: Arc<dyn LlmProvider>,
        tool_manager: Arc<ToolManager>,
        event_tx: mpsc::UnboundedSender<AppEvent>,
        session_id: String,
    ) -> Self {
        Self {
            provider,
            tool_manager,
            event_tx,
            session_id,
        }
    }
    
    /// Send a message to the agent and get a response
    pub async fn send_message(
        &self,
        messages: Vec<Message>,
        system_message: Option<String>,
    ) -> Result<ProviderResponse> {
        debug!("Agent sending message to provider: {}", self.provider.name());
        
        let request = ChatRequest {
            messages,
            tools: self.tool_manager.get_tool_definitions(),
            system_message,
            max_tokens: None,
            temperature: None,
            top_p: None,
            stream: false,
            metadata: std::collections::HashMap::new(),
            tool_choice: None,
            stop: None,
        };
        
        match self.provider.chat_completion(request).await {
            Ok(response) => {
                info!(
                    "Agent received response from provider: {} tokens",
                    response.usage.total_tokens
                );
                
                // Send event
                let _ = self.event_tx.send(AppEvent::MessageReceived {
                    session_id: self.session_id.clone(),
                    message_id: uuid::Uuid::new_v4().to_string(),
                });
                
                Ok(response)
            }
            Err(e) => {
                error!("Agent error: {}", e);
                
                // Send error event
                let _ = self.event_tx.send(AppEvent::Error {
                    error: e.to_string(),
                });
                
                Err(e.into())
            }
        }
    }
    
    /// Send a message and stream the response
    pub async fn send_message_stream(
        &self,
        messages: Vec<Message>,
        system_message: Option<String>,
    ) -> Result<mpsc::UnboundedReceiver<String>> {
        debug!("Agent sending streaming message to provider: {}", self.provider.name());
        
        let request = ChatRequest {
            messages,
            tools: self.tool_manager.get_tool_definitions(),
            system_message,
            max_tokens: None,
            temperature: None,
            top_p: None,
            stream: true,
            metadata: std::collections::HashMap::new(),
            tool_choice: None,
            stop: None,
        };
        
        let (tx, rx) = mpsc::unbounded_channel();
        let provider = self.provider.clone();
        let event_tx = self.event_tx.clone();
        let session_id = self.session_id.clone();
        let message_id = uuid::Uuid::new_v4().to_string();
        
        tokio::spawn(async move {
            match provider.chat_completion_stream(request).await {
                Ok(mut stream) => {
                    // Send stream started event
                    let _ = event_tx.send(AppEvent::StreamStarted {
                        session_id: session_id.clone(),
                        message_id: message_id.clone(),
                    });
                    
                    use futures::StreamExt;
                    while let Some(event_result) = stream.next().await {
                        match event_result {
                            Ok(event) => {
                                match event {
                                    crate::llm::ProviderEvent::ContentDelta { delta } => {
                                        if tx.send(delta.clone()).is_err() {
                                            break; // Receiver dropped
                                        }
                                        
                                        let _ = event_tx.send(AppEvent::StreamChunk {
                                            session_id: session_id.clone(),
                                            message_id: message_id.clone(),
                                            chunk: delta,
                                        });
                                    }
                                    crate::llm::ProviderEvent::ContentStop => {
                                        break;
                                    }
                                    _ => {} // Handle other events as needed
                                }
                            }
                            Err(e) => {
                                error!("Stream error: {}", e);
                                let _ = event_tx.send(AppEvent::Error {
                                    error: e.to_string(),
                                });
                                break;
                            }
                        }
                    }
                    
                    // Send stream ended event
                    let _ = event_tx.send(AppEvent::StreamEnded {
                        session_id,
                        message_id,
                    });
                }
                Err(e) => {
                    error!("Agent streaming error: {}", e);
                    let _ = event_tx.send(AppEvent::Error {
                        error: e.to_string(),
                    });
                }
            }
        });
        
        Ok(rx)
    }
    
    /// Handle tool calls from LLM response
    pub async fn handle_tool_calls(&self, tool_calls: Vec<crate::llm::types::ToolCall>) -> Result<Vec<Message>> {
        let mut tool_results = Vec::new();
        
        for tool_call in tool_calls {
            debug!("Executing tool: {} with id: {}", tool_call.name, tool_call.id);
            
            // Convert JSON arguments to HashMap
            let parameters = if let serde_json::Value::Object(map) = tool_call.arguments {
                map.into_iter()
                    .map(|(k, v)| (k, v))
                    .collect()
            } else {
                std::collections::HashMap::new()
            };
            
            // Execute the tool
            match self.tool_manager.execute_tool(&tool_call.name, parameters).await {
                Ok(response) => {
                    debug!("Tool '{}' executed successfully", tool_call.name);
                    
                    // Create tool result message
                    let tool_result = Message {
                        id: uuid::Uuid::new_v4().to_string(),
                        role: MessageRole::Tool,
                        content: vec![crate::llm::types::ContentBlock::ToolResult {
                            tool_call_id: tool_call.id,
                            content: response.content,
                        }],
                        timestamp: chrono::Utc::now(),
                        metadata: std::collections::HashMap::new(),
                    };
                    
                    tool_results.push(tool_result);
                }
                Err(e) => {
                    error!("Tool '{}' execution failed: {}", tool_call.name, e);
                    
                    // Create error result message
                    let error_result = Message {
                        id: uuid::Uuid::new_v4().to_string(),
                        role: MessageRole::Tool,
                        content: vec![crate::llm::types::ContentBlock::ToolResult {
                            tool_call_id: tool_call.id,
                            content: format!("Error executing tool: {}", e),
                        }],
                        timestamp: chrono::Utc::now(),
                        metadata: std::collections::HashMap::new(),
                    };
                    
                    tool_results.push(error_result);
                }
            }
        }
        
        Ok(tool_results)
    }
    
    /// Get the provider name
    pub fn provider_name(&self) -> &str {
        self.provider.name()
    }
    
    /// Get the model name
    pub fn model_name(&self) -> &str {
        self.provider.model()
    }
}