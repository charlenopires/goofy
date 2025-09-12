//! Database-backed session manager
//!
//! This module provides a session manager that uses the new database
//! module for persistent storage, matching Crush's implementation.

use anyhow::{Result, Context};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::Utc;
use tracing::{info, debug, error};

use crate::db::{
    connect, DatabaseConfig, DatabaseOperations, Queries,
    Session as DbSession, Message as DbMessage, File as DbFile,
};
use crate::llm::Message as LlmMessage;
use rusqlite::Connection;

/// Database-backed session manager
pub struct DatabaseSessionManager {
    conn: Arc<RwLock<Connection>>,
    current_session_id: Arc<RwLock<Option<String>>>,
}

impl DatabaseSessionManager {
    /// Create a new database-backed session manager
    pub async fn new(config: Option<DatabaseConfig>) -> Result<Self> {
        let config = config.unwrap_or_default();
        let conn = connect(config)?;
        
        Ok(Self {
            conn: Arc::new(RwLock::new(conn)),
            current_session_id: Arc::new(RwLock::new(None)),
        })
    }
    
    /// Create a new session
    pub async fn create_session(&self, title: String) -> Result<DbSession> {
        let session_id = Uuid::new_v4().to_string();
        let session = DbSession::new(session_id.clone(), title);
        
        let conn = self.conn.read().await;
        let queries = Queries::new(&conn);
        queries.create_session(&session)?;
        
        // Set as current session
        *self.current_session_id.write().await = Some(session_id.clone());
        
        info!("Created new session: {}", session_id);
        Ok(session)
    }
    
    /// Create a child session
    pub async fn create_child_session(&self, parent_id: String, title: String) -> Result<DbSession> {
        let session_id = Uuid::new_v4().to_string();
        let session = DbSession::new_child(session_id.clone(), parent_id, title);
        
        let conn = self.conn.read().await;
        let queries = Queries::new(&conn);
        queries.create_session(&session)?;
        
        info!("Created child session: {} (parent: {})", session_id, session.parent_session_id.as_ref().unwrap());
        Ok(session)
    }
    
    /// Get a session by ID
    pub async fn get_session(&self, id: &str) -> Result<Option<DbSession>> {
        let conn = self.conn.read().await;
        let queries = Queries::new(&conn);
        queries.get_session(id)
    }
    
    /// Get the current session
    pub async fn get_current_session(&self) -> Result<Option<DbSession>> {
        let current_id = self.current_session_id.read().await;
        if let Some(id) = current_id.as_ref() {
            self.get_session(id).await
        } else {
            Ok(None)
        }
    }
    
    /// Set the current session
    pub async fn set_current_session(&self, id: String) -> Result<()> {
        // Verify session exists
        if self.get_session(&id).await?.is_none() {
            return Err(anyhow::anyhow!("Session not found: {}", id));
        }
        
        *self.current_session_id.write().await = Some(id.clone());
        info!("Set current session to: {}", id);
        Ok(())
    }
    
    /// List all sessions
    pub async fn list_sessions(&self, limit: Option<usize>) -> Result<Vec<DbSession>> {
        let conn = self.conn.read().await;
        let queries = Queries::new(&conn);
        queries.list_sessions(limit)
    }
    
    /// Update a session
    pub async fn update_session(&self, session: &DbSession) -> Result<()> {
        let conn = self.conn.read().await;
        let queries = Queries::new(&conn);
        queries.update_session(session)?;
        debug!("Updated session: {}", session.id);
        Ok(())
    }
    
    /// Delete a session
    pub async fn delete_session(&self, id: &str) -> Result<()> {
        let conn = self.conn.read().await;
        let queries = Queries::new(&conn);
        queries.delete_session(id)?;
        
        // Clear current session if it was deleted
        let mut current = self.current_session_id.write().await;
        if current.as_ref() == Some(&id.to_string()) {
            *current = None;
        }
        
        info!("Deleted session: {}", id);
        Ok(())
    }
    
    /// Add a message to a session
    pub async fn add_message(&self, session_id: &str, message: LlmMessage) -> Result<DbMessage> {
        let message_id = message.id.clone();
        
        // Convert LLM message to database message
        let parts = serde_json::to_string(&message.content)?;
        let role = match message.role {
            crate::llm::MessageRole::System => "system",
            crate::llm::MessageRole::User => "user",
            crate::llm::MessageRole::Assistant => "assistant",
            crate::llm::MessageRole::Tool => "tool",
        };
        
        let mut db_message = DbMessage::new(
            message_id.clone(),
            session_id.to_string(),
            role.to_string(),
            parts,
        );
        
        // Set provider and model if available
        db_message.provider = message.metadata.get("provider").and_then(|v| v.as_str()).map(String::from);
        db_message.model = message.metadata.get("model").and_then(|v| v.as_str()).map(String::from);
        
        let conn = self.conn.read().await;
        let queries = Queries::new(&conn);
        queries.create_message(&db_message)?;
        
        // Update session's updated_at
        if let Some(mut session) = queries.get_session(session_id)? {
            session.updated_at = Utc::now().timestamp_millis();
            queries.update_session(&session)?;
        }
        
        debug!("Added message {} to session {}", message_id, session_id);
        Ok(db_message)
    }
    
    /// Get messages for a session
    pub async fn get_messages(&self, session_id: &str) -> Result<Vec<DbMessage>> {
        let conn = self.conn.read().await;
        let queries = Queries::new(&conn);
        queries.list_messages_by_session(session_id)
    }
    
