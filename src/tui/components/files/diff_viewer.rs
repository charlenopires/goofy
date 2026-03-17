//! Diff viewer component with unified and split-view modes.
//!
//! This module provides a comprehensive diff viewing interface that supports:
//! - Unified and split-view diff modes
//! - Syntax highlighting for various file types
//! - Line numbers and context display
//! - Scrolling and navigation
//! - Configurable styling and themes

use crate::tui::{
    components::Component,
    themes::Theme,
    Frame,
};
use anyhow::Result;
use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::io::Write;

/// Diff viewer component
#[derive(Debug)]
pub struct DiffViewer {
    /// Layout mode (unified or split)
    layout: DiffLayout,
    
    /// Before file (left side in split view)
    before_file: DiffFile,
    
    /// After file (right side in split view)
    after_file: DiffFile,
    
    /// Diff configuration
    config: DiffConfig,
    
    /// Computed diff hunks
    hunks: Vec<DiffHunk>,
    
    /// Current scroll offset
    scroll_offset: usize,
    
    /// Current horizontal offset
    horizontal_offset: usize,
    
    /// Component area
    area: Rect,
    
    /// Whether component has focus
    has_focus: bool,
    
    /// Syntax highlighting cache
    syntax_cache: HashMap<String, Vec<Line<'static>>>,
    
    /// Error message
    error_message: Option<String>,
}

/// Diff layout modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLayout {
    /// Unified view (traditional diff format)
    Unified,
    /// Split view (side-by-side comparison)
    Split,
}

/// File information for diff
#[derive(Debug, Clone)]
pub struct DiffFile {
    /// File path
    pub path: PathBuf,
    /// File content
    pub content: String,
    /// File language for syntax highlighting
    pub language: Option<String>,
}

impl DiffFile {
    /// Create a new diff file
    pub fn new<P: AsRef<Path>>(path: P, content: String) -> Self {
        let path = path.as_ref().to_path_buf();
        let language = detect_language(&path);
        
        Self {
            path,
            content,
            language,
        }
    }
    
    /// Create from file path (reads content)
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)?;
        Ok(Self::new(path, content))
    }
}

/// Diff configuration
#[derive(Debug, Clone)]
pub struct DiffConfig {
    /// Number of context lines around changes
    pub context_lines: usize,
    
    /// Whether to show line numbers
    pub show_line_numbers: bool,
    
    /// Tab width for display
    pub tab_width: usize,
    
    /// Whether to enable syntax highlighting
    pub enable_syntax_highlighting: bool,
    
    /// Maximum line width before wrapping
    pub max_line_width: usize,
    
    /// Whether to show whitespace changes
    pub show_whitespace: bool,
    
    /// Styling configuration
    pub styling: DiffStyling,
}

impl Default for DiffConfig {
    fn default() -> Self {
        Self {
            context_lines: 3,
            show_line_numbers: true,
            tab_width: 4,
            enable_syntax_highlighting: true,
            max_line_width: 120,
            show_whitespace: false,
            styling: DiffStyling::default(),
        }
    }
}

/// Styling configuration for diff viewer
#[derive(Debug, Clone)]
pub struct DiffStyling {
    /// Style for unchanged lines
    pub equal_style: Style,
    
    /// Style for added lines
    pub insert_style: Style,
    
    /// Style for removed lines
    pub delete_style: Style,
    
    /// Style for line numbers
    pub line_number_style: Style,
    
    /// Style for hunk headers
    pub hunk_header_style: Style,
    
    /// Style for context
    pub context_style: Style,
}

impl Default for DiffStyling {
    fn default() -> Self {
        Self {
            equal_style: Style::default(),
            insert_style: Style::default()
                .bg(Color::Rgb(40, 60, 40))
                .fg(Color::Rgb(180, 255, 180)),
            delete_style: Style::default()
                .bg(Color::Rgb(60, 40, 40))
                .fg(Color::Rgb(255, 180, 180)),
            line_number_style: Style::default()
                .fg(Color::Rgb(128, 128, 128)),
            hunk_header_style: Style::default()
                .bg(Color::Rgb(60, 60, 100))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            context_style: Style::default()
                .fg(Color::Rgb(180, 180, 180)),
        }
    }
}

