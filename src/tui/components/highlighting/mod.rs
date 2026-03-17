//! Syntax highlighting system for Goofy TUI
//! 
//! This module provides comprehensive syntax highlighting capabilities
//! for code blocks and file content, using syntect for Rust-based
//! highlighting with theme integration.

use anyhow::Result;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::collections::HashMap;
use syntect::{
    easy::HighlightLines,
    highlighting::{Theme as SyntectTheme, ThemeSet},
    parsing::{SyntaxSet, SyntaxReference},
    util::LinesWithEndings,
};

pub mod chroma;
pub mod themes;

/// Syntax highlighter for code content
#[derive(Debug)]
pub struct SyntaxHighlighter {
    /// Syntax set for language detection
    syntax_set: SyntaxSet,
    
    /// Available highlighting themes
    theme_set: ThemeSet,
    
    /// Current theme name
    current_theme: String,
    
    /// Language syntax cache
    syntax_cache: HashMap<String, String>,
    
    /// Configuration options
    config: HighlightConfig,
}

/// Configuration for syntax highlighting
#[derive(Debug, Clone)]
pub struct HighlightConfig {
    /// Whether to enable syntax highlighting
    pub enabled: bool,
    
    /// Fallback theme name
    pub fallback_theme: String,
    
    /// Whether to show line numbers
    pub show_line_numbers: bool,
    
    /// Line number width
    pub line_number_width: usize,
    
    /// Whether to highlight current line
    pub highlight_current_line: bool,
    
    /// Tab width for rendering
    pub tab_width: usize,
    
    /// Maximum lines to highlight (performance limit)
    pub max_lines: usize,
}

impl Default for HighlightConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            fallback_theme: "base16-ocean.dark".to_string(),
            show_line_numbers: true,
            line_number_width: 4,
            highlight_current_line: false,
            tab_width: 4,
            max_lines: 10000,
        }
    }
}

/// Highlighted content with styling information
#[derive(Debug, Clone)]
pub struct HighlightedContent {
    /// Highlighted lines with styling
    pub lines: Vec<Line<'static>>,
    
    /// Original language name
    pub language: String,
    
    /// Theme used for highlighting
    pub theme: String,
    
    /// Total number of lines
    pub line_count: usize,
}

/// Language detection result
#[derive(Debug, Clone)]
pub struct LanguageInfo {
    /// Language name
    pub name: String,
    
    /// File extensions
    pub extensions: Vec<String>,
    
    /// MIME types
    pub mime_types: Vec<String>,
    
    /// Whether the language supports highlighting
    pub highlightable: bool,
}

impl SyntaxHighlighter {
    /// Create a new syntax highlighter
    pub fn new() -> Result<Self> {
        Self::with_config(HighlightConfig::default())
    }
    
    /// Create a new syntax highlighter with custom configuration
    pub fn with_config(config: HighlightConfig) -> Result<Self> {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        
        Ok(Self {
            syntax_set,
            theme_set,
            current_theme: config.fallback_theme.clone(),
            syntax_cache: HashMap::new(),
            config,
        })
    }
    
    /// Highlight code with automatic language detection
    pub fn highlight(&mut self, code: &str, filename: Option<&str>) -> Result<HighlightedContent> {
        if !self.config.enabled {
            return Ok(self.create_plain_content(code));
        }

        // Detect syntax and cache the name to avoid borrow issues
        let syntax_name = {
            let syntax = self.detect_syntax(code, filename)?;
            syntax.name.clone()
        };

        let syntax = self.syntax_set.find_syntax_by_name(&syntax_name)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
        let theme = self.get_current_theme()?;

        self.highlight_with_syntax(code, syntax, theme)
    }
    
    /// Highlight code with explicit language
    pub fn highlight_language(&mut self, code: &str, language: &str) -> Result<HighlightedContent> {
        if !self.config.enabled {
            return Ok(self.create_plain_content(code));
        }

        // Try exact match first, then case-insensitive, then extension
        let syntax = self.syntax_set.find_syntax_by_name(language)
            .or_else(|| {
                // Try title-cased name (e.g., "rust" -> "Rust")
                let title_case = {
                    let mut c = language.chars();
                    match c.next() {
                        None => String::new(),
                        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                    }
                };
                self.syntax_set.find_syntax_by_name(&title_case)
            })
            .or_else(|| self.syntax_set.find_syntax_by_extension(language))
            .or_else(|| {
                // Try common language name to extension mappings
                let ext = match language.to_lowercase().as_str() {
                    "rust" => Some("rs"),
                    "python" => Some("py"),
                    "javascript" => Some("js"),
                    "typescript" => Some("ts"),
                    "ruby" => Some("rb"),
                    "csharp" | "c#" => Some("cs"),
                    "cpp" | "c++" => Some("cpp"),
                    "golang" | "go" => Some("go"),
                    "shell" | "bash" | "sh" => Some("sh"),
                    _ => None,
                };
                ext.and_then(|e| self.syntax_set.find_syntax_by_extension(e))
            })
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme = self.get_current_theme()?;
        self.highlight_with_syntax(code, syntax, &theme)
    }
    
