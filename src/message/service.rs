use anyhow::Result;
use async_trait::async_trait;
use serde_json;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use crate::db::Database;
use crate::session::{Event, EventType};
use super::{Message, MessageRole, ContentPart, CreateMessageParams, FinishReason};

#[async_trait]
pub trait MessageService: Send + Sync {
    fn subscribe(&self) -> mpsc::UnboundedReceiver<Event<Message>>;
    async fn create(&self, session_id: &str, params: CreateMessageParams) -> Result<Message>;
    async fn update(&self, message: &Message) -> Result<()>;
    async fn get(&self, id: &str) -> Result<Message>;
    async fn list(&self, session_id: &str) -> Result<Vec<Message>>;
    async fn delete(&self, id: &str) -> Result<()>;
    async fn delete_session_messages(&self, session_id: &str) -> Result<()>;
}

pub struct MessageServiceImpl {
    db: Arc<Database>,
    sender: mpsc::UnboundedSender<Event<Message>>,
    receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<Event<Message>>>>>,
}

impl MessageServiceImpl {
    pub fn new(db: Arc<Database>) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            db,
            sender,
            receiver: Arc::new(RwLock::new(Some(receiver))),
        }
    }

    fn publish(&self, event_type: EventType, message: Message) {
        let event = Event {
            event_type,
            data: message,
        };
        let _ = self.sender.send(event);
    }

    fn serialize_parts(parts: &[ContentPart]) -> Result<String> {
        // Wrap parts with type information for deserialization
        let wrapped_parts: Vec<serde_json::Value> = parts.iter().map(|part| {
            match part {
                ContentPart::Text { text } => serde_json::json!({
                    "type": "text",
                    "data": { "text": text }
                }),
                ContentPart::Image { url, alt } => serde_json::json!({
                    "type": "image",
                    "data": { "url": url, "alt": alt }
                }),
                ContentPart::ToolUse { id, name, input } => serde_json::json!({
                    "type": "tool_use",
                    "data": { "id": id, "name": name, "input": input }
                }),
                ContentPart::ToolResult { tool_use_id, output, is_error } => serde_json::json!({
                    "type": "tool_result",
                    "data": { "tool_use_id": tool_use_id, "output": output, "is_error": is_error }
                }),
                ContentPart::Thinking { text } => serde_json::json!({
                    "type": "thinking",
                    "data": { "text": text }
                }),
                ContentPart::Finish { reason } => serde_json::json!({
                    "type": "finish",
                    "data": { "reason": reason }
                }),
            }
        }).collect();
        
        serde_json::to_string(&wrapped_parts).map_err(Into::into)
    }

    fn deserialize_parts(data: &str) -> Result<Vec<ContentPart>> {
        let wrapped_parts: Vec<serde_json::Value> = serde_json::from_str(data)?;
        let mut parts = Vec::new();
        
        for wrapped in wrapped_parts {
            let part_type = wrapped["type"].as_str().unwrap_or("");
            let data = &wrapped["data"];
            
            let part = match part_type {
                "text" => ContentPart::Text {
                    text: data["text"].as_str().unwrap_or("").to_string(),
                },
                "image" => ContentPart::Image {
                    url: data["url"].as_str().unwrap_or("").to_string(),
                    alt: data["alt"].as_str().map(|s| s.to_string()),
                },
                "tool_use" => ContentPart::ToolUse {
                    id: data["id"].as_str().unwrap_or("").to_string(),
                    name: data["name"].as_str().unwrap_or("").to_string(),
                    input: data["input"].clone(),
                },
                "tool_result" => ContentPart::ToolResult {
                    tool_use_id: data["tool_use_id"].as_str().unwrap_or("").to_string(),
                    output: data["output"].as_str().unwrap_or("").to_string(),
                    is_error: data["is_error"].as_bool(),
                },
                "thinking" => ContentPart::Thinking {
                    text: data["text"].as_str().unwrap_or("").to_string(),
                },
                "finish" => ContentPart::Finish {
                    reason: data["reason"].as_str().unwrap_or("stop").to_string(),
                },
                _ => continue,
            };
            
            parts.push(part);
        }
        
        Ok(parts)
    }
}

