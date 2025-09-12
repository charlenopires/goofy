//! Persistent shell singleton for maintaining state across the application
//!
//! This module provides a singleton shell instance that maintains its state
//! (working directory, environment variables) throughout the application lifecycle.

use super::shell::{Shell, ShellOptions, ShellType};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, OnceCell};
use tracing::info;

/// Global persistent shell instance
static PERSISTENT_SHELL: OnceCell<Arc<Mutex<PersistentShell>>> = OnceCell::const_new();

/// A persistent shell that maintains state across commands
pub struct PersistentShell {
    shell: Shell,
    command_history: Vec<String>,
    max_history: usize,
}

impl PersistentShell {
    /// Create a new persistent shell
    pub fn new(working_dir: Option<PathBuf>) -> Result<Self> {
        let options = ShellOptions {
            working_dir,
            ..Default::default()
        };
        
        let shell = Shell::new(options)?;
        
        Ok(Self {
            shell,
            command_history: Vec::new(),
            max_history: 1000,
        })
    }
    
    /// Execute a command and record it in history
    pub async fn execute(&mut self, command: &str) -> Result<(String, String, i32)> {
        // Add to history
        self.command_history.push(command.to_string());
        
        // Trim history if it's too long
        if self.command_history.len() > self.max_history {
            self.command_history.drain(0..100);
        }
        
        // Execute the command
        self.shell.execute(command).await
    }
    
    /// Get the command history
    pub fn get_history(&self) -> &[String] {
        &self.command_history
    }
    
    /// Clear the command history
    pub fn clear_history(&mut self) {
        self.command_history.clear();
    }
    
    /// Get the last N commands from history
    pub fn get_recent_history(&self, n: usize) -> Vec<String> {
        let start = self.command_history.len().saturating_sub(n);
        self.command_history[start..].to_vec()
    }
    
    /// Search history for commands containing a pattern
    pub fn search_history(&self, pattern: &str) -> Vec<String> {
        self.command_history
            .iter()
            .filter(|cmd| cmd.contains(pattern))
            .cloned()
            .collect()
    }
    
    /// Get the underlying shell
    pub fn shell(&self) -> &Shell {
        &self.shell
    }
    
    /// Get mutable access to the underlying shell
    pub fn shell_mut(&mut self) -> &mut Shell {
        &mut self.shell
    }
}

/// Get or create the global persistent shell instance
pub async fn get_persistent_shell() -> Arc<Mutex<PersistentShell>> {
    PERSISTENT_SHELL
        .get_or_init(|| async {
            info!("Initializing persistent shell");
            
            let working_dir = std::env::current_dir().ok();
            let shell = PersistentShell::new(working_dir)
                .expect("Failed to create persistent shell");
            
            Arc::new(Mutex::new(shell))
        })
        .await
        .clone()
}

/// Initialize the persistent shell with custom options
pub async fn init_persistent_shell(working_dir: Option<PathBuf>) -> Result<()> {
    if PERSISTENT_SHELL.get().is_some() {
        return Err(anyhow::anyhow!("Persistent shell already initialized"));
    }
    
    let shell = PersistentShell::new(working_dir)?;
    let _ = PERSISTENT_SHELL.set(Arc::new(Mutex::new(shell)));
    
    info!("Persistent shell initialized");
    Ok(())
}

/// Execute a command in the persistent shell
pub async fn execute_in_persistent(command: &str) -> Result<(String, String, i32)> {
    let shell = get_persistent_shell().await;
    let mut shell = shell.lock().await;
    shell.execute(command).await
}

/// Get the working directory of the persistent shell
pub async fn get_persistent_working_dir() -> PathBuf {
    let shell = get_persistent_shell().await;
    let shell = shell.lock().await;
    shell.shell().get_working_dir().await
}

/// Set the working directory of the persistent shell
pub async fn set_persistent_working_dir(dir: &str) -> Result<()> {
    let shell = get_persistent_shell().await;
    let shell = shell.lock().await;
    shell.shell().set_working_dir(dir).await
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_persistent_shell() {
        let shell1 = get_persistent_shell().await;
        let shell2 = get_persistent_shell().await;
        
        // Should be the same instance
        assert!(Arc::ptr_eq(&shell1, &shell2));
    }
    
    #[tokio::test]
    async fn test_command_history() {
        let shell = PersistentShell::new(None).unwrap();
        let shell = Arc::new(Mutex::new(shell));
        
        let mut shell = shell.lock().await;
        shell.execute("echo test1").await.unwrap();
        shell.execute("echo test2").await.unwrap();
        shell.execute("echo test3").await.unwrap();
        
        let history = shell.get_history();
        assert_eq!(history.len(), 3);
        assert_eq!(history[0], "echo test1");
        
        let recent = shell.get_recent_history(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[1], "echo test3");
    }
}