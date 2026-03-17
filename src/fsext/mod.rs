//! File system extension utilities
//!
//! This module provides enhanced file system operations including
//! path expansion, globbing, and ignore file support.

use anyhow::Result;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::{DirEntry, WalkDir};

/// Expand shell variables and tilde in paths
pub fn expand(path: &str) -> Result<String> {
    if path.is_empty() {
        return Ok(String::new());
    }
    
    let expanded = shellexpand::full(path)?;
    Ok(expanded.into_owned())
}

/// File information with metadata
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub mod_time: SystemTime,
}

/// Check if a path should be skipped (hidden or commonly ignored)
pub fn should_skip(path: &Path) -> bool {
    // Check for hidden files (starting with a dot)
    if let Some(name) = path.file_name() {
        if let Some(name_str) = name.to_str() {
            if name_str != "." && name_str.starts_with('.') {
                return true;
            }
        }
    }
    
    // Common directories to ignore
    let ignored_dirs = [
        ".goofy", ".crush", "node_modules", "vendor", "dist", "build",
        "target", ".git", ".idea", ".vscode", "__pycache__", "bin",
        "obj", "out", "coverage", "logs", "generated", "bower_components",
        "jspm_packages", ".mypy_cache", ".pytest_cache", ".tox",
    ];
    
    for component in path.components() {
        if let Some(name) = component.as_os_str().to_str() {
            if ignored_dirs.contains(&name) {
                return true;
            }
        }
    }
    
    false
}

/// Walk a directory with ignore rules
pub fn walk_dir(root: &Path) -> impl Iterator<Item = DirEntry> {
    WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| !should_skip(e.path()))
        .filter_map(|e| e.ok())
}

/// Glob with double-star support
pub fn glob_with_double_star(
    pattern: &str,
    search_path: &Path,
    limit: Option<usize>,
) -> Result<(Vec<String>, bool)> {
    let mut matches = Vec::new();
    let mut truncated = false;
    
    for entry in walk_dir(search_path) {
        if entry.file_type().is_file() {
            let path = entry.path();
            
            // Simple pattern matching (you could use glob crate for more complex patterns)
            let path_str = path.to_string_lossy();
            if pattern_matches(&path_str, pattern) {
                matches.push(path.to_string_lossy().into_owned());
                
                if let Some(limit) = limit {
                    if matches.len() >= limit {
                        truncated = true;
                        break;
                    }
                }
            }
        }
    }
    
    // Sort by modification time (newest first)
    matches.sort_by(|a, b| {
        let a_meta = fs::metadata(a).and_then(|m| m.modified()).ok();
        let b_meta = fs::metadata(b).and_then(|m| m.modified()).ok();
        b_meta.cmp(&a_meta)
    });
    
    Ok((matches, truncated))
}

/// Simple pattern matching (supports * and **)
fn pattern_matches(path: &str, pattern: &str) -> bool {
    // Handle ** (match any depth) by replacing with recursive check
    if pattern.contains("**") {
        let parts: Vec<&str> = pattern.split("**").collect();
        if parts.len() == 2 {
            let prefix = parts[0];
            let suffix_pattern = parts[1].trim_start_matches('/');
            // Path must start with the prefix and the filename portion must match the suffix pattern
            if !prefix.is_empty() && !path.starts_with(prefix) {
                return false;
            }
            // Match the suffix pattern (which may contain *) against the rest of the path
            return pattern_matches(path, suffix_pattern);
        }
    }

    if pattern.contains('*') {
        // Simple wildcard matching: split on * and check prefix/suffix
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            return path.starts_with(parts[0]) && path.ends_with(parts[1]);
        }
    }

    // Exact match or suffix match
    path.ends_with(pattern)
}

/// Pretty print a path (shorten home directory)
pub fn pretty_path(path: &Path) -> String {
    if let Ok(home) = std::env::var("HOME") {
        if let Ok(stripped) = path.strip_prefix(&home) {
            return format!("~/{}", stripped.display());
        }
    }
    path.display().to_string()
}

/// Trim directory path for display
pub fn dir_trim(pwd: &str, limit: usize) -> String {
    let parts: Vec<&str> = pwd.split('/').filter(|s| !s.is_empty()).collect();
    
    if limit == 0 || limit >= parts.len() {
        return pwd.to_string();
    }
    
    let mut result = String::new();
    for (i, part) in parts.iter().enumerate().rev() {
        if i >= parts.len() - limit {
            if i == parts.len() - 1 {
                result = part.to_string();
            } else {
                result = format!("{}/{}", part.chars().next().unwrap_or('?'), result);
            }
        } else {
            result = format!(".../{}", result);
            break;
        }
    }
    
    if pwd.starts_with('/') {
        result = format!("/{}", result);
    }
    
    result
}

/// Check if path has a prefix
pub fn has_prefix(path: &Path, prefix: &Path) -> bool {
    path.starts_with(prefix)
}

/// Convert line endings to Unix (LF)
pub fn to_unix_line_endings(content: &str) -> (String, bool) {
    if content.contains("\r\n") {
        (content.replace("\r\n", "\n"), true)
    } else {
        (content.to_string(), false)
    }
}

/// Convert line endings to Windows (CRLF)
pub fn to_windows_line_endings(content: &str) -> (String, bool) {
    if !content.contains("\r\n") && content.contains('\n') {
        (content.replace('\n', "\r\n"), true)
    } else {
        (content.to_string(), false)
    }
}

/// List files in a directory
pub fn list_files(dir: &Path, recursive: bool) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    
    if recursive {
        for entry in walk_dir(dir) {
            if entry.file_type().is_file() {
                files.push(entry.path().to_path_buf());
            }
        }
    } else {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                files.push(entry.path());
            }
        }
    }
    
    Ok(files)
}

/// Get file owner (placeholder for cross-platform compatibility)
pub fn get_file_owner(_path: &Path) -> Result<String> {
    // This would need platform-specific implementation
    Ok("unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    
    #[test]
    fn test_should_skip() {
        assert!(should_skip(Path::new(".hidden")));
        assert!(should_skip(Path::new("node_modules")));
        assert!(should_skip(Path::new("path/to/.git")));
        assert!(!should_skip(Path::new("normal_file.txt")));
    }
    
    #[test]
    fn test_pattern_matches() {
        assert!(pattern_matches("src/main.rs", "*.rs"));
        assert!(pattern_matches("src/lib/mod.rs", "**/*.rs"));
        assert!(pattern_matches("test.txt", "test.txt"));
        assert!(!pattern_matches("test.rs", "*.txt"));
    }
    
    #[test]
    fn test_pretty_path() {
        std::env::set_var("HOME", "/home/user");
        assert_eq!(pretty_path(Path::new("/home/user/project")), "~/project");
        assert_eq!(pretty_path(Path::new("/other/path")), "/other/path");
    }
    
    #[test]
    fn test_dir_trim() {
        assert_eq!(dir_trim("/home/user/project/src", 2), "/.../p/src");
        assert_eq!(dir_trim("/home/user", 3), "/home/user");
    }
    
    #[test]
    fn test_line_endings() {
        let (unix, changed) = to_unix_line_endings("hello\r\nworld");
        assert_eq!(unix, "hello\nworld");
        assert!(changed);
        
        let (windows, changed) = to_windows_line_endings("hello\nworld");
        assert_eq!(windows, "hello\r\nworld");
        assert!(changed);
    }
}