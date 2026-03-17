//! File history and versioning system
//!
//! This module manages file versions and history for sessions,
//! enabling tracking changes and reverting to previous versions.

use anyhow::{Result, Context};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Mutex};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::db::Database;
use crate::session::{Event, EventType};

pub const INITIAL_VERSION: i64 = 0;

/// File with version history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    pub id: String,
    pub session_id: String,
    pub path: String,
    pub content: String,
    pub version: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Parameters for creating a file
#[derive(Debug, Clone)]
pub struct CreateFileParams {
    pub path: String,
    pub content: String,
}

/// File history service interface
#[async_trait]
pub trait FileHistoryService: Send + Sync {
    /// Subscribe to file events
    fn subscribe(&self) -> mpsc::UnboundedReceiver<Event<File>>;
    
    /// Create a new file with initial version
    async fn create(&self, session_id: &str, path: &str, content: &str) -> Result<File>;
    
    /// Create a new version of an existing file
    async fn create_version(&self, session_id: &str, path: &str, content: &str) -> Result<File>;
    
    /// Get a file by ID
    async fn get(&self, id: &str) -> Result<File>;
    
    /// Get a file by path and session
    async fn get_by_path_and_session(&self, path: &str, session_id: &str) -> Result<File>;
    
    /// List all files for a session
    async fn list_by_session(&self, session_id: &str) -> Result<Vec<File>>;
    
    /// List latest version of each file for a session
    async fn list_latest_session_files(&self, session_id: &str) -> Result<Vec<File>>;
    
    /// Delete a file
    async fn delete(&self, id: &str) -> Result<()>;
    
    /// Delete all files for a session
    async fn delete_session_files(&self, session_id: &str) -> Result<()>;
}

/// File history service implementation
pub struct FileHistoryServiceImpl {
    db: Arc<Database>,
    sender: mpsc::UnboundedSender<Event<File>>,
    receiver: Arc<Mutex<Option<mpsc::UnboundedReceiver<Event<File>>>>>,
}

impl FileHistoryServiceImpl {
    /// Create a new file history service
    pub fn new(db: Arc<Database>) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            db,
            sender,
            receiver: Arc::new(Mutex::new(Some(receiver))),
        }
    }
    
    /// Publish an event
    fn publish(&self, event_type: EventType, file: File) {
        let event = Event {
            event_type,
            payload: file,
        };
        let _ = self.sender.send(event);
    }
    
    /// Create a file with a specific version
    async fn create_with_version(
        &self,
        session_id: &str,
        path: &str,
        content: &str,
        version: i64,
    ) -> Result<File> {
        const MAX_RETRIES: usize = 3;
        
        for attempt in 0..MAX_RETRIES {
            let file_id = Uuid::new_v4().to_string();
            let now = Utc::now().timestamp();
            
            // Try to insert the file
            let conn = self.db.connection().await?;
            let result = conn.lock().await.execute(
                "INSERT INTO files (id, session_id, path, content, version, created_at, updated_at) 
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    &file_id,
                    session_id,
                    path,
                    content,
                    version,
                    now,
                    now,
                ],
            );
            
            match result {
                Ok(_) => {
                    let file = File {
                        id: file_id,
                        session_id: session_id.to_string(),
                        path: path.to_string(),
                        content: content.to_string(),
                        version,
                        created_at: now,
                        updated_at: now,
                    };
                    
                    self.publish(EventType::Created, file.clone());
                    return Ok(file);
                }
                Err(e) if e.to_string().contains("UNIQUE constraint failed") => {
                    // Version conflict, try next version if we have retries left
                    if attempt < MAX_RETRIES - 1 {
                        continue;
                    }
                    return Err(anyhow::anyhow!("Failed to create file version after {} attempts", MAX_RETRIES));
                }
                Err(e) => return Err(e.into()),
            }
        }
        
        Err(anyhow::anyhow!("Failed to create file version"))
    }
}

#[async_trait]
impl FileHistoryService for FileHistoryServiceImpl {
    fn subscribe(&self) -> mpsc::UnboundedReceiver<Event<File>> {
        let (tx, rx) = mpsc::unbounded_channel();
        // Clone sender for forwarding events
        let sender = self.sender.clone();
        tokio::spawn(async move {
            // In a real implementation, forward events from the main channel
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });
        rx
    }
    
