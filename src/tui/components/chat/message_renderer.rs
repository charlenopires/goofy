//! Rich message renderer for chat messages
//!
//! This module provides sophisticated rendering of chat messages with support for
//! markdown, syntax highlighting, tool calls, attachments, and streaming updates.

use super::message_types::{ChatMessage, MessageDisplayOptions, ToolResult, MessageAttachment, CodeBlock};
use crate::llm::types::{ContentBlock, MessageRole, ToolCall};
use crate::tui::themes::{Theme, ThemeManager};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Wrap},
    Frame,
};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Message renderer with rich formatting capabilities
pub struct MessageRenderer {
    theme_manager: ThemeManager,
    display_options: MessageDisplayOptions,
    markdown_parser: MarkdownParser,
    syntax_highlighter: SyntaxHighlighter,
    animation_state: AnimationState,
}

/// Markdown parsing helper
struct MarkdownParser {
    code_block_style: Style,
    inline_code_style: Style,
    bold_style: Style,
    italic_style: Style,
    link_style: Style,
    quote_style: Style,
}

/// Syntax highlighting helper
struct SyntaxHighlighter {
    language_styles: HashMap<String, LanguageStyle>,
}

/// Language-specific syntax highlighting styles
#[derive(Clone)]
struct LanguageStyle {
    keyword_style: Style,
    string_style: Style,
    comment_style: Style,
    number_style: Style,
    function_style: Style,
    type_style: Style,
}

/// Animation state for loading indicators
struct AnimationState {
    frame: usize,
    last_update: std::time::Instant,
    spinner_chars: Vec<char>,
    thinking_chars: Vec<char>,
}

/// Rendered message information
#[derive(Debug, Clone)]
pub struct RenderedMessage {
    pub header_height: u16,
    pub content_height: u16,
    pub total_height: u16,
    pub tool_calls_height: u16,
    pub attachments_height: u16,
}

impl MessageRenderer {
    /// Create a new message renderer
    pub fn new() -> Self {
        let theme_manager = ThemeManager::new();
        let current_theme = theme_manager.current_theme().clone();

        Self {
            markdown_parser: MarkdownParser::new(&current_theme),
            syntax_highlighter: SyntaxHighlighter::new(&current_theme),
            theme_manager,
            display_options: MessageDisplayOptions::default(),
            animation_state: AnimationState::new(),
        }
    }

    /// Update display options
    pub fn set_display_options(&mut self, options: MessageDisplayOptions) {
        self.display_options = options;
    }

    /// Set theme
    pub fn set_theme(&mut self, theme_name: &str) -> Result<(), String> {
        self.theme_manager.set_theme(theme_name)?;
        let current_theme = self.theme_manager.current_theme();
        self.markdown_parser.update_theme(current_theme);
        self.syntax_highlighter.update_theme(current_theme);
        Ok(())
    }

