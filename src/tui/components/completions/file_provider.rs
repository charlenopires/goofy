//! File system completion provider for paths and filenames

use super::{CompletionItem, CompletionContext, CompletionProvider, ProviderConfig};
use crate::utils::fs::{walk_directory, WalkConfig, FileInfo};
use anyhow::{Result, Context as AnyhowContext};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::env;
use tracing::{debug, warn};

/// File system completion provider
#[derive(Debug, Clone)]
pub struct FileProvider {
    config: ProviderConfig,
    max_depth: usize,
    show_hidden: bool,
    max_results: usize,
    include_directories: bool,
    working_directory: Option<PathBuf>,
}

impl FileProvider {
    /// Create a new file provider
    pub fn new() -> Self {
        Self {
            config: ProviderConfig::default(),
            max_depth: 3,
            show_hidden: false,
            max_results: 50,
            include_directories: true,
            working_directory: None,
        }
    }

    /// Set maximum directory traversal depth
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Enable/disable hidden file completion
    pub fn with_hidden_files(mut self, show_hidden: bool) -> Self {
        self.show_hidden = show_hidden;
        self
    }

    /// Set maximum number of results
    pub fn with_max_results(mut self, max_results: usize) -> Self {
        self.max_results = max_results;
        self
    }

    /// Enable/disable directory completion
    pub fn with_directories(mut self, include_directories: bool) -> Self {
        self.include_directories = include_directories;
        self
    }

    /// Set working directory
    pub fn with_working_directory(mut self, dir: PathBuf) -> Self {
        self.working_directory = Some(dir);
        self
    }

    /// Get the base directory for completions
    fn get_base_directory(&self, context: &CompletionContext) -> PathBuf {
        // Use context working directory first
        if let Some(ref wd) = context.working_dir {
            return PathBuf::from(wd);
        }
        
        // Use provider working directory
        if let Some(ref wd) = self.working_directory {
            return wd.clone();
        }
        
        // Fall back to current directory
        env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    }

    /// Parse the path context from input
    fn parse_path_context(&self, text: &str, cursor_pos: usize) -> (PathBuf, String) {
        let input = &text[..cursor_pos];
        
        // Find the start of the current path component
        let start = input.rfind(|c: char| c.is_whitespace())
            .map(|i| i + 1)
            .unwrap_or(0);
        
        let path_text = &input[start..];
        
        // Split into directory and partial filename
        if let Some(last_sep) = path_text.rfind(['/', '\\']) {
            let dir_part = &path_text[..=last_sep];
            let file_part = &path_text[last_sep + 1..];
            
            (PathBuf::from(dir_part), file_part.to_string())
        } else {
            // No directory separator, complete in current directory
            (PathBuf::from("."), path_text.to_string())
        }
    }