    /// Detect syntax from code content and filename
    fn detect_syntax(&mut self, code: &str, filename: Option<&str>) -> Result<&SyntaxReference> {
        // Try filename-based detection first
        if let Some(filename) = filename {
            if let Some(cached) = self.syntax_cache.get(filename) {
                if let Some(syntax) = self.syntax_set.find_syntax_by_name(cached) {
                    return Ok(syntax);
                }
            }
            
            // Try extension-based detection
            if let Some(extension) = std::path::Path::new(filename).extension() {
                if let Some(ext_str) = extension.to_str() {
                    if let Some(syntax) = self.syntax_set.find_syntax_by_extension(ext_str) {
                        self.syntax_cache.insert(filename.to_string(), syntax.name.clone());
                        return Ok(syntax);
                    }
                }
            }
            
            // Try filename pattern matching using find_syntax_for_file
            if let Some(syntax) = self.syntax_set.find_syntax_for_file(filename)
                .ok()
                .flatten()
            {
                self.syntax_cache.insert(filename.to_string(), syntax.name.clone());
                return Ok(syntax);
            }
        }
        
        // Try content-based detection
        if let Some(syntax) = self.syntax_set.find_syntax_by_first_line(code) {
            return Ok(syntax);
        }
        
        // Fallback to plain text
        Ok(self.syntax_set.find_syntax_plain_text())
    }
    
    /// Highlight code with specific syntax and theme
    fn highlight_with_syntax(
        &self, 
        code: &str, 
        syntax: &SyntaxReference, 
        theme: &SyntectTheme
    ) -> Result<HighlightedContent> {
        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut lines = Vec::new();
        
        for (line_num, line) in LinesWithEndings::from(code).enumerate() {
            if line_num >= self.config.max_lines {
                break;
            }
            
            let highlighted = highlighter.highlight_line(line, &self.syntax_set)?;
            let rendered_line = self.render_highlighted_line(
                &highlighted, 
                line_num + 1, 
                line.trim_end_matches('\n')
            );
            
            lines.push(rendered_line);
        }
        
        Ok(HighlightedContent {
            lines,
            language: syntax.name.clone(),
            theme: self.current_theme.clone(),
            line_count: code.lines().count(),
        })
    }
    
    /// Render a highlighted line with optional line numbers
    fn render_highlighted_line(
        &self,
        highlighted: &[(syntect::highlighting::Style, &str)],
        line_number: usize,
        original_line: &str,
    ) -> Line<'static> {
        let mut spans = Vec::new();
        
