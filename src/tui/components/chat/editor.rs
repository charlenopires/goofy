//! Enhanced chat editor with multiline editing and syntax highlighting
//!
//! This module provides a sophisticated text editor for composing chat messages
//! with support for syntax highlighting, auto-completion, file attachments,
//! and keyboard shortcuts.

use super::message_types::{ChatMessage, MessageAttachment};
use crate::tui::{
    components::{Component, ComponentState, TextInput},
    themes::{Theme, ThemeManager},
    Frame,
};
use anyhow::Result;
use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};
use std::{
    collections::VecDeque,
    path::Path,
    time::{Duration, Instant},
};

/// Maximum number of attachments allowed
const MAX_ATTACHMENTS: usize = 10;

/// Maximum attachment size (10MB)
const MAX_ATTACHMENT_SIZE: usize = 10 * 1024 * 1024;

/// Enhanced chat editor component
pub struct ChatEditor {
    state: ComponentState,
    content: String,
    cursor_position: usize,
    scroll_offset: usize,
    
    // Display properties
    line_numbers: bool,
    syntax_highlighting: bool,
    word_wrap: bool,
    theme_manager: ThemeManager,
    
    // Input history
    history: VecDeque<String>,
    history_index: Option<usize>,
    max_history_size: usize,
    
    // Attachments
    attachments: Vec<MessageAttachment>,
    
    // Auto-completion
    completion_popup: Option<CompletionPopup>,
    completions: Vec<CompletionItem>,
    
    // Editor modes
    mode: EditorMode,
    placeholder_text: String,
    
    // Performance optimization
    last_content_hash: u64,
    cached_rendered_lines: Vec<Line<'static>>,
    
    // Multi-line editing
    lines: Vec<String>,
    cursor_line: usize,
    cursor_column: usize,
    selection_start: Option<(usize, usize)>,
    selection_end: Option<(usize, usize)>,
    
    // Animation and feedback
    last_activity: Instant,
    blink_state: bool,
    
    // File operations
    last_file_drop: Option<Instant>,
}

/// Editor operation modes
#[derive(Debug, Clone, PartialEq)]
pub enum EditorMode {
    /// Normal text editing
    Normal,
    /// Command mode (for special commands)
    Command,
    /// Search mode
    Search(String),
    /// Attachment mode (showing attachment options)
    AttachmentSelect,
}

/// Auto-completion popup
#[derive(Debug, Clone)]
pub struct CompletionPopup {
    items: Vec<CompletionItem>,
    selected_index: usize,
    position: (u16, u16),
    visible: bool,
    filter: String,
}

/// Completion item for auto-complete
#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub detail: Option<String>,
    pub kind: CompletionKind,
    pub insert_text: String,
}

/// Types of completions
#[derive(Debug, Clone, PartialEq)]
pub enum CompletionKind {
    File,
    Command,
    Snippet,
    Variable,
    Function,
}

/// Editor events
#[derive(Debug, Clone)]
pub enum EditorEvent {
    /// User wants to send the message
    SendMessage(String, Vec<MessageAttachment>),
    /// User wants to attach a file
    AttachFile(String),
    /// User wants to remove an attachment
    RemoveAttachment(usize),
    /// Content changed
    ContentChanged(String),
    /// Editor mode changed
    ModeChanged(EditorMode),
}

impl ChatEditor {
    /// Create a new chat editor
    pub fn new() -> Self {
        Self {
            state: ComponentState::new(),
            content: String::new(),
            cursor_position: 0,
            scroll_offset: 0,
            line_numbers: false,
            syntax_highlighting: true,
            word_wrap: true,
            theme_manager: ThemeManager::new(),
            history: VecDeque::new(),
            history_index: None,
            max_history_size: 50,
            attachments: Vec::new(),
            completion_popup: None,
            completions: Vec::new(),
            mode: EditorMode::Normal,
            placeholder_text: "Type your message here...".to_string(),
            last_content_hash: 0,
            cached_rendered_lines: Vec::new(),
            lines: vec![String::new()],
            cursor_line: 0,
            cursor_column: 0,
            selection_start: None,
            selection_end: None,
            last_activity: Instant::now(),
            blink_state: false,
            last_file_drop: None,
        }
    }

    /// Set editor configuration
    pub fn with_line_numbers(mut self, show: bool) -> Self {
        self.line_numbers = show;
        self
    }

    pub fn with_syntax_highlighting(mut self, enable: bool) -> Self {
        self.syntax_highlighting = enable;
        self
    }