    /// Update a message
    pub async fn update_message(&self, message: &DbMessage) -> Result<()> {
        let conn = self.conn.read().await;
        let queries = Queries::new(&conn);
        queries.update_message(message)?;
        debug!("Updated message: {}", message.id);
        Ok(())
    }
    
    /// Mark a message as finished
    pub async fn finish_message(&self, message_id: &str) -> Result<()> {
        let conn = self.conn.read().await;
        let queries = Queries::new(&conn);
        
        if let Some(mut message) = queries.get_message(message_id)? {
            message.finished_at = Some(Utc::now().timestamp_millis());
            queries.update_message(&message)?;
            debug!("Marked message {} as finished", message_id);
        }
        
        Ok(())
    }
    
    /// Save a file associated with a session
    pub async fn save_file(&self, session_id: &str, path: String, content: String) -> Result<DbFile> {
        let file_id = Uuid::new_v4().to_string();
        
        let conn = self.conn.read().await;
        let queries = Queries::new(&conn);
        
        // Check if file already exists for this session
        let existing = queries.get_file_by_path_and_session(&path, session_id)?;
        let version = existing.map(|f| f.version + 1).unwrap_or(0);
        
        let mut file = DbFile::new(file_id.clone(), session_id.to_string(), path.clone(), content);
        file.version = version;
        
        queries.create_file(&file)?;
        
        debug!("Saved file {} (version {}) for session {}", path, version, session_id);
        Ok(file)
    }
    
    /// Get files for a session
    pub async fn get_session_files(&self, session_id: &str) -> Result<Vec<DbFile>> {
        let conn = self.conn.read().await;
        let queries = Queries::new(&conn);
        queries.list_files_by_session(session_id)
    }
    
    /// Get a specific file by path and session
    pub async fn get_file(&self, session_id: &str, path: &str) -> Result<Option<DbFile>> {
        let conn = self.conn.read().await;
        let queries = Queries::new(&conn);
        queries.get_file_by_path_and_session(path, session_id)
    }
    
    /// Update session token usage and cost
    pub async fn update_token_usage(
        &self, 
        session_id: &str, 
        prompt_tokens: i64, 
        completion_tokens: i64,
        cost: f64,
    ) -> Result<()> {
        let conn = self.conn.read().await;
        let queries = Queries::new(&conn);
        
        if let Some(mut session) = queries.get_session(session_id)? {
            session.prompt_tokens += prompt_tokens;
            session.completion_tokens += completion_tokens;
            session.cost += cost;
            session.updated_at = Utc::now().timestamp_millis();
            
            queries.update_session(&session)?;
            debug!("Updated token usage for session {}: +{} prompt, +{} completion, +${:.4}", 
                   session_id, prompt_tokens, completion_tokens, cost);
        }
        
        Ok(())
    }
    
    /// Begin a database transaction
    pub async fn begin_transaction(&self) -> Result<rusqlite::Transaction> {
        let conn = self.conn.write().await;
        let tx = unsafe {
            // This is safe because we hold the write lock
            std::ptr::read(&*conn as *const Connection).transaction()?
        };
        Ok(tx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_session_crud() {
        let dir = tempdir().unwrap();
        let config = DatabaseConfig {
            data_dir: dir.path().to_path_buf(),
            db_name: "test.db".to_string(),
        };
        
        let manager = DatabaseSessionManager::new(Some(config)).await.unwrap();
        
        // Create session
        let session = manager.create_session("Test Session".to_string()).await.unwrap();
        assert_eq!(session.title, "Test Session");
        
        // Get session
        let retrieved = manager.get_session(&session.id).await.unwrap().unwrap();
        assert_eq!(retrieved.id, session.id);
        
        // List sessions
        let sessions = manager.list_sessions(None).await.unwrap();
        assert_eq!(sessions.len(), 1);
        
        // Update session
        let mut updated = session.clone();
        updated.title = "Updated Title".to_string();
        manager.update_session(&updated).await.unwrap();
        
        let retrieved = manager.get_session(&session.id).await.unwrap().unwrap();
        assert_eq!(retrieved.title, "Updated Title");
        
        // Delete session
        manager.delete_session(&session.id).await.unwrap();
        let retrieved = manager.get_session(&session.id).await.unwrap();
        assert!(retrieved.is_none());
    }
    
    #[tokio::test]
    async fn test_messages() {
        let dir = tempdir().unwrap();
        let config = DatabaseConfig {
            data_dir: dir.path().to_path_buf(),
            db_name: "test.db".to_string(),
        };
        
        let manager = DatabaseSessionManager::new(Some(config)).await.unwrap();
        let session = manager.create_session("Test Session".to_string()).await.unwrap();
        
        // Add message
        let llm_message = LlmMessage::new_text(
            crate::llm::MessageRole::User,
            "Test message".to_string(),
        );
        
        let message = manager.add_message(&session.id, llm_message).await.unwrap();
        assert_eq!(message.session_id, session.id);
        
        // Get messages
        let messages = manager.get_messages(&session.id).await.unwrap();
        assert_eq!(messages.len(), 1);
        
        // Finish message
        manager.finish_message(&message.id).await.unwrap();
        
        let conn = manager.conn.read().await;
        let queries = Queries::new(&conn);
        let updated = queries.get_message(&message.id).unwrap().unwrap();
        assert!(updated.finished_at.is_some());
    }
}