//! Core shell implementation with cross-platform support
//!
//! This module provides a shell executor that works on both Unix and Windows,
//! maintaining state across commands and providing security features.

use anyhow::{Result, anyhow};
use std::{
    collections::HashMap,
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};
use tokio::process::Command as TokioCommand;
use tokio::sync::RwLock;
use tracing::{info, debug, warn};

/// Type of shell to use for execution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellType {
    /// POSIX-compliant shell (sh/bash)
    Posix,
    /// Windows Command Prompt
    Cmd,
    /// Windows PowerShell
    PowerShell,
}

impl ShellType {
    /// Detect the appropriate shell type for the current platform
    pub fn detect() -> Self {
        if cfg!(target_os = "windows") {
            // Default to PowerShell on Windows as it's more capable
            ShellType::PowerShell
        } else {
            ShellType::Posix
        }
    }
}

/// Function type for blocking commands
pub type BlockFunc = Box<dyn Fn(&[String]) -> bool + Send + Sync>;

/// Command blocker for security
pub struct CommandBlocker {
    blocked_commands: Vec<String>,
    blocked_patterns: Vec<String>,
    block_funcs: Vec<BlockFunc>,
}

impl CommandBlocker {
    /// Create a new command blocker
    pub fn new() -> Self {
        Self {
            blocked_commands: Vec::new(),
            blocked_patterns: Vec::new(),
            block_funcs: Vec::new(),
        }
    }
    
    /// Add exact commands to block
    pub fn block_commands(mut self, commands: Vec<String>) -> Self {
        self.blocked_commands.extend(commands);
        self
    }
    
    /// Add patterns to block (substrings)
    pub fn block_patterns(mut self, patterns: Vec<String>) -> Self {
        self.blocked_patterns.extend(patterns);
        self
    }
    
    /// Add a custom block function
    pub fn add_block_func<F>(mut self, func: F) -> Self 
    where
        F: Fn(&[String]) -> bool + Send + Sync + 'static,
    {
        self.block_funcs.push(Box::new(func));
        self
    }
    
    /// Check if a command should be blocked
    pub fn should_block(&self, command: &str, args: &[String]) -> bool {
        // Check exact command matches
        if self.blocked_commands.contains(&command.to_string()) {
            return true;
        }
        
        // Check pattern matches
        let full_command = format!("{} {}", command, args.join(" "));
        for pattern in &self.blocked_patterns {
            if full_command.contains(pattern) {
                return true;
            }
        }
        
        // Check custom functions
        let mut all_args = vec![command.to_string()];
        all_args.extend(args.iter().cloned());
        for func in &self.block_funcs {
            if func(&all_args) {
                return true;
            }
        }
        
        false
    }
}

impl Default for CommandBlocker {
    fn default() -> Self {
        Self::new()
            .block_patterns(vec![
                "rm -rf /".to_string(),
                "rm -rf /*".to_string(),
                ":(){ :|:& };:".to_string(), // Fork bomb
                "dd if=/dev/zero".to_string(),
                "mkfs".to_string(),
                "fdisk".to_string(),
            ])
            .block_commands(vec![
                "shutdown".to_string(),
                "reboot".to_string(),
                "halt".to_string(),
                "poweroff".to_string(),
            ])
    }
}

/// Options for creating a shell instance
pub struct ShellOptions {
    pub shell_type: ShellType,
    pub working_dir: Option<PathBuf>,
    pub env: Option<HashMap<String, String>>,
    pub blocker: Option<CommandBlocker>,
}

impl Default for ShellOptions {
    fn default() -> Self {
        Self {
            shell_type: ShellType::detect(),
            working_dir: None,
            env: None,
            blocker: Some(CommandBlocker::default()),
        }
    }
}

/// Shell executor with state management
pub struct Shell {
    shell_type: ShellType,
    working_dir: Arc<RwLock<PathBuf>>,
    env: Arc<RwLock<HashMap<String, String>>>,
    blocker: Option<CommandBlocker>,
}

impl Shell {
    /// Create a new shell instance
    pub fn new(options: ShellOptions) -> Result<Self> {
        let working_dir = options.working_dir
            .or_else(|| env::current_dir().ok())
            .ok_or_else(|| anyhow!("Failed to determine working directory"))?;
        
        let env = options.env.unwrap_or_else(|| {
            env::vars().collect()
        });
        
        Ok(Self {
            shell_type: options.shell_type,
            working_dir: Arc::new(RwLock::new(working_dir)),
            env: Arc::new(RwLock::new(env)),
            blocker: options.blocker,
        })
    }
    