    pub fn with_word_wrap(mut self, enable: bool) -> Self {
        self.word_wrap = enable;
        self
    }

    pub fn with_placeholder(mut self, text: String) -> Self {
        self.placeholder_text = text;
        self
    }

    /// Get current content
    pub fn get_content(&self) -> &str {
        &self.content
    }

    /// Set content programmatically
    pub fn set_content(&mut self, content: String) {
        self.content = content;
        self.lines = self.content.lines().map(|s| s.to_string()).collect();
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.cursor_position = self.content.len();
        self.update_cursor_from_position();
        self.invalidate_cache();
    }

    /// Clear all content
    pub fn clear(&mut self) {
        self.content.clear();
        self.lines = vec![String::new()];
        self.cursor_position = 0;
        self.cursor_line = 0;
        self.cursor_column = 0;
        self.scroll_offset = 0;
        self.selection_start = None;
        self.selection_end = None;
        self.invalidate_cache();
    }

    /// Add an attachment
    pub fn add_attachment(&mut self, attachment: MessageAttachment) -> Result<()> {
        if self.attachments.len() >= MAX_ATTACHMENTS {
            return Err(anyhow::anyhow!("Maximum number of attachments ({}) reached", MAX_ATTACHMENTS));
        }
        
        if attachment.size > MAX_ATTACHMENT_SIZE as u64 {
            return Err(anyhow::anyhow!("Attachment too large (max {} MB)", MAX_ATTACHMENT_SIZE / (1024 * 1024)));
        }
        
        self.attachments.push(attachment);
        Ok(())
    }

    /// Remove an attachment by index
    pub fn remove_attachment(&mut self, index: usize) -> Result<()> {
        if index < self.attachments.len() {
            self.attachments.remove(index);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid attachment index"))
        }
    }

    /// Get all attachments
    pub fn get_attachments(&self) -> &[MessageAttachment] {
        &self.attachments
    }

    /// Add to history
    pub fn add_to_history(&mut self, content: String) {
        if !content.trim().is_empty() {
            self.history.push_back(content);
            if self.history.len() > self.max_history_size {
                self.history.pop_front();
            }
        }
        self.history_index = None;
    }

    /// Navigate history
    pub fn history_previous(&mut self) -> bool {
        if self.history.is_empty() {
            return false;
        }

        let new_index = match self.history_index {
            None => Some(self.history.len() - 1),
            Some(index) if index > 0 => Some(index - 1),
            Some(_) => return false,
        };

        if let Some(index) = new_index {
            if let Some(content) = self.history.get(index) {
                self.set_content(content.clone());
                self.history_index = new_index;
                return true;
            }
        }
        false
    }

    pub fn history_next(&mut self) -> bool {
        if let Some(current_index) = self.history_index {
            if current_index + 1 < self.history.len() {
                let new_index = current_index + 1;
                if let Some(content) = self.history.get(new_index) {
                    self.set_content(content.clone());
                    self.history_index = Some(new_index);
                    return true;
                }
            } else {
                // Clear content when going past last history item
                self.clear();
                self.history_index = None;
                return true;
            }
        }
        false
    }

    /// Handle file drop
    pub fn handle_file_drop(&mut self, file_path: &str) -> Result<()> {
        match MessageAttachment::from_file_path(file_path) {
            Ok(attachment) => self.add_attachment(attachment),
            Err(e) => Err(anyhow::anyhow!("Failed to add file attachment: {}", e)),
        }
    }

    /// Show completions
    pub fn show_completions(&mut self, items: Vec<CompletionItem>) {
        if !items.is_empty() {
            let cursor_pos = self.get_cursor_screen_position();
            self.completion_popup = Some(CompletionPopup {
                items,
                selected_index: 0,
                position: cursor_pos,
                visible: true,
                filter: String::new(),
            });
        }
    }

    /// Hide completions
    pub fn hide_completions(&mut self) {
        self.completion_popup = None;
    }

    /// Move cursor
    pub fn move_cursor(&mut self, direction: CursorDirection) {
        match direction {
            CursorDirection::Left => self.move_cursor_left(),
            CursorDirection::Right => self.move_cursor_right(),
            CursorDirection::Up => self.move_cursor_up(),
            CursorDirection::Down => self.move_cursor_down(),
            CursorDirection::Home => self.move_cursor_home(),
            CursorDirection::End => self.move_cursor_end(),
            CursorDirection::PageUp => self.move_cursor_page_up(),
            CursorDirection::PageDown => self.move_cursor_page_down(),
        }
        self.update_position_from_cursor();
    }

