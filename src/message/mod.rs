//! Message management system for conversations
//!
//! This module provides comprehensive message handling with
//! support for different content types and conversation roles.

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::db::{Message as DbMessage, Queries, DatabaseOperations};
use crate::session::pubsub::{Broker, Event, EventType};

/// Message role in a conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

impl ToString for MessageRole {
    fn to_string(&self) -> String {
        match self {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        }.to_string()
    }
}

/// Content part in a message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String },
    Image { url: String, alt: Option<String> },
    ToolUse { id: String, name: String, input: serde_json::Value },
    ToolResult { tool_use_id: String, output: String },
    Thinking { text: String },
    Finish { reason: String },
}

/// Attachment to a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: String,
    pub name: String,
    pub content_type: String,
    pub size: usize,
    pub url: Option<String>,
    pub data: Option<Vec<u8>>,
}

/// Message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub role: MessageRole,
    pub parts: Vec<ContentPart>,
    pub attachments: Vec<Attachment>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

impl Message {
    /// Create a new message
    pub fn new(session_id: String, role: MessageRole, parts: Vec<ContentPart>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            session_id,
            role,
            parts,
            attachments: Vec::new(),
            model: None,
            provider: None,
            created_at: now,
            updated_at: now,
            finished_at: None,
        }
    }
    
    /// Create a simple text message
    pub fn text(session_id: String, role: MessageRole, text: String) -> Self {
        Self::new(session_id, role, vec![ContentPart::Text { text }])
    }
    
    /// Add a content part
    pub fn add_part(&mut self, part: ContentPart) {
        self.parts.push(part);
        self.updated_at = Utc::now();
    }
    
    /// Add an attachment
    pub fn add_attachment(&mut self, attachment: Attachment) {
        self.attachments.push(attachment);
        self.updated_at = Utc::now();
    }
    
    /// Mark message as finished
    pub fn finish(&mut self, reason: &str) {
        self.parts.push(ContentPart::Finish { 
            reason: reason.to_string() 
        });
        self.finished_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }
    
    /// Get text content
    pub fn text_content(&self) -> String {
        self.parts.iter()
            .filter_map(|part| match part {
                ContentPart::Text { text } => Some(text.clone()),
                ContentPart::Thinking { text } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }
}

/// Parameters for creating a message
#[derive(Debug, Clone)]
pub struct CreateMessageParams {
    pub role: MessageRole,
    pub parts: Vec<ContentPart>,
    pub attachments: Vec<Attachment>,
    pub model: Option<String>,
    pub provider: Option<String>,
}

/// Message service interface
#[async_trait::async_trait]
pub trait MessageService: Send + Sync {
    /// Subscribe to message events
    fn subscribe(&self) -> tokio::sync::mpsc::UnboundedReceiver<Event<Message>>;

    /// Create a new message
    async fn create(&self, session_id: &str, params: CreateMessageParams) -> Result<Message>;

    /// Update an existing message
    async fn update(&self, message: &Message) -> Result<()>;

    /// Get a message by ID
    async fn get(&self, id: &str) -> Result<Option<Message>>;

    /// List messages for a session
    async fn list(&self, session_id: &str) -> Result<Vec<Message>>;

    /// Delete a message
    async fn delete(&self, id: &str) -> Result<()>;

    /// Delete all messages for a session
    async fn delete_session_messages(&self, session_id: &str) -> Result<()>;
}

/// Message service implementation
pub struct MessageServiceImpl {
    broker: Arc<Broker<Message>>,
    conn: Arc<std::sync::Mutex<rusqlite::Connection>>,
}

impl MessageServiceImpl {
    /// Create a new message service
    pub fn new(conn: Arc<std::sync::Mutex<rusqlite::Connection>>) -> Self {
        Self {
            broker: Arc::new(Broker::new()),
            conn,
        }
    }
    
    /// Convert database message to domain message
    fn from_db_message(&self, db_msg: DbMessage) -> Result<Message> {
        let parts: Vec<ContentPart> = serde_json::from_str(&db_msg.parts)
            .context("Failed to parse message parts")?;
        
        let role = match db_msg.role.as_str() {
            "system" => MessageRole::System,
            "user" => MessageRole::User,
            "assistant" => MessageRole::Assistant,
            "tool" => MessageRole::Tool,
            _ => MessageRole::User,
        };
        
        Ok(Message {
            id: db_msg.id,
            session_id: db_msg.session_id,
            role,
            parts,
            attachments: Vec::new(), // TODO: Load attachments from separate table
            model: db_msg.model,
            provider: db_msg.provider,
            created_at: DateTime::from_timestamp_millis(db_msg.created_at)
                .unwrap_or_else(Utc::now),
            updated_at: DateTime::from_timestamp_millis(db_msg.updated_at)
                .unwrap_or_else(Utc::now),
            finished_at: db_msg.finished_at
                .and_then(DateTime::from_timestamp_millis),
        })
    }
    
    /// Convert domain message to database message
    fn to_db_message(&self, msg: &Message) -> DbMessage {
        DbMessage {
            id: msg.id.clone(),
            session_id: msg.session_id.clone(),
            role: msg.role.to_string(),
            parts: serde_json::to_string(&msg.parts).unwrap_or_default(),
            model: msg.model.clone(),
            created_at: msg.created_at.timestamp_millis(),
            updated_at: msg.updated_at.timestamp_millis(),
            finished_at: msg.finished_at.map(|dt| dt.timestamp_millis()),
            provider: msg.provider.clone(),
        }
    }
}

#[async_trait::async_trait]
impl MessageService for MessageServiceImpl {
    fn subscribe(&self) -> tokio::sync::mpsc::UnboundedReceiver<Event<Message>> {
        self.broker.subscribe()
    }
    
    async fn create(&self, session_id: &str, params: CreateMessageParams) -> Result<Message> {
        let mut message = Message::new(
            session_id.to_string(),
            params.role,
            params.parts,
        );
        
        message.attachments = params.attachments;
        message.model = params.model;
        message.provider = params.provider;
        
        // Add finish part for non-assistant messages
        if params.role != MessageRole::Assistant {
            message.finish("stop");
        }
        
        // Save to database
        let db_msg = self.to_db_message(&message);
        let conn = self.conn.lock().unwrap();
        let queries = Queries::new(&conn);
        queries.create_message(&db_msg)?;
        
        // Publish event
        self.broker.publish(EventType::Created, message.clone());
        
        Ok(message)
    }
    
    async fn update(&self, message: &Message) -> Result<()> {
        let db_msg = self.to_db_message(message);
        let conn = self.conn.lock().unwrap();
        let queries = Queries::new(&conn);
        queries.update_message(&db_msg)?;
        
        // Publish event
        self.broker.publish(EventType::Updated, message.clone());
        
        Ok(())
    }
    
    async fn get(&self, id: &str) -> Result<Option<Message>> {
        let conn = self.conn.lock().unwrap();
        let queries = Queries::new(&conn);
        
        if let Some(db_msg) = queries.get_message(id)? {
            Ok(Some(self.from_db_message(db_msg)?))
        } else {
            Ok(None)
        }
    }
    
    async fn list(&self, session_id: &str) -> Result<Vec<Message>> {
        let conn = self.conn.lock().unwrap();
        let queries = Queries::new(&conn);
        let db_messages = queries.list_messages_by_session(session_id)?;
        
        let mut messages = Vec::new();
        for db_msg in db_messages {
            messages.push(self.from_db_message(db_msg)?);
        }
        
        Ok(messages)
    }
    
    async fn delete(&self, id: &str) -> Result<()> {
        // Get message before deletion for event
        let message = self.get(id).await?
            .ok_or_else(|| anyhow::anyhow!("Message not found"))?;
        
        let conn = self.conn.lock().unwrap();
        let queries = Queries::new(&conn);
        queries.delete_message(id)?;
        
        // Publish event
        self.broker.publish(EventType::Deleted, message);
        
        Ok(())
    }
    
    async fn delete_session_messages(&self, session_id: &str) -> Result<()> {
        let messages = self.list(session_id).await?;
        
        for message in messages {
            self.delete(&message.id).await?;
        }
        
        Ok(())
    }
}

/// Create a new message service
pub fn new_service(conn: Arc<std::sync::Mutex<rusqlite::Connection>>) -> Arc<dyn MessageService> {
    Arc::new(MessageServiceImpl::new(conn))
}