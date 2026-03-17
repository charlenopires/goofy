//! Advanced chat components for rich messaging experience
//! 
//! This module provides enhanced chat components including a multi-line editor,
//! rich message rendering, and interactive message list.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use tui_textarea::{TextArea, Input, Key};
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;
use crate::{
    llm::{Message, MessageRole, ContentBlock},
    tui::{
        themes::{Theme, ThemeManager, StylePresets, TextStyler, SyntaxTokenType},
        events::Event,
    },
};

/// Enhanced chat editor with syntax highlighting and auto-completion
pub struct ChatEditor {
    textarea: TextArea<'static>,
    theme_manager: ThemeManager,
    syntax_highlighting: bool,
    auto_complete: bool,
    completion_popup: Option<CompletionPopup>,
    placeholder: String,
}

impl ChatEditor {
    /// Create a new chat editor
    pub fn new() -> Self {
        let mut textarea = TextArea::default();
        textarea.set_placeholder_text("Type your message here...");
        textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title("Message")
        );
        
        Self {
            textarea,
            theme_manager: ThemeManager::new(),
            syntax_highlighting: true,
            auto_complete: true,
            completion_popup: None,
            placeholder: "Type your message here...".to_string(),
        }
    }
    
    /// Set the theme
    pub fn set_theme(&mut self, theme_name: &str) -> Result<(), String> {
        self.theme_manager.set_theme(theme_name)?;
        self.update_textarea_style();
        Ok(())
    }
    
    /// Update textarea styling based on current theme
    fn update_textarea_style(&mut self) {
        let theme = self.theme_manager.current_theme();
        let focused_style = StylePresets::input_field(theme, true, false);
        let blurred_style = StylePresets::input_field(theme, false, false);
        
        self.textarea.set_style(focused_style);
        // Note: tui-textarea doesn't directly support theme switching
        // This would need to be implemented through custom styling
    }
    
    /// Handle input events
    pub fn handle_input(&mut self, input: Input) -> bool {
        match input.key {
            Key::Enter if input.ctrl => {
                // Ctrl+Enter submits the message
                return true;
            }
            Key::Tab => {
                // Handle auto-completion
                if self.auto_complete {
                    self.trigger_completion();
                }
            }
            _ => {
                self.textarea.input(input);
                
                // Update completion popup if needed
                if self.auto_complete && self.completion_popup.is_some() {
                    self.update_completion();
                }
            }
        }
        false
    }
    
    /// Get the current text content
    pub fn content(&self) -> String {
        self.textarea.lines().join("\n")
    }
    
    /// Clear the editor content
    pub fn clear(&mut self) {
        self.textarea = TextArea::default();
        self.textarea.set_placeholder_text(&self.placeholder);
        self.update_textarea_style();
        self.completion_popup = None;
    }
    
    /// Set placeholder text
    pub fn set_placeholder(&mut self, placeholder: String) {
        self.placeholder = placeholder;
        self.textarea.set_placeholder_text(&self.placeholder);
    }
    
    /// Trigger auto-completion
    fn trigger_completion(&mut self) {
        let cursor_pos = self.textarea.cursor();
        let current_line = self.textarea.lines().get(cursor_pos.0).unwrap_or(&String::new());
        let current_word = self.get_current_word(current_line, cursor_pos.1);
        
        if !current_word.is_empty() {
            let completions = self.get_completions(&current_word);
            if !completions.is_empty() {
                self.completion_popup = Some(CompletionPopup::new(completions, cursor_pos));
            }
        }
    }
    
    /// Update completion popup based on current input
    fn update_completion(&mut self) {
        if let Some(popup) = &mut self.completion_popup {
            let cursor_pos = self.textarea.cursor();
            let current_line = self.textarea.lines().get(cursor_pos.0).unwrap_or(&String::new());
            let current_word = self.get_current_word(current_line, cursor_pos.1);
            
            if current_word.is_empty() {
                self.completion_popup = None;
            } else {
                popup.filter(&current_word);
                if popup.is_empty() {
                    self.completion_popup = None;
                }
            }
        }
    }
    
    /// Get the current word at cursor position
    fn get_current_word(&self, line: &str, col: usize) -> String {
        let chars: Vec<&str> = line.graphemes(true).collect();
        let mut start = col;
        let mut end = col;
        
        // Find word boundaries
        while start > 0 && chars.get(start - 1).map_or(false, |c| c.is_alphanumeric() || *c == "_") {
            start -= 1;
        }
        
        while end < chars.len() && chars.get(end).map_or(false, |c| c.is_alphanumeric() || *c == "_") {
            end += 1;
        }
        
        chars[start..end].join("")
    }
    
    /// Get completion suggestions for a word
    fn get_completions(&self, word: &str) -> Vec<String> {
        // This is a simplified completion system
        // In a full implementation, this would integrate with LSP or other completion sources
        let mut completions = Vec::new();
        
        // Common programming keywords
        let keywords = vec![
            "function", "const", "let", "var", "if", "else", "for", "while",
            "class", "interface", "type", "import", "export", "return",
            "async", "await", "try", "catch", "finally", "throw",
        ];
        
        for keyword in keywords {
            if keyword.starts_with(word) {
                completions.push(keyword.to_string());
            }
        }
        
        // Limit to top 10 completions
        completions.truncate(10);
        completions
    }
    
    /// Render the editor
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        // Apply syntax highlighting if enabled
        if self.syntax_highlighting {
            self.apply_syntax_highlighting();
        }
        
        frame.render_widget(&self.textarea, area);
        
        // Render completion popup if active
        if let Some(popup) = &self.completion_popup {
            popup.render(frame, area);
        }
    }
    
    /// Apply syntax highlighting to the textarea content
    fn apply_syntax_highlighting(&mut self) {
        // This is a simplified syntax highlighting implementation
        // A full implementation would use a proper syntax highlighting library
        let theme = self.theme_manager.current_theme();
        
        // For now, we'll just highlight keywords in a simple way
        // Note: tui-textarea has limited support for syntax highlighting
        // This would need to be implemented through custom rendering
    }
}