    /// Insert text at cursor
    pub fn insert_text(&mut self, text: &str) {
        self.lines[self.cursor_line].insert_str(self.cursor_column, text);
        self.cursor_column += text.len();
        self.update_content_from_lines();
        self.invalidate_cache();
        self.last_activity = Instant::now();
    }

    /// Delete character at cursor
    pub fn delete_char(&mut self) {
        if self.cursor_column < self.lines[self.cursor_line].len() {
            self.lines[self.cursor_line].remove(self.cursor_column);
            self.update_content_from_lines();
            self.invalidate_cache();
        } else if self.cursor_line + 1 < self.lines.len() {
            // Join with next line
            let next_line = self.lines.remove(self.cursor_line + 1);
            self.lines[self.cursor_line].push_str(&next_line);
            self.update_content_from_lines();
            self.invalidate_cache();
        }
        self.last_activity = Instant::now();
    }

    /// Delete character before cursor (backspace)
    pub fn delete_previous_char(&mut self) {
        if self.cursor_column > 0 {
            self.cursor_column -= 1;
            self.lines[self.cursor_line].remove(self.cursor_column);
            self.update_content_from_lines();
            self.invalidate_cache();
        } else if self.cursor_line > 0 {
            // Move to end of previous line and join
            let current_line = self.lines.remove(self.cursor_line);
            self.cursor_line -= 1;
            self.cursor_column = self.lines[self.cursor_line].len();
            self.lines[self.cursor_line].push_str(&current_line);
            self.update_content_from_lines();
            self.invalidate_cache();
        }
        self.last_activity = Instant::now();
    }

    /// Insert new line
    pub fn insert_newline(&mut self) {
        let before = self.lines[self.cursor_line][..self.cursor_column].to_string();
        let after = self.lines[self.cursor_line][self.cursor_column..].to_string();

        self.lines[self.cursor_line] = before;
        self.lines.insert(self.cursor_line + 1, after);
        self.cursor_line += 1;
        self.cursor_column = 0;

        self.update_content_from_lines();
        self.invalidate_cache();
        self.last_activity = Instant::now();
    }

    /// Select all text
    pub fn select_all(&mut self) {
        self.selection_start = Some((0, 0));
        self.selection_end = Some((self.lines.len() - 1, self.lines.last().unwrap().len()));
    }