/// A single diff hunk
#[derive(Debug, Clone)]
pub struct DiffHunk {
    /// Starting line in before file
    pub before_start: usize,
    
    /// Number of lines in before file
    pub before_count: usize,
    
    /// Starting line in after file
    pub after_start: usize,
    
    /// Number of lines in after file
    pub after_count: usize,
    
    /// Lines in this hunk
    pub lines: Vec<DiffLine>,
    
    /// Context information
    pub context: Option<String>,
}

/// A single line in a diff
#[derive(Debug, Clone)]
pub struct DiffLine {
    /// Line type
    pub kind: DiffLineKind,
    
    /// Line content
    pub content: String,
    
    /// Line number in before file (if applicable)
    pub before_line: Option<usize>,
    
    /// Line number in after file (if applicable)
    pub after_line: Option<usize>,
}

/// Type of diff line
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    /// Unchanged line (context)
    Equal,
    /// Added line
    Insert,
    /// Removed line
    Delete,
    /// Context separator
    Context,
}

impl DiffViewer {
    /// Create a new diff viewer
    pub fn new() -> Self {
        Self {
            layout: DiffLayout::Unified,
            before_file: DiffFile::new("", String::new()),
            after_file: DiffFile::new("", String::new()),
            config: DiffConfig::default(),
            hunks: Vec::new(),
            scroll_offset: 0,
            horizontal_offset: 0,
            area: Rect::default(),
            has_focus: false,
            syntax_cache: HashMap::new(),
            error_message: None,
        }
    }
    
    /// Set the layout mode
    pub fn set_layout(&mut self, layout: DiffLayout) -> &mut Self {
        self.layout = layout;
        self
    }
    
    /// Set the before file
    pub fn set_before_file(&mut self, file: DiffFile) -> &mut Self {
        self.before_file = file;
        self.syntax_cache.clear();
        self.compute_diff();
        self
    }
    
    /// Set the after file
    pub fn set_after_file(&mut self, file: DiffFile) -> &mut Self {
        self.after_file = file;
        self.syntax_cache.clear();
        self.compute_diff();
        self
    }
    
    /// Set diff configuration
    pub fn set_config(&mut self, config: DiffConfig) -> &mut Self {
        self.config = config;
        self.syntax_cache.clear();
        self.compute_diff();
        self
    }
    
    /// Compute the diff between files
    fn compute_diff(&mut self) {
        match self.compute_diff_internal() {
            Ok(hunks) => {
                self.hunks = hunks;
                self.error_message = None;
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to compute diff: {}", e));
                self.hunks.clear();
            }
        }
    }
    