/// Auto-completion popup
struct CompletionPopup {
    items: Vec<String>,
    selected: usize,
    position: (usize, usize), // (row, col)
}

impl CompletionPopup {
    fn new(items: Vec<String>, position: (usize, usize)) -> Self {
        Self {
            items,
            selected: 0,
            position,
        }
    }
    
    fn filter(&mut self, prefix: &str) {
        self.items = self.items.iter()
            .filter(|item| item.starts_with(prefix))
            .cloned()
            .collect();
        self.selected = 0;
    }
    
    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    
    fn render(&self, frame: &mut Frame, editor_area: Rect) {
        if self.items.is_empty() {
            return;
        }
        
        let popup_height = (self.items.len() + 2).min(8) as u16;
        let popup_width = self.items.iter()
            .map(|item| item.len())
            .max()
            .unwrap_or(10)
            .max(10) as u16 + 2;
        
        // Position popup near cursor
        let popup_x = editor_area.x + self.position.1 as u16;
        let popup_y = editor_area.y + self.position.0 as u16 + 1;
        
        let popup_area = Rect {
            x: popup_x.min(frame.size().width.saturating_sub(popup_width)),
            y: popup_y.min(frame.size().height.saturating_sub(popup_height)),
            width: popup_width,
            height: popup_height,
        };
        
        // Clear the popup area
        frame.render_widget(Clear, popup_area);
        
        // Render the completion list
        let items: Vec<ListItem> = self.items.iter()
            .enumerate()
            .map(|(i, item)| {
                let style = if i == self.selected {
                    Style::default().bg(Color::Blue).fg(Color::White)
                } else {
                    Style::default()
                };
                ListItem::new(item.as_str()).style(style)
            })
            .collect();
        
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Completions"))
            .highlight_style(Style::default().bg(Color::Blue));
        
        frame.render_widget(list, popup_area);
    }
}

/// Rich message renderer with markdown support and syntax highlighting
pub struct MessageRenderer {
    theme_manager: ThemeManager,
    syntax_highlighter: SyntaxHighlighter,
    markdown_enabled: bool,
}

impl MessageRenderer {
    /// Create a new message renderer
    pub fn new() -> Self {
        Self {
            theme_manager: ThemeManager::new(),
            syntax_highlighter: SyntaxHighlighter::new(),
            markdown_enabled: true,
        }
    }
    
    /// Set the theme
    pub fn set_theme(&mut self, theme_name: &str) -> Result<(), String> {
        self.theme_manager.set_theme(theme_name)
    }
    
