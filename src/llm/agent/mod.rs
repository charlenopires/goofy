//! Enhanced agent system with advanced capabilities
//!
//! This module provides an enhanced agent that can handle complex interactions,
//! conversation summarization, and title generation.

use anyhow::{Result, Context};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Mutex};
use std::collections::HashMap;
use uuid::Uuid;
use tracing::{info, debug, error, warn};
use futures::StreamExt;

use crate::llm::{
    LlmProvider, Message, MessageRole, ContentBlock, 
    prompt::{PromptId, get_prompt},
    tools::ToolManager,
};
use crate::session::DatabaseSessionManager;
use crate::db::{Session, Message as DbMessage};

/// Agent event types
#[derive(Debug, Clone)]
pub enum AgentEventType {
    Error,
    Response,
    Summarize,
    ToolUse,
    Thinking,
}

/// Agent event for streaming updates
#[derive(Debug, Clone)]
pub struct AgentEvent {
    pub event_type: AgentEventType,
    pub content: Option<String>,
    pub error: Option<String>,
    pub session_id: Option<String>,
    pub progress: Option<String>,
    pub done: bool,
}

/// Agent service interface
pub struct AgentService {
    provider: Arc<dyn LlmProvider>,
    tool_manager: Arc<ToolManager>,
    db_manager: Arc<DatabaseSessionManager>,
    active_requests: Arc<Mutex<HashMap<String, mpsc::Sender<()>>>>,
    prompt_id: PromptId,
    event_tx: mpsc::UnboundedSender<AgentEvent>,
    event_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<AgentEvent>>>>,
}

