//! Core application logic and orchestration
//!
//! This module provides the main application structure that coordinates
//! sessions, LLM providers, and conversation management.

mod agent;
mod events;

pub use agent::*;
pub use events::*;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, error};

use crate::{
    config::Config,
    llm::{LlmProvider, ProviderFactory, ProviderConfig, tools::{ToolManager, ToolPermissions}},
    session::{SessionManager, Session, ConversationManager, SessionService, SessionServiceFactory},
};

/// Main application structure
pub struct App {
    config: Config,
    session_manager: Arc<SessionManager>,
    session_service: Arc<dyn SessionService>,
    conversation_manager: Arc<ConversationManager>,
    llm_provider: Arc<dyn LlmProvider>,
    tool_manager: Arc<ToolManager>,
    event_tx: mpsc::UnboundedSender<AppEvent>,
    event_rx: RwLock<Option<mpsc::UnboundedReceiver<AppEvent>>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl App {
    /// Create a new application instance
    pub async fn new(config: Config) -> Result<Self> {
        debug!("Creating new App instance");
        
        // Initialize session manager (legacy)
        let session_manager = Arc::new(SessionManager::new(&config.data_dir).await?);
        
        // Initialize new session service
        let session_service = SessionServiceFactory::create(&config.data_dir).await?;
        
        // Initialize conversation manager
        let conversation_manager = Arc::new(ConversationManager::new());
        
        // Create LLM provider from config
        let provider_config = ProviderConfig {
            provider_type: config.provider.clone(),
            api_key: config.api_key.clone(),
            base_url: config.base_url.clone(),
            model: config.model.clone(),
            max_tokens: config.max_tokens,
            temperature: config.temperature,
            top_p: config.top_p,
            stream: config.stream,
            tools: Vec::new(), // TODO: Load from config
            extra_headers: config.extra_headers.clone(),
            extra_body: config.extra_body.clone(),
        };
        
        let llm_provider = ProviderFactory::create_provider(provider_config)?;
        llm_provider.validate_config()?;
        
        // Initialize tool manager with permissions from config
        let tool_permissions = ToolPermissions {
            yolo_mode: config.yolo_mode.unwrap_or(false),
            allow_read: true,
            allow_write: !config.read_only.unwrap_or(false),
            allow_execute: !config.read_only.unwrap_or(false),
            allow_network: false,
            restricted_paths: vec![
                "/etc".to_string(),
                "/sys".to_string(),
                "/proc".to_string(),
                "/dev".to_string(),
            ],
        };
        let tool_manager = Arc::new(ToolManager::new(tool_permissions));
        
        // Create event channel
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        Ok(App {
            config,
            session_manager,
            session_service,
            conversation_manager,
            llm_provider: Arc::from(llm_provider),
            tool_manager,
            event_tx,
            event_rx: RwLock::new(Some(event_rx)),
            shutdown_tx: None,
        })
    }
    
    /// Get the session manager
    pub fn session_manager(&self) -> &Arc<SessionManager> {
        &self.session_manager
    }
    
    /// Get the session service
    pub fn session_service(&self) -> &Arc<dyn SessionService> {
        &self.session_service
    }
    
    /// Get the conversation manager
    pub fn conversation_manager(&self) -> &Arc<ConversationManager> {
        &self.conversation_manager
    }
    
    /// Get the LLM provider
    pub fn llm_provider(&self) -> &Arc<dyn LlmProvider> {
        &self.llm_provider
    }
    
    /// Get the tool manager
    pub fn tool_manager(&self) -> &Arc<ToolManager> {
        &self.tool_manager
    }
    
    /// Get the event sender
    pub fn event_sender(&self) -> &mpsc::UnboundedSender<AppEvent> {
        &self.event_tx
    }
    
    /// Start the application event loop
    pub async fn start_event_loop(&mut self) -> Result<()> {
        let mut event_rx = self.event_rx.write().await.take()
            .ok_or_else(|| anyhow::anyhow!("Event loop already started"))?;
        
        // Subscribe to session events
        let mut session_events = self.session_service.subscribe();
        
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);
        
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(event) = event_rx.recv() => {
                        if let Err(e) = Self::handle_event(event).await {
                            error!("Error handling event: {}", e);
                        }
                    }
                    Some(session_event) = session_events.recv() => {
                        // Convert session event to app event
                        let app_event = match session_event.event_type {
                            crate::session::EventType::Created => {
                                AppEvent::SessionCreated { 
                                    session_id: session_event.payload.id 
                                }
                            }
                            crate::session::EventType::Updated => {
                                AppEvent::SessionUpdated { 
                                    session_id: session_event.payload.id 
                                }
                            }
                            crate::session::EventType::Deleted => {
                                AppEvent::SessionDeleted { 
                                    session_id: session_event.payload.id 
                                }
                            }
                        };
                        
                        if let Err(e) = Self::handle_event(app_event).await {
                            error!("Error handling session event: {}", e);
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Shutting down event loop");
                        break;
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Handle application events
    async fn handle_event(event: AppEvent) -> Result<()> {
        match event {
            AppEvent::SessionCreated { session_id } => {
                info!("Session created: {}", session_id);
            }
            AppEvent::SessionUpdated { session_id } => {
                debug!("Session updated: {}", session_id);
            }
            AppEvent::SessionDeleted { session_id } => {
                debug!("Session deleted: {}", session_id);
            }
            AppEvent::MessageSent { session_id, message_id } => {
                debug!("Message sent in session {}: {}", session_id, message_id);
            }
            AppEvent::MessageReceived { session_id, message_id } => {
                debug!("Message received in session {}: {}", session_id, message_id);
            }
            AppEvent::ConversationStarted { session_id } => {
                info!("Conversation started in session: {}", session_id);
            }
            AppEvent::ConversationEnded { session_id } => {
                info!("Conversation ended in session: {}", session_id);
            }
            AppEvent::StreamStarted { session_id, message_id } => {
                debug!("Stream started in session {}: {}", session_id, message_id);
            }
            AppEvent::StreamChunk { session_id, message_id, chunk: _ } => {
                debug!("Stream chunk received in session {}: {}", session_id, message_id);
            }
            AppEvent::StreamEnded { session_id, message_id } => {
                debug!("Stream ended in session {}: {}", session_id, message_id);
            }
            AppEvent::ToolCalled { session_id, tool_name, tool_id } => {
                debug!("Tool called in session {}: {} ({})", session_id, tool_name, tool_id);
            }
            AppEvent::ToolCompleted { session_id, tool_id, result: _ } => {
                debug!("Tool completed in session {}: {}", session_id, tool_id);
            }
            AppEvent::Error { error } => {
                error!("Application error: {}", error);
            }
            AppEvent::Shutdown => {
                info!("Application shutdown requested");
            }
        }
        
        Ok(())
    }
    
    /// Run the application in interactive mode (TUI)
    pub async fn run_interactive(&mut self) -> Result<()> {
        info!("Starting interactive mode");
        
        // Start event loop
        self.start_event_loop().await?;
        
        // Launch the TUI
        crate::tui::run_with_app(self).await?;
        
        Ok(())
    }
    
    /// Run a single prompt non-interactively
    pub async fn run_non_interactive(&mut self, prompt: &str, quiet: bool) -> Result<String> {
        info!("Running non-interactive prompt");
        debug!("Prompt: {}", prompt);
        debug!("Quiet mode: {}", quiet);
        
        if !quiet {
            println!("Processing prompt...");
        }
        
        // Create a new session using the session service
        let title = if prompt.len() > 100 {
            format!("{}...", &prompt[..100])
        } else {
            prompt.to_string()
        };
        
        let session = self.session_service.create(title).await?;
        
        // Start conversation
        let conversation = self.conversation_manager.start_conversation(
            session.id.clone(),
            self.llm_provider.clone(),
        ).await?;
        
        // Send the prompt and get response
        let response = conversation.send_message(prompt.to_string()).await?;
        
        // Update session with token usage
        if let Some(usage) = response.metadata.get("usage") {
            // Update session with completion details
            let mut updated_session = session.clone();
            updated_session.message_count += 2; // User message + assistant response
            
            // Parse usage if it's a JSON value
            if let Ok(usage_obj) = serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(usage.clone()) {
                if let Some(prompt_tokens) = usage_obj.get("prompt_tokens").and_then(|v| v.as_i64()) {
                    updated_session.prompt_tokens = prompt_tokens;
                }
                if let Some(completion_tokens) = usage_obj.get("completion_tokens").and_then(|v| v.as_i64()) {
                    updated_session.completion_tokens = completion_tokens;
                }
            }
            
            // Save the updated session
            let _ = self.session_service.save(updated_session).await;
        }
        
        if !quiet {
            println!("Response received.");
        }
        
        Ok(response.content)
    }
    
    /// Shutdown the application gracefully
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down application");
        
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(()).await;
        }
        
        // TODO: Clean up resources, close database connections, etc.
        
        Ok(())
    }
}