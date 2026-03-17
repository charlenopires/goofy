//! Database layer for session persistence

use anyhow::Result;
use rusqlite::{Connection, params, Row};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::{DateTime, Utc};
use serde_json;

use crate::llm::{Message};
// use super::queries::{SessionQueries, MessageQueries}; // Complex type system needs reconciliation

/// Database manager for session persistence
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Create a new database connection
    pub async fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        let db = Self { conn: Arc::new(Mutex::new(conn)) };
        db.create_tables().await?;

        Ok(db)
    }

    // Note: Type-safe queries temporarily disabled until type system is reconciled
    // pub fn sessions(&self) -> SessionQueries<'_> {
    //     SessionQueries::new(&self.conn)
    // }

    // pub fn messages(&self) -> MessageQueries<'_> {
    //     MessageQueries::new(&self.conn)
    // }
    
    /// Create the necessary database tables
    async fn create_tables(&self) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                parent_session_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                message_count INTEGER DEFAULT 0,
                total_input_tokens INTEGER DEFAULT 0,
                total_output_tokens INTEGER DEFAULT 0,
                total_cost REAL DEFAULT 0.0,
                metadata TEXT
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                metadata TEXT,
                FOREIGN KEY (session_id) REFERENCES sessions (id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_messages_session_id ON messages (session_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages (timestamp)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_created_at ON sessions (created_at)",
            [],
        )?;

        Ok(())
    }
    
    /// Insert a new session
    pub async fn insert_session(
        &self,
        id: &str,
        title: &str,
        parent_session_id: Option<&str>,
        metadata: Option<&serde_json::Value>,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let metadata_str = metadata.map(|m| serde_json::to_string(m)).transpose()?;

        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO sessions (
                id, title, parent_session_id, created_at, updated_at, metadata
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, title, parent_session_id, now, now, metadata_str],
        )?;

        Ok(())
    }
    
    /// Update a session
    pub async fn update_session(
        &self,
        id: &str,
        title: Option<&str>,
        message_count: Option<i32>,
        total_input_tokens: Option<i32>,
        total_output_tokens: Option<i32>,
        total_cost: Option<f64>,
        metadata: Option<&serde_json::Value>,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let metadata_str = metadata.map(|m| serde_json::to_string(m)).transpose()?;

        let conn = self.conn.lock().await;
        conn.execute(
            "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;

        if let Some(title) = title {
            conn.execute(
                "UPDATE sessions SET title = ?1 WHERE id = ?2",
                params![title, id],
            )?;
        }

        if let Some(count) = message_count {
            conn.execute(
                "UPDATE sessions SET message_count = ?1 WHERE id = ?2",
                params![count, id],
            )?;
        }

        if let Some(input_tokens) = total_input_tokens {
            conn.execute(
                "UPDATE sessions SET total_input_tokens = ?1 WHERE id = ?2",
                params![input_tokens, id],
            )?;
        }

        if let Some(output_tokens) = total_output_tokens {
            conn.execute(
                "UPDATE sessions SET total_output_tokens = ?1 WHERE id = ?2",
                params![output_tokens, id],
            )?;
        }

        if let Some(cost) = total_cost {
            conn.execute(
                "UPDATE sessions SET total_cost = ?1 WHERE id = ?2",
                params![cost, id],
            )?;
        }

        if let Some(metadata_str) = metadata_str {
            conn.execute(
                "UPDATE sessions SET metadata = ?1 WHERE id = ?2",
                params![metadata_str, id],
            )?;
        }

        Ok(())
    }
    
    /// Get a session by ID
    pub async fn get_session(&self, id: &str) -> Result<Option<SessionRow>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, title, parent_session_id, created_at, updated_at,
                    message_count, total_input_tokens, total_output_tokens,
                    total_cost, metadata
             FROM sessions WHERE id = ?1"
        )?;

        let session_iter = stmt.query_map([id], |row| {
            Ok(SessionRow::from_row(row)?)
        })?;

        for session in session_iter {
            return Ok(Some(session?));
        }

        Ok(None)
    }
    
    /// List all sessions
    pub async fn list_sessions(&self, limit: Option<i32>) -> Result<Vec<SessionRow>> {
        let query = if let Some(limit) = limit {
            format!(
                "SELECT id, title, parent_session_id, created_at, updated_at,
                        message_count, total_input_tokens, total_output_tokens,
                        total_cost, metadata
                 FROM sessions ORDER BY updated_at DESC LIMIT {}",
                limit
            )
        } else {
            "SELECT id, title, parent_session_id, created_at, updated_at,
                    message_count, total_input_tokens, total_output_tokens,
                    total_cost, metadata
             FROM sessions ORDER BY updated_at DESC".to_string()
        };
        
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(&query)?;
        let session_iter = stmt.query_map([], |row| {
            Ok(SessionRow::from_row(row)?)
        })?;
        
        let mut sessions = Vec::new();
        for session in session_iter {
            sessions.push(session?);
        }
        
        Ok(sessions)
    }
    
    /// Delete a session
    pub async fn delete_session(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM sessions WHERE id = ?1", [id])?;
        Ok(())
    }
    
    /// Insert a message
    pub async fn insert_message(&self, message: &Message, session_id: &str) -> Result<()> {
        let content_str = serde_json::to_string(&message.content)?;
        let metadata_str = if message.metadata.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&message.metadata)?)
        };
        
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO messages (id, session_id, role, content, timestamp, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                message.id,
                session_id,
                serde_json::to_string(&message.role)?,
                content_str,
                message.timestamp.to_rfc3339(),
                metadata_str
            ],
        )?;
        
        Ok(())
    }
    
    /// Get messages for a session
    pub async fn get_messages(&self, session_id: &str, limit: Option<i32>) -> Result<Vec<Message>> {
        let query = if let Some(limit) = limit {
            format!(
                "SELECT id, role, content, timestamp, metadata
                 FROM messages WHERE session_id = ?1 
                 ORDER BY timestamp ASC LIMIT {}",
                limit
            )
        } else {
            "SELECT id, role, content, timestamp, metadata
             FROM messages WHERE session_id = ?1 
             ORDER BY timestamp ASC".to_string()
        };
        
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(&query)?;
        let message_iter = stmt.query_map([session_id], |row| {
            let id: String = row.get(0)?;
            let role_str: String = row.get(1)?;
            let content_str: String = row.get(2)?;
            let timestamp_str: String = row.get(3)?;
            let metadata_str: Option<String> = row.get(4)?;
            
            let role = serde_json::from_str(&role_str)
                .map_err(|e| rusqlite::Error::InvalidColumnType(0, "role".to_string(), rusqlite::types::Type::Text))?;
            let content = serde_json::from_str(&content_str)
                .map_err(|e| rusqlite::Error::InvalidColumnType(0, "content".to_string(), rusqlite::types::Type::Text))?;
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map_err(|e| rusqlite::Error::InvalidColumnType(0, "timestamp".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);
            let metadata = if let Some(metadata_str) = metadata_str {
                serde_json::from_str(&metadata_str)
                    .map_err(|e| rusqlite::Error::InvalidColumnType(0, "metadata".to_string(), rusqlite::types::Type::Text))?
            } else {
                std::collections::HashMap::new()
            };
            
            Ok(Message {
                id,
                role,
                content,
                timestamp,
                metadata,
            })
        })?;
        
        let mut messages = Vec::new();
        for message in message_iter {
            messages.push(message?);
        }
        
        Ok(messages)
    }
    
    /// Delete messages for a session
    pub async fn delete_messages(&self, session_id: &str) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM messages WHERE session_id = ?1", [session_id])?;
        Ok(())
    }
    
    /// Get message count for a session
    pub async fn get_message_count(&self, session_id: &str) -> Result<i32> {
        let conn = self.conn.lock().await;
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM messages WHERE session_id = ?1",
            [session_id],
            |row| row.get(0),
        )?;
        
        Ok(count)
    }
}

