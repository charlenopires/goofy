//! Database queries and operations

use anyhow::{Result, Context};
use rusqlite::{Connection, Transaction, params, Row, OptionalExtension};
use uuid::Uuid;
use crate::db::models::{Session, Message, File};

/// Database operations trait
pub trait DatabaseOperations {
    // Session operations
    fn create_session(&self, session: &Session) -> Result<()>;
    fn get_session(&self, id: &str) -> Result<Option<Session>>;
    fn list_sessions(&self, limit: Option<usize>) -> Result<Vec<Session>>;
    fn update_session(&self, session: &Session) -> Result<()>;
    fn delete_session(&self, id: &str) -> Result<()>;
    
    // Message operations
    fn create_message(&self, message: &Message) -> Result<()>;
    fn get_message(&self, id: &str) -> Result<Option<Message>>;
    fn list_messages_by_session(&self, session_id: &str) -> Result<Vec<Message>>;
    fn update_message(&self, message: &Message) -> Result<()>;
    fn delete_message(&self, id: &str) -> Result<()>;
    
    // File operations
    fn create_file(&self, file: &File) -> Result<()>;
    fn get_file(&self, id: &str) -> Result<Option<File>>;
    fn get_file_by_path_and_session(&self, path: &str, session_id: &str) -> Result<Option<File>>;
    fn list_files_by_session(&self, session_id: &str) -> Result<Vec<File>>;
    fn list_files_by_path(&self, path: &str) -> Result<Vec<File>>;
    fn delete_file(&self, id: &str) -> Result<()>;
}

/// Queries struct for database operations
pub struct Queries<'a> {
    conn: &'a Connection,
}

impl<'a> Queries<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
}

