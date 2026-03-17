//! Database connection and initialization

use anyhow::{Result, Context};
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use std::fs;
use tracing::{info, debug, error};

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub data_dir: PathBuf,
    pub db_name: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("goofy");
            
        Self {
            data_dir,
            db_name: "goofy.db".to_string(),
        }
    }
}

/// Connect to the database and initialize it
pub fn connect(config: DatabaseConfig) -> Result<Connection> {
    // Ensure data directory exists
    fs::create_dir_all(&config.data_dir)
        .context("Failed to create data directory")?;
    
    let db_path = config.data_dir.join(&config.db_name);
    info!("Connecting to database at: {:?}", db_path);
    
    // Open the SQLite database
    let mut conn = Connection::open(&db_path)
        .context("Failed to open database")?;
    
    // First, disable foreign keys to allow table creation
    conn.execute_batch("PRAGMA foreign_keys = OFF;")
        .context("Failed to disable foreign keys")?;
    
    // Apply migrations (creates tables)
    apply_migrations(&mut conn)?;
    
    // Now set pragmas including re-enabling foreign keys
    set_pragmas(&conn)?;
    
    Ok(conn)
}

/// Set SQLite pragmas for better performance
fn set_pragmas(conn: &Connection) -> Result<()> {
    // journal_mode returns results, so use execute_batch
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;
         PRAGMA page_size = 4096;
         PRAGMA cache_size = -8000;
         PRAGMA synchronous = NORMAL;"
    ).context("Failed to set pragmas")?;
    debug!("Set all pragmas");

    Ok(())
}

/// Apply database migrations
fn apply_migrations(conn: &mut Connection) -> Result<()> {
    use crate::db::migrations;
    
    // Create migrations table if it doesn't exist
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at INTEGER NOT NULL
        )",
        params![],
    )?;
    
    // Get current version
    let current_version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            params![],
            |row| row.get(0),
        )
        .unwrap_or(0);
    
    debug!("Current migration version: {}", current_version);
    
    // Apply migrations
    for migration in migrations::MIGRATIONS {
        if migration.version > current_version {
            info!("Applying migration {}: {}", migration.version, migration.name);
            
            // Begin transaction
            let tx = conn.transaction()?;
            
            // Execute migration SQL
            tx.execute_batch(migration.up_sql)?;
            
            // Record migration
            tx.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
                params![
                    migration.version,
                    chrono::Utc::now().timestamp_millis()
                ],
            )?;
            
            // Commit transaction
            tx.commit()?;
            
            info!("Migration {} applied successfully", migration.version);
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_database_connection() {
        let dir = tempdir().unwrap();
        let config = DatabaseConfig {
            data_dir: dir.path().to_path_buf(),
            db_name: "test.db".to_string(),
        };
        
        let conn = connect(config).unwrap();
        
        // Test that pragmas are set
        let journal_mode: String = conn
            .query_row("PRAGMA journal_mode", params![], |row| row.get(0))
            .unwrap();
        assert_eq!(journal_mode, "wal");
        
        // Test that migrations table exists
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_migrations'",
                params![],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}