    /// Get selected text
    pub fn get_selected_text(&self) -> Option<String> {
        if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
            let (start_line, start_col) = start;
            let (end_line, end_col) = end;
            
            if start_line == end_line {
                let line = &self.lines[start_line];
                return Some(line[start_col..end_col].to_string());
            } else {
                let mut result = String::new();
                for line_idx in start_line..=end_line {
                    let line = &self.lines[line_idx];
                    if line_idx == start_line {
                        result.push_str(&line[start_col..]);
                    } else if line_idx == end_line {
                        result.push_str(&line[..end_col]);
                    } else {
                        result.push_str(line);
                    }
                    if line_idx < end_line {
                        result.push('\n');
                    }
                }
                return Some(result);
            }
        }
        None
    }

    /// Copy selected text
    pub fn copy_selection(&self) -> Option<String> {
        self.get_selected_text()
    }

    /// Cut selected text
    pub fn cut_selection(&mut self) -> Option<String> {
        if let Some(text) = self.get_selected_text() {
            self.delete_selection();
            Some(text)
        } else {
            None
        }
    }

    /// Delete selected text
    pub fn delete_selection(&mut self) {
        if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
            let (start_line, start_col) = start;
            let (end_line, end_col) = end;
            
            if start_line == end_line {
                let line = &mut self.lines[start_line];
                line.replace_range(start_col..end_col, "");
                self.cursor_line = start_line;
                self.cursor_column = start_col;
            } else {
                // Remove complete lines in between
                for _ in (start_line + 1)..end_line {
                    self.lines.remove(start_line + 1);
                }
                
                // Merge start and end lines
                let end_part = self.lines[start_line + 1][end_col..].to_string();
                self.lines[start_line].truncate(start_col);
                self.lines[start_line].push_str(&end_part);
                self.lines.remove(start_line + 1);
                
                self.cursor_line = start_line;
                self.cursor_column = start_col;
            }
            
            self.selection_start = None;
            self.selection_end = None;
            self.update_content_from_lines();
            self.invalidate_cache();
        }
    }

    /// Paste text at cursor
    pub fn paste_text(&mut self, text: &str) {
        if self.selection_start.is_some() {
            self.delete_selection();
        }
        
        let lines: Vec<&str> = text.lines().collect();
        if lines.is_empty() {
            return;
        }
        
        if lines.len() == 1 {
            // Single line paste
            self.insert_text(text);
        } else {
            // Multi-line paste
            let before = self.lines[self.cursor_line][..self.cursor_column].to_string();
            let after = self.lines[self.cursor_line][self.cursor_column..].to_string();

            // Replace current line with first part + first line of paste
            self.lines[self.cursor_line] = format!("{}{}", before, lines[0]);

            // Insert middle lines
            for (i, line) in lines.iter().enumerate().skip(1).take(lines.len() - 2) {
                self.lines.insert(self.cursor_line + i, line.to_string());
            }

            // Insert last line + remaining part of original line
            if lines.len() > 1 {
                let last_line = format!("{}{}", lines.last().unwrap(), &after);
                self.lines.insert(self.cursor_line + lines.len() - 1, last_line);
                self.cursor_line += lines.len() - 1;
                self.cursor_column = lines.last().unwrap().len();
            }

            self.update_content_from_lines();
            self.invalidate_cache();
        }
    }

    // Helper methods
    
    fn move_cursor_left(&mut self) {
        if self.cursor_column > 0 {
            self.cursor_column -= 1;
        } else if self.cursor_line > 0 {
            self.cursor_line -= 1;
            self.cursor_column = self.lines[self.cursor_line].len();
        }
    }

    fn move_cursor_right(&mut self) {
        if self.cursor_column < self.lines[self.cursor_line].len() {
            self.cursor_column += 1;
        } else if self.cursor_line + 1 < self.lines.len() {
            self.cursor_line += 1;
            self.cursor_column = 0;
        }
    }

    fn move_cursor_up(&mut self) {
        if self.cursor_line > 0 {
            self.cursor_line -= 1;
            self.cursor_column = self.cursor_column.min(self.lines[self.cursor_line].len());
        }
    }

    fn move_cursor_down(&mut self) {
        if self.cursor_line + 1 < self.lines.len() {
            self.cursor_line += 1;
            self.cursor_column = self.cursor_column.min(self.lines[self.cursor_line].len());
        }
    }

    fn move_cursor_home(&mut self) {
        self.cursor_column = 0;
    }

    fn move_cursor_end(&mut self) {
        self.cursor_column = self.lines[self.cursor_line].len();
    }

    fn move_cursor_page_up(&mut self) {
        let page_size = self.state.size.height.saturating_sub(2) as usize;
        self.cursor_line = self.cursor_line.saturating_sub(page_size);
        self.cursor_column = self.cursor_column.min(self.lines[self.cursor_line].len());
    }

    fn move_cursor_page_down(&mut self) {
        let page_size = self.state.size.height.saturating_sub(2) as usize;
        self.cursor_line = (self.cursor_line + page_size).min(self.lines.len() - 1);
        self.cursor_column = self.cursor_column.min(self.lines[self.cursor_line].len());
    }

    fn update_content_from_lines(&mut self) {
        self.content = self.lines.join("\n");
        self.update_position_from_cursor();
    }

    fn update_cursor_from_position(&mut self) {
        let mut pos = 0;
        self.cursor_line = 0;
        self.cursor_column = 0;

        for (line_idx, line) in self.lines.iter().enumerate() {
            if pos + line.len() >= self.cursor_position {
                self.cursor_line = line_idx;
                self.cursor_column = self.cursor_position - pos;
                break;
            }
            pos += line.len() + 1; // +1 for newline
        }
    }

    fn update_position_from_cursor(&mut self) {
        let mut pos = 0;
        for line_idx in 0..self.cursor_line {
            pos += self.lines[line_idx].len() + 1; // +1 for newline
        }
        pos += self.cursor_column;
        self.cursor_position = pos;
    }

    fn get_cursor_screen_position(&self) -> (u16, u16) {
        // Calculate screen position considering scroll offset and line numbers
        let x = if self.line_numbers { 6 } else { 2 } + self.cursor_column as u16;
        let y = 1 + (self.cursor_line - self.scroll_offset) as u16;
        (x, y)
    }

    fn invalidate_cache(&mut self) {
        self.last_content_hash = 0;
        self.cached_rendered_lines.clear();
    }

    fn should_show_cursor(&self) -> bool {
        self.state.has_focus && 
        (self.last_activity.elapsed() < Duration::from_millis(500) || 
         self.last_activity.elapsed().as_millis() % 1000 < 500)
    }

    fn render_line_numbers(&self, line_count: usize) -> Vec<Line<'static>> {
        let theme = self.theme_manager.current_theme();
        let mut lines = Vec::new();
        
        for i in 0..line_count {
            let line_num = i + self.scroll_offset + 1;
            let style = if i + self.scroll_offset == self.cursor_line {
                theme.styles.text_area_line_number.add_modifier(Modifier::BOLD)
            } else {
                theme.styles.text_area_line_number
            };
            
            lines.push(Line::from(Span::styled(
                format!("{:4} ", line_num),
                style,
            )));
        }
        
        lines
    }

    fn render_content_lines(&self, visible_height: usize) -> Vec<Line<'static>> {
        let theme = self.theme_manager.current_theme();
        let mut lines = Vec::new();

        let start_line = self.scroll_offset;
        let end_line = (start_line + visible_height).min(self.lines.len());

        for line_idx in start_line..end_line {
            let line_content = self.lines[line_idx].clone();
            let spans;

            if self.syntax_highlighting && self.mode == EditorMode::Normal {
                // Simple syntax highlighting for common patterns
                spans = self.highlight_syntax(&line_content);
            } else {
                spans = vec![Span::styled(line_content.clone(), theme.styles.text)];
            }

            let mut final_spans = spans;

            // Add cursor if on this line
            if line_idx == self.cursor_line && self.should_show_cursor() {
                // Insert cursor span at correct position
                if self.cursor_column <= line_content.len() {
                    let cursor_char = if self.cursor_column == line_content.len() {
                        " ".to_string()
                    } else {
                        line_content[self.cursor_column..self.cursor_column + 1].to_string()
                    };

                    // This is a simplified cursor rendering - in practice you'd need
                    // to split the spans at the cursor position
                    final_spans.push(Span::styled(cursor_char, theme.styles.text_area_cursor_line));
                }
            }

            lines.push(Line::from(final_spans));
        }

        // Fill remaining space with empty lines
        while lines.len() < visible_height {
            lines.push(Line::from(Span::raw("")));
        }

        lines
    }

    fn highlight_syntax(&self, line: &str) -> Vec<Span<'static>> {
        let theme = self.theme_manager.current_theme();
        let mut spans = Vec::new();
        
        // Simple keyword highlighting
        let words: Vec<&str> = line.split_whitespace().collect();
        let mut current_pos = 0;
        
        for word in words {
            // Find the word position in the original line
            if let Some(pos) = line[current_pos..].find(word) {
                let actual_pos = current_pos + pos;
                
                // Add any whitespace before the word
                if actual_pos > current_pos {
                    spans.push(Span::raw(line[current_pos..actual_pos].to_string()));
                }
                
                // Style the word based on patterns
                let style = if is_keyword(word) {
                    Style::default().fg(theme.colors.blue).add_modifier(Modifier::BOLD)
                } else if word.starts_with('"') && word.ends_with('"') {
                    Style::default().fg(theme.colors.green)
                } else if word.parse::<f64>().is_ok() {
                    Style::default().fg(theme.colors.yellow)
                } else {
                    theme.styles.text
                };
                
                spans.push(Span::styled(word.to_string(), style));
                current_pos = actual_pos + word.len();
            }
        }
        
        // Add any remaining text
        if current_pos < line.len() {
            spans.push(Span::raw(line[current_pos..].to_string()));
        }
        
        if spans.is_empty() {
            spans.push(Span::styled(line.to_string(), theme.styles.text));
        }
        
        spans
    }

    fn render_attachments(&self, frame: &mut Frame, area: Rect) {
        if self.attachments.is_empty() {
            return;
        }

        let theme = self.theme_manager.current_theme();
        let mut items = Vec::new();

        for (i, attachment) in self.attachments.iter().enumerate() {
            let icon = if attachment.is_image() {
                "🖼️"
            } else if attachment.is_text() {
                "📄"
            } else {
                "📁"
            };

            let item = ListItem::new(Line::from(vec![
                Span::styled(format!("{}. ", i + 1), theme.styles.muted),
                Span::styled(icon, theme.styles.info),
                Span::raw(" "),
                Span::styled(&attachment.filename, theme.styles.text),
                Span::raw(" "),
                Span::styled(
                    format!("({})", attachment.formatted_size()),
                    theme.styles.muted,
                ),
            ]));

            items.push(item);
        }

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Attachments")
                    .borders(Borders::ALL)
                    .border_style(theme.styles.dialog_border),
            )
            .style(theme.styles.base);

        frame.render_widget(list, area);
    }

    fn render_completion_popup(&self, frame: &mut Frame) {
        if let Some(popup) = &self.completion_popup {
            if !popup.visible || popup.items.is_empty() {
                return;
            }

            let theme = self.theme_manager.current_theme();
            let (x, y) = popup.position;
            
            let popup_width = popup.items.iter()
                .map(|item| item.label.len() + item.detail.as_ref().map_or(0, |d| d.len() + 3))
                .max()
                .unwrap_or(20)
                .min(50) as u16;
            
            let popup_height = (popup.items.len() + 2).min(10) as u16;
            
            let popup_area = Rect {
                x: x.min(frame.size().width.saturating_sub(popup_width)),
                y: y.min(frame.size().height.saturating_sub(popup_height)),
                width: popup_width,
                height: popup_height,
            };

            // Clear the popup area
            frame.render_widget(Clear, popup_area);

            let mut items = Vec::new();
            for (i, item) in popup.items.iter().enumerate() {
                let style = if i == popup.selected_index {
                    theme.styles.selected_base
                } else {
                    theme.styles.base
                };

                let mut spans = vec![
                    Span::styled(get_completion_icon(&item.kind), theme.styles.info),
                    Span::raw(" "),
                    Span::styled(&item.label, style),
                ];

                if let Some(detail) = &item.detail {
                    spans.extend([
                        Span::raw(" - "),
                        Span::styled(detail, theme.styles.muted),
                    ]);
                }

                items.push(ListItem::new(Line::from(spans)).style(style));
            }

            let list = List::new(items)
                .block(
                    Block::default()
                        .title("Completions")
                        .borders(Borders::ALL)
                        .border_style(theme.styles.dialog_border),
                )
                .highlight_style(theme.styles.selected_base);

            frame.render_widget(list, popup_area);
        }
    }
}