    /// Generate completions for a directory
    async fn complete_directory(&self, dir_path: &Path, prefix: &str, base_dir: &Path) -> Result<Vec<CompletionItem>> {
        debug!("Completing directory: {} with prefix: '{}'", dir_path.display(), prefix);
        
        let mut items = Vec::new();
        
        // Make path absolute for walking
        let search_path = if dir_path.is_absolute() {
            dir_path.to_path_buf()
        } else {
            base_dir.join(dir_path)
        };
        
        if !search_path.exists() || !search_path.is_dir() {
            debug!("Directory does not exist: {}", search_path.display());
            return Ok(items);
        }

        let walk_config = WalkConfig {
            max_depth: Some(1), // Only immediate children
            include_hidden: self.show_hidden,
            include_extensions: vec![],
            ignore_patterns: if self.show_hidden {
                vec![] // Don't ignore anything if showing hidden
            } else {
                vec![
                    ".git".to_string(),
                    ".DS_Store".to_string(),
                    "node_modules".to_string(),
                    "target".to_string(),
                    ".cache".to_string(),
                ]
            },
            follow_links: false,
        };

        let files = walk_directory(&search_path, Some(walk_config))
            .context("Failed to walk directory")?;

        for file_info in files {
            let filename = file_info.path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Skip if doesn't match prefix
            if !prefix.is_empty() && !filename.to_lowercase().starts_with(&prefix.to_lowercase()) {
                continue;
            }

            // Skip current directory entries
            if filename == "." || filename == ".." {
                continue;
            }

            // Skip if not including directories and this is a directory
            if file_info.is_dir && !self.include_directories {
                continue;
            }

            let display_name = if file_info.is_dir {
                format!("{}/", &filename)
            } else {
                filename.clone()
            };

            let completion_value = if dir_path == Path::new(".") {
                filename.clone()
            } else {
                format!("{}{}", dir_path.to_string_lossy(), &filename)
            };

            let description = if file_info.is_dir {
                Some("Directory".to_string())
            } else if let Some(ref ext) = file_info.extension {
                Some(format!("{} file", ext.to_uppercase()))
            } else {
                Some("File".to_string())
            };

            let score = self.calculate_file_score(&filename, prefix, &file_info);

            let item = CompletionItem::new(display_name, completion_value, "file")
                .with_description(description.unwrap_or_default())
                .with_score(score);

            items.push(item);

            if items.len() >= self.max_results {
                break;
            }
        }

        // Sort by score and then alphabetically
        items.sort_by(|a, b| {
            b.score.partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.title.cmp(&b.title))
        });

        Ok(items)
    }

    /// Calculate relevance score for a file
    fn calculate_file_score(&self, filename: &str, prefix: &str, file_info: &FileInfo) -> f64 {
        let mut score: f64 = 1.0;

        // Exact prefix match gets higher score
        if filename.to_lowercase().starts_with(&prefix.to_lowercase()) {
            score += 0.5;
        }

        // Directories get slight boost if we're including them
        if file_info.is_dir && self.include_directories {
            score += 0.1;
        }

        // Recently modified files get boost
        if let Some(modified) = file_info.modified {
            if let Ok(elapsed) = modified.elapsed() {
                if elapsed.as_secs() < 3600 { // Within last hour
                    score += 0.2;
                } else if elapsed.as_secs() < 86400 { // Within last day
                    score += 0.1;
                }
            }
        }

        // Common file types get slight boost
        if let Some(ref ext) = file_info.extension {
            match ext.as_str() {
                "rs" | "go" | "py" | "js" | "ts" | "md" | "txt" => score += 0.1,
                "json" | "yaml" | "toml" | "cfg" => score += 0.05,
                _ => {}
            }
        }

        // Penalty for very long names
        if filename.len() > 50 {
            score -= 0.1;
        }

        // Penalty for hidden files (unless explicitly showing them)
        if filename.starts_with('.') && !self.show_hidden {
            score -= 0.2;
        }

        score.max(0.1_f64) // Minimum score
    }

    /// Complete environment variables (for paths starting with $)
    fn complete_environment_variables(&self, prefix: &str) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        
        if !prefix.starts_with('$') {
            return items;
        }

        let var_prefix = &prefix[1..]; // Remove $
        let common_vars = [
            ("HOME", "User home directory"),
            ("PATH", "Executable search path"),
            ("PWD", "Current working directory"),
            ("USER", "Current user name"),
            ("SHELL", "Current shell"),
            ("TEMP", "Temporary directory"),
            ("TMP", "Temporary directory"),
            ("CARGO_HOME", "Cargo home directory"),
            ("RUST_LOG", "Rust logging configuration"),
        ];

        for (var_name, description) in &common_vars {
            if var_name.to_lowercase().starts_with(&var_prefix.to_lowercase()) {
                if let Ok(value) = env::var(var_name) {
                    let item = CompletionItem::new(
                        format!("${}", var_name),
                        format!("${}", var_name),
                        "env"
                    )
                    .with_description(format!("{}: {}", description, value))
                    .with_score(0.8);
                    
                    items.push(item);
                }
            }
        }

        items
    }
}

impl Default for FileProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CompletionProvider for FileProvider {
    fn name(&self) -> &str {
        "file"
    }

