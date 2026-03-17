//! Database wrapper for Goofy
//!
//! This module provides a thread-safe database wrapper that integrates
//! with the existing database infrastructure.

use anyhow::Result;
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info};

use super::connect::{connect, DatabaseConfig};

/// Thread-safe database wrapper
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Create a new database connection
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config = DatabaseConfig {
            data_dir: path.as_ref().parent().unwrap_or(Path::new(".")).to_path_buf(),
            db_name: path.as_ref().file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("goofy.db")
                .to_string(),
        };
        
        let conn = connect(config)?;
        
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
    
    /// Get a reference to the connection (for compatibility)
    pub fn connection(&self) -> impl std::future::Future<Output = Result<Arc<Mutex<Connection>>>> {
        let conn = Arc::clone(&self.conn);
        async move { Ok(conn) }
    }
    
    /// Insert a session into the database
    pub async fn insert_session(
        &self,
        id: &str,
        title: &str,
        parent_session_id: Option<&str>,
        _metadata: Option<&serde_json::Value>,
    ) -> Result<()> {
        let conn = self.conn.lock().await;
        let now = chrono::Utc::now().timestamp_millis();
        
        conn.execute(
            "INSERT INTO sessions (
                id, title, parent_session_id, created_at, updated_at,
                message_count, prompt_tokens, completion_tokens, cost
            ) VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, 0, 0.0)",
            params![id, title, parent_session_id, now, now],
        )?;
        
        debug!("Inserted session: {}", id);
        Ok(())
    }
    
    /// Update a session in the database
    pub async fn update_session(
        &self,
        id: &str,
        title: Option<&str>,
        message_count: Option<i32>,
        prompt_tokens: Option<i32>,
        completion_tokens: Option<i32>,
        cost: Option<f64>,
        _metadata: Option<&serde_json::Value>,
    ) -> Result<()> {
        let conn = self.conn.lock().await;
        let now = chrono::Utc::now().timestamp_millis();
        
        // Update updated_at first
        conn.execute(
            "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        
        // Update optional fields
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
        
        if let Some(tokens) = prompt_tokens {
            conn.execute(
                "UPDATE sessions SET prompt_tokens = ?1 WHERE id = ?2",
                params![tokens, id],
            )?;
        }
        
        if let Some(tokens) = completion_tokens {
            conn.execute(
                "UPDATE sessions SET completion_tokens = ?1 WHERE id = ?2",
                params![tokens, id],
            )?;
        }
        
        if let Some(cost) = cost {
            conn.execute(
                "UPDATE sessions SET cost = ?1 WHERE id = ?2",
                params![cost, id],
            )?;
        }
        
        debug!("Updated session: {}", id);
        Ok(())
    }
    
    /// Get a session from the database
    pub async fn get_session(&self, id: &str) -> Result<Option<SessionRow>> {
        let conn = self.conn.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT id, title, parent_session_id, created_at, updated_at,
                    message_count, prompt_tokens, completion_tokens, cost
             FROM sessions WHERE id = ?1"
        )?;
        
        let session = stmt.query_row([id], |row| {
            Ok(SessionRow {
                id: row.get(0)?,
                title: row.get(1)?,
                parent_session_id: row.get(2)?,
                created_at: chrono::DateTime::from_timestamp_millis(row.get(3)?)
                    .unwrap_or_else(chrono::Utc::now),
                updated_at: chrono::DateTime::from_timestamp_millis(row.get(4)?)
                    .unwrap_or_else(chrono::Utc::now),
                message_count: row.get(5)?,
                total_input_tokens: row.get(6)?,
                total_output_tokens: row.get(7)?,
                total_cost: row.get(8)?,
            })
        }).ok();
        
        Ok(session)
    }
    
    /// List all sessions
    pub async fn list_sessions(&self, limit: Option<i32>) -> Result<Vec<SessionRow>> {
        let conn = self.conn.lock().await;
        
        let query = if let Some(limit) = limit {
            format!(
                "SELECT id, title, parent_session_id, created_at, updated_at,
                        message_count, prompt_tokens, completion_tokens, cost
                 FROM sessions ORDER BY updated_at DESC LIMIT {}",
                limit
            )
        } else {
            "SELECT id, title, parent_session_id, created_at, updated_at,
                    message_count, prompt_tokens, completion_tokens, cost
             FROM sessions ORDER BY updated_at DESC".to_string()
        };
        
        let mut stmt = conn.prepare(&query)?;
        let sessions = stmt.query_map([], |row| {
            Ok(SessionRow {
                id: row.get(0)?,
                title: row.get(1)?,
                parent_session_id: row.get(2)?,
                created_at: chrono::DateTime::from_timestamp_millis(row.get(3)?)
                    .unwrap_or_else(chrono::Utc::now),
                updated_at: chrono::DateTime::from_timestamp_millis(row.get(4)?)
                    .unwrap_or_else(chrono::Utc::now),
                message_count: row.get(5)?,
                total_input_tokens: row.get(6)?,
                total_output_tokens: row.get(7)?,
                total_cost: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
        
        Ok(sessions)
    }
    
    /// Delete a session
    pub async fn delete_session(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM sessions WHERE id = ?1", [id])?;
        debug!("Deleted session: {}", id);
        Ok(())
    }
}

/// Session row from database
#[derive(Debug, Clone)]
pub struct SessionRow {
    pub id: String,
    pub title: String,
    pub parent_session_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub message_count: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cost: f64,
}