    /// Internal diff computation
    fn compute_diff_internal(&self) -> Result<Vec<DiffHunk>> {
        let before_lines: Vec<&str> = self.before_file.content.lines().collect();
        let after_lines: Vec<&str> = self.after_file.content.lines().collect();
        
        // Simple diff algorithm - in a real implementation you'd use a proper diff library
        let mut hunks = Vec::new();
        let mut before_pos = 0;
        let mut after_pos = 0;
        
        // Find differences
        while before_pos < before_lines.len() || after_pos < after_lines.len() {
            // Find next difference
            let hunk_start_before = before_pos;
            let hunk_start_after = after_pos;
            
            // Skip equal lines
            while before_pos < before_lines.len() 
                && after_pos < after_lines.len() 
                && before_lines[before_pos] == after_lines[after_pos] {
                before_pos += 1;
                after_pos += 1;
            }
            
            if before_pos >= before_lines.len() && after_pos >= after_lines.len() {
                break;
            }
            
            // Start a new hunk
            let mut hunk_lines = Vec::new();
            let context_start = hunk_start_before.saturating_sub(self.config.context_lines);
            
            // Add context before change
            for i in context_start..hunk_start_before {
                if i < before_lines.len() {
                    hunk_lines.push(DiffLine {
                        kind: DiffLineKind::Equal,
                        content: before_lines[i].to_string(),
                        before_line: Some(i + 1),
                        after_line: Some(i + 1),
                    });
                }
            }
            
            // Find end of difference
            let diff_start_before = before_pos;
            let diff_start_after = after_pos;
            
            // Simple approach: find next common line
            let mut found_common = false;
            while !found_common && (before_pos < before_lines.len() || after_pos < after_lines.len()) {
                if before_pos < before_lines.len() && after_pos < after_lines.len() {
                    if before_lines[before_pos] == after_lines[after_pos] {
                        found_common = true;
                        break;
                    }
                }
                
                // Add deleted lines
                if before_pos < before_lines.len() && 
                   (after_pos >= after_lines.len() || 
                    before_lines[before_pos] != after_lines.get(after_pos).copied().unwrap_or("")) {
                    hunk_lines.push(DiffLine {
                        kind: DiffLineKind::Delete,
                        content: before_lines[before_pos].to_string(),
                        before_line: Some(before_pos + 1),
                        after_line: None,
                    });
                    before_pos += 1;
                }
                
                // Add inserted lines
                if after_pos < after_lines.len() && 
                   (before_pos >= before_lines.len() || 
                    after_lines[after_pos] != before_lines.get(before_pos).copied().unwrap_or("")) {
                    hunk_lines.push(DiffLine {
                        kind: DiffLineKind::Insert,
                        content: after_lines[after_pos].to_string(),
                        before_line: None,
                        after_line: Some(after_pos + 1),
                    });
                    after_pos += 1;
                }
            }
            
            // Add context after change
            let context_end = (before_pos + self.config.context_lines).min(before_lines.len());
            for i in before_pos..context_end {
                if i < before_lines.len() && i < after_lines.len() && before_lines[i] == after_lines[i] {
                    hunk_lines.push(DiffLine {
                        kind: DiffLineKind::Equal,
                        content: before_lines[i].to_string(),
                        before_line: Some(i + 1),
                        after_line: Some(i + 1),
                    });
                }
            }
            
            if !hunk_lines.is_empty() {
                hunks.push(DiffHunk {
                    before_start: diff_start_before + 1,
                    before_count: before_pos - diff_start_before,
                    after_start: diff_start_after + 1,
                    after_count: after_pos - diff_start_after,
                    lines: hunk_lines,
                    context: None,
                });
            }
        }
        
        Ok(hunks)
    }
    
    /// Scroll down
    pub fn scroll_down(&mut self, lines: usize) {
        let total_lines = self.get_total_display_lines();
        let max_scroll = total_lines.saturating_sub(self.area.height as usize);
        self.scroll_offset = (self.scroll_offset + lines).min(max_scroll);
    }
    