    /// Execute a command in the shell
    pub async fn execute(&self, command: &str) -> Result<(String, String, i32)> {
        // Parse the command to check for blocking
        if let Some(blocker) = &self.blocker {
            let parts: Vec<String> = command.split_whitespace()
                .map(|s| s.to_string())
                .collect();
            
            if !parts.is_empty() {
                let (cmd, args) = parts.split_first().unwrap();
                if blocker.should_block(cmd, args) {
                    return Err(anyhow!("Command blocked for security: {}", command));
                }
            }
        }
        
        let cwd = self.working_dir.read().await.clone();
        let env_vars = self.env.read().await.clone();
        
        debug!("Executing command: {} in {:?}", command, cwd);
        
        match self.shell_type {
            ShellType::Posix => self.execute_posix(command, cwd, env_vars).await,
            ShellType::Cmd => self.execute_cmd(command, cwd, env_vars).await,
            ShellType::PowerShell => self.execute_powershell(command, cwd, env_vars).await,
        }
    }
    
    /// Execute command with POSIX shell
    async fn execute_posix(
        &self,
        command: &str,
        cwd: PathBuf,
        env: HashMap<String, String>,
    ) -> Result<(String, String, i32)> {
        let output = TokioCommand::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(cwd)
            .envs(env)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;
        
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);
        
        // Handle cd command specially to update working directory
        if command.trim().starts_with("cd ") {
            if exit_code == 0 {
                if let Some(new_dir) = command.trim().strip_prefix("cd ") {
                    let new_dir = new_dir.trim();
                    if let Err(e) = self.set_working_dir(new_dir).await {
                        warn!("Failed to update working directory: {}", e);
                    }
                }
            }
        }
        
        Ok((stdout, stderr, exit_code))
    }
    
    /// Execute command with Windows CMD
    async fn execute_cmd(
        &self,
        command: &str,
        cwd: PathBuf,
        env: HashMap<String, String>,
    ) -> Result<(String, String, i32)> {
        let output = TokioCommand::new("cmd")
            .args(["/C", command])
            .current_dir(cwd)
            .envs(env)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;
        
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);
        
        Ok((stdout, stderr, exit_code))
    }
    
    /// Execute command with PowerShell
    async fn execute_powershell(
        &self,
        command: &str,
        cwd: PathBuf,
        env: HashMap<String, String>,
    ) -> Result<(String, String, i32)> {
        let output = TokioCommand::new("powershell")
            .args(["-NoProfile", "-Command", command])
            .current_dir(cwd)
            .envs(env)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;
        
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);
        
        Ok((stdout, stderr, exit_code))
    }
    
    /// Get the current working directory
    pub async fn get_working_dir(&self) -> PathBuf {
        self.working_dir.read().await.clone()
    }
    
    /// Set the working directory
    pub async fn set_working_dir(&self, dir: &str) -> Result<()> {
        let path = PathBuf::from(dir);
        let absolute_path = if path.is_absolute() {
            path
        } else {
            let current = self.working_dir.read().await;
            current.join(path)
        };
        
        // Verify the directory exists
        if !absolute_path.exists() {
            return Err(anyhow!("Directory does not exist: {:?}", absolute_path));
        }
        
        if !absolute_path.is_dir() {
            return Err(anyhow!("Path is not a directory: {:?}", absolute_path));
        }
        
        let canonical = absolute_path.canonicalize()?;
        *self.working_dir.write().await = canonical;
        
        info!("Changed working directory to: {:?}", self.working_dir.read().await);
        Ok(())
    }
    
    /// Get an environment variable
    pub async fn get_env(&self, key: &str) -> Option<String> {
        self.env.read().await.get(key).cloned()
    }
    
    /// Set an environment variable
    pub async fn set_env(&self, key: String, value: String) {
        self.env.write().await.insert(key, value);
    }
    
    /// Get all environment variables
    pub async fn get_all_env(&self) -> HashMap<String, String> {
        self.env.read().await.clone()
    }
    
    /// Clear all environment variables
    pub async fn clear_env(&self) {
        self.env.write().await.clear();
    }
}

/// Exit status from command execution
#[derive(Debug, Clone)]
pub struct ExitStatus {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl ExitStatus {
    /// Check if the command succeeded (exit code 0)
    pub fn success(&self) -> bool {
        self.code == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_shell_creation() {
        let shell = Shell::new(ShellOptions::default()).unwrap();
        assert!(!shell.get_working_dir().await.as_os_str().is_empty());
    }
    
    #[tokio::test]
    async fn test_command_blocking() {
        let blocker = CommandBlocker::default();
        assert!(blocker.should_block("rm", &["-rf".to_string(), "/".to_string()]));
        assert!(blocker.should_block("shutdown", &[]));
        assert!(!blocker.should_block("ls", &["-la".to_string()]));
    }
    
    #[tokio::test]
    async fn test_simple_command() {
        let shell = Shell::new(ShellOptions::default()).unwrap();
        let (stdout, _, code) = shell.execute("echo Hello").await.unwrap();
        assert_eq!(code, 0);
        assert!(stdout.contains("Hello"));
    }
}