    async fn create(&self, session_id: &str, path: &str, content: &str) -> Result<File> {
        self.create_with_version(session_id, path, content, INITIAL_VERSION).await
    }
    
    async fn create_version(&self, session_id: &str, path: &str, content: &str) -> Result<File> {
        // Get the latest version for this path
        let next_version = {
            let conn = self.db.connection().await?;
            let conn = conn.lock().await;
            
            let mut stmt = conn.prepare(
                "SELECT version FROM files 
                 WHERE path = ?1 AND session_id = ?2 
                 ORDER BY version DESC, created_at DESC 
                 LIMIT 1"
            )?;
            
            let latest_version: Option<i64> = stmt.query_row(
                rusqlite::params![path, session_id],
                |row| row.get(0),
            ).ok();
            
            latest_version.unwrap_or(INITIAL_VERSION) + 1
        }; // conn is dropped here
        
        self.create_with_version(session_id, path, content, next_version).await
    }
    
    async fn get(&self, id: &str) -> Result<File> {
        let conn = self.db.connection().await?;
        let conn = conn.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT id, session_id, path, content, version, created_at, updated_at 
             FROM files WHERE id = ?1"
        )?;
        
        let file = stmt.query_row([id], |row| {
            Ok(File {
                id: row.get(0)?,
                session_id: row.get(1)?,
                path: row.get(2)?,
                content: row.get(3)?,
                version: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        
        Ok(file)
    }
    
    async fn get_by_path_and_session(&self, path: &str, session_id: &str) -> Result<File> {
        let conn = self.db.connection().await?;
        let conn = conn.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT id, session_id, path, content, version, created_at, updated_at 
             FROM files 
             WHERE path = ?1 AND session_id = ?2 
             ORDER BY version DESC, created_at DESC 
             LIMIT 1"
        )?;
        
        let file = stmt.query_row(rusqlite::params![path, session_id], |row| {
            Ok(File {
                id: row.get(0)?,
                session_id: row.get(1)?,
                path: row.get(2)?,
                content: row.get(3)?,
                version: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        
        Ok(file)
    }
    
    async fn list_by_session(&self, session_id: &str) -> Result<Vec<File>> {
        let conn = self.db.connection().await?;
        let conn = conn.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT id, session_id, path, content, version, created_at, updated_at 
             FROM files 
             WHERE session_id = ?1 
             ORDER BY path, version DESC"
        )?;
        
        let files = stmt.query_map([session_id], |row| {
            Ok(File {
                id: row.get(0)?,
                session_id: row.get(1)?,
                path: row.get(2)?,
                content: row.get(3)?,
                version: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
        
        Ok(files)
    }
    
    async fn list_latest_session_files(&self, session_id: &str) -> Result<Vec<File>> {
        let conn = self.db.connection().await?;
        let conn = conn.lock().await;
        
        // Get the latest version of each file
        let mut stmt = conn.prepare(
            "SELECT f1.id, f1.session_id, f1.path, f1.content, f1.version, f1.created_at, f1.updated_at 
             FROM files f1
             INNER JOIN (
                 SELECT path, MAX(version) as max_version
                 FROM files
                 WHERE session_id = ?1
                 GROUP BY path
             ) f2 ON f1.path = f2.path AND f1.version = f2.max_version
             WHERE f1.session_id = ?1
             ORDER BY f1.path"
        )?;
        
        let files = stmt.query_map([session_id], |row| {
            Ok(File {
                id: row.get(0)?,
                session_id: row.get(1)?,
                path: row.get(2)?,
                content: row.get(3)?,
                version: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
        
        Ok(files)
    }
    
    async fn delete(&self, id: &str) -> Result<()> {
        let file = self.get(id).await?;
        
        let conn = self.db.connection().await?;
        conn.lock().await.execute(
            "DELETE FROM files WHERE id = ?1",
            rusqlite::params![id],
        )?;
        
        self.publish(EventType::Deleted, file);
        Ok(())
    }
    
    async fn delete_session_files(&self, session_id: &str) -> Result<()> {
        let files = self.list_by_session(session_id).await?;
        
        for file in files {
            self.delete(&file.id).await?;
        }
        
        Ok(())
    }
}

/// Create a new file history service
pub fn new_service(db: Arc<Database>) -> Arc<dyn FileHistoryService> {
    Arc::new(FileHistoryServiceImpl::new(db))
}