    /// Render a message
    pub fn render_message(&self, message: &Message, area: Rect, frame: &mut Frame) {
        let theme = self.theme_manager.current_theme();
        
        // Create message header with role and timestamp
        let header = self.create_message_header(message, theme);
        
        // Process message content
        let content = self.process_message_content(message, theme);
        
        // Layout: header + content
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(1)])
            .split(area);
        
        // Render header
        frame.render_widget(Paragraph::new(header), chunks[0]);
        
        // Render content
        let content_widget = Paragraph::new(content)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::NONE));
        
        frame.render_widget(content_widget, chunks[1]);
    }
    
    /// Create message header with role and timestamp
    fn create_message_header(&self, message: &Message, theme: &Theme) -> Text<'static> {
        let role_style = match message.role {
            MessageRole::User => theme.styles.chat_user_message,
            MessageRole::Assistant => theme.styles.chat_assistant_message,
            MessageRole::System => theme.styles.chat_system_message,
            MessageRole::Tool => theme.styles.chat_tool_message,
        };
        
        let role_icon = match message.role {
            MessageRole::User => &theme.icons.user,
            MessageRole::Assistant => &theme.icons.assistant,
            MessageRole::System => &theme.icons.system,
            MessageRole::Tool => &theme.icons.tool,
        };
        
        let timestamp = message.timestamp.format("%H:%M:%S").to_string();
        
        Text::from(vec![
            Line::from(vec![
                Span::styled(role_icon.clone(), role_style),
                Span::raw(" "),
                Span::styled(format!("{:?}", message.role), role_style),
                Span::raw(" • "),
                Span::styled(timestamp, theme.styles.muted),
            ])
        ])
    }
    
    /// Process message content with markdown and syntax highlighting
    fn process_message_content(&self, message: &Message, theme: &Theme) -> Text<'static> {
        let mut lines = Vec::new();
        
        for block in &message.content {
            match block {
                ContentBlock::Text { text } => {
                    if self.markdown_enabled {
                        lines.extend(self.render_markdown(text, theme));
                    } else {
                        lines.push(Line::from(text.clone()));
                    }
                }
                ContentBlock::ToolUse { name, input, .. } => {
                    lines.push(Line::from(vec![
                        Span::styled("🔧 ", theme.styles.chat_tool_message),
                        Span::styled(format!("Tool: {}", name), theme.styles.chat_tool_message),
                    ]));
                    if let Ok(pretty_input) = serde_json::to_string_pretty(input) {
                        lines.extend(self.syntax_highlighter.highlight_json(&pretty_input, theme));
                    }
                }
                ContentBlock::ToolResult { content, .. } => {
                    lines.push(Line::from(vec![
                        Span::styled("📋 ", theme.styles.success),
                        Span::styled("Tool Result:", theme.styles.success),
                    ]));
                    lines.push(Line::from(content.clone()));
                }
                ContentBlock::Image { .. } => {
                    lines.push(Line::from(vec![
                        Span::styled("🖼️ ", theme.styles.info),
                        Span::styled("[Image]", theme.styles.info),
                    ]));
                }
            }
        }
        
        Text::from(lines)
    }
    
    /// Render markdown text with basic formatting
    fn render_markdown(&self, text: &str, theme: &Theme) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        
        for line in text.lines() {
            if line.starts_with("# ") {
                // H1
                lines.push(Line::from(Span::styled(
                    line.trim_start_matches("# "),
                    theme.styles.title,
                )));
            } else if line.starts_with("## ") {
                // H2
                lines.push(Line::from(Span::styled(
                    line.trim_start_matches("## "),
                    theme.styles.subtitle,
                )));
            } else if line.starts_with("```") {
                // Code block start/end
                if line.len() > 3 {
                    let lang = line.trim_start_matches("```");
                    lines.push(Line::from(Span::styled(
                        format!("📄 {}", lang),
                        theme.styles.subtle,
                    )));
                }
            } else if line.starts_with("`") && line.ends_with("`") && line.len() > 2 {
                // Inline code
                let code = line.trim_matches('`');
                lines.push(Line::from(Span::styled(
                    code,
                    StylePresets::inline_code(theme),
                )));
            } else {
                // Regular text with basic markdown parsing
                lines.push(self.parse_inline_markdown(line, theme));
            }
        }
        
        lines
    }
    
    /// Parse inline markdown formatting
    fn parse_inline_markdown(&self, text: &str, theme: &Theme) -> Line<'static> {
        let mut spans = Vec::new();
        let mut current = String::new();
        let mut chars = text.chars().peekable();
        
        while let Some(ch) = chars.next() {
            match ch {
                '*' if chars.peek() == Some(&'*') => {
                    // Bold text **text**
                    if !current.is_empty() {
                        spans.push(Span::raw(current.clone()));
                        current.clear();
                    }
                    chars.next(); // consume second *
                    
                    let mut bold_text = String::new();
                    let mut found_end = false;
                    
                    while let Some(ch) = chars.next() {
                        if ch == '*' && chars.peek() == Some(&'*') {
                            chars.next(); // consume second *
                            found_end = true;
                            break;
                        }
                        bold_text.push(ch);
                    }
                    
                    if found_end {
                        spans.push(Span::styled(bold_text, theme.styles.text.add_modifier(Modifier::BOLD)));
                    } else {
                        spans.push(Span::raw(format!("**{}", bold_text)));
                    }
                }
                '*' => {
                    // Italic text *text*
                    if !current.is_empty() {
                        spans.push(Span::raw(current.clone()));
                        current.clear();
                    }
                    
                    let mut italic_text = String::new();
                    let mut found_end = false;
                    
                    while let Some(ch) = chars.next() {
                        if ch == '*' {
                            found_end = true;
                            break;
                        }
                        italic_text.push(ch);
                    }
                    
                    if found_end {
                        spans.push(Span::styled(italic_text, theme.styles.text.add_modifier(Modifier::ITALIC)));
                    } else {
                        spans.push(Span::raw(format!("*{}", italic_text)));
                    }
                }
                _ => {
                    current.push(ch);
                }
            }
        }
        
        if !current.is_empty() {
            spans.push(Span::raw(current));
        }
        
        Line::from(spans)
    }
}

