//! Core utilities implementation for cross-platform shell commands
//!
//! This module provides built-in implementations of common Unix utilities
//! that work consistently across platforms.

use anyhow::{Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};
use std::io::{self, Read, Write};
use walkdir::WalkDir;

/// Core utilities implementations
pub struct CoreUtils;

impl CoreUtils {
    /// List directory contents (ls)
    pub fn ls(path: Option<&Path>, all: bool, long: bool) -> Result<String> {
        let path = path.unwrap_or(Path::new("."));
        
        if !path.exists() {
            return Err(anyhow!("Path does not exist: {:?}", path));
        }
        
        let mut output = String::new();
        
        if path.is_file() {
            if long {
                let metadata = fs::metadata(path)?;
                output.push_str(&Self::format_long_entry(path, &metadata)?);
            } else {
                output.push_str(&path.display().to_string());
            }
        } else {
            let mut entries: Vec<_> = fs::read_dir(path)?
                .filter_map(|e| e.ok())
                .collect();
            
            entries.sort_by_key(|e| e.file_name());
            
            for entry in entries {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                
                // Skip hidden files unless -a flag is set
                if !all && name_str.starts_with('.') {
                    continue;
                }
                
                if long {
                    let metadata = entry.metadata()?;
                    output.push_str(&Self::format_long_entry(&entry.path(), &metadata)?);
                } else {
                    output.push_str(&name_str);
                    if entry.file_type()?.is_dir() {
                        output.push('/');
                    }
                }
                output.push('\n');
            }
        }
        
        Ok(output)
    }
    
    /// Format a long listing entry
    fn format_long_entry(path: &Path, metadata: &fs::Metadata) -> Result<String> {
        let size = metadata.len();
        let name = path.file_name()
            .ok_or_else(|| anyhow!("Invalid file name"))?
            .to_string_lossy();
        
        let type_char = if metadata.is_dir() { 'd' } else { '-' };
        
        Ok(format!("{} {:10} {}", type_char, size, name))
    }
    
    /// Concatenate files (cat)
    pub fn cat(paths: &[&Path]) -> Result<String> {
        let mut output = String::new();
        
        for path in paths {
            if !path.exists() {
                return Err(anyhow!("File does not exist: {:?}", path));
            }
            
            let contents = fs::read_to_string(path)?;
            output.push_str(&contents);
        }
        
        Ok(output)
    }
    
    /// Create directories (mkdir)
    pub fn mkdir(path: &Path, parents: bool) -> Result<()> {
        if parents {
            fs::create_dir_all(path)?;
        } else {
            fs::create_dir(path)?;
        }
        Ok(())
    }
    
    /// Remove files or directories (rm)
    pub fn rm(path: &Path, recursive: bool, force: bool) -> Result<()> {
        if !path.exists() {
            if force {
                return Ok(());
            } else {
                return Err(anyhow!("Path does not exist: {:?}", path));
            }
        }
        
        if path.is_dir() {
            if recursive {
                fs::remove_dir_all(path)?;
            } else {
                fs::remove_dir(path)?;
            }
        } else {
            fs::remove_file(path)?;
        }
        
        Ok(())
    }
    
    /// Copy files or directories (cp)
    pub fn cp(source: &Path, dest: &Path, recursive: bool) -> Result<()> {
        if !source.exists() {
            return Err(anyhow!("Source does not exist: {:?}", source));
        }
        
        if source.is_dir() {
            if !recursive {
                return Err(anyhow!("Source is a directory (use -r flag)"));
            }
            Self::copy_dir_recursive(source, dest)?;
        } else {
            if dest.is_dir() {
                let file_name = source.file_name()
                    .ok_or_else(|| anyhow!("Invalid source file name"))?;
                let dest_file = dest.join(file_name);
                fs::copy(source, dest_file)?;
            } else {
                fs::copy(source, dest)?;
            }
        }
        
        Ok(())
    }
    
    /// Copy directory recursively
    fn copy_dir_recursive(source: &Path, dest: &Path) -> Result<()> {
        fs::create_dir_all(dest)?;
        
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            let entry_path = entry.path();
            let file_name = entry.file_name();
            let dest_path = dest.join(file_name);
            
            if entry_path.is_dir() {
                Self::copy_dir_recursive(&entry_path, &dest_path)?;
            } else {
                fs::copy(&entry_path, &dest_path)?;
            }
        }
        