#[async_trait]
impl MessageService for MessageServiceImpl {
    fn subscribe(&self) -> mpsc::UnboundedReceiver<Event<Message>> {
        let (tx, rx) = mpsc::unbounded_channel();
        // Clone sender for new subscriber
        let sender = self.sender.clone();
        tokio::spawn(async move {
            // Forward events to subscriber
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });
        rx
    }

    async fn create(&self, session_id: &str, params: CreateMessageParams) -> Result<Message> {
        let mut parts = params.parts.clone();
        
        // Add finish part for non-assistant messages
        if params.role != MessageRole::Assistant {
            parts.push(ContentPart::Finish {
                reason: "stop".to_string(),
            });
        }
        
        let parts_json = Self::serialize_parts(&parts)?;
        let message_id = Uuid::new_v4().to_string();
        
        // Insert into database
        let conn = self.db.connection().await?;
        conn.lock().await.execute(
            "INSERT INTO messages (id, session_id, role, parts, model, provider, created_at, updated_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                &message_id,
                session_id,
                params.role.to_string(),
                &parts_json,
                &params.model,
                &params.provider,
                chrono::Utc::now().timestamp(),
                chrono::Utc::now().timestamp(),
            ],
        )?;
        
        let message = Message {
            id: message_id,
            session_id: session_id.to_string(),
            role: params.role,
            parts,
            model: Some(params.model),
            provider: Some(params.provider),
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        };
        
        self.publish(EventType::Created, message.clone());
        Ok(message)
    }

    async fn update(&self, message: &Message) -> Result<()> {
        let parts_json = Self::serialize_parts(&message.parts)?;
        
        let finished_at = message.parts.iter()
            .find_map(|p| match p {
                ContentPart::Finish { .. } => Some(chrono::Utc::now().timestamp()),
                _ => None
            });
        
        let conn = self.db.connection().await?;
        if let Some(finished_at) = finished_at {
            conn.lock().await.execute(
                "UPDATE messages SET parts = ?1, finished_at = ?2, updated_at = ?3 WHERE id = ?4",
                rusqlite::params![&parts_json, finished_at, chrono::Utc::now().timestamp(), &message.id],
            )?;
        } else {
            conn.lock().await.execute(
                "UPDATE messages SET parts = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![&parts_json, chrono::Utc::now().timestamp(), &message.id],
            )?;
        }
        
        self.publish(EventType::Updated, message.clone());
        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Message> {
        let conn = self.db.connection().await?;
        let conn = conn.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT id, session_id, role, parts, model, provider, created_at, updated_at 
             FROM messages WHERE id = ?1"
        )?;
        
        let message = stmt.query_row([id], |row| {
            let parts_json: String = row.get(3)?;
            let parts = Self::deserialize_parts(&parts_json)
                .unwrap_or_else(|_| vec![]);
            
            Ok(Message {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: MessageRole::from_str(&row.get::<_, String>(2)?),
                parts,
                model: row.get(4)?,
                provider: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        
        Ok(message)
    }

    async fn list(&self, session_id: &str) -> Result<Vec<Message>> {
        let conn = self.db.connection().await?;
        let conn = conn.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT id, session_id, role, parts, model, provider, created_at, updated_at 
             FROM messages WHERE session_id = ?1 ORDER BY created_at"
        )?;
        
        let messages = stmt.query_map([session_id], |row| {
            let parts_json: String = row.get(3)?;
            let parts = Self::deserialize_parts(&parts_json)
                .unwrap_or_else(|_| vec![]);
            
            Ok(Message {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: MessageRole::from_str(&row.get::<_, String>(2)?),
                parts,
                model: row.get(4)?,
                provider: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
        
        Ok(messages)
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let message = self.get(id).await?;
        
        let conn = self.db.connection().await?;
        conn.lock().await.execute(
            "DELETE FROM messages WHERE id = ?1",
            rusqlite::params![id],
        )?;
        
        self.publish(EventType::Deleted, message);
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