//! File permissions and security utilities for file operations.
//!
//! This module provides utilities for handling file permissions, security
//! validation, and access control for file operations in the TUI.

use anyhow::Result;
use std::io::Write;
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// File access permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Permissions {
    /// Owner can read
    pub owner_read: bool,
    /// Owner can write
    pub owner_write: bool,
    /// Owner can execute
    pub owner_execute: bool,
    /// Group can read
    pub group_read: bool,
    /// Group can write
    pub group_write: bool,
    /// Group can execute
    pub group_execute: bool,
    /// Others can read
    pub other_read: bool,
    /// Others can write
    pub other_write: bool,
    /// Others can execute
    pub other_execute: bool,
}

impl Permissions {
    /// Create permissions from Unix mode bits
    #[cfg(unix)]
    pub fn from_mode(mode: u32) -> Self {
        Self {
            owner_read: mode & 0o400 != 0,
            owner_write: mode & 0o200 != 0,
            owner_execute: mode & 0o100 != 0,
            group_read: mode & 0o040 != 0,
            group_write: mode & 0o020 != 0,
            group_execute: mode & 0o010 != 0,
            other_read: mode & 0o004 != 0,
            other_write: mode & 0o002 != 0,
            other_execute: mode & 0o001 != 0,
        }
    }
    
    /// Create permissions for non-Unix systems
    #[cfg(not(unix))]
    pub fn from_readonly(readonly: bool) -> Self {
        Self {
            owner_read: true,
            owner_write: !readonly,
            owner_execute: false,
            group_read: true,
            group_write: false,
            group_execute: false,
            other_read: true,
            other_write: false,
            other_execute: false,
        }
    }
    
    /// Get permissions for a file path
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let metadata = std::fs::metadata(path)?;
        
        #[cfg(unix)]
        {
            Ok(Self::from_mode(metadata.permissions().mode()))
        }
        
        #[cfg(not(unix))]
        {
            Ok(Self::from_readonly(metadata.permissions().readonly()))
        }
    }
    
    /// Convert to Unix permission string (e.g., "rwxr-xr-x")
    pub fn to_string(&self) -> String {
        format!(
            "{}{}{}{}{}{}{}{}{}",
            if self.owner_read { "r" } else { "-" },
            if self.owner_write { "w" } else { "-" },
            if self.owner_execute { "x" } else { "-" },
            if self.group_read { "r" } else { "-" },
            if self.group_write { "w" } else { "-" },
            if self.group_execute { "x" } else { "-" },
            if self.other_read { "r" } else { "-" },
            if self.other_write { "w" } else { "-" },
            if self.other_execute { "x" } else { "-" },
        )
    }
    
    /// Convert to octal mode
    pub fn to_mode(&self) -> u32 {
        let mut mode = 0;
        
        if self.owner_read { mode |= 0o400; }
        if self.owner_write { mode |= 0o200; }
        if self.owner_execute { mode |= 0o100; }
        if self.group_read { mode |= 0o040; }
        if self.group_write { mode |= 0o020; }
        if self.group_execute { mode |= 0o010; }
        if self.other_read { mode |= 0o004; }
        if self.other_write { mode |= 0o002; }
        if self.other_execute { mode |= 0o001; }
        
        mode
    }
    
    /// Check if current user can read the file
    pub fn can_read(&self) -> bool {
        // Simplified check - in a real implementation you'd check current user/group
        self.owner_read || self.group_read || self.other_read
    }
    
    /// Check if current user can write to the file
    pub fn can_write(&self) -> bool {
        // Simplified check - in a real implementation you'd check current user/group
        self.owner_write || self.group_write || self.other_write
    }
    
    /// Check if current user can execute the file
    pub fn can_execute(&self) -> bool {
        // Simplified check - in a real implementation you'd check current user/group
        self.owner_execute || self.group_execute || self.other_execute
    }
    
    /// Get permission level description
    pub fn get_level_description(&self) -> String {
        if !self.can_read() {
            "No access".to_string()
        } else if !self.can_write() {
            "Read-only".to_string()
        } else if self.can_execute() {
            "Full access".to_string()
        } else {
            "Read-write".to_string()
        }
    }
}

/// Security validation for file operations
pub struct SecurityValidator {
    /// Allowed directories for file operations
    allowed_directories: Vec<String>,
    /// Forbidden file patterns
    forbidden_patterns: Vec<String>,
    /// Maximum file size for operations
    max_file_size: u64,
}

impl SecurityValidator {
    /// Create a new security validator
    pub fn new() -> Self {
        Self {
            allowed_directories: Vec::new(),
            forbidden_patterns: vec![
                "..".to_string(),      // Path traversal
                ".git".to_string(),    // Git metadata
                ".env".to_string(),    // Environment files
                "*.key".to_string(),   // Key files
                "*.pem".to_string(),   // Certificate files
                "id_rsa*".to_string(), // SSH keys
            ],
            max_file_size: 100 * 1024 * 1024, // 100MB default
        }
    }
    