    /// Render a complete message
    pub fn render_message(
        &mut self,
        message: &ChatMessage,
        frame: &mut Frame,
        area: Rect,
    ) -> RenderedMessage {
        let theme = self.theme_manager.current_theme();
        let mut current_y = area.y;
        let mut heights = RenderedMessage {
            header_height: 0,
            content_height: 0,
            total_height: 0,
            tool_calls_height: 0,
            attachments_height: 0,
        };

        // Render message header
        if !self.display_options.compact_mode {
            let header_area = Rect {
                x: area.x,
                y: current_y,
                width: area.width,
                height: 2,
            };
            self.render_message_header(message, frame, header_area);
            heights.header_height = 2;
            current_y += 2;
        }

        // Render thinking content if available and streaming
        if message.has_thinking_content() && self.display_options.show_thinking {
            let thinking_height = self.render_thinking_content(
                message,
                frame,
                Rect {
                    x: area.x,
                    y: current_y,
                    width: area.width,
                    height: 3,
                },
            );
            current_y += thinking_height;
            heights.content_height += thinking_height;
        }

        // Render main content
        let content_height = self.render_message_content(
            message,
            frame,
            Rect {
                x: area.x,
                y: current_y,
                width: area.width,
                height: area.height.saturating_sub(current_y - area.y),
            },
        );
        heights.content_height += content_height;
        current_y += content_height;

        // Render attachments
        if message.has_attachments() {
            let attachments_height = self.render_attachments(
                &message.attachments,
                frame,
                Rect {
                    x: area.x,
                    y: current_y,
                    width: area.width,
                    height: area.height.saturating_sub(current_y - area.y),
                },
            );
            heights.attachments_height = attachments_height;
            current_y += attachments_height;
        }

        // Render tool calls
        if message.has_tool_calls() {
            let tool_calls_height = self.render_tool_calls(
                &message.tool_calls,
                &message.tool_results,
                frame,
                Rect {
                    x: area.x,
                    y: current_y,
                    width: area.width,
                    height: area.height.saturating_sub(current_y - area.y),
                },
            );
            heights.tool_calls_height = tool_calls_height;
            current_y += tool_calls_height;
        }

        // Render streaming indicator
        if message.is_streaming() {
            self.render_streaming_indicator(
                frame,
                Rect {
                    x: area.x,
                    y: current_y,
                    width: area.width,
                    height: 1,
                },
            );
            current_y += 1;
        }

        heights.total_height = current_y - area.y;
        heights
    }

    /// Render message header with role, timestamp, and metadata
    fn render_message_header(&self, message: &ChatMessage, frame: &mut Frame, area: Rect) {
        let theme = self.theme_manager.current_theme();
        
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

        let mut spans = vec![
            Span::styled(role_icon.clone(), role_style),
            Span::raw(" "),
            Span::styled(format!("{:?}", message.role), role_style),
        ];

        if self.display_options.show_timestamps {
            let timestamp = message.timestamp.format("%H:%M:%S").to_string();
            spans.extend([
                Span::raw(" • "),
                Span::styled(timestamp, theme.styles.muted),
            ]);
        }

        if self.display_options.show_metadata && !message.metadata.is_empty() {
            spans.extend([
                Span::raw(" • "),
                Span::styled(
                    format!("{} meta", message.metadata.len()),
                    theme.styles.subtle,
                ),
            ]);
        }

        let header = Paragraph::new(Line::from(spans))
            .style(theme.styles.base)
            .wrap(Wrap { trim: true });

        frame.render_widget(header, area);
    }

    /// Render thinking content with animation
    fn render_thinking_content(&mut self, message: &ChatMessage, frame: &mut Frame, area: Rect) -> u16 {
        let theme = self.theme_manager.current_theme();
        
        if let Some(thinking_content) = &message.thinking_content {
            let mut lines = vec![
                Line::from(vec![
                    Span::styled("🤔 ", theme.styles.info),
                    if message.is_streaming() {
                        Span::styled(
                            format!("Thinking{}", self.animation_state.get_thinking_indicator()),
                            theme.styles.info,
                        )
                    } else {
                        Span::styled("Thinking complete", theme.styles.success)
                    },
                ]),
            ];

            if !thinking_content.is_empty() && thinking_content.len() < 100 {
                lines.push(Line::from(Span::styled(
                    thinking_content.clone(),
                    theme.styles.subtle,
                )));
            }

            let num_lines = lines.len() as u16;
            let thinking_widget = Paragraph::new(Text::from(lines))
                .block(Block::default().borders(Borders::LEFT).border_style(theme.styles.info))
                .wrap(Wrap { trim: true });

            frame.render_widget(thinking_widget, area);
            num_lines
        } else {
            0
        }
    }

