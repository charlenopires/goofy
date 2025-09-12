//! Database models matching Crush's schema

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// File record in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    pub id: String,
    pub session_id: String,
    pub path: String,
    pub content: String,
    pub version: i64,
    pub created_at: i64,  // Unix timestamp in milliseconds
    pub updated_at: i64,  // Unix timestamp in milliseconds
}

/// Message record in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub parts: String,  // JSON array stored as string
    pub model: Option<String>,
    pub created_at: i64,  // Unix timestamp in milliseconds
    pub updated_at: i64,  // Unix timestamp in milliseconds
    pub finished_at: Option<i64>,  // Unix timestamp in milliseconds
    pub provider: Option<String>,
}

/// Session record in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub parent_session_id: Option<String>,
    pub title: String,
    pub message_count: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cost: f64,
    pub updated_at: i64,  // Unix timestamp in milliseconds
    pub created_at: i64,  // Unix timestamp in milliseconds
    pub summary_message_id: Option<String>,
}

impl Session {
    /// Create a new session with defaults
    pub fn new(id: String, title: String) -> Self {
        let now = Utc::now().timestamp_millis();
        Self {
            id,
            parent_session_id: None,
            title,
            message_count: 0,
            prompt_tokens: 0,
            completion_tokens: 0,
            cost: 0.0,
            updated_at: now,
            created_at: now,
            summary_message_id: None,
        }
    }
    
    /// Create a child session
    pub fn new_child(id: String, parent_id: String, title: String) -> Self {
        let mut session = Self::new(id, title);
        session.parent_session_id = Some(parent_id);
        session
    }
}

impl Message {
    /// Create a new message
    pub fn new(id: String, session_id: String, role: String, parts: String) -> Self {
        let now = Utc::now().timestamp_millis();
        Self {
            id,
            session_id,
            role,
            parts,
            model: None,
            created_at: now,
            updated_at: now,
            finished_at: None,
            provider: None,
        }
    }
}

impl File {
    /// Create a new file record
    pub fn new(id: String, session_id: String, path: String, content: String) -> Self {
        let now = Utc::now().timestamp_millis();
        Self {
            id,
            session_id,
            path,
            content,
            version: 0,
            created_at: now,
            updated_at: now,
        }
    }
}