    /// Scroll up
    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }
    
    /// Scroll horizontally
    pub fn scroll_horizontal(&mut self, chars: isize) {
        if chars > 0 {
            self.horizontal_offset += chars as usize;
        } else {
            self.horizontal_offset = self.horizontal_offset.saturating_sub((-chars) as usize);
        }
    }
    
    /// Get total number of display lines
    fn get_total_display_lines(&self) -> usize {
        self.hunks.iter().map(|h| h.lines.len() + 1).sum() // +1 for hunk header
    }
    
    /// Render unified diff view
    fn render_unified(&self, area: Rect, theme: &Theme) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let mut current_line = 0;
        
        for hunk in &self.hunks {
            // Skip lines before scroll offset
            if current_line < self.scroll_offset {
                let hunk_lines = hunk.lines.len() + 1; // +1 for header
                if current_line + hunk_lines <= self.scroll_offset {
                    current_line += hunk_lines;
                    continue;
                }
            }
            
            // Stop if we've filled the visible area
            if lines.len() >= area.height as usize {
                break;
            }
            
            // Render hunk header
            if current_line >= self.scroll_offset {
                let header = format!(
                    "@@ -{},{} +{},{} @@",
                    hunk.before_start, hunk.before_count,
                    hunk.after_start, hunk.after_count
                );
                
                lines.push(Line::from(vec![
                    Span::styled(header, self.config.styling.hunk_header_style)
                ]));
            }
            current_line += 1;
            
            // Render hunk lines
            for line in &hunk.lines {
                if current_line < self.scroll_offset {
                    current_line += 1;
                    continue;
                }
                
                if lines.len() >= area.height as usize {
                    break;
                }
                
                let mut spans = Vec::new();
                
                // Line numbers
                if self.config.show_line_numbers {
                    let before_num = line.before_line
                        .map(|n| format!("{:4}", n))
                        .unwrap_or_else(|| "    ".to_string());
                    let after_num = line.after_line
                        .map(|n| format!("{:4}", n))
                        .unwrap_or_else(|| "    ".to_string());
                    
                    spans.push(Span::styled(
                        format!("{} {} ", before_num, after_num),
                        self.config.styling.line_number_style,
                    ));
                }
                
                // Line prefix
                let (prefix, style) = match line.kind {
                    DiffLineKind::Equal => (" ", self.config.styling.equal_style),
                    DiffLineKind::Insert => ("+", self.config.styling.insert_style),
                    DiffLineKind::Delete => ("-", self.config.styling.delete_style),
                    DiffLineKind::Context => (" ", self.config.styling.context_style),
                };
                
                spans.push(Span::styled(prefix, style));
                
                // Line content
                let content = if self.horizontal_offset < line.content.len() {
                    &line.content[self.horizontal_offset..]
                } else {
                    ""
                };
                
                spans.push(Span::styled(content.to_string(), style));
                
                lines.push(Line::from(spans));
                current_line += 1;
            }
        }
        
        lines
    }
    
    /// Render split diff view
    fn render_split(&self, area: Rect, theme: &Theme) -> (Vec<Line<'static>>, Vec<Line<'static>>) {
        let mut before_lines = Vec::new();
        let mut after_lines = Vec::new();
        let mut current_line = 0;
        
        for hunk in &self.hunks {
            // Skip lines before scroll offset
            if current_line < self.scroll_offset {
                let hunk_lines = hunk.lines.len() + 1;
                if current_line + hunk_lines <= self.scroll_offset {
                    current_line += hunk_lines;
                    continue;
                }
            }
            
            if before_lines.len() >= area.height as usize {
                break;
            }
            
            // Render hunk header for both sides
            if current_line >= self.scroll_offset {
                let header = format!(
                    "@@ -{},{} +{},{} @@",
                    hunk.before_start, hunk.before_count,
                    hunk.after_start, hunk.after_count
                );
                
                before_lines.push(Line::from(vec![
                    Span::styled(header.clone(), self.config.styling.hunk_header_style)
                ]));
                after_lines.push(Line::from(vec![
                    Span::styled(header, self.config.styling.hunk_header_style)
                ]));
            }
            current_line += 1;
            
            // Render lines for split view
            for line in &hunk.lines {
                if current_line < self.scroll_offset {
                    current_line += 1;
                    continue;
                }
                
                if before_lines.len() >= area.height as usize {
                    break;
                }
                
                match line.kind {
                    DiffLineKind::Equal => {
                        // Show on both sides
                        let content = if self.horizontal_offset < line.content.len() {
                            &line.content[self.horizontal_offset..]
                        } else {
                            ""
                        };
                        
                        let line_spans = self.create_line_spans(
                            line.before_line,
                            " ",
                            content,
                            self.config.styling.equal_style,
                        );
                        
                        before_lines.push(Line::from(line_spans.clone()));
                        after_lines.push(Line::from(line_spans));
                    }
                    DiffLineKind::Delete => {
                        // Show only on before side
                        let content = if self.horizontal_offset < line.content.len() {
                            &line.content[self.horizontal_offset..]
                        } else {
                            ""
                        };
                        
                        before_lines.push(Line::from(self.create_line_spans(
                            line.before_line,
                            "-",
                            content,
                            self.config.styling.delete_style,
                        )));
                        after_lines.push(Line::from(vec![Span::raw("")])); // Empty line
                    }
                    DiffLineKind::Insert => {
                        // Show only on after side
                        let content = if self.horizontal_offset < line.content.len() {
                            &line.content[self.horizontal_offset..]
                        } else {
                            ""
                        };
                        
                        before_lines.push(Line::from(vec![Span::raw("")])); // Empty line
                        after_lines.push(Line::from(self.create_line_spans(
                            line.after_line,
                            "+",
                            content,
                            self.config.styling.insert_style,
                        )));
                    }
                    DiffLineKind::Context => {
                        // Context lines (similar to equal)
                        let content = if self.horizontal_offset < line.content.len() {
                            &line.content[self.horizontal_offset..]
                        } else {
                            ""
                        };
                        
                        let line_spans = self.create_line_spans(
                            line.before_line,
                            " ",
                            content,
                            self.config.styling.context_style,
                        );
                        
                        before_lines.push(Line::from(line_spans.clone()));
                        after_lines.push(Line::from(line_spans));
                    }
                }
                
                current_line += 1;
            }
        }
        
        (before_lines, after_lines)
    }
    
    /// Create spans for a line with line number and content
    fn create_line_spans(&self, line_number: Option<usize>, prefix: &str, content: &str, style: Style) -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        
        // Line number
        if self.config.show_line_numbers {
            let num_str = line_number
                .map(|n| format!("{:4} ", n))
                .unwrap_or_else(|| "     ".to_string());
            
            spans.push(Span::styled(num_str, self.config.styling.line_number_style));
        }
        
        // Prefix
        spans.push(Span::styled(prefix.to_string(), style));
        
        // Content
        spans.push(Span::styled(content.to_string(), style));
        
        spans
    }
}

