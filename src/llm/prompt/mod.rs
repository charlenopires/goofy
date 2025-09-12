//! Prompt management system for different LLM tasks
//!
//! This module provides specialized prompts for different scenarios,
//! matching Crush's prompt system implementation.

mod coder;
mod title;
mod task;
mod summarizer;

pub use coder::CoderPrompt;
pub use title::TitlePrompt;
pub use task::TaskPrompt;
pub use summarizer::SummarizerPrompt;

use std::path::{Path, PathBuf};
use std::fs;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashSet;

/// Prompt identifier for different use cases
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptId {
    Coder,
    Title,
    Task,
    Summarizer,
    Default,
}

/// Get a prompt by ID with optional context paths
pub fn get_prompt(prompt_id: PromptId, provider: &str, context_paths: &[PathBuf]) -> String {
    match prompt_id {
        PromptId::Coder => CoderPrompt::generate(provider, context_paths),
        PromptId::Title => TitlePrompt::generate(),
        PromptId::Task => TaskPrompt::generate(),
        PromptId::Summarizer => SummarizerPrompt::generate(),
        PromptId::Default => "You are a helpful assistant".to_string(),
    }
}

/// Process context paths and read their contents
pub async fn process_context_paths(working_dir: &Path, paths: &[PathBuf]) -> String {
    let mut contents = Vec::new();
    let mut processed_files = HashSet::new();
    
    for path in paths {
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            working_dir.join(path)
        };
        
        // Skip if already processed
        if processed_files.contains(&full_path) {
            continue;
        }
        processed_files.insert(full_path.clone());
        
        if let Ok(metadata) = fs::metadata(&full_path) {
            if metadata.is_file() {
                if let Ok(content) = fs::read_to_string(&full_path) {
                    contents.push(format!(
                        "## File: {}\n\n```\n{}\n```\n",
                        full_path.display(),
                        content
                    ));
                }
            } else if metadata.is_dir() {
                // Process directory contents recursively (limited depth)
                if let Ok(entries) = fs::read_dir(&full_path) {
                    for entry in entries.flatten() {
                        if let Ok(metadata) = entry.metadata() {
                            if metadata.is_file() {
                                if let Ok(content) = fs::read_to_string(entry.path()) {
                                    contents.push(format!(
                                        "## File: {}\n\n```\n{}\n```\n",
                                        entry.path().display(),
                                        content
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    contents.join("\n")
}

/// Expand path with tilde and environment variables
pub fn expand_path(path: &str) -> PathBuf {
    let mut expanded = path.to_string();
    
    // Handle tilde expansion
    if expanded.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            expanded = expanded.replacen("~", &home, 1);
        }
    }
    
    // Handle environment variable expansion
    if expanded.starts_with('$') {
        let var_name = &expanded[1..];
        if let Ok(value) = std::env::var(var_name) {
            expanded = value;
        }
    }
    
    PathBuf::from(expanded)
}