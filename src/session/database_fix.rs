//! Database fixes for FOREIGN KEY constraints and non-interactive mode
//! 
//! This module provides improved database handling with proper constraint
//! management and support for different execution modes.

use anyhow::Result;
use rusqlite::{Connection, params, OptionalExtension};
use std::path::Path;
use chrono::{DateTime, Utc};
use serde_json;
use tracing::{debug, error, warn};

/// Enhanced database manager with foreign key support
pub struct DatabaseManager {
    conn: Connection,
    foreign_keys_enabled: bool,
}

impl DatabaseManager {
    /// Create a new database connection with proper constraint handling
    pub async fn new<P: AsRef<Path>>(db_path: P, enable_foreign_keys: bool) -> Result<Self> {
        let mut conn = Connection::open(db_path)?;
        
        // Configure foreign key constraints
        if enable_foreign_keys {
            conn.execute("PRAGMA foreign_keys = ON", [])?;
            debug!("Foreign key constraints enabled");
        } else {
            conn.execute("PRAGMA foreign_keys = OFF", [])?;
            debug!("Foreign key constraints disabled for compatibility");
        }
        
        // Set other performance pragmas (use execute_batch since some return results)
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA cache_size = 1000;
             PRAGMA temp_store = memory;"
        )?;
        
        let mut db = Self { 
            conn,
            foreign_keys_enabled: enable_foreign_keys,
        };
        
        db.create_tables().await?;
        