#[async_trait]
impl Component for DiffViewer {
    async fn handle_key_event(&mut self, event: KeyEvent) -> Result<()> {
        if !self.has_focus {
            return Ok(());
        }
        
        match event.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll_down(1);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll_up(1);
            }
            KeyCode::PageDown => {
                let page_size = self.area.height as usize;
                self.scroll_down(page_size);
            }
            KeyCode::PageUp => {
                let page_size = self.area.height as usize;
                self.scroll_up(page_size);
            }
            KeyCode::Home => {
                self.scroll_offset = 0;
            }
            KeyCode::End => {
                let total_lines = self.get_total_display_lines();
                self.scroll_offset = total_lines.saturating_sub(self.area.height as usize);
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.scroll_horizontal(-5);
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.scroll_horizontal(5);
            }
            KeyCode::Char('u') => {
                self.layout = DiffLayout::Unified;
            }
            KeyCode::Char('s') => {
                self.layout = DiffLayout::Split;
            }
            KeyCode::Char('n') => {
                self.config.show_line_numbers = !self.config.show_line_numbers;
            }
            _ => {}
        }
        
        Ok(())
    }
    
    async fn handle_mouse_event(&mut self, _event: MouseEvent) -> Result<()> {
        // Mouse support for scrolling could be implemented here
        Ok(())
    }
    
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.area = area;
        
        // Clear the area
        frame.render_widget(Clear, area);
        
        // Main container
        let main_block = Block::default()
            .title(format!("Diff Viewer ({})", 
                match self.layout {
                    DiffLayout::Unified => "Unified",
                    DiffLayout::Split => "Split",
                }))
            .borders(Borders::ALL)
            .border_style(if self.has_focus {
                Style::default().fg(theme.colors.primary)
            } else {
                Style::default().fg(theme.colors.border)
            });
        
        frame.render_widget(main_block, area);
        
        let inner = area.inner(&ratatui::layout::Margin { horizontal: 1, vertical: 1 });
        
        // Render error message if present
        if let Some(ref error) = self.error_message {
            let error_widget = Paragraph::new(error.as_str())
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            
            frame.render_widget(error_widget, inner);
            return;
        }
        
        // Render diff content
        match self.layout {
            DiffLayout::Unified => {
                let lines = self.render_unified(inner, theme);
                let paragraph = Paragraph::new(lines)
                    .wrap(Wrap { trim: false });
                frame.render_widget(paragraph, inner);
            }
            DiffLayout::Split => {
                let (before_lines, after_lines) = self.render_split(inner, theme);
                
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(inner);
                
                // Before (left) side
                let before_block = Block::default()
                    .title(format!("Before: {}", self.before_file.path.display()))
                    .borders(Borders::RIGHT)
                    .border_style(Style::default().fg(theme.colors.border));
                
                let before_inner = chunks[0].inner(&ratatui::layout::Margin { horizontal: 0, vertical: 0 });
                frame.render_widget(before_block, chunks[0]);
                
                let before_paragraph = Paragraph::new(before_lines)
                    .wrap(Wrap { trim: false });
                frame.render_widget(before_paragraph, before_inner);
                
                // After (right) side
                let after_block = Block::default()
                    .title(format!("After: {}", self.after_file.path.display()))
                    .borders(Borders::NONE)
                    .border_style(Style::default().fg(theme.colors.border));
                
                let after_inner = chunks[1].inner(&ratatui::layout::Margin { horizontal: 1, vertical: 0 });
                frame.render_widget(after_block, chunks[1]);
                
                let after_paragraph = Paragraph::new(after_lines)
                    .wrap(Wrap { trim: false });
                frame.render_widget(after_paragraph, after_inner);
            }
        }
        
        // Status line at bottom
        if inner.height > 2 {
            let status_area = Rect {
                x: inner.x,
                y: inner.y + inner.height - 1,
                width: inner.width,
                height: 1,
            };
            
            let status_text = format!(
                "Line {}/{} | {} hunks | {} (u)nified (s)plit (n)umbers ↑↓←→ scroll",
                self.scroll_offset + 1,
                self.get_total_display_lines(),
                self.hunks.len(),
                match self.layout {
                    DiffLayout::Unified => "Unified",
                    DiffLayout::Split => "Split",
                }
            );
            
            let status_widget = Paragraph::new(status_text)
                .style(Style::default().fg(theme.colors.muted))
                .alignment(Alignment::Left);
            
            frame.render_widget(status_widget, status_area);
        }
    }
    
    fn size(&self) -> Rect {
        self.area
    }
    
    fn set_size(&mut self, size: Rect) {
        self.area = size;
    }
    
    fn has_focus(&self) -> bool {
        self.has_focus
    }
    
    fn set_focus(&mut self, focus: bool) {
        self.has_focus = focus;
    }
}