    /// Render main message content with markdown and syntax highlighting
    fn render_message_content(&mut self, message: &ChatMessage, frame: &mut Frame, area: Rect) -> u16 {
        let theme = self.theme_manager.current_theme();
        let mut lines = Vec::new();

        for block in &message.content {
            match block {
                ContentBlock::Text { text } => {
                    if self.display_options.markdown_rendering {
                        lines.extend(self.markdown_parser.parse_markdown(text));
                    } else {
                        lines.extend(self.render_plain_text(text));
                    }
                }
                ContentBlock::Image { .. } => {
                    lines.push(Line::from(vec![
                        Span::styled("🖼️ ", theme.styles.info),
                        Span::styled("[Image]", theme.styles.info),
                    ]));
                }
                ContentBlock::ToolUse { name, input, .. } => {
                    lines.push(Line::from(vec![
                        Span::styled("🔧 ", theme.styles.chat_tool_message),
                        Span::styled(format!("Using tool: {}", name), theme.styles.chat_tool_message),
                    ]));
                    if let Ok(formatted_input) = serde_json::to_string_pretty(input) {
                        lines.extend(self.syntax_highlighter.highlight_json(&formatted_input));
                    }
                }
                ContentBlock::ToolResult { content, .. } => {
                    lines.push(Line::from(vec![
                        Span::styled("📋 ", theme.styles.success),
                        Span::styled("Tool Result:", theme.styles.success),
                    ]));
                    lines.extend(self.render_plain_text(content));
                }
            }
        }

        // Calculate actual height used before moving lines
        let available_width = area.width as usize;
        let wrapped_lines = lines.iter()
            .map(|line| {
                let line_width = line.width();
                if line_width > available_width {
                    (line_width + available_width - 1) / available_width
                } else {
                    1
                }
            })
            .sum::<usize>() as u16;

        let content_widget = Paragraph::new(Text::from(lines))
            .style(theme.styles.text)
            .wrap(if self.display_options.word_wrap {
                Wrap { trim: true }
            } else {
                Wrap { trim: false }
            });

        frame.render_widget(content_widget, area);

        wrapped_lines.min(area.height)
    }

