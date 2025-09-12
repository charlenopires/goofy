//! Factory for creating database session managers

use anyhow::Result;
use std::sync::Arc;
use std::path::Path;

use crate::db::DatabaseConfig;
use super::DatabaseSessionManager;

/// Factory for creating database session managers
pub struct DatabaseSessionFactory;

impl DatabaseSessionFactory {
    /// Create a new database session manager
    pub async fn create<P: AsRef<Path>>(data_dir: P) -> Result<Arc<DatabaseSessionManager>> {
        let config = DatabaseConfig {
            data_dir: data_dir.as_ref().to_path_buf(),
            db_name: "goofy.db".to_string(),
        };
        
        let manager = DatabaseSessionManager::new(Some(config)).await?;
        Ok(Arc::new(manager))
    }
    
    /// Create with custom config
    pub async fn create_with_config(config: DatabaseConfig) -> Result<Arc<DatabaseSessionManager>> {
        let manager = DatabaseSessionManager::new(Some(config)).await?;
        Ok(Arc::new(manager))
    }
}