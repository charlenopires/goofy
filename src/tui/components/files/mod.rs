//! File operations and diff viewer components for the TUI.
//!
//! This module provides sophisticated file handling capabilities including:
//! - File picker with image preview support
//! - Diff viewer with unified and split-view modes
//! - Syntax highlighting integration
//! - File system navigation and permissions
//! - Image and attachment handling

pub mod diff_viewer;
pub mod file_picker;
pub mod permissions;

use anyhow::Result;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Trait for file system items that can be displayed and selected
pub trait FileItem: std::fmt::Debug + Clone + Send + Sync {
    /// Get the file/directory name
    fn name(&self) -> &str;
    
    /// Get the full path
    fn path(&self) -> &Path;
    
    /// Check if this is a directory
    fn is_directory(&self) -> bool;
    
    /// Check if this is a file
    fn is_file(&self) -> bool;
    
    /// Get file size in bytes (None for directories)
    fn size(&self) -> Option<u64>;
    
    /// Get file permissions
    fn permissions(&self) -> Option<FilePermissions>;
    
    /// Get last modified time
    fn modified(&self) -> Option<Instant>;
    
    /// Check if the file is hidden
    fn is_hidden(&self) -> bool;
    
    /// Get file extension
    fn extension(&self) -> Option<&str>;
    
    /// Check if file is selectable
    fn is_selectable(&self) -> bool {
        true
    }
    
    /// Get MIME type for files
    fn mime_type(&self) -> Option<String>;
    
    /// Render the file item as a line
    fn render_line(&self, theme: &crate::tui::themes::Theme, selected: bool) -> Line<'static>;
}

/// File permissions information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilePermissions {
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
    pub owner_read: bool,
    pub owner_write: bool,
    pub owner_execute: bool,
    pub group_read: bool,
    pub group_write: bool,
    pub group_execute: bool,
    pub other_read: bool,
    pub other_write: bool,
    pub other_execute: bool,
}

impl FilePermissions {
    /// Create permissions from Unix mode bits
    pub fn from_mode(mode: u32) -> Self {
        Self {
            readable: mode & 0o400 != 0,
            writable: mode & 0o200 != 0,
            executable: mode & 0o100 != 0,
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
}

/// Standard file system item implementation
#[derive(Debug, Clone)]
pub struct StandardFileItem {
    name: String,
    path: PathBuf,
    is_directory: bool,
    size: Option<u64>,
    permissions: Option<FilePermissions>,
    modified: Option<Instant>,
    is_hidden: bool,
    mime_type: Option<String>,
}

impl StandardFileItem {
    /// Create a new file item from a path
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let metadata = std::fs::metadata(path)?;
        
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        
        let is_hidden = name.starts_with('.');
        
        let permissions = {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                Some(FilePermissions::from_mode(metadata.permissions().mode()))
            }
            #[cfg(not(unix))]
            {
                Some(FilePermissions {
                    readable: true,
                    writable: !metadata.permissions().readonly(),
                    executable: false,
                    owner_read: true,
                    owner_write: !metadata.permissions().readonly(),
                    owner_execute: false,
                    group_read: true,
                    group_write: false,
                    group_execute: false,
                    other_read: true,
                    other_write: false,
                    other_execute: false,
                })
            }
        };
        
        let modified = metadata.modified()
            .ok()
            .and_then(|time| time.elapsed().ok())
            .map(|elapsed| Instant::now() - elapsed);
        
        let mime_type = if metadata.is_file() {
            Self::detect_mime_type(path)
        } else {
            None
        };
        
        Ok(Self {
            name,
            path: path.to_path_buf(),
            is_directory: metadata.is_dir(),
            size: if metadata.is_file() { Some(metadata.len()) } else { None },
            permissions,
            modified,
            is_hidden,
            mime_type,
        })
    }
    
