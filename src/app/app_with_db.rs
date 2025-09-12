//! Application with database-backed session management

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, error};

use crate::{
    config::Config,
    llm::{LlmProvider, ProviderFactory, ProviderConfig, tools::{ToolManager, ToolPermissions}, MessageRole},
    session::{DatabaseSessionManager, DatabaseSessionFactory},
};

use super::{AppEvent, Agent};

/// Application with database-backed session management
pub struct AppWithDb {
    config: Config,
    db_manager: Arc<DatabaseSessionManager>,
    llm_provider: Arc<dyn LlmProvider>,
    tool_manager: Arc<ToolManager>,
    event_tx: mpsc::UnboundedSender<AppEvent>,
    event_rx: RwLock<Option<mpsc::UnboundedReceiver<AppEvent>>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl AppWithDb {
    /// Create a new application instance with database backing
    pub async fn new(config: Config) -> Result<Self> {
        debug!("Creating new AppWithDb instance");
        
        // Initialize database session manager
        let db_manager = DatabaseSessionFactory::create(&config.data_dir).await?;
        
        // Create LLM provider from config
        let provider_config = ProviderConfig::from_config(&config)?;
        let llm_provider = ProviderFactory::create(provider_config).await?;
        
        // Initialize tool manager
        let permissions = ToolPermissions {
            allow_read: true,
            allow_write: !config.read_only,
            allow_execute: config.yolo,
            allow_network: config.yolo,
            restricted_paths: vec![],
            yolo_mode: config.yolo,
        };
        let tool_manager = Arc::new(ToolManager::new(permissions));
        
        // Create event channels
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        Ok(Self {
            config,
            db_manager,
            llm_provider,
            tool_manager,
            event_tx,
            event_rx: RwLock::new(Some(event_rx)),
            shutdown_tx: None,
        })
    }
    
    /// Run interactive mode with TUI
    pub async fn run_interactive(&mut self) -> Result<()> {
        info!("Starting interactive mode with database backing");
        
        // Create or get the current session
        let session = if let Some(session) = self.db_manager.get_current_session().await? {
            info!("Resuming session: {}", session.id);
            session
        } else {
            info!("Creating new session");
            self.db_manager.create_session("New Chat".to_string()).await?
        };
        
        // TODO: Launch TUI with database-backed session
        info!("Session {} ready for interaction", session.id);
        
        Ok(())
    }
    
    /// Run a single prompt non-interactively
    pub async fn run_prompt(&mut self, prompt: String) -> Result<()> {
        info!("Running non-interactive prompt with database backing");
        
        // Create a new session for this prompt
        let session = self.db_manager.create_session(
            prompt.chars().take(50).collect::<String>()
        ).await?;
        
        // Add user message to database
        let user_message = crate::llm::Message::new_text(
            MessageRole::User,
            prompt.clone(),
        );
        self.db_manager.add_message(&session.id, user_message).await?;
        
        // Create agent for this session
        let agent = Agent::new(
            self.llm_provider.clone(),
            self.tool_manager.clone(),
            self.event_tx.clone(),
        );
        
        // Process the prompt
        info!("Processing prompt in session {}", session.id);
        match agent.process_message(&prompt).await {
            Ok(response) => {
                // Add assistant response to database
                let mut assistant_message = crate::llm::Message::new_text(
                    MessageRole::Assistant,
                    response.clone(),
                );
                assistant_message.metadata.insert(
                    "model".to_string(),
                    serde_json::json!(self.llm_provider.model()),
                );
                assistant_message.metadata.insert(
                    "provider".to_string(),
                    serde_json::json!(self.llm_provider.name()),
                );
                let msg = self.db_manager.add_message(&session.id, assistant_message).await?;
                
                // Mark message as finished
                self.db_manager.finish_message(&msg.id).await?;
                
                // Update token usage if available
                // TODO: Extract token usage from response metadata
                
                println!("{}", response);
            }
            Err(e) => {
                error!("Failed to process prompt: {}", e);
                return Err(e);
            }
        }
        
        Ok(())
    }
    
    /// Get the database manager
    pub fn db_manager(&self) -> &Arc<DatabaseSessionManager> {
        &self.db_manager
    }
    
    /// List recent sessions
    pub async fn list_sessions(&self, limit: usize) -> Result<Vec<crate::db::Session>> {
        self.db_manager.list_sessions(Some(limit)).await
    }
    
    /// Resume a specific session
    pub async fn resume_session(&self, session_id: String) -> Result<()> {
        self.db_manager.set_current_session(session_id).await
    }
    
    /// Get messages for the current session
    pub async fn get_current_messages(&self) -> Result<Vec<crate::db::Message>> {
        if let Some(session) = self.db_manager.get_current_session().await? {
            self.db_manager.get_messages(&session.id).await
        } else {
            Ok(vec![])
        }
    }
    
    /// Save a file to the current session
    pub async fn save_file(&self, path: String, content: String) -> Result<()> {
        if let Some(session) = self.db_manager.get_current_session().await? {
            self.db_manager.save_file(&session.id, path, content).await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("No active session"))
        }
    }
}