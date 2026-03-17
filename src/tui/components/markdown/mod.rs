//! Advanced markdown rendering for Goofy TUI
//! 
//! This module provides comprehensive markdown rendering capabilities
//! that integrate with the syntax highlighting and theme systems,
//! supporting rich text formatting, code blocks, tables, and images.

use anyhow::Result;
use pulldown_cmark::{Parser, Event, Tag, TagEnd, CodeBlockKind, CowStr, HeadingLevel};
use ratatui::{
    layout::Rect,
    style::{Color, Style, Modifier},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Widget, Paragraph, Wrap},
};
use std::collections::HashMap;

use crate::tui::{
    themes::{Theme, ThemeManager},
    components::highlighting::{SyntaxHighlighter, HighlightConfig},
};

pub mod renderer;
pub mod styles;
pub mod table;
pub mod image;

use renderer::MarkdownRenderer;
use styles::MarkdownStyles;

/// Markdown display component for TUI
#[derive(Debug)]
pub struct MarkdownWidget {
    /// Markdown content
    content: String,
    
    /// Display configuration
    config: MarkdownConfig,
    
    /// Current theme
    theme: Option<Theme>,
    
    /// Cached rendered content
    cached_content: Option<Text<'static>>,
    
    /// Cache invalidation flag
    cache_dirty: bool,
}

/// Configuration for markdown display
#[derive(Debug, Clone)]
pub struct MarkdownConfig {
    /// Maximum width for text wrapping
    pub max_width: u16,
    
    /// Whether to show line numbers for code blocks
    pub show_line_numbers: bool,
    
    /// Syntax highlighting configuration
    pub highlight_config: HighlightConfig,
    
    /// Whether to render images (if supported)
    pub render_images: bool,
    
    /// Whether to render tables
    pub render_tables: bool,
    
    /// Border style
    pub border: Option<Borders>,
    
    /// Title display
    pub title: Option<String>,
    
    /// Base indentation level
    pub base_indent: u16,
    
    /// List item indent
    pub list_indent: u16,
    
    /// Quote block indent
    pub quote_indent: u16,
    
    /// Code block margins
    pub code_margin: u16,
}

/// Markdown rendering context
#[derive(Debug)]
struct RenderContext {
    /// Current line buffer
    current_line: Vec<Span<'static>>,
    
    /// All rendered lines
    lines: Vec<Line<'static>>,
    
    /// Current indentation level
    indent_level: u16,
    
    /// Current list nesting level
    list_level: u16,
    
    /// Whether we're in a code block
    in_code_block: bool,
    
    /// Current code block language
    code_language: Option<String>,
    
    /// Whether we're in a quote block
    in_quote: bool,
    
    /// Current table state
    table_state: Option<TableState>,
    
    /// Theme styles
    styles: MarkdownStyles,
}

/// Table rendering state
#[derive(Debug)]
struct TableState {
    /// Table headers
    headers: Vec<String>,
    
    /// Table rows
    rows: Vec<Vec<String>>,
    
    /// Current row being built
    current_row: Vec<String>,
    
    /// Current cell content
    current_cell: String,
    
    /// Whether we're in header row
    in_header: bool,
}

impl Default for MarkdownConfig {
    fn default() -> Self {
        Self {
            max_width: 80,
            show_line_numbers: true,
            highlight_config: HighlightConfig::default(),
            render_images: true,
            render_tables: true,
            border: Some(Borders::ALL),
            title: None,
            base_indent: 0,
            list_indent: 2,
            quote_indent: 2,
            code_margin: 1,
        }
    }
}

impl MarkdownWidget {
    /// Create a new markdown widget
    pub fn new() -> Self {
        Self {
            content: String::new(),
            config: MarkdownConfig::default(),
            theme: None,
            cached_content: None,
            cache_dirty: true,
        }
    }
    
    /// Create with custom configuration
    pub fn with_config(config: MarkdownConfig) -> Self {
        Self {
            content: String::new(),
            config,
            theme: None,
            cached_content: None,
            cache_dirty: true,
        }
    }
    