    /// Detect MIME type for a file
    fn detect_mime_type(path: &Path) -> Option<String> {
        // Simple MIME type detection based on file extension
        match path.extension()?.to_str()? {
            "jpg" | "jpeg" => Some("image/jpeg".to_string()),
            "png" => Some("image/png".to_string()),
            "gif" => Some("image/gif".to_string()),
            "svg" => Some("image/svg+xml".to_string()),
            "txt" => Some("text/plain".to_string()),
            "md" => Some("text/markdown".to_string()),
            "json" => Some("application/json".to_string()),
            "yaml" | "yml" => Some("application/yaml".to_string()),
            "toml" => Some("application/toml".to_string()),
            "rs" => Some("text/x-rust".to_string()),
            "go" => Some("text/x-go".to_string()),
            "py" => Some("text/x-python".to_string()),
            "js" => Some("application/javascript".to_string()),
            "html" => Some("text/html".to_string()),
            "css" => Some("text/css".to_string()),
            _ => None,
        }
    }
    
    /// Format file size for display
    pub fn format_size(&self) -> String {
        match self.size {
            Some(size) => format_file_size(size),
            None => "-".to_string(),
        }
    }
    
    /// Format modification time for display
    pub fn format_modified(&self) -> String {
        match self.modified {
            Some(time) => {
                let elapsed = time.elapsed();
                if elapsed.as_secs() < 60 {
                    "now".to_string()
                } else if elapsed.as_secs() < 3600 {
                    format!("{}m ago", elapsed.as_secs() / 60)
                } else if elapsed.as_secs() < 86400 {
                    format!("{}h ago", elapsed.as_secs() / 3600)
                } else {
                    format!("{}d ago", elapsed.as_secs() / 86400)
                }
            }
            None => "-".to_string(),
        }
    }
}

impl FileItem for StandardFileItem {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn path(&self) -> &Path {
        &self.path
    }
    
    fn is_directory(&self) -> bool {
        self.is_directory
    }
    
    fn is_file(&self) -> bool {
        !self.is_directory
    }
    
    fn size(&self) -> Option<u64> {
        self.size
    }
    
    fn permissions(&self) -> Option<FilePermissions> {
        self.permissions
    }
    
    fn modified(&self) -> Option<Instant> {
        self.modified
    }
    
    fn is_hidden(&self) -> bool {
        self.is_hidden
    }
    
    fn extension(&self) -> Option<&str> {
        self.path.extension()?.to_str()
    }
    
    fn mime_type(&self) -> Option<String> {
        self.mime_type.clone()
    }
    
    fn render_line(&self, theme: &crate::tui::themes::Theme, selected: bool) -> Line<'static> {
        let mut spans = Vec::new();
        
        // File type indicator
        let type_indicator = if self.is_directory {
            "📁"
        } else {
            match self.extension() {
                Some("rs") => "🦀",
                Some("go") => "🐹",
                Some("py") => "🐍",
                Some("js") => "🟨",
                Some("html") => "🌐",
                Some("md") => "📝",
                Some("json") => "📋",
                Some("jpg") | Some("jpeg") | Some("png") | Some("gif") => "🖼️",
                _ => "📄",
            }
        };
        
        spans.push(Span::raw(format!("{} ", type_indicator)));
        
        // File name
        let name_style = if selected {
            Style::default()
                .fg(theme.colors.background)
                .bg(theme.colors.primary)
        } else if self.is_directory {
            Style::default().fg(theme.colors.primary)
        } else {
            Style::default().fg(theme.colors.text)
        };
        
        spans.push(Span::styled(self.name.clone(), name_style));
        
        // File size for files
        if let Some(size) = self.size {
            spans.push(Span::styled(
                format!(" ({})", format_file_size(size)),
                Style::default().fg(theme.colors.muted),
            ));
        }
        
        // Permissions indicator
        if let Some(perms) = self.permissions {
            if !perms.readable {
                spans.push(Span::styled(" 🚫", Style::default().fg(Color::Red)));
            } else if !perms.writable {
                spans.push(Span::styled(" 🔒", Style::default().fg(Color::Yellow)));
            }
        }
        
        Line::from(spans)
    }
}

impl crate::tui::components::lists::ListItem for StandardFileItem {
    fn id(&self) -> String {
        self.path.to_string_lossy().to_string()
    }

    fn content(&self) -> Vec<Line<'static>> {
        // Use a default theme for rendering content
        // This provides a basic text representation
        let mut spans = Vec::new();

        let type_indicator = if self.is_directory {
            "📁 "
        } else {
            "📄 "
        };

        spans.push(Span::raw(type_indicator.to_string()));
        spans.push(Span::raw(self.name.clone()));