    async fn get_completions(&self, context: &CompletionContext) -> Result<Vec<CompletionItem>> {
        let current_word = context.current_word();
        
        // Handle environment variables
        if current_word.starts_with('$') {
            return Ok(self.complete_environment_variables(current_word));
        }

        let base_dir = self.get_base_directory(context);
        let (dir_path, prefix) = self.parse_path_context(&context.text, context.cursor_pos);
        
        debug!("File completion - base: {}, dir: {}, prefix: '{}'", 
               base_dir.display(), dir_path.display(), prefix);

        self.complete_directory(&dir_path, &prefix, &base_dir).await
    }

    fn is_applicable(&self, context: &CompletionContext) -> bool {
        // Check the text before cursor for path indicators
        let text_before = &context.text[..context.cursor_pos];
        // Find the current token (after last whitespace)
        let token_start = text_before.rfind(char::is_whitespace)
            .map(|i| i + 1)
            .unwrap_or(0);
        let current_token = &text_before[token_start..];

        // Apply to file paths or environment variables
        current_token.contains('/') ||
        current_token.contains('\\') ||
        current_token.starts_with('.') ||
        current_token.starts_with('$') ||
        current_token.starts_with('~')
    }

    fn get_priority(&self, context: &CompletionContext) -> i32 {
        if self.is_applicable(context) {
            10 // High priority for file paths
        } else {
            2  // Low priority for general text
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_provider_basic() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test files
        fs::write(temp_path.join("test1.rs"), "// Test file 1").unwrap();
        fs::write(temp_path.join("test2.py"), "# Test file 2").unwrap();
        fs::create_dir(temp_path.join("testdir")).unwrap();

        let provider = FileProvider::new()
            .with_working_directory(temp_path.to_path_buf());

        let context = CompletionContext {
            text: "test".to_string(),
            cursor_pos: 4,
            working_dir: Some(temp_path.to_string_lossy().to_string()),
            ..Default::default()
        };

        let completions = provider.get_completions(&context).await.unwrap();

        assert!(!completions.is_empty());
        assert!(completions.iter().any(|c| c.title.contains("test1.rs")));
        assert!(completions.iter().any(|c| c.title.contains("test2.py")));
        assert!(completions.iter().any(|c| c.title.contains("testdir")));
    }

    #[tokio::test]
    async fn test_file_provider_path_parsing() {
        let provider = FileProvider::new();
        
        // Test various path formats
        let (dir, prefix) = provider.parse_path_context("src/main.rs", 8);
        assert_eq!(dir, PathBuf::from("src/"));
        assert_eq!(prefix, "main"); // cursor at pos 8 gives "src/main"
        
        let (dir, prefix) = provider.parse_path_context("./config", 8);
        assert_eq!(dir, PathBuf::from("./"));
        assert_eq!(prefix, "config");
        
        let (dir, prefix) = provider.parse_path_context("filename", 8);
        assert_eq!(dir, PathBuf::from("."));
        assert_eq!(prefix, "filename");
    }

    #[test]
    fn test_environment_variable_completion() {
        let provider = FileProvider::new();
        
        // Test environment variable completion
        let completions = provider.complete_environment_variables("$HO");
        
        // Should find $HOME if it exists
        if env::var("HOME").is_ok() {
            assert!(completions.iter().any(|c| c.title == "$HOME"));
        }
    }

    #[test]
    fn test_file_provider_applicability() {
        let provider = FileProvider::new();
        
        // Should apply to file paths
        let context1 = CompletionContext::new("./src/main.rs", 10);
        assert!(provider.is_applicable(&context1));
        
        let context2 = CompletionContext::new("config.json", 6);
        assert!(!provider.is_applicable(&context2)); // Simple filename, not a path
        
        let context3 = CompletionContext::new("$HOME/docs", 6);
        assert!(provider.is_applicable(&context3));
        
        let context4 = CompletionContext::new("../parent", 8);
        assert!(provider.is_applicable(&context4));
    }
}