/// Syntax highlighter for code blocks
struct SyntaxHighlighter {
    // In a full implementation, this would integrate with syntect or similar
}

impl SyntaxHighlighter {
    fn new() -> Self {
        Self {}
    }
    
    /// Highlight JSON code
    fn highlight_json(&self, code: &str, theme: &Theme) -> Vec<Line<'static>> {
        // Simplified JSON highlighting
        let mut lines = Vec::new();
        
        for line in code.lines() {
            let mut spans = Vec::new();
            let trimmed = line.trim();
            
            if trimmed.starts_with('"') && trimmed.contains(':') {
                // Property name
                if let Some(colon_pos) = trimmed.find(':') {
                    let key = &trimmed[..colon_pos];
                    let value = &trimmed[colon_pos..];
                    spans.push(Span::styled(key, TextStyler::syntax_highlight(theme, SyntaxTokenType::String)));
                    spans.push(Span::styled(value, theme.styles.text));
                } else {
                    spans.push(Span::raw(line.to_string()));
                }
            } else if trimmed.starts_with('"') {
                // String value
                spans.push(Span::styled(line.to_string(), TextStyler::syntax_highlight(theme, SyntaxTokenType::String)));
            } else if trimmed.parse::<f64>().is_ok() {
                // Number
                spans.push(Span::styled(line.to_string(), TextStyler::syntax_highlight(theme, SyntaxTokenType::Number)));
            } else {
                spans.push(Span::raw(line.to_string()));
            }
            
            lines.push(Line::from(spans));
        }
        
        lines
    }
}

impl Default for ChatEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for MessageRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MessageRole;
    use chrono::Utc;
    use std::collections::HashMap;
    
    #[test]
    fn test_chat_editor_creation() {
        let editor = ChatEditor::new();
        assert!(editor.content().is_empty());
        assert!(editor.syntax_highlighting);
        assert!(editor.auto_complete);
    }
    
    #[test]
    fn test_message_renderer_creation() {
        let renderer = MessageRenderer::new();
        assert!(renderer.markdown_enabled);
    }
    
    #[test]
    fn test_completion_popup() {
        let items = vec!["function".to_string(), "const".to_string()];
        let mut popup = CompletionPopup::new(items, (0, 0));
        
        assert_eq!(popup.items.len(), 2);
        
        popup.filter("func");
        assert_eq!(popup.items.len(), 1);
        assert_eq!(popup.items[0], "function");
        
        popup.filter("xyz");
        assert!(popup.is_empty());
    }
    
    #[test]
    fn test_get_current_word() {
        let editor = ChatEditor::new();
        
        let word = editor.get_current_word("hello world", 3);
        assert_eq!(word, "hello");
        
        let word = editor.get_current_word("hello world", 7);
        assert_eq!(word, "world");
        
        let word = editor.get_current_word("hello_world", 5);
        assert_eq!(word, "hello_world");
    }
}