/// Cursor movement directions
#[derive(Debug, Clone, PartialEq)]
pub enum CursorDirection {
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
}

// Helper functions

fn is_keyword(word: &str) -> bool {
    matches!(word, 
        "if" | "else" | "while" | "for" | "function" | "class" | "def" | "import" | 
        "from" | "return" | "break" | "continue" | "try" | "catch" | "finally" |
        "const" | "let" | "var" | "async" | "await" | "true" | "false" | "null" |
        "undefined" | "new" | "this" | "super" | "static" | "public" | "private"
    )
}

fn get_completion_icon(kind: &CompletionKind) -> &'static str {
    match kind {
        CompletionKind::File => "📄",
        CompletionKind::Command => "⚡",
        CompletionKind::Snippet => "📝",
        CompletionKind::Variable => "🔤",
        CompletionKind::Function => "🔧",
    }
}

#[async_trait]
impl Component for ChatEditor {
    async fn handle_key_event(&mut self, event: KeyEvent) -> Result<()> {
        self.last_activity = Instant::now();

        match self.mode {
            EditorMode::Normal => self.handle_normal_mode_key(event).await,
            EditorMode::Command => self.handle_command_mode_key(event).await,
            EditorMode::Search(_) => self.handle_search_mode_key(event).await,
            EditorMode::AttachmentSelect => self.handle_attachment_mode_key(event).await,
        }
    }