    /// Add an allowed directory
    pub fn add_allowed_directory<S: Into<String>>(&mut self, directory: S) {
        self.allowed_directories.push(directory.into());
    }
    
    /// Add a forbidden pattern
    pub fn add_forbidden_pattern<S: Into<String>>(&mut self, pattern: S) {
        self.forbidden_patterns.push(pattern.into());
    }
    
    /// Set maximum file size
    pub fn set_max_file_size(&mut self, size: u64) {
        self.max_file_size = size;
    }
    
    /// Validate a file path for security
    pub fn validate_path<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        let path_str = path.to_string_lossy();
        
        // Check for path traversal
        if path_str.contains("..") {
            return Err(anyhow::anyhow!("Path traversal not allowed"));
        }
        
        // Check forbidden patterns
        for pattern in &self.forbidden_patterns {
            if self.matches_pattern(&path_str, pattern) {
                return Err(anyhow::anyhow!("Access to '{}' is forbidden", pattern));
            }
        }
        
        // Check allowed directories if specified
        if !self.allowed_directories.is_empty() {
            let canonical_path = path.canonicalize()
                .map_err(|_| anyhow::anyhow!("Cannot resolve path: {}", path.display()))?;
            
            let is_allowed = self.allowed_directories.iter().any(|allowed| {
                if let Ok(allowed_path) = std::path::Path::new(allowed).canonicalize() {
                    canonical_path.starts_with(allowed_path)
                } else {
                    false
                }
            });
            
            if !is_allowed {
                return Err(anyhow::anyhow!("Access to directory not allowed"));
            }
        }
        
        // Check file size if it exists
        if path.exists() && path.is_file() {
            let metadata = std::fs::metadata(path)?;
            if metadata.len() > self.max_file_size {
                return Err(anyhow::anyhow!(
                    "File too large: {} bytes (max: {} bytes)",
                    metadata.len(),
                    self.max_file_size
                ));
            }
        }
        
        Ok(())
    }
    
    /// Check if a string matches a pattern (simple glob-like matching)
    fn matches_pattern(&self, text: &str, pattern: &str) -> bool {
        if pattern.contains('*') {
            // Simple wildcard matching
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                text.starts_with(parts[0]) && text.ends_with(parts[1])
            } else {
                text.contains(pattern.trim_matches('*'))
            }
        } else {
            text.contains(pattern)
        }
    }
    
    /// Validate file for reading
    pub fn validate_read<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        self.validate_path(path)?;
        
        // Check read permissions
        let permissions = Permissions::from_path(path)?;
        if !permissions.can_read() {
            return Err(anyhow::anyhow!("No read permission for file"));
        }
        
        Ok(())
    }
    
    /// Validate file for writing
    pub fn validate_write<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        self.validate_path(path)?;
        
        // Check write permissions
        if path.exists() {
            let permissions = Permissions::from_path(path)?;
            if !permissions.can_write() {
                return Err(anyhow::anyhow!("No write permission for file"));
            }
        } else {
            // Check parent directory permissions
            if let Some(parent) = path.parent() {
                if parent.exists() {
                    let permissions = Permissions::from_path(parent)?;
                    if !permissions.can_write() {
                        return Err(anyhow::anyhow!("No write permission for parent directory"));
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Validate file for execution
    pub fn validate_execute<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        self.validate_path(path)?;
        
        // Check execute permissions
        let permissions = Permissions::from_path(path)?;
        if !permissions.can_execute() {
            return Err(anyhow::anyhow!("No execute permission for file"));
        }
        
        Ok(())
    }
}

impl Default for SecurityValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// File operation context for security tracking
#[derive(Debug, Clone)]
pub struct FileOperationContext {
    /// Operation type
    pub operation: FileOperation,
    /// Source path
    pub source: Option<String>,
    /// Destination path
    pub destination: Option<String>,
    /// User information
    pub user: Option<String>,
    /// Timestamp
    pub timestamp: std::time::Instant,
}

/// Types of file operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileOperation {
    Read,
    Write,
    Execute,
    Delete,
    Copy,
    Move,
    Create,
    List,
}

impl std::fmt::Display for FileOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileOperation::Read => write!(f, "read"),
            FileOperation::Write => write!(f, "write"),
            FileOperation::Execute => write!(f, "execute"),
            FileOperation::Delete => write!(f, "delete"),
            FileOperation::Copy => write!(f, "copy"),
            FileOperation::Move => write!(f, "move"),
            FileOperation::Create => write!(f, "create"),
            FileOperation::List => write!(f, "list"),
        }
    }
}

/// Security audit logger for file operations
pub struct SecurityAuditLogger {
    /// Log file path
    log_file: Option<String>,
    /// In-memory log for recent operations
    recent_operations: std::collections::VecDeque<FileOperationContext>,
    /// Maximum number of recent operations to keep
    max_recent: usize,
}

impl SecurityAuditLogger {
    /// Create a new audit logger
    pub fn new() -> Self {
        Self {
            log_file: None,
            recent_operations: std::collections::VecDeque::new(),
            max_recent: 1000,
        }
    }
    