        Ok(db)
    }
    
    /// Create database tables with proper constraint handling
    async fn create_tables(&mut self) -> Result<()> {
        // Create sessions table first (parent table)
        self.conn.execute(
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
                metadata TEXT,
                FOREIGN KEY (parent_session_id) REFERENCES sessions (id) ON DELETE SET NULL
            )",
            [],
        )?;
        
        // Create messages table with proper foreign key handling
        if self.foreign_keys_enabled {
            self.conn.execute(
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
        } else {
            // Create without foreign key constraint for compatibility
            self.conn.execute(
                "CREATE TABLE IF NOT EXISTS messages (
                    id TEXT PRIMARY KEY,
                    session_id TEXT NOT NULL,
                    role TEXT NOT NULL,
                    content TEXT NOT NULL,
                    timestamp TEXT NOT NULL,
                    metadata TEXT
                )",
                [],
            )?;
        }
        
        // Create indexes for performance
        self.create_indexes().await?;
        
        debug!("Database tables created successfully");
        Ok(())
    }
    
    /// Create database indexes
    async fn create_indexes(&self) -> Result<()> {
        let indexes = [
            "CREATE INDEX IF NOT EXISTS idx_messages_session_id ON messages (session_id)",
            "CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages (timestamp)",
            "CREATE INDEX IF NOT EXISTS idx_messages_role ON messages (role)",
            "CREATE INDEX IF NOT EXISTS idx_sessions_created_at ON sessions (created_at)",
            "CREATE INDEX IF NOT EXISTS idx_sessions_updated_at ON sessions (updated_at)",
            "CREATE INDEX IF NOT EXISTS idx_sessions_parent ON sessions (parent_session_id)",
        ];
        
        for index_sql in &indexes {
            self.conn.execute(index_sql, [])?;
        }
        
        debug!("Database indexes created successfully");
        Ok(())
    }
    
    /// Safely insert a session, ensuring no foreign key conflicts
    pub async fn insert_session_safe(
        &mut self,
        id: &str,
        title: &str,
        parent_session_id: Option<&str>,
        metadata: Option<&serde_json::Value>,
    ) -> Result<()> {
        // Validate parent session exists if specified
        if let Some(parent_id) = parent_session_id {
            if self.foreign_keys_enabled {
                let parent_exists = self.session_exists(parent_id).await?;
                if !parent_exists {
                    warn!("Parent session {} does not exist, setting to None", parent_id);
                    return self.insert_session_safe(id, title, None, metadata).await;
                }
            }
        }
        
        let now = Utc::now().to_rfc3339();
        let metadata_str = metadata.map(|m| serde_json::to_string(m)).transpose()?;
        
        self.conn.execute(
            "INSERT OR REPLACE INTO sessions (
                id, title, parent_session_id, created_at, updated_at, metadata
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, title, parent_session_id, now, now, metadata_str],
        )?;
        
        debug!("Session {} inserted successfully", id);
        Ok(())
    }
    
    /// Safely insert a message, ensuring session exists
    pub async fn insert_message_safe(
        &mut self,
        id: &str,
        session_id: &str,
        role: &str,
        content: &str,
        metadata: Option<&serde_json::Value>,
    ) -> Result<()> {
        // Ensure session exists
        if self.foreign_keys_enabled {
            let session_exists = self.session_exists(session_id).await?;
            if !session_exists {
                // Create a default session if it doesn't exist
                self.insert_session_safe(
                    session_id,
                    "Auto-created Session",
                    None,
                    None,
                ).await?;
            }
        }
        
        let now = Utc::now().to_rfc3339();
        let metadata_str = metadata.map(|m| serde_json::to_string(m)).transpose()?;
        
        self.conn.execute(
            "INSERT INTO messages (
                id, session_id, role, content, timestamp, metadata
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, session_id, role, content, now, metadata_str],
        )?;
        
        debug!("Message {} inserted successfully for session {}", id, session_id);
        Ok(())
    }
    
    /// Check if a session exists
    pub async fn session_exists(&self, id: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE id = ?1",
            [id],
            |row| row.get(0),
        )?;
        
        Ok(count > 0)
    }
    
    /// Get or create a session
    pub async fn get_or_create_session(
        &mut self,
        id: &str,
        title: Option<&str>,
    ) -> Result<SessionInfo> {
        if let Some(session) = self.get_session(id).await? {
            return Ok(session);
        }
        
        // Create new session
        let session_title = title.unwrap_or("New Session");
        self.insert_session_safe(id, session_title, None, None).await?;
        
        // Return the created session
        self.get_session(id).await?
            .ok_or_else(|| anyhow::anyhow!("Failed to create session"))
    }
    
    /// Get session information
    pub async fn get_session(&self, id: &str) -> Result<Option<SessionInfo>> {
        let session: Option<SessionInfo> = self.conn.query_row(
            "SELECT id, title, parent_session_id, created_at, updated_at, 
                    message_count, total_input_tokens, total_output_tokens, 
                    total_cost, metadata
             FROM sessions WHERE id = ?1",
            [id],
            |row| {
                let metadata_str: Option<String> = row.get(9)?;
                let metadata = metadata_str
                    .map(|s| serde_json::from_str(&s))
                    .transpose()
                    .unwrap_or(None);
                
                Ok(SessionInfo {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    parent_session_id: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                    message_count: row.get(5)?,
                    total_input_tokens: row.get(6)?,
                    total_output_tokens: row.get(7)?,
                    total_cost: row.get(8)?,
                    metadata,
                })
            },
        ).optional()?;
        
        Ok(session)
    }
    
    /// Get messages for a session
    pub async fn get_session_messages(&self, session_id: &str) -> Result<Vec<MessageInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, role, content, timestamp, metadata
             FROM messages 
             WHERE session_id = ?1 
             ORDER BY timestamp ASC"
        )?;
        
        let message_iter = stmt.query_map([session_id], |row| {
            let metadata_str: Option<String> = row.get(5)?;
            let metadata = metadata_str
                .map(|s| serde_json::from_str(&s))
                .transpose()
                .unwrap_or(None);
            
            Ok(MessageInfo {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                timestamp: row.get(4)?,
                metadata,
            })
        })?;
        
        let mut messages = Vec::new();
        for message in message_iter {
            messages.push(message?);
        }
        
        Ok(messages)
    }
    
    /// Clean orphaned messages (if foreign keys are disabled)
    pub async fn clean_orphaned_messages(&mut self) -> Result<usize> {
        if self.foreign_keys_enabled {
            // No need to clean if foreign keys are properly enforced
            return Ok(0);
        }
        
        let deleted = self.conn.execute(
            "DELETE FROM messages 
             WHERE session_id NOT IN (SELECT id FROM sessions)",
            [],
        )?;
        
        if deleted > 0 {
            warn!("Cleaned {} orphaned messages", deleted);
        }
        
        Ok(deleted)
    }
    
    /// Vacuum the database for optimal performance
    pub async fn vacuum(&mut self) -> Result<()> {
        self.conn.execute("VACUUM", [])?;
        debug!("Database vacuumed successfully");
        Ok(())
    }
    
    /// Get database statistics
    pub async fn get_stats(&self) -> Result<DatabaseStats> {
        let session_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM sessions",
            [],
            |row| row.get(0),
        )?;
        
        let message_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM messages",
            [],
            |row| row.get(0),
        )?;
        
        let db_size: i64 = self.conn.query_row(
            "SELECT page_count * page_size as size FROM pragma_page_count(), pragma_page_size()",
            [],
            |row| row.get(0),
        ).unwrap_or(0);
        
        Ok(DatabaseStats {
            session_count: session_count as usize,
            message_count: message_count as usize,
            database_size_bytes: db_size as usize,
            foreign_keys_enabled: self.foreign_keys_enabled,
        })
    }
    
    /// Enable or disable foreign key constraints (requires reconnection)
    pub async fn set_foreign_keys(&mut self, enabled: bool) -> Result<()> {
        if enabled != self.foreign_keys_enabled {
            if enabled {
                self.conn.execute("PRAGMA foreign_keys = ON", [])?;
            } else {
                self.conn.execute("PRAGMA foreign_keys = OFF", [])?;
            }
            self.foreign_keys_enabled = enabled;
            debug!("Foreign keys {}", if enabled { "enabled" } else { "disabled" });
        }
        Ok(())
    }
}