    async fn handle_mouse_event(&mut self, _event: MouseEvent) -> Result<()> {
        // TODO: Handle mouse events for cursor positioning and selection
        Ok(())
    }

    async fn tick(&mut self) -> Result<()> {
        // Update cursor blink state
        if self.last_activity.elapsed() > Duration::from_millis(500) {
            self.blink_state = !self.blink_state;
        }
        Ok(())
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &crate::tui::themes::Theme) {
        let chunks = if self.attachments.is_empty() {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1)])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(1)])
                .split(area)
        };

        // Render attachments if any
        if !self.attachments.is_empty() {
            self.render_attachments(frame, chunks[0]);
        }

        let editor_area = if self.attachments.is_empty() { chunks[0] } else { chunks[1] };
        
        // Render editor
        let border_style = if self.state.has_focus {
            theme.styles.dialog_border.add_modifier(Modifier::BOLD)
        } else {
            theme.styles.dialog_border
        };

        let block = Block::default()
            .title("Message Editor")
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner_area = block.inner(editor_area);
        frame.render_widget(block, editor_area);

        // Render content
        if self.content.is_empty() && !self.state.has_focus {
            // Show placeholder
            let placeholder = Paragraph::new(self.placeholder_text.as_str())
                .style(theme.styles.muted)
                .wrap(Wrap { trim: true });
            frame.render_widget(placeholder, inner_area);
        } else {
            let visible_height = inner_area.height as usize;
            
            if self.line_numbers {
                // Split area for line numbers and content
                let content_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Length(6), Constraint::Min(1)])
                    .split(inner_area);

                // Render line numbers
                let line_number_lines = self.render_line_numbers(visible_height);
                let line_numbers_paragraph = Paragraph::new(Text::from(line_number_lines))
                    .style(theme.styles.text_area_line_number);
                frame.render_widget(line_numbers_paragraph, content_chunks[0]);

                // Render content
                let content_lines = self.render_content_lines(visible_height);
                let content_paragraph = Paragraph::new(Text::from(content_lines))
                    .style(theme.styles.text)
                    .wrap(if self.word_wrap { Wrap { trim: true } } else { Wrap { trim: false } });
                frame.render_widget(content_paragraph, content_chunks[1]);
            } else {
                // Render content only
                let content_lines = self.render_content_lines(visible_height);
                let content_paragraph = Paragraph::new(Text::from(content_lines))
                    .style(theme.styles.text)
                    .wrap(if self.word_wrap { Wrap { trim: true } } else { Wrap { trim: false } });
                frame.render_widget(content_paragraph, inner_area);
            }
        }

        // Render completion popup
        self.render_completion_popup(frame);
    }

    fn size(&self) -> Rect {
        self.state.size
    }

    fn set_size(&mut self, size: Rect) {
        self.state.size = size;
    }

    fn has_focus(&self) -> bool {
        self.state.has_focus
    }

    fn set_focus(&mut self, focus: bool) {
        self.state.has_focus = focus;
        if !focus {
            self.hide_completions();
        }
    }

    fn is_visible(&self) -> bool {
        self.state.is_visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.state.is_visible = visible;
    }
}