    /// Render attachments with file information
    fn render_attachments(&self, attachments: &[MessageAttachment], frame: &mut Frame, area: Rect) -> u16 {
        let theme = self.theme_manager.current_theme();
        
        if attachments.is_empty() {
            return 0;
        }

        let mut lines = vec![
            Line::from(vec![
                Span::styled("📎 ", theme.styles.info),
                Span::styled("Attachments:", theme.styles.info),
            ]),
        ];

        for attachment in attachments {
            let icon = if attachment.is_image() {
                "🖼️"
            } else if attachment.is_text() {
                "📄"
            } else {
                "📁"
            };

            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(icon, theme.styles.info),
                Span::raw(" "),
                Span::styled(&attachment.filename, theme.styles.text),
                Span::raw(" "),
                Span::styled(
                    format!("({})", attachment.formatted_size()),
                    theme.styles.muted,
                ),
            ]));
        }

        let attachments_widget = Paragraph::new(Text::from(lines))
            .block(Block::default().borders(Borders::LEFT).border_style(theme.styles.info))
            .wrap(Wrap { trim: true });

        frame.render_widget(attachments_widget, area);
        (attachments.len() + 1) as u16
    }

    /// Render tool calls with their status and results
    fn render_tool_calls(
        &self,
        tool_calls: &[ToolCall],
        tool_results: &[ToolResult],
        frame: &mut Frame,
        area: Rect,
    ) -> u16 {
        let theme = self.theme_manager.current_theme();
        
        if tool_calls.is_empty() {
            return 0;
        }

        let mut lines = Vec::new();
        let mut total_height = 0u16;

        for tool_call in tool_calls {
            // Find corresponding result
            let result = tool_results.iter().find(|r| r.tool_call_id == tool_call.id);
            
            let status_icon = match result {
                Some(r) if r.is_error() => "❌",
                Some(_) => "✅",
                None => "⏳",
            };

            let status_style = match result {
                Some(r) if r.is_error() => theme.styles.error,
                Some(_) => theme.styles.success,
                None => theme.styles.warning,
            };

            lines.push(Line::from(vec![
                Span::styled(status_icon, status_style),
                Span::raw(" "),
                Span::styled(format!("Tool: {}", tool_call.name), theme.styles.chat_tool_message),
            ]));
            total_height += 1;

            // Render arguments
            if let Ok(formatted_args) = serde_json::to_string_pretty(&tool_call.arguments) {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Arguments:", theme.styles.muted),
                ]));
                let arg_lines = self.syntax_highlighter.highlight_json(&formatted_args);
                for arg_line in arg_lines.iter().take(3) { // Limit to 3 lines
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        arg_line.spans[0].clone(),
                    ]));
                }
                total_height += arg_lines.len().min(3) as u16 + 1;
            }

            // Render result if available
            if let Some(result) = result {
                if result.is_error() {
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled("Error:", theme.styles.error),
                        Span::raw(" "),
                        Span::styled(
                            result.error.as_ref().unwrap_or(&result.content),
                            theme.styles.error,
                        ),
                    ]));
                    total_height += 1;
                } else if !result.content.is_empty() {
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled("Result:", theme.styles.success),
                    ]));
                    total_height += 1;
                    
                    // Show first few lines of result
                    for line in result.content.lines().take(3) {
                        lines.push(Line::from(vec![
                            Span::raw("    "),
                            Span::styled(line, theme.styles.text),
                        ]));
                        total_height += 1;
                    }
                }
            }

            lines.push(Line::from(Span::raw(""))); // Spacing
            total_height += 1;
        }

        let tool_calls_widget = Paragraph::new(Text::from(lines))
            .block(Block::default().borders(Borders::LEFT).border_style(theme.styles.chat_tool_message))
            .wrap(Wrap { trim: true });

        frame.render_widget(tool_calls_widget, area);
        total_height
    }

    /// Render streaming indicator with animation
    fn render_streaming_indicator(&mut self, frame: &mut Frame, area: Rect) {
        let theme = self.theme_manager.current_theme();
        
        let indicator = Line::from(vec![
            Span::styled(
                self.animation_state.get_spinner(),
                theme.styles.info,
            ),
            Span::raw(" "),
            Span::styled("Streaming...", theme.styles.info),
        ]);

        let widget = Paragraph::new(indicator).style(theme.styles.base);
        frame.render_widget(widget, area);
        
        self.animation_state.update();
    }

    /// Render plain text without markdown processing
    fn render_plain_text(&self, text: &str) -> Vec<Line<'static>> {
        text.lines()
            .map(|line| Line::from(Span::raw(line.to_string())))
            .collect()
    }

    /// Calculate the height needed to render a message
    pub fn calculate_message_height(&self, message: &ChatMessage, width: u16) -> u16 {
        let mut height = 0u16;

        // Header height
        if !self.display_options.compact_mode {
            height += 2;
        }

        // Thinking content height
        if message.has_thinking_content() && self.display_options.show_thinking {
            height += 3;
        }

        // Content height (approximate)
        let text_content = message.get_text_content();
        let content_lines = text_content.lines().count() as u16;
        let estimated_wrapped_lines = content_lines * 2; // Conservative estimate
        height += estimated_wrapped_lines;

        // Attachments height
        if message.has_attachments() {
            height += message.attachments.len() as u16 + 1;
        }

        // Tool calls height (approximate)
        if message.has_tool_calls() {
            height += message.tool_calls.len() as u16 * 5; // Conservative estimate
        }

        // Streaming indicator
        if message.is_streaming() {
            height += 1;
        }

        height
    }
}

impl MarkdownParser {
    fn new(theme: &Theme) -> Self {
        Self {
            code_block_style: theme.styles.text.add_modifier(Modifier::ITALIC),
            inline_code_style: theme.styles.text.add_modifier(Modifier::BOLD),
            bold_style: theme.styles.text.add_modifier(Modifier::BOLD),
            italic_style: theme.styles.text.add_modifier(Modifier::ITALIC),
            link_style: Style::default().fg(theme.colors.blue),
            quote_style: theme.styles.muted.add_modifier(Modifier::ITALIC),
        }
    }