    /// Set markdown content
    pub fn set_content<S: Into<String>>(&mut self, content: S) {
        self.content = content.into();
        self.cache_dirty = true;
    }
    
    /// Get current content
    pub fn content(&self) -> &str {
        &self.content
    }
    
    /// Set configuration
    pub fn set_config(&mut self, config: MarkdownConfig) {
        self.config = config;
        self.cache_dirty = true;
    }
    
    /// Get current configuration
    pub fn config(&self) -> &MarkdownConfig {
        &self.config
    }
    
    /// Set theme
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = Some(theme);
        self.cache_dirty = true;
    }
    
    /// Clear cached content
    pub fn clear_cache(&mut self) {
        self.cached_content = None;
        self.cache_dirty = true;
    }
    
    /// Render markdown content to Text
    pub fn render_to_text(&mut self, area: Rect) -> Result<Text<'static>> {
        if !self.cache_dirty && self.cached_content.is_some() {
            return Ok(self.cached_content.as_ref().unwrap().clone());
        }

        let default_theme = Theme::default();
        let theme = self.theme.as_ref()
            .unwrap_or(&default_theme);

        let styles = MarkdownStyles::from_theme(theme);
        let renderer = MarkdownRenderer::new(&self.config, styles);

        let text = renderer.render(&self.content, area.width)?;

        // Cache the result
        self.cached_content = Some(text.clone());
        self.cache_dirty = false;

        Ok(text)
    }
    
    /// Render markdown content from string
    pub fn render_string(content: &str, config: &MarkdownConfig, theme: &Theme, width: u16) -> Result<Text<'static>> {
        let styles = MarkdownStyles::from_theme(theme);
        let renderer = MarkdownRenderer::new(config, styles);
        renderer.render(content, width)
    }
}

impl Default for MarkdownWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for MarkdownWidget {
    fn render(mut self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        // Create a block if border is configured
        let block = if let Some(borders) = self.config.border {
            let mut block = Block::default().borders(borders);
            if let Some(title) = &self.config.title {
                block = block.title(title.as_str());
            }
            Some(block)
        } else {
            None
        };

        // Calculate the inner area
        let inner_area = if let Some(ref block) = block {
            let outer_area = area;
            block.clone().render(outer_area, buf);
            block.inner(outer_area)
        } else {
            area
        };

        // Render the markdown content
        if let Ok(text) = self.render_to_text(inner_area) {
            let paragraph = Paragraph::new(text)
                .wrap(Wrap { trim: true });

            paragraph.render(inner_area, buf);
        }
    }
}

/// Utility functions for markdown processing
pub mod utils {
    use super::*;
    
    /// Extract plain text from markdown
    pub fn extract_text(markdown: &str) -> String {
        let parser = Parser::new(markdown);
        let mut text = String::new();

        for event in parser {
            match event {
                Event::Text(content) => {
                    // Avoid double spaces when appending text after an End tag
                    if text.ends_with(' ') && content.starts_with(' ') {
                        text.push_str(content.trim_start());
                    } else {
                        text.push_str(&content);
                    }
                }
                Event::Code(content) => text.push_str(&content),
                Event::SoftBreak | Event::HardBreak => text.push(' '),
                Event::End(_) => {
                    // Add a space after block-level elements to separate content
                    if !text.is_empty() && !text.ends_with(' ') {
                        text.push(' ');
                    }
                }
                _ => {}
            }
        }

        text.trim().to_string()
    }
    
    /// Count lines in markdown content
    pub fn count_lines(markdown: &str) -> usize {
        markdown.lines().count()
    }
    