impl ChatEditor {
    async fn handle_normal_mode_key(&mut self, event: KeyEvent) -> Result<()> {
        match (event.code, event.modifiers) {
            // Send message
            (KeyCode::Enter, KeyModifiers::NONE) => {
                if !self.content.trim().is_empty() {
                    // TODO: Emit SendMessage event
                    self.add_to_history(self.content.clone());
                    let attachments = self.attachments.clone();
                    self.clear();
                    self.attachments.clear();
                    // In a real implementation, you'd emit an event here
                }
            }
            
            // Insert newline
            (KeyCode::Enter, KeyModifiers::SHIFT) => {
                self.insert_newline();
            }

            // Character input
            (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                self.insert_text(&c.to_string());
            }

            // Navigation
            (KeyCode::Left, KeyModifiers::NONE) => self.move_cursor(CursorDirection::Left),
            (KeyCode::Right, KeyModifiers::NONE) => self.move_cursor(CursorDirection::Right),
            (KeyCode::Up, KeyModifiers::NONE) => self.move_cursor(CursorDirection::Up),
            (KeyCode::Down, KeyModifiers::NONE) => self.move_cursor(CursorDirection::Down),
            (KeyCode::Home, KeyModifiers::NONE) => self.move_cursor(CursorDirection::Home),
            (KeyCode::End, KeyModifiers::NONE) => self.move_cursor(CursorDirection::End),
            (KeyCode::PageUp, KeyModifiers::NONE) => self.move_cursor(CursorDirection::PageUp),
            (KeyCode::PageDown, KeyModifiers::NONE) => self.move_cursor(CursorDirection::PageDown),

            // History navigation
            (KeyCode::Up, KeyModifiers::CONTROL) => {
                self.history_previous();
            }
            (KeyCode::Down, KeyModifiers::CONTROL) => {
                self.history_next();
            }

            // Editing
            (KeyCode::Backspace, KeyModifiers::NONE) => self.delete_previous_char(),
            (KeyCode::Delete, KeyModifiers::NONE) => self.delete_char(),

            // Selection
            (KeyCode::Char('a'), KeyModifiers::CONTROL) => self.select_all(),

            // Copy/Cut/Paste (simplified - would need clipboard integration)
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                // TODO: Copy to clipboard
            }
            (KeyCode::Char('x'), KeyModifiers::CONTROL) => {
                // TODO: Cut to clipboard
            }
            (KeyCode::Char('v'), KeyModifiers::CONTROL) => {
                // TODO: Paste from clipboard
            }

            // Toggle modes
            (KeyCode::Char(':'), KeyModifiers::NONE) => {
                self.mode = EditorMode::Command;
            }
            (KeyCode::Char('/'), KeyModifiers::CONTROL) => {
                self.mode = EditorMode::Search(String::new());
            }

            _ => {}
        }
        Ok(())
    }

    async fn handle_command_mode_key(&mut self, event: KeyEvent) -> Result<()> {
        match event.code {
            KeyCode::Esc => {
                self.mode = EditorMode::Normal;
            }
            KeyCode::Enter => {
                // TODO: Execute command
                self.mode = EditorMode::Normal;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_search_mode_key(&mut self, event: KeyEvent) -> Result<()> {
        match event.code {
            KeyCode::Esc => {
                self.mode = EditorMode::Normal;
            }
            KeyCode::Enter => {
                // TODO: Perform search
                self.mode = EditorMode::Normal;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_attachment_mode_key(&mut self, event: KeyEvent) -> Result<()> {
        match event.code {
            KeyCode::Esc => {
                self.mode = EditorMode::Normal;
            }
            _ => {}
        }
        Ok(())
    }
}

#[async_trait]
impl TextInput for ChatEditor {
    async fn insert_char(&mut self, c: char) -> Result<()> {
        self.insert_text(&c.to_string());
        Ok(())
    }

    async fn delete_char(&mut self) -> Result<()> {
        self.delete_char();
        Ok(())
    }

    async fn delete_previous_char(&mut self) -> Result<()> {
        self.delete_previous_char();
        Ok(())
    }

    fn get_text(&self) -> &str {
        &self.content
    }

    fn set_text(&mut self, text: String) {
        self.set_content(text);
    }

    fn cursor_position(&self) -> usize {
        self.cursor_position
    }

    fn set_cursor_position(&mut self, pos: usize) {
        self.cursor_position = pos.min(self.content.len());
        self.update_cursor_from_position();
    }
}

impl Default for ChatEditor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_creation() {
        let editor = ChatEditor::new();
        assert_eq!(editor.get_content(), "");
        assert_eq!(editor.cursor_position, 0);
        assert_eq!(editor.lines.len(), 1);
    }

    #[test]
    fn test_text_insertion() {
        let mut editor = ChatEditor::new();
        editor.insert_text("Hello");
        assert_eq!(editor.get_content(), "Hello");
        
        editor.insert_text(" World");
        assert_eq!(editor.get_content(), "Hello World");
    }

    #[test]
    fn test_multiline_editing() {
        let mut editor = ChatEditor::new();
        editor.insert_text("Line 1");
        editor.insert_newline();
        editor.insert_text("Line 2");
        
        assert_eq!(editor.get_content(), "Line 1\nLine 2");
        assert_eq!(editor.lines.len(), 2);
        assert_eq!(editor.cursor_line, 1);
    }

    #[test]
    fn test_cursor_movement() {
        let mut editor = ChatEditor::new();
        editor.insert_text("Hello\nWorld");
        
        // Move to start
        editor.move_cursor(CursorDirection::Home);
        assert_eq!(editor.cursor_column, 0);
        
        // Move up
        editor.move_cursor(CursorDirection::Up);
        assert_eq!(editor.cursor_line, 0);
    }

    #[test]
    fn test_history() {
        let mut editor = ChatEditor::new();
        editor.add_to_history("First message".to_string());
        editor.add_to_history("Second message".to_string());
        
        assert!(editor.history_previous());
        assert_eq!(editor.get_content(), "Second message");
        
        assert!(editor.history_previous());
        assert_eq!(editor.get_content(), "First message");
    }

    #[test]
    fn test_attachments() {
        let mut editor = ChatEditor::new();
        let attachment = MessageAttachment::new(
            "test.txt".to_string(),
            "text/plain".to_string(),
            b"test content".to_vec(),
        );
        
        assert!(editor.add_attachment(attachment).is_ok());
        assert_eq!(editor.attachments.len(), 1);
        
        assert!(editor.remove_attachment(0).is_ok());
        assert_eq!(editor.attachments.len(), 0);
    }
}