/// Database row representation of a session
#[derive(Debug, Clone)]
pub struct SessionRow {
    pub id: String,
    pub title: String,
    pub parent_session_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: i32,
    pub total_input_tokens: i32,
    pub total_output_tokens: i32,
    pub total_cost: f64,
    pub metadata: Option<serde_json::Value>,
}

impl SessionRow {
    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let created_at_str: String = row.get(3)?;
        let updated_at_str: String = row.get(4)?;
        let metadata_str: Option<String> = row.get(9)?;
        
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(3, "created_at".to_string(), rusqlite::types::Type::Text))?
            .with_timezone(&Utc);
        
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(4, "updated_at".to_string(), rusqlite::types::Type::Text))?
            .with_timezone(&Utc);
        
        let metadata = if let Some(metadata_str) = metadata_str {
            Some(serde_json::from_str(&metadata_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(9, "metadata".to_string(), rusqlite::types::Type::Text))?)
        } else {
            None
        };
        
        Ok(SessionRow {
            id: row.get(0)?,
            title: row.get(1)?,
            parent_session_id: row.get(2)?,
            created_at,
            updated_at,
            message_count: row.get(5)?,
            total_input_tokens: row.get(6)?,
            total_output_tokens: row.get(7)?,
            total_cost: row.get(8)?,
            metadata,
        })
    }
}