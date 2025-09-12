//! Session service with pub/sub event system
//!
//! This module provides a complete session management service with
//! event publishing capabilities, matching the Crush session architecture.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::Path,
    sync::Arc,
};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, error};
use uuid::Uuid;

use crate::session::{
    database::{Database, SessionRow},
    pubsub::{Broker, Event, EventType},
};

/// Session model matching Crush's session structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceSession {
    pub id: String,
    pub parent_session_id: Option<String>,
    pub title: String,
    pub message_count: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub summary_message_id: Option<String>,
    pub cost: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ServiceSession {
    /// Create a new session
    pub fn new(id: String, title: String, parent_session_id: Option<String>) -> Self {
        let now = Utc::now();
        
        Self {
            id,
            parent_session_id,
            title,
            message_count: 0,
            prompt_tokens: 0,
            completion_tokens: 0,
            summary_message_id: None,
            cost: 0.0,
            created_at: now,
            updated_at: now,
        }
    }
    
    /// Create from database row
    pub fn from_row(row: SessionRow) -> Self {
        Self {
            id: row.id,
            parent_session_id: row.parent_session_id,
            title: row.title,
            message_count: row.message_count,
            prompt_tokens: row.total_input_tokens,
            completion_tokens: row.total_output_tokens,
            summary_message_id: None, // Not stored in current DB schema
            cost: row.total_cost,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Session service trait defining all operations
#[async_trait]
pub trait SessionService: Send + Sync {
    /// Subscribe to session events
    fn subscribe(&self) -> mpsc::UnboundedReceiver<Event<ServiceSession>>;
    
    /// Create a new session
    async fn create(&self, title: String) -> Result<ServiceSession>;
    
    /// Create a title generation session
    async fn create_title_session(&self, parent_session_id: String) -> Result<ServiceSession>;
    
    /// Create a task session for tool calls
    async fn create_task_session(
        &self,
        tool_call_id: String,
        parent_session_id: String,
        title: String,
    ) -> Result<ServiceSession>;
    
    /// Get a session by ID
    async fn get(&self, id: &str) -> Result<ServiceSession>;
    
    /// List all sessions
    async fn list(&self) -> Result<Vec<ServiceSession>>;
    
    /// Save/update a session
    async fn save(&self, session: ServiceSession) -> Result<ServiceSession>;
    
    /// Delete a session
    async fn delete(&self, id: &str) -> Result<()>;
}

/// Default implementation of SessionService
pub struct DefaultSessionService {
    broker: Arc<Broker<ServiceSession>>,
    db: Arc<Database>,
    cache: Arc<RwLock<HashMap<String, ServiceSession>>>,
}

impl DefaultSessionService {
    /// Create a new session service
    pub async fn new<P: AsRef<Path>>(data_dir: P) -> Result<Self> {
        let db_path = data_dir.as_ref().join("sessions.db");
        let db = Arc::new(Database::new(db_path).await?);
        let broker = Arc::new(Broker::new());
        let cache = Arc::new(RwLock::new(HashMap::new()));
        
        Ok(Self {
            broker,
            db,
            cache,
        })
    }
    
    /// Create with existing database
    pub fn with_database(db: Arc<Database>) -> Self {
        Self {
            broker: Arc::new(Broker::new()),
            db,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Publish an event
    fn publish(&self, event_type: EventType, session: ServiceSession) {
        self.broker.publish(event_type, session);
    }
}

#[async_trait]
impl SessionService for DefaultSessionService {
    fn subscribe(&self) -> mpsc::UnboundedReceiver<Event<ServiceSession>> {
        self.broker.subscribe()
    }
    
    async fn create(&self, title: String) -> Result<ServiceSession> {
        let id = Uuid::new_v4().to_string();
        let session = ServiceSession::new(id.clone(), title.clone(), None);
        
        // Insert into database
        self.db.insert_session(
            &session.id,
            &session.title,
            session.parent_session_id.as_deref(),
            None,
        ).await?;
        
        // Cache in memory
        self.cache.write().await.insert(id.clone(), session.clone());
        
        // Publish event
        self.publish(EventType::Created, session.clone());
        
        info!("Created session: {}", id);
        Ok(session)
    }
    
    async fn create_title_session(&self, parent_session_id: String) -> Result<ServiceSession> {
        let id = format!("title-{}", parent_session_id);
        let session = ServiceSession::new(
            id.clone(),
            "Generate a title".to_string(),
            Some(parent_session_id.clone()),
        );
        
        // Insert into database
        self.db.insert_session(
            &session.id,
            &session.title,
            session.parent_session_id.as_deref(),
            None,
        ).await?;
        
        // Cache in memory
        self.cache.write().await.insert(id.clone(), session.clone());
        
        // Publish event
        self.publish(EventType::Created, session.clone());
        
        info!("Created title session: {}", id);
        Ok(session)
    }
    
    async fn create_task_session(
        &self,
        tool_call_id: String,
        parent_session_id: String,
        title: String,
    ) -> Result<ServiceSession> {
        let session = ServiceSession::new(
            tool_call_id.clone(),
            title.clone(),
            Some(parent_session_id.clone()),
        );
        
        // Insert into database
        self.db.insert_session(
            &session.id,
            &session.title,
            session.parent_session_id.as_deref(),
            None,
        ).await?;
        
        // Cache in memory
        self.cache.write().await.insert(tool_call_id.clone(), session.clone());
        
        // Publish event
        self.publish(EventType::Created, session.clone());
        
        info!("Created task session: {}", tool_call_id);
        Ok(session)
    }
    
    async fn get(&self, id: &str) -> Result<ServiceSession> {
        // Check cache first
        if let Some(session) = self.cache.read().await.get(id) {
            return Ok(session.clone());
        }
        
        // Load from database
        let row = self.db.get_session(id).await?
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", id))?;
        
        let session = ServiceSession::from_row(row);
        
        // Update cache
        self.cache.write().await.insert(id.to_string(), session.clone());
        
        Ok(session)
    }
    
    async fn list(&self) -> Result<Vec<ServiceSession>> {
        let rows = self.db.list_sessions(None).await?;
        let sessions: Vec<ServiceSession> = rows.into_iter()
            .map(ServiceSession::from_row)
            .collect();
        
        // Update cache with all sessions
        let mut cache = self.cache.write().await;
        for session in &sessions {
            cache.insert(session.id.clone(), session.clone());
        }
        
        Ok(sessions)
    }
    
    async fn save(&self, mut session: ServiceSession) -> Result<ServiceSession> {
        // Update timestamp
        session.updated_at = Utc::now();
        
        // Update in database
        self.db.update_session(
            &session.id,
            Some(&session.title),
            Some(session.message_count as i32),
            Some(session.prompt_tokens as i32),
            Some(session.completion_tokens as i32),
            Some(session.cost),
            None,
        ).await?;
        
        // Update cache
        self.cache.write().await.insert(session.id.clone(), session.clone());
        
        // Publish event
        self.publish(EventType::Updated, session.clone());
        
        debug!("Updated session: {}", session.id);
        Ok(session)
    }
    
    async fn delete(&self, id: &str) -> Result<()> {
        // Get session for event
        let session = self.get(id).await?;
        
        // Delete from database
        self.db.delete_session(id).await?;
        
        // Remove from cache
        self.cache.write().await.remove(id);
        
        // Publish event
        self.publish(EventType::Deleted, session);
        
        info!("Deleted session: {}", id);
        Ok(())
    }
}

/// Session service factory
pub struct SessionServiceFactory;

impl SessionServiceFactory {
    /// Create a new session service
    pub async fn create<P: AsRef<Path>>(data_dir: P) -> Result<Arc<dyn SessionService>> {
        let service = DefaultSessionService::new(data_dir).await?;
        Ok(Arc::new(service))
    }
    
    /// Create with existing database
    pub fn with_database(db: Arc<Database>) -> Arc<dyn SessionService> {
        Arc::new(DefaultSessionService::with_database(db))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_session_service() {
        let dir = tempdir().unwrap();
        let service = DefaultSessionService::new(dir.path()).await.unwrap();
        
        // Test create
        let session = service.create("Test Session".to_string()).await.unwrap();
        assert_eq!(session.title, "Test Session");
        assert_eq!(session.message_count, 0);
        
        // Test get
        let retrieved = service.get(&session.id).await.unwrap();
        assert_eq!(retrieved.id, session.id);
        
        // Test update
        let mut updated = session.clone();
        updated.message_count = 5;
        updated.cost = 0.05;
        let saved = service.save(updated).await.unwrap();
        assert_eq!(saved.message_count, 5);
        
        // Test list
        let sessions = service.list().await.unwrap();
        assert_eq!(sessions.len(), 1);
        
        // Test delete
        service.delete(&session.id).await.unwrap();
        let result = service.get(&session.id).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_task_session() {
        let dir = tempdir().unwrap();
        let service = DefaultSessionService::new(dir.path()).await.unwrap();
        
        // Create parent session
        let parent = service.create("Parent".to_string()).await.unwrap();
        
        // Create task session
        let task = service.create_task_session(
            "tool-123".to_string(),
            parent.id.clone(),
            "Execute Tool".to_string(),
        ).await.unwrap();
        
        assert_eq!(task.id, "tool-123");
        assert_eq!(task.parent_session_id, Some(parent.id));
    }
}