    fn update_theme(&mut self, theme: &Theme) {
        self.code_block_style = theme.styles.text.add_modifier(Modifier::ITALIC);
        self.inline_code_style = theme.styles.text.add_modifier(Modifier::BOLD);
        self.bold_style = theme.styles.text.add_modifier(Modifier::BOLD);
        self.italic_style = theme.styles.text.add_modifier(Modifier::ITALIC);
        self.link_style = Style::default().fg(theme.colors.blue);
        self.quote_style = theme.styles.muted.add_modifier(Modifier::ITALIC);
    }

    fn parse_markdown(&self, text: &str) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let mut in_code_block = false;
        let mut code_language: Option<String> = None;

        for line in text.lines() {
            if line.starts_with("```") {
                if in_code_block {
                    // End of code block
                    in_code_block = false;
                    code_language = None;
                } else {
                    // Start of code block
                    in_code_block = true;
                    let lang = line.trim_start_matches("```").trim();
                    code_language = if lang.is_empty() {
                        None
                    } else {
                        Some(lang.to_string())
                    };
                    lines.push(Line::from(Span::styled(
                        format!("📄 {}", lang),
                        self.code_block_style,
                    )));
                    continue;
                }
            }

            if in_code_block {
                lines.push(Line::from(Span::styled(
                    line.to_string(),
                    self.code_block_style,
                )));
            } else {
                lines.push(self.parse_inline_markdown(line));
            }
        }

        lines
    }

    fn parse_inline_markdown(&self, line: &str) -> Line<'static> {
        let mut spans = Vec::new();
        let mut chars = line.chars().peekable();
        let mut current_text = String::new();

        while let Some(ch) = chars.next() {
            match ch {
                '*' if chars.peek() == Some(&'*') => {
                    // Bold text **text**
                    if !current_text.is_empty() {
                        spans.push(Span::raw(current_text.clone()));
                        current_text.clear();
                    }
                    chars.next(); // consume second *
                    
                    let bold_text = self.consume_until(&mut chars, "**");
                    spans.push(Span::styled(bold_text, self.bold_style));
                }
                '*' => {
                    // Italic text *text*
                    if !current_text.is_empty() {
                        spans.push(Span::raw(current_text.clone()));
                        current_text.clear();
                    }
                    
                    let italic_text = self.consume_until(&mut chars, "*");
                    spans.push(Span::styled(italic_text, self.italic_style));
                }
                '`' => {
                    // Inline code `code`
                    if !current_text.is_empty() {
                        spans.push(Span::raw(current_text.clone()));
                        current_text.clear();
                    }
                    
                    let code_text = self.consume_until(&mut chars, "`");
                    spans.push(Span::styled(code_text, self.inline_code_style));
                }
                _ => {
                    current_text.push(ch);
                }
            }
        }

        if !current_text.is_empty() {
            spans.push(Span::raw(current_text));
        }

        Line::from(spans)
    }

    fn consume_until(&self, chars: &mut std::iter::Peekable<std::str::Chars>, delimiter: &str) -> String {
        let mut text = String::new();
        let delimiter_chars: Vec<char> = delimiter.chars().collect();
        
        while let Some(ch) = chars.next() {
            if ch == delimiter_chars[0] {
                // Check if this is the start of our delimiter
                let mut is_delimiter = true;
                let mut lookahead = Vec::new();
                
                for &expected_char in &delimiter_chars[1..] {
                    if let Some(next_ch) = chars.next() {
                        lookahead.push(next_ch);
                        if next_ch != expected_char {
                            is_delimiter = false;
                            break;
                        }
                    } else {
                        is_delimiter = false;
                        break;
                    }
                }
                
                if is_delimiter {
                    // Found the delimiter, return the text
                    return text;
                } else {
                    // Not the delimiter, add all chars to text
                    text.push(ch);
                    text.extend(lookahead);
                }
            } else {
                text.push(ch);
            }
        }
        
        text
    }
}