impl Default for DiffViewer {
    fn default() -> Self {
        Self::new()
    }
}

/// Detect programming language from file extension
fn detect_language(path: &Path) -> Option<String> {
    match path.extension()?.to_str()? {
        "rs" => Some("rust".to_string()),
        "go" => Some("go".to_string()),
        "py" => Some("python".to_string()),
        "js" | "mjs" => Some("javascript".to_string()),
        "ts" => Some("typescript".to_string()),
        "html" => Some("html".to_string()),
        "css" => Some("css".to_string()),
        "json" => Some("json".to_string()),
        "yaml" | "yml" => Some("yaml".to_string()),
        "toml" => Some("toml".to_string()),
        "md" => Some("markdown".to_string()),
        "c" => Some("c".to_string()),
        "cpp" | "cc" | "cxx" => Some("cpp".to_string()),
        "java" => Some("java".to_string()),
        "sh" | "bash" => Some("bash".to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_diff_viewer_creation() {
        let viewer = DiffViewer::new();
        assert_eq!(viewer.layout, DiffLayout::Unified);
        assert!(viewer.hunks.is_empty());
    }
    
    #[test]
    fn test_language_detection() {
        assert_eq!(detect_language(Path::new("test.rs")), Some("rust".to_string()));
        assert_eq!(detect_language(Path::new("test.py")), Some("python".to_string()));
        assert_eq!(detect_language(Path::new("test.unknown")), None);
    }
    
    #[test]
    fn test_diff_computation() {
        let mut viewer = DiffViewer::new();
        
        let before = DiffFile::new("test.txt", "line1\nline2\nline3".to_string());
        let after = DiffFile::new("test.txt", "line1\nmodified line2\nline3\nline4".to_string());
        
        viewer.set_before_file(before);
        viewer.set_after_file(after);
        
        assert!(!viewer.hunks.is_empty());
    }
    
    #[test]
    fn test_diff_from_files() {
        let mut before_file = NamedTempFile::new().unwrap();
        let mut after_file = NamedTempFile::new().unwrap();
        
        std::fs::write(&before_file, "original content\nline 2\nline 3").unwrap();
        std::fs::write(&after_file, "modified content\nline 2\nline 3\nnew line").unwrap();
        
        let before_diff = DiffFile::from_path(before_file.path()).unwrap();
        let after_diff = DiffFile::from_path(after_file.path()).unwrap();
        
        let mut viewer = DiffViewer::new();
        viewer.set_before_file(before_diff);
        viewer.set_after_file(after_diff);
        
        assert!(!viewer.hunks.is_empty());
    }
}