        if let Some(size) = self.size {
            spans.push(Span::raw(format!(" ({})", format_file_size(size))));
        }

        vec![Line::from(spans)]
    }

    fn height(&self) -> u16 {
        1
    }

    fn selectable(&self) -> bool {
        true
    }

    fn style(&self) -> Option<ratatui::style::Style> {
        None
    }

    fn data(&self) -> Option<serde_json::Value> {
        None
    }
}

/// File operations events
#[derive(Debug, Clone)]
pub enum FileEvent {
    /// File was selected
    FileSelected { path: PathBuf },
    
    /// Directory was opened
    DirectoryOpened { path: PathBuf },
    
    /// File operation completed
    OperationCompleted { operation: String, success: bool },
    
    /// Error occurred
    Error { message: String },
}

/// Format file size in human-readable format
pub fn format_file_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

/// Validate file path for security
pub fn validate_file_path(path: &Path) -> Result<()> {
    // Check for path traversal attempts
    if path.to_string_lossy().contains("..") {
        return Err(anyhow::anyhow!("Path traversal not allowed"));
    }

    // Check for sensitive system paths
    let sensitive_paths = [
        "/etc/passwd", "/etc/shadow", "/etc/sudoers",
        "/etc/gshadow", "/etc/master.passwd",
    ];
    let path_str = path.to_string_lossy();
    for sensitive in &sensitive_paths {
        if path_str == *sensitive || path_str.ends_with(sensitive) {
            return Err(anyhow::anyhow!("Access to sensitive system file is not allowed: {}", path.display()));
        }
    }

    // Check if path exists
    if !path.exists() {
        return Err(anyhow::anyhow!("Path does not exist: {}", path.display()));
    }

    Ok(())
}

/// Check if a file is too large for certain operations
pub fn is_file_too_large(path: &Path, size_limit: u64) -> Result<bool> {
    let metadata = std::fs::metadata(path)?;
    Ok(metadata.len() > size_limit)
}

/// Get file extension in lowercase
pub fn get_file_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
}

/// Check if file has allowed extension
pub fn has_allowed_extension(path: &Path, allowed_extensions: &[&str]) -> bool {
    if let Some(ext) = get_file_extension(path) {
        allowed_extensions.iter().any(|&allowed| allowed == ext)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_file_size_formatting() {
        assert_eq!(format_file_size(500), "500 B");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(1048576), "1.0 MB");
        assert_eq!(format_file_size(1073741824), "1.0 GB");
    }
    
    #[test]
    fn test_file_extension() {
        let path = Path::new("test.txt");
        assert_eq!(get_file_extension(path), Some("txt".to_string()));
        
        let path = Path::new("test.TXT");
        assert_eq!(get_file_extension(path), Some("txt".to_string()));
        
        let path = Path::new("test");
        assert_eq!(get_file_extension(path), None);
    }
    
    #[test]
    fn test_allowed_extensions() {
        let path = Path::new("image.jpg");
        let allowed = &["jpg", "png", "gif"];
        assert!(has_allowed_extension(path, allowed));
        
        let path = Path::new("document.pdf");
        assert!(!has_allowed_extension(path, allowed));
    }
    
    #[test]
    fn test_file_item_creation() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "test content").unwrap();
        
        let item = StandardFileItem::from_path(&file_path).unwrap();
        assert_eq!(item.name(), "test.txt");
        assert!(!item.is_directory());
        assert!(item.is_file());
        assert_eq!(item.extension(), Some("txt"));
    }
    
    #[test]
    fn test_path_validation() {
        assert!(validate_file_path(Path::new("/etc/passwd")).is_err());
        assert!(validate_file_path(Path::new("../../../etc/passwd")).is_err());
        assert!(validate_file_path(Path::new("nonexistent")).is_err());
    }
    
    #[test]
    fn test_permissions() {
        let perms = FilePermissions::from_mode(0o755);
        assert!(perms.owner_read);
        assert!(perms.owner_write);
        assert!(perms.owner_execute);
        assert!(perms.group_read);
        assert!(!perms.group_write);
        assert!(perms.group_execute);
        assert!(perms.other_read);
        assert!(!perms.other_write);
        assert!(perms.other_execute);
        
        assert_eq!(perms.to_string(), "rwxr-xr-x");
    }
}