impl SyntaxHighlighter {
    fn new(theme: &Theme) -> Self {
        let mut language_styles = HashMap::new();
        
        // Add language styles
        language_styles.insert("json".to_string(), LanguageStyle {
            keyword_style: Style::default().fg(theme.colors.blue),
            string_style: Style::default().fg(theme.colors.green),
            comment_style: theme.styles.muted,
            number_style: Style::default().fg(theme.colors.yellow),
            function_style: Style::default().fg(theme.colors.blue_light),
            type_style: Style::default().fg(theme.colors.green_light),
        });
        
        Self { language_styles }
    }

    fn update_theme(&mut self, theme: &Theme) {
        // Update all language styles with new theme
        for style in self.language_styles.values_mut() {
            style.keyword_style = Style::default().fg(theme.colors.blue);
            style.string_style = Style::default().fg(theme.colors.green);
            style.comment_style = theme.styles.muted;
            style.number_style = Style::default().fg(theme.colors.yellow);
            style.function_style = Style::default().fg(theme.colors.blue_light);
            style.type_style = Style::default().fg(theme.colors.green_light);
        }
    }

    fn highlight_json(&self, code: &str) -> Vec<Line<'static>> {
        let style = self.language_styles.get("json").unwrap();
        let mut lines = Vec::new();

        for line in code.lines() {
            let mut spans = Vec::new();
            let trimmed = line.trim();

            if trimmed.starts_with('"') && trimmed.contains(':') {
                // Property name
                if let Some(colon_pos) = trimmed.find(':') {
                    let key = &trimmed[..colon_pos];
                    let value = &trimmed[colon_pos..];
                    spans.push(Span::styled(key.to_string(), style.string_style));
                    spans.push(Span::styled(value.to_string(), Style::default()));
                } else {
                    spans.push(Span::raw(line.to_string()));
                }
            } else if trimmed.starts_with('"') {
                // String value
                spans.push(Span::styled(line.to_string(), style.string_style));
            } else if trimmed.parse::<f64>().is_ok() {
                // Number
                spans.push(Span::styled(line.to_string(), style.number_style));
            } else {
                spans.push(Span::raw(line.to_string()));
            }

            lines.push(Line::from(spans));
        }

        lines
    }
}

impl AnimationState {
    fn new() -> Self {
        Self {
            frame: 0,
            last_update: std::time::Instant::now(),
            spinner_chars: vec!['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'],
            thinking_chars: vec!['.', '.', '.'],
        }
    }

    fn get_spinner(&self) -> String {
        self.spinner_chars[self.frame % self.spinner_chars.len()].to_string()
    }

    fn get_thinking_indicator(&self) -> String {
        self.thinking_chars[self.frame % self.thinking_chars.len()].to_string()
    }

    fn update(&mut self) {
        let now = std::time::Instant::now();
        if now.duration_since(self.last_update).as_millis() > 200 {
            self.frame += 1;
            self.last_update = now;
        }
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
    use crate::llm::types::MessageRole;

    #[test]
    fn test_message_renderer_creation() {
        let renderer = MessageRenderer::new();
        // Just ensure it creates without panicking
        assert!(!renderer.theme_manager.list_themes().is_empty());
    }

    #[test]
    fn test_markdown_parsing() {
        let theme = ThemeManager::new();
        let parser = MarkdownParser::new(theme.current_theme());
        
        let lines = parser.parse_markdown("This is **bold** and *italic* text with `code`.");
        assert_eq!(lines.len(), 1);
        
        // Should have multiple spans for different formatting
        assert!(lines[0].spans.len() > 1);
    }

    #[test]
    fn test_height_calculation() {
        let renderer = MessageRenderer::new();
        let message = super::super::message_types::ChatMessage::new_user_text("Hello, world!".to_string());
        
        let height = renderer.calculate_message_height(&message, 80);
        assert!(height > 0);
    }
}