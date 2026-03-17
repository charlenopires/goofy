//! Core application logic and orchestration
//!
//! This module provides the main application structure that coordinates
//! sessions, LLM providers, and conversation management.

mod agent;
mod events;
mod app_with_db;

pub use agent::*;
pub use events::*;
pub use app_with_db::AppWithDb;

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
        
        // Initialize new session service (creates DB with correct schema via migrations)
        let session_service = SessionServiceFactory::create(&config.data_dir).await?;

        // Initialize legacy session manager using a separate DB file to avoid schema conflicts
        let legacy_db_dir = config.data_dir.join("legacy");
        if !legacy_db_dir.exists() {
            let _ = std::fs::create_dir_all(&legacy_db_dir);
        }
        let session_manager = Arc::new(SessionManager::new(&legacy_db_dir).await?);
        
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
                            crate::pubsub::EventType::Created => {
                                AppEvent::SessionCreated {
                                    session_id: session_event.payload.id
                                }
                            }
                            crate::pubsub::EventType::Updated => {
                                AppEvent::SessionUpdated {
                                    session_id: session_event.payload.id
                                }
                            }
                            crate::pubsub::EventType::Deleted => {
                                AppEvent::SessionDeleted {
                                    session_id: session_event.payload.id
                                }
                            }
                            _ => {
                                continue;
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

        if !quiet {
            eprintln!("Processing...");
        }

        // Use the Agent directly with the LLM provider
        let messages = vec![
            crate::llm::Message::new_user(prompt.to_string()),
        ];

        let request = crate::llm::ChatRequest {
            messages,
            tools: self.tool_manager.get_tool_definitions(),
            system_message: self.config.system_message.clone(),
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            top_p: self.config.top_p,
            stream: false,
            metadata: std::collections::HashMap::new(),
            tool_choice: None,
            stop: None,
        };

        let response = self.llm_provider.chat_completion(request).await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

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