/// Session information structure
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: String,
    pub title: String,
    pub parent_session_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: i32,
    pub total_input_tokens: i32,
    pub total_output_tokens: i32,
    pub total_cost: f64,
    pub metadata: Option<serde_json::Value>,
}

/// Message information structure
#[derive(Debug, Clone)]
pub struct MessageInfo {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub timestamp: String,
    pub metadata: Option<serde_json::Value>,
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    pub session_count: usize,
    pub message_count: usize,
    pub database_size_bytes: usize,
    pub foreign_keys_enabled: bool,
}

/// Database migration utilities
pub struct DatabaseMigrator {
    conn: Connection,
}

impl DatabaseMigrator {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }
    
    /// Run database migrations
    pub async fn migrate(&mut self) -> Result<()> {
        let current_version = self.get_schema_version().await?;
        debug!("Current database schema version: {}", current_version);
        
        // Add future migrations here
        match current_version {
            0 => {
                self.migrate_to_v1().await?;
                self.set_schema_version(1).await?;
            }
            _ => {
                debug!("Database schema is up to date");
            }
        }
        
        Ok(())
    }
    
    async fn get_schema_version(&self) -> Result<i32> {
        // Check if version table exists
        let table_exists: bool = self.conn.query_row(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='schema_version'",
            [],
            |_| Ok(true),
        ).unwrap_or(false);
        
        if !table_exists {
            // Create version table
            self.conn.execute(
                "CREATE TABLE schema_version (version INTEGER PRIMARY KEY)",
                [],
            )?;
            self.conn.execute(
                "INSERT INTO schema_version (version) VALUES (0)",
                [],
            )?;
            return Ok(0);
        }
        
        let version: i32 = self.conn.query_row(
            "SELECT version FROM schema_version LIMIT 1",
            [],
            |row| row.get(0),
        ).unwrap_or(0);
        
        Ok(version)
    }
    
    async fn set_schema_version(&self, version: i32) -> Result<()> {
        self.conn.execute(
            "UPDATE schema_version SET version = ?1",
            [version],
        )?;
        Ok(())
    }
    
    async fn migrate_to_v1(&self) -> Result<()> {
        // Example migration: add indexes if they don't exist
        debug!("Running migration to version 1");
        // Migration logic would go here
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_database_with_foreign_keys() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let mut db = DatabaseManager::new(&db_path, true).await.unwrap();
        
        // Test session creation
        db.insert_session_safe("test-session", "Test Session", None, None)
            .await.unwrap();
        
        // Test message creation
        db.insert_message_safe(
            "test-message",
            "test-session",
            "user",
            "Hello world",
            None,
        ).await.unwrap();
        
        let messages = db.get_session_messages("test-session").await.unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Hello world");
    }
    
    #[tokio::test]
    async fn test_database_without_foreign_keys() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let mut db = DatabaseManager::new(&db_path, false).await.unwrap();
        
        // Test message creation without session (should auto-create)
        db.insert_message_safe(
            "test-message",
            "nonexistent-session",
            "user",
            "Hello world",
            None,
        ).await.unwrap();
        
        let session = db.get_session("nonexistent-session").await.unwrap();
        assert!(session.is_some());
        assert_eq!(session.unwrap().title, "Auto-created Session");
    }
    
    #[tokio::test]
    async fn test_orphaned_message_cleanup() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let mut db = DatabaseManager::new(&db_path, false).await.unwrap();
        
        // Manually insert orphaned message
        db.conn.execute(
            "INSERT INTO messages (id, session_id, role, content, timestamp) 
             VALUES ('orphan', 'nonexistent', 'user', 'orphaned', datetime('now'))",
            [],
        ).unwrap();
        
        let cleaned = db.clean_orphaned_messages().await.unwrap();
        assert_eq!(cleaned, 1);
    }
    
    #[tokio::test]
    async fn test_database_stats() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let mut db = DatabaseManager::new(&db_path, true).await.unwrap();
        
        db.insert_session_safe("test-session", "Test Session", None, None)
            .await.unwrap();
        
        let stats = db.get_stats().await.unwrap();
        assert_eq!(stats.session_count, 1);
        assert!(stats.foreign_keys_enabled);
    }
}