        Ok(())
    }
    
    /// Move files or directories (mv)
    pub fn mv(source: &Path, dest: &Path) -> Result<()> {
        if !source.exists() {
            return Err(anyhow!("Source does not exist: {:?}", source));
        }
        
        fs::rename(source, dest)?;
        Ok(())
    }
    
    /// Create empty files or update timestamps (touch)
    pub fn touch(path: &Path) -> Result<()> {
        if !path.exists() {
            fs::File::create(path)?;
        } else {
            // Update modification time
            filetime::set_file_mtime(
                path,
                filetime::FileTime::now(),
            )?;
        }
        Ok(())
    }
    
    /// Find files (find)
    pub fn find(
        path: &Path,
        name_pattern: Option<&str>,
        type_filter: Option<&str>,
    ) -> Result<Vec<PathBuf>> {
        let mut results = Vec::new();
        
        for entry in WalkDir::new(path) {
            let entry = entry?;
            let entry_path = entry.path();
            
            // Apply type filter
            if let Some(type_filter) = type_filter {
                match type_filter {
                    "f" if !entry_path.is_file() => continue,
                    "d" if !entry_path.is_dir() => continue,
                    _ => {}
                }
            }
            
            // Apply name pattern
            if let Some(pattern) = name_pattern {
                let file_name = entry_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                
                if !Self::matches_pattern(file_name, pattern) {
                    continue;
                }
            }
            
            results.push(entry_path.to_path_buf());
        }
        
        Ok(results)
    }
    
    /// Simple pattern matching (supports * wildcard)
    fn matches_pattern(text: &str, pattern: &str) -> bool {
        if pattern.contains('*') {
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.is_empty() {
                return true;
            }
            
            let mut text = text;
            for (i, part) in parts.iter().enumerate() {
                if part.is_empty() {
                    continue;
                }
                
                if i == 0 && !text.starts_with(part) {
                    return false;
                }
                
                if i == parts.len() - 1 && !text.ends_with(part) {
                    return false;
                }
                
                if let Some(pos) = text.find(part) {
                    text = &text[pos + part.len()..];
                } else {
                    return false;
                }
            }
            true
        } else {
            text == pattern
        }
    }
    
    /// Print working directory (pwd)
    pub fn pwd() -> Result<String> {
        Ok(std::env::current_dir()?.display().to_string())
    }
    
    /// Echo text
    pub fn echo(args: &[&str]) -> String {
        args.join(" ")
    }
    
    /// Word count (wc)
    pub fn wc(path: &Path, lines: bool, words: bool, chars: bool) -> Result<String> {
        let content = fs::read_to_string(path)?;
        
        let mut output = String::new();
        
        if lines {
            let line_count = content.lines().count();
            output.push_str(&format!("{} ", line_count));
        }
        
        if words {
            let word_count = content.split_whitespace().count();
            output.push_str(&format!("{} ", word_count));
        }
        
        if chars {
            let char_count = content.chars().count();
            output.push_str(&format!("{} ", char_count));
        }
        
        output.push_str(&path.display().to_string());
        Ok(output)
    }
    
    /// Grep - search for patterns in files
    pub fn grep(pattern: &str, paths: &[&Path], ignore_case: bool) -> Result<String> {
        let mut output = String::new();
        let pattern = if ignore_case {
            pattern.to_lowercase()
        } else {
            pattern.to_string()
        };
        
        for path in paths {
            if !path.exists() {
                continue;
            }
            
            let content = fs::read_to_string(path)?;
            for (line_num, line) in content.lines().enumerate() {
                let search_line = if ignore_case {
                    line.to_lowercase()
                } else {
                    line.to_string()
                };
                
                if search_line.contains(&pattern) {
                    if paths.len() > 1 {
                        output.push_str(&format!("{}:{}:{}\n", 
                            path.display(), line_num + 1, line));
                    } else {
                        output.push_str(&format!("{}:{}\n", line_num + 1, line));
                    }
                }
            }
        }
        
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_pattern_matching() {
        assert!(CoreUtils::matches_pattern("test.txt", "*.txt"));
        assert!(CoreUtils::matches_pattern("hello.rs", "*.rs"));
        assert!(!CoreUtils::matches_pattern("test.txt", "*.rs"));
        assert!(CoreUtils::matches_pattern("test", "test"));
        assert!(CoreUtils::matches_pattern("hello_world.txt", "*world*"));
    }
    
    #[test]
    fn test_mkdir_and_rm() {
        let dir = tempdir().unwrap();
        let test_path = dir.path().join("test_dir");
        
        CoreUtils::mkdir(&test_path, false).unwrap();
        assert!(test_path.exists());
        
        CoreUtils::rm(&test_path, false, false).unwrap();
        assert!(!test_path.exists());
    }
    
    #[test]
    fn test_touch() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        
        CoreUtils::touch(&file_path).unwrap();
        assert!(file_path.exists());
    }
}