impl<'a> DatabaseOperations for Queries<'a> {
    // Session operations
    fn create_session(&self, session: &Session) -> Result<()> {
        self.conn.execute(
            "INSERT INTO sessions (
                id, parent_session_id, title, message_count, 
                prompt_tokens, completion_tokens, cost, 
                updated_at, created_at, summary_message_id
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                session.id,
                session.parent_session_id,
                session.title,
                session.message_count,
                session.prompt_tokens,
                session.completion_tokens,
                session.cost,
                session.updated_at,
                session.created_at,
                session.summary_message_id,
            ],
        )?;
        Ok(())
    }
    
    fn get_session(&self, id: &str) -> Result<Option<Session>> {
        self.conn
            .query_row(
                "SELECT id, parent_session_id, title, message_count,
                        prompt_tokens, completion_tokens, cost,
                        updated_at, created_at, summary_message_id
                 FROM sessions WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Session {
                        id: row.get(0)?,
                        parent_session_id: row.get(1)?,
                        title: row.get(2)?,
                        message_count: row.get(3)?,
                        prompt_tokens: row.get(4)?,
                        completion_tokens: row.get(5)?,
                        cost: row.get(6)?,
                        updated_at: row.get(7)?,
                        created_at: row.get(8)?,
                        summary_message_id: row.get(9)?,
                    })
                },
            )
            .optional()
            .context("Failed to get session")
    }
    
    fn list_sessions(&self, limit: Option<usize>) -> Result<Vec<Session>> {
        let query = if let Some(limit) = limit {
            format!("SELECT id, parent_session_id, title, message_count,
                           prompt_tokens, completion_tokens, cost,
                           updated_at, created_at, summary_message_id
                    FROM sessions 
                    ORDER BY created_at DESC 
                    LIMIT {}", limit)
        } else {
            "SELECT id, parent_session_id, title, message_count,
                    prompt_tokens, completion_tokens, cost,
                    updated_at, created_at, summary_message_id
             FROM sessions 
             ORDER BY created_at DESC".to_string()
        };
        
        let mut stmt = self.conn.prepare(&query)?;
        let sessions = stmt
            .query_map(params![], |row| {
                Ok(Session {
                    id: row.get(0)?,
                    parent_session_id: row.get(1)?,
                    title: row.get(2)?,
                    message_count: row.get(3)?,
                    prompt_tokens: row.get(4)?,
                    completion_tokens: row.get(5)?,
                    cost: row.get(6)?,
                    updated_at: row.get(7)?,
                    created_at: row.get(8)?,
                    summary_message_id: row.get(9)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        
        Ok(sessions)
    }
    
    fn update_session(&self, session: &Session) -> Result<()> {
        self.conn.execute(
            "UPDATE sessions SET 
                parent_session_id = ?2,
                title = ?3,
                message_count = ?4,
                prompt_tokens = ?5,
                completion_tokens = ?6,
                cost = ?7,
                updated_at = ?8,
                summary_message_id = ?9
             WHERE id = ?1",
            params![
                session.id,
                session.parent_session_id,
                session.title,
                session.message_count,
                session.prompt_tokens,
                session.completion_tokens,
                session.cost,
                session.updated_at,
                session.summary_message_id,
            ],
        )?;
        Ok(())
    }
    
    fn delete_session(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])?;
        Ok(())
    }
    
    // Message operations
    fn create_message(&self, message: &Message) -> Result<()> {
        self.conn.execute(
            "INSERT INTO messages (
                id, session_id, role, parts, model,
                created_at, updated_at, finished_at, provider
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                message.id,
                message.session_id,
                message.role,
                message.parts,
                message.model,
                message.created_at,
                message.updated_at,
                message.finished_at,
                message.provider,
            ],
        )?;
        Ok(())
    }
    
    fn get_message(&self, id: &str) -> Result<Option<Message>> {
        self.conn
            .query_row(
                "SELECT id, session_id, role, parts, model,
                        created_at, updated_at, finished_at, provider
                 FROM messages WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Message {
                        id: row.get(0)?,
                        session_id: row.get(1)?,
                        role: row.get(2)?,
                        parts: row.get(3)?,
                        model: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                        finished_at: row.get(7)?,
                        provider: row.get(8)?,
                    })
                },
            )
            .optional()
            .context("Failed to get message")
    }
    
    fn list_messages_by_session(&self, session_id: &str) -> Result<Vec<Message>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, role, parts, model,
                    created_at, updated_at, finished_at, provider
             FROM messages 
             WHERE session_id = ?1
             ORDER BY created_at ASC",
        )?;
        
        let messages = stmt
            .query_map(params![session_id], |row| {
                Ok(Message {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    role: row.get(2)?,
                    parts: row.get(3)?,
                    model: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    finished_at: row.get(7)?,
                    provider: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        
        Ok(messages)
    }
    
    fn update_message(&self, message: &Message) -> Result<()> {
        self.conn.execute(
            "UPDATE messages SET 
                role = ?2,
                parts = ?3,
                model = ?4,
                updated_at = ?5,
                finished_at = ?6,
                provider = ?7
             WHERE id = ?1",
            params![
                message.id,
                message.role,
                message.parts,
                message.model,
                message.updated_at,
                message.finished_at,
                message.provider,
            ],
        )?;
        Ok(())
    }
    
    fn delete_message(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM messages WHERE id = ?1", params![id])?;
        Ok(())
    }
    
    // File operations
    fn create_file(&self, file: &File) -> Result<()> {
        self.conn.execute(
            "INSERT INTO files (
                id, session_id, path, content, version,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                file.id,
                file.session_id,
                file.path,
                file.content,
                file.version,
                file.created_at,
                file.updated_at,
            ],
        )?;
        Ok(())
    }
    
    fn get_file(&self, id: &str) -> Result<Option<File>> {
        self.conn
            .query_row(
                "SELECT id, session_id, path, content, version,
                        created_at, updated_at
                 FROM files WHERE id = ?1",
                params![id],
                |row| {
                    Ok(File {
                        id: row.get(0)?,
                        session_id: row.get(1)?,
                        path: row.get(2)?,
                        content: row.get(3)?,
                        version: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    })
                },
            )
            .optional()
            .context("Failed to get file")
    }
    
    fn get_file_by_path_and_session(&self, path: &str, session_id: &str) -> Result<Option<File>> {
        self.conn
            .query_row(
                "SELECT id, session_id, path, content, version,
                        created_at, updated_at
                 FROM files 
                 WHERE path = ?1 AND session_id = ?2
                 ORDER BY version DESC
                 LIMIT 1",
                params![path, session_id],
                |row| {
                    Ok(File {
                        id: row.get(0)?,
                        session_id: row.get(1)?,
                        path: row.get(2)?,
                        content: row.get(3)?,
                        version: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    })
                },
            )
            .optional()
            .context("Failed to get file by path and session")
    }
    
    fn list_files_by_session(&self, session_id: &str) -> Result<Vec<File>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, path, content, version,
                    created_at, updated_at
             FROM files 
             WHERE session_id = ?1
             ORDER BY path ASC, version DESC",
        )?;
        
        let files = stmt
            .query_map(params![session_id], |row| {
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
    
    fn list_files_by_path(&self, path: &str) -> Result<Vec<File>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, path, content, version,
                    created_at, updated_at
             FROM files 
             WHERE path = ?1
             ORDER BY created_at DESC",
        )?;
        
        let files = stmt
            .query_map(params![path], |row| {
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
    
    fn delete_file(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM files WHERE id = ?1", params![id])?;
        Ok(())
    }
}

/// Transaction-based queries
pub struct TransactionQueries<'a> {
    tx: &'a Transaction<'a>,
}

impl<'a> TransactionQueries<'a> {
    pub fn new(tx: &'a Transaction<'a>) -> Self {
        Self { tx }
    }
}

// Implement DatabaseOperations for TransactionQueries with same methods
// but using self.tx instead of self.conn (omitted for brevity, same implementation)