    /// Set log file path
    pub fn set_log_file<S: Into<String>>(&mut self, path: S) {
        self.log_file = Some(path.into());
    }
    
    /// Log a file operation
    pub fn log_operation(&mut self, context: FileOperationContext) -> Result<()> {
        // Add to recent operations
        self.recent_operations.push_back(context.clone());
        if self.recent_operations.len() > self.max_recent {
            self.recent_operations.pop_front();
        }
        
        // Write to log file if configured
        if let Some(ref log_file) = self.log_file {
            let log_entry = format!(
                "{} [{}] {} {} -> {}\n",
                context.timestamp.elapsed().as_secs(),
                context.user.as_deref().unwrap_or("unknown"),
                context.operation,
                context.source.as_deref().unwrap_or("N/A"),
                context.destination.as_deref().unwrap_or("N/A")
            );
            
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_file)?
                .write_all(log_entry.as_bytes())?;
        }
        
        Ok(())
    }
    
    /// Get recent operations
    pub fn get_recent_operations(&self) -> impl Iterator<Item = &FileOperationContext> {
        self.recent_operations.iter()
    }
    
    /// Clear recent operations log
    pub fn clear_recent(&mut self) {
        self.recent_operations.clear();
    }
}

impl Default for SecurityAuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility functions for file permissions
pub mod utils {
    use super::*;
    
    /// Check if a file is readable
    pub fn is_readable<P: AsRef<Path>>(path: P) -> bool {
        Permissions::from_path(path)
            .map(|p| p.can_read())
            .unwrap_or(false)
    }
    
    /// Check if a file is writable
    pub fn is_writable<P: AsRef<Path>>(path: P) -> bool {
        Permissions::from_path(path)
            .map(|p| p.can_write())
            .unwrap_or(false)
    }
    
    /// Check if a file is executable
    pub fn is_executable<P: AsRef<Path>>(path: P) -> bool {
        Permissions::from_path(path)
            .map(|p| p.can_execute())
            .unwrap_or(false)
    }
    
    /// Get permission summary for display
    pub fn get_permission_summary<P: AsRef<Path>>(path: P) -> String {
        match Permissions::from_path(path) {
            Ok(perms) => perms.to_string(),
            Err(_) => "???".to_string(),
        }
    }
    
    /// Check if path is safe for file operations
    pub fn is_safe_path<P: AsRef<Path>>(path: P) -> bool {
        let validator = SecurityValidator::new();
        validator.validate_path(path).is_ok()
    }
    
    /// Sanitize filename for safe filesystem operations
    pub fn sanitize_filename(filename: &str) -> String {
        filename
            .chars()
            .map(|c| match c {
                // Replace unsafe characters
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                // Keep safe characters
                c if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' || c == ' ' => c,
                // Replace other characters
                _ => '_',
            })
            .collect::<String>()
            .trim_matches('.')  // Remove leading/trailing dots
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_permissions_creation() {
        #[cfg(unix)]
        {
            let perms = Permissions::from_mode(0o755);
            assert!(perms.owner_read);
            assert!(perms.owner_write);
            assert!(perms.owner_execute);
            assert!(perms.group_read);
            assert!(!perms.group_write);
            assert!(perms.group_execute);
            assert_eq!(perms.to_string(), "rwxr-xr-x");
            assert_eq!(perms.to_mode(), 0o755);
        }
    }
    
    #[test]
    fn test_security_validator() {
        let mut validator = SecurityValidator::new();
        
        // Test path traversal detection
        assert!(validator.validate_path("../etc/passwd").is_err());
        assert!(validator.validate_path("safe/path/file.txt").is_ok());
        
        // Test forbidden patterns
        assert!(validator.validate_path("secret.key").is_err());
        assert!(validator.validate_path("config.env").is_err());
        
        // Test allowed directories
        validator.add_allowed_directory("/tmp");
        // Note: This test would need actual filesystem access to work properly
    }
    
    #[test]
    fn test_filename_sanitization() {
        assert_eq!(utils::sanitize_filename("file:name*test?.txt"), "file_name_test_.txt");
        assert_eq!(utils::sanitize_filename("normal_file.txt"), "normal_file.txt");
        assert_eq!(utils::sanitize_filename("../dangerous"), "_dangerous");
    }
    
    #[test]
    fn test_audit_logger() {
        let mut logger = SecurityAuditLogger::new();
        
        let context = FileOperationContext {
            operation: FileOperation::Read,
            source: Some("test.txt".to_string()),
            destination: None,
            user: Some("test_user".to_string()),
            timestamp: std::time::Instant::now(),
        };
        
        assert!(logger.log_operation(context).is_ok());
        assert_eq!(logger.get_recent_operations().count(), 1);
    }
    
    #[test]
    fn test_permission_utilities() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "test content").unwrap();
        
        // These tests depend on the actual file system permissions
        assert!(utils::is_readable(&file_path));
        assert_ne!(utils::get_permission_summary(&file_path), "???");
    }
}

// Re-export commonly used items
pub use utils::{is_readable, is_writable, is_executable, sanitize_filename};