    /// Extract headings from markdown
    pub fn extract_headings(markdown: &str) -> Vec<(u8, String)> {
        let parser = Parser::new(markdown);
        let mut headings = Vec::new();
        let mut in_heading = false;
        let mut current_level = 1;
        let mut current_text = String::new();
        
        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    in_heading = true;
                    current_level = level as u8;
                    current_text.clear();
                }
                Event::End(TagEnd::Heading(_)) => {
                    if in_heading {
                        headings.push((current_level, current_text.clone()));
                        in_heading = false;
                    }
                }
                Event::Text(content) if in_heading => {
                    current_text.push_str(&content);
                }
                _ => {}
            }
        }
        
        headings
    }
    
    /// Extract code blocks from markdown
    pub fn extract_code_blocks(markdown: &str) -> Vec<(Option<String>, String)> {
        let parser = Parser::new(markdown);
        let mut code_blocks = Vec::new();
        let mut in_code_block = false;
        let mut current_language = None;
        let mut current_code = String::new();
        
        for event in parser {
            match event {
                Event::Start(Tag::CodeBlock(kind)) => {
                    in_code_block = true;
                    current_language = match kind {
                        CodeBlockKind::Fenced(lang) => {
                            if lang.is_empty() {
                                None
                            } else {
                                Some(lang.to_string())
                            }
                        }
                        CodeBlockKind::Indented => None,
                    };
                    current_code.clear();
                }
                Event::End(TagEnd::CodeBlock) => {
                    if in_code_block {
                        code_blocks.push((current_language.clone(), current_code.clone()));
                        in_code_block = false;
                    }
                }
                Event::Text(content) if in_code_block => {
                    current_code.push_str(&content);
                }
                _ => {}
            }
        }
        
        code_blocks
    }
    
    /// Check if markdown contains specific elements
    pub fn contains_tables(markdown: &str) -> bool {
        markdown.contains('|') && markdown.lines().any(|line| {
            let trimmed = line.trim();
            trimmed.starts_with('|') || trimmed.contains("---")
        })
    }
    
    pub fn contains_images(markdown: &str) -> bool {
        markdown.contains("![")
    }
    
    pub fn contains_links(markdown: &str) -> bool {
        markdown.contains("[") && markdown.contains("](")
    }
    
    pub fn contains_code_blocks(markdown: &str) -> bool {
        markdown.contains("```") || markdown.lines().any(|line| line.starts_with("    "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_markdown_widget_creation() {
        let widget = MarkdownWidget::new();
        assert!(widget.content().is_empty());
        assert!(widget.cached_content.is_none());
        assert!(widget.cache_dirty);
    }
    
    #[test]
    fn test_content_setting() {
        let mut widget = MarkdownWidget::new();
        widget.set_content("# Hello World");
        assert_eq!(widget.content(), "# Hello World");
        assert!(widget.cache_dirty);
    }
    
    #[test]
    fn test_config_setting() {
        let mut widget = MarkdownWidget::new();
        let mut config = MarkdownConfig::default();
        config.max_width = 120;
        
        widget.set_config(config.clone());
        assert_eq!(widget.config().max_width, 120);
        assert!(widget.cache_dirty);
    }
    
    #[test]
    fn test_extract_text() {
        let markdown = "# Hello\n\nThis is **bold** text with `code`.";
        let text = utils::extract_text(markdown);
        assert_eq!(text, "Hello This is bold text with code.");
    }
    
    #[test]
    fn test_extract_headings() {
        let markdown = "# H1\n## H2\n### H3";
        let headings = utils::extract_headings(markdown);
        assert_eq!(headings, vec![(1, "H1".to_string()), (2, "H2".to_string()), (3, "H3".to_string())]);
    }
    
    #[test]
    fn test_extract_code_blocks() {
        let markdown = "```rust\nfn main() {}\n```\n\n```\nplain code\n```";
        let blocks = utils::extract_code_blocks(markdown);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].0, Some("rust".to_string()));
        assert_eq!(blocks[0].1, "fn main() {}\n");
        assert_eq!(blocks[1].0, None);
        assert_eq!(blocks[1].1, "plain code\n");
    }
    
    #[test]
    fn test_content_detection() {
        assert!(utils::contains_tables("| a | b |\n|---|---|"));
        assert!(utils::contains_images("![alt](image.png)"));
        assert!(utils::contains_links("[text](url)"));
        assert!(utils::contains_code_blocks("```rust\ncode\n```"));
    }
}