        // Add line number if enabled
        if self.config.show_line_numbers {
            let line_num_str = format!("{:width$} ", line_number, width = self.config.line_number_width);
            spans.push(Span::styled(
                line_num_str,
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::DIM),
            ));
        }
        
        // Add highlighted content
        for (style, text) in highlighted {
            let expanded_text = text.replace('\t', &" ".repeat(self.config.tab_width));
            
            let fg_color = Color::Rgb(
                style.foreground.r,
                style.foreground.g,
                style.foreground.b,
            );
            
            let mut span_style = Style::default().fg(fg_color);
            
            // Apply text styling
            if style.font_style.contains(syntect::highlighting::FontStyle::BOLD) {
                span_style = span_style.add_modifier(Modifier::BOLD);
            }
            if style.font_style.contains(syntect::highlighting::FontStyle::ITALIC) {
                span_style = span_style.add_modifier(Modifier::ITALIC);
            }
            if style.font_style.contains(syntect::highlighting::FontStyle::UNDERLINE) {
                span_style = span_style.add_modifier(Modifier::UNDERLINED);
            }
            
            spans.push(Span::styled(expanded_text, span_style));
        }
        
        // If empty line, add a space to maintain layout
        if spans.len() == if self.config.show_line_numbers { 1 } else { 0 } {
            spans.push(Span::raw(" "));
        }
        
        Line::from(spans)
    }
    
    /// Create plain content without highlighting
    fn create_plain_content(&self, code: &str) -> HighlightedContent {
        let lines: Vec<Line> = code
            .lines()
            .enumerate()
            .take(self.config.max_lines)
            .map(|(line_num, line)| {
                let mut spans = Vec::new();
                
                if self.config.show_line_numbers {
                    let line_num_str = format!("{:width$} ", line_num + 1, width = self.config.line_number_width);
                    spans.push(Span::styled(
                        line_num_str,
                        Style::default().fg(Color::DarkGray),
                    ));
                }
                
                let expanded_line = line.replace('\t', &" ".repeat(self.config.tab_width));
                spans.push(Span::raw(expanded_line));
                
                Line::from(spans)
            })
            .collect();
        
        HighlightedContent {
            lines,
            language: "text".to_string(),
            theme: "none".to_string(),
            line_count: code.lines().count(),
        }
    }
    
    /// Get the current highlighting theme
    fn get_current_theme(&self) -> Result<&SyntectTheme> {
        self.theme_set
            .themes
            .get(&self.current_theme)
            .or_else(|| self.theme_set.themes.get(&self.config.fallback_theme))
            .or_else(|| self.theme_set.themes.values().next())
            .ok_or_else(|| anyhow::anyhow!("No themes available"))
    }
    
    /// Set the current theme
    pub fn set_theme(&mut self, theme_name: &str) -> Result<()> {
        if self.theme_set.themes.contains_key(theme_name) {
            self.current_theme = theme_name.to_string();
            Ok(())
        } else {
            Err(anyhow::anyhow!("Theme not found: {}", theme_name))
        }
    }
    
    /// Get available theme names
    pub fn available_themes(&self) -> Vec<String> {
        self.theme_set.themes.keys().cloned().collect()
    }
    
    /// Get supported languages
    pub fn supported_languages(&self) -> Vec<LanguageInfo> {
        self.syntax_set
            .syntaxes()
            .iter()
            .map(|syntax| LanguageInfo {
                name: syntax.name.clone(),
                extensions: syntax.file_extensions.clone(),
                mime_types: vec![], // syntect doesn't provide MIME types directly
                highlightable: !syntax.hidden,
            })
            .collect()
    }
    
    /// Get language info by name
    pub fn get_language_info(&self, name: &str) -> Option<LanguageInfo> {
        self.syntax_set
            .find_syntax_by_name(name)
            .map(|syntax| LanguageInfo {
                name: syntax.name.clone(),
                extensions: syntax.file_extensions.clone(),
                mime_types: vec![],
                highlightable: !syntax.hidden,
            })
    }
    
    /// Clear syntax cache
    pub fn clear_cache(&mut self) {
        self.syntax_cache.clear();
    }
    
    /// Get current configuration
    pub fn config(&self) -> &HighlightConfig {
        &self.config
    }
    
    /// Update configuration
    pub fn set_config(&mut self, config: HighlightConfig) {
        self.config = config;
    }
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new().expect("Failed to create default syntax highlighter")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_highlighter_creation() {
        let highlighter = SyntaxHighlighter::new();
        assert!(highlighter.is_ok());
    }
    
    #[test]
    fn test_plain_highlighting() {
        let mut highlighter = SyntaxHighlighter::new().unwrap();
        let code = "fn main() {\n    println!(\"Hello, world!\");\n}";
        let result = highlighter.highlight(code, Some("main.rs"));
        
        assert!(result.is_ok());
        let highlighted = result.unwrap();
        assert_eq!(highlighted.language, "Rust");
        assert!(highlighted.lines.len() > 0);
    }
    
    #[test]
    fn test_language_detection() {
        let mut highlighter = SyntaxHighlighter::new().unwrap();
        
        // Test Rust detection
        let rust_code = "fn main() {}";
        let result = highlighter.highlight(rust_code, Some("test.rs")).unwrap();
        assert_eq!(result.language, "Rust");
        
        // Test Python detection
        let python_code = "def main():\n    pass";
        let result = highlighter.highlight(python_code, Some("test.py")).unwrap();
        assert_eq!(result.language, "Python");
        
        // Test JavaScript detection
        let js_code = "function main() {}";
        let result = highlighter.highlight(js_code, Some("test.js")).unwrap();
        assert_eq!(result.language, "JavaScript");
    }
    
    #[test]
    fn test_theme_switching() {
        let mut highlighter = SyntaxHighlighter::new().unwrap();
        let themes = highlighter.available_themes();
        
        assert!(!themes.is_empty());
        
        if let Some(theme) = themes.first() {
            assert!(highlighter.set_theme(theme).is_ok());
        }
    }
    
    #[test]
    fn test_language_info() {
        let highlighter = SyntaxHighlighter::new().unwrap();
        let languages = highlighter.supported_languages();
        
        assert!(!languages.is_empty());
        
        let rust_info = highlighter.get_language_info("Rust");
        assert!(rust_info.is_some());
        
        let info = rust_info.unwrap();
        assert_eq!(info.name, "Rust");
        assert!(info.extensions.contains(&"rs".to_string()));
    }
    
    #[test]
    fn test_configuration() {
        let config = HighlightConfig {
            show_line_numbers: false,
            tab_width: 8,
            ..Default::default()
        };
        
        let mut highlighter = SyntaxHighlighter::with_config(config).unwrap();
        assert!(!highlighter.config().show_line_numbers);
        assert_eq!(highlighter.config().tab_width, 8);
        
        let new_config = HighlightConfig {
            show_line_numbers: true,
            ..highlighter.config().clone()
        };
        
        highlighter.set_config(new_config);
        assert!(highlighter.config().show_line_numbers);
    }
}