impl AgentService {
    /// Create a new agent service
    pub fn new(
        provider: Arc<dyn LlmProvider>,
        tool_manager: Arc<ToolManager>,
        db_manager: Arc<DatabaseSessionManager>,
        prompt_id: PromptId,
    ) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        Self {
            provider,
            tool_manager,
            db_manager,
            active_requests: Arc::new(Mutex::new(HashMap::new())),
            prompt_id,
            event_tx,
            event_rx: Arc::new(RwLock::new(Some(event_rx))),
        }
    }
    
    /// Run the agent with a user message
    pub async fn run(
        &self,
        session_id: String,
        content: String,
        context_paths: Vec<std::path::PathBuf>,
    ) -> Result<mpsc::UnboundedReceiver<AgentEvent>> {
        // Check if session is busy
        if self.is_session_busy(&session_id).await {
            return Err(anyhow::anyhow!("Session is currently processing another request"));
        }
        
        // Create cancellation channel
        let (cancel_tx, mut cancel_rx) = mpsc::channel(1);
        self.active_requests.lock().await.insert(session_id.clone(), cancel_tx);
        
        let provider = self.provider.clone();
        let tool_manager = self.tool_manager.clone();
        let db_manager = self.db_manager.clone();
        let event_tx = self.event_tx.clone();
        let prompt_id = self.prompt_id;
        let session_id_clone = session_id.clone();
        
        // Spawn async task to handle the request
        tokio::spawn(async move {
            let result = Self::process_request(
                provider,
                tool_manager,
                db_manager,
                session_id_clone,
                content,
                context_paths,
                prompt_id,
                event_tx.clone(),
                cancel_rx,
            ).await;
            
            if let Err(e) = result {
                error!("Agent processing error: {}", e);
                let _ = event_tx.send(AgentEvent {
                    event_type: AgentEventType::Error,
                    content: None,
                    error: Some(e.to_string()),
                    session_id: Some(session_id),
                    progress: None,
                    done: true,
                });
            }
        });
        
        // Return event receiver
        let mut rx = self.event_rx.write().await;
        rx.take().ok_or_else(|| anyhow::anyhow!("Event receiver already taken"))
    }
    
    /// Process a request
    async fn process_request(
        provider: Arc<dyn LlmProvider>,
        tool_manager: Arc<ToolManager>,
        db_manager: Arc<DatabaseSessionManager>,
        session_id: String,
        content: String,
        context_paths: Vec<std::path::PathBuf>,
        prompt_id: PromptId,
        event_tx: mpsc::UnboundedSender<AgentEvent>,
        mut cancel_rx: mpsc::Receiver<()>,
    ) -> Result<()> {
        // Get or create session
        let session = if let Some(session) = db_manager.get_session(&session_id).await? {
            session
        } else {
            db_manager.create_session("New Conversation".to_string()).await?
        };
        
        // Add user message to database
        let user_message = Message::new_text(MessageRole::User, content.clone());
        db_manager.add_message(&session.id, user_message.clone()).await?;
        
        // Get conversation history
        let messages = db_manager.get_messages(&session.id).await?;
        let mut conversation = Vec::new();
        
        // Add system prompt
        let system_prompt = get_prompt(prompt_id, provider.name(), &context_paths);
        conversation.push(Message::new_text(MessageRole::System, system_prompt));
        
        // Convert database messages to LLM messages
        for db_msg in messages {
            let role = match db_msg.role.as_str() {
                "user" => MessageRole::User,
                "assistant" => MessageRole::Assistant,
                "system" => MessageRole::System,
                "tool" => MessageRole::Tool,
                _ => MessageRole::User,
            };
            
            // Parse parts JSON back to content blocks
            if let Ok(content_blocks) = serde_json::from_str::<Vec<ContentBlock>>(&db_msg.parts) {
                conversation.push(Message {
                    id: db_msg.id,
                    role,
                    content: content_blocks,
                    timestamp: chrono::Utc::now(),
                    metadata: HashMap::new(),
                });
            }
        }
        
        // Stream response from provider
        let request = crate::llm::ChatRequest {
            messages: conversation,
            temperature: Some(0.7),
            max_tokens: None,
            tools: tool_manager.get_tool_definitions(),
            tool_choice: None,
            stop: None,
            stream: true,
            system_message: None,
            top_p: None,
            metadata: HashMap::new(),
        };
        
        let mut stream = provider.chat_completion_stream(request).await?;
        let mut response_content = String::new();

        // Process stream
        while let Some(event) = stream.next().await {
            // Check for cancellation
            if cancel_rx.try_recv().is_ok() {
                info!("Request cancelled for session {}", session_id);
                break;
            }

            match event {
                Ok(provider_event) => {
                    match provider_event {
                        crate::llm::ProviderEvent::ContentDelta { delta } => {
                            response_content.push_str(&delta);
                            let _ = event_tx.send(AgentEvent {
                                event_type: AgentEventType::Response,
                                content: Some(delta),
                                error: None,
                                session_id: Some(session_id.clone()),
                                progress: None,
                                done: false,
                            });
                        }
                        crate::llm::ProviderEvent::ToolUseStart { tool_call } => {
                            debug!("Tool use: {} - {}", tool_call.name, tool_call.id);
                            let _ = event_tx.send(AgentEvent {
                                event_type: AgentEventType::ToolUse,
                                content: Some(format!("Using tool: {}", tool_call.name)),
                                error: None,
                                session_id: Some(session_id.clone()),
                                progress: None,
                                done: false,
                            });

                            // Execute tool
                            // TODO: Implement tool execution
                        }
                        crate::llm::ProviderEvent::ContentStop |
                        crate::llm::ProviderEvent::Done { .. } => {
                            // Save assistant response as a new message
                            let final_message = Message::new_text(
                                MessageRole::Assistant,
                                response_content.clone(),
                            );

                            let db_msg = db_manager.add_message(&session.id, final_message).await?;
                            db_manager.finish_message(&db_msg.id).await?;

                            // Send completion event
                            let _ = event_tx.send(AgentEvent {
                                event_type: AgentEventType::Response,
                                content: None,
                                error: None,
                                session_id: Some(session_id.clone()),
                                progress: None,
                                done: true,
                            });
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    error!("Stream error: {}", e);
                    return Err(anyhow::anyhow!("{}", e));
                }
            }
        }
        
        Ok(())
    }
    
    /// Check if a session is currently busy
    pub async fn is_session_busy(&self, session_id: &str) -> bool {
        self.active_requests.lock().await.contains_key(session_id)
    }
    
    /// Cancel a session's active request
    pub async fn cancel(&self, session_id: &str) {
        if let Some(cancel_tx) = self.active_requests.lock().await.remove(session_id) {
            let _ = cancel_tx.send(()).await;
            info!("Cancelled request for session {}", session_id);
        }
    }
    
    /// Cancel all active requests
    pub async fn cancel_all(&self) {
        let mut requests = self.active_requests.lock().await;
        for (session_id, cancel_tx) in requests.drain() {
            let _ = cancel_tx.send(()).await;
            info!("Cancelled request for session {}", session_id);
        }
    }
    
    /// Summarize a conversation
    pub async fn summarize(&self, session_id: &str) -> Result<String> {
        // Get conversation messages
        let messages = self.db_manager.get_messages(session_id).await?;
        if messages.is_empty() {
            return Ok("No messages to summarize".to_string());
        }
        
        // Build conversation text
        let mut conversation_text = String::new();
        for msg in messages {
            conversation_text.push_str(&format!("{}: {}\n\n", msg.role, msg.parts));
        }
        
        // Create summarization prompt
        let system_prompt = get_prompt(PromptId::Summarizer, self.provider.name(), &[]);
        let user_prompt = format!("Please summarize the following conversation:\n\n{}", conversation_text);
        
        let request = crate::llm::ChatRequest {
            messages: vec![
                Message::new_text(MessageRole::System, system_prompt),
                Message::new_text(MessageRole::User, user_prompt),
            ],
            temperature: Some(0.3),
            max_tokens: Some(800),
            tools: vec![],
            tool_choice: None,
            stop: None,
            stream: false,
            system_message: None,
            top_p: None,
            metadata: HashMap::new(),
        };

        let response = self.provider.chat_completion(request).await?;

        // Update session with summary
        if let Some(mut session) = self.db_manager.get_session(session_id).await? {
            // Store summary as a special message
            let summary_message = Message::new_text(
                MessageRole::System,
                format!("## Conversation Summary\n\n{}", response.content),
            );
            let summary_msg = self.db_manager.add_message(session_id, summary_message).await?;
            
            // Update session with summary message ID
            session.summary_message_id = Some(summary_msg.id);
            self.db_manager.update_session(&session).await?;
        }
        
        Ok(response.content)
    }
    
    /// Generate a title for a conversation
    pub async fn generate_title(&self, session_id: &str) -> Result<String> {
        // Get first few messages
        let messages = self.db_manager.get_messages(session_id).await?;
        let preview = messages.iter()
            .take(3)
            .map(|m| format!("{}: {}", m.role, 
                m.parts.chars().take(200).collect::<String>()))
            .collect::<Vec<_>>()
            .join("\n\n");
        
        // Create title generation prompt
        let system_prompt = get_prompt(PromptId::Title, self.provider.name(), &[]);
        let user_prompt = format!("Generate a title for this conversation:\n\n{}", preview);
        
        let request = crate::llm::ChatRequest {
            messages: vec![
                Message::new_text(MessageRole::System, system_prompt),
                Message::new_text(MessageRole::User, user_prompt),
            ],
            temperature: Some(0.5),
            max_tokens: Some(20),
            tools: vec![],
            tool_choice: None,
            stop: None,
            stream: false,
            system_message: None,
            top_p: None,
            metadata: HashMap::new(),
        };
        
        let response = self.provider.chat_completion(request).await?;
        let title = response.content.trim().to_string();
        
        // Update session title
        if let Some(mut session) = self.db_manager.get_session(session_id).await? {
            session.title = title.clone();
            self.db_manager.update_session(&session).await?;
        }
        
        Ok(title)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_agent_event_types() {
        let event = AgentEvent {
            event_type: AgentEventType::Response,
            content: Some("Test response".to_string()),
            error: None,
            session_id: Some("test-session".to_string()),
            progress: None,
            done: false,
        };
        
        assert!(matches!(event.event_type, AgentEventType::Response));
        assert_eq!(event.content, Some("Test response".to_string()));
    }
}