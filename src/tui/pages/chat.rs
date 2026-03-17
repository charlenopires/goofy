//! Chat page for interactive conversations

use super::{Page, PageId};
use crate::tui::{themes::Theme, Frame};
use anyhow::Result;
use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};
use std::collections::VecDeque;

/// Chat message structure
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

/// Chat page for conversation interface
pub struct ChatPage {
    id: PageId,
    messages: VecDeque<ChatMessage>,
    input_buffer: String,
    cursor_position: usize,
    scroll_offset: usize,
    has_focus: bool,
}

impl ChatPage {
    /// Create a new chat page
    pub fn new() -> Self {
        Self {
            id: "chat".to_string(),
            messages: VecDeque::new(),
            input_buffer: String::new(),
            cursor_position: 0,
            scroll_offset: 0,
            has_focus: true,
        }
    }
    
    /// Add a message to the chat
    pub fn add_message(&mut self, role: String, content: String) {
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        self.messages.push_back(ChatMessage {
            role,
            content,
            timestamp,
        });
        
        // Keep only last 100 messages
        while self.messages.len() > 100 {
            self.messages.pop_front();
        }
    }
    
    /// Send the current input
    fn send_input(&mut self) {
        if !self.input_buffer.is_empty() {
            self.add_message("You".to_string(), self.input_buffer.clone());
            self.input_buffer.clear();
            self.cursor_position = 0;
            
            // TODO: Send to backend for processing
            // For now, add a placeholder response
            self.add_message(
                "Assistant".to_string(),
                "I'm processing your request...".to_string(),
            );
        }
    }
    
    /// Handle character input
    fn insert_char(&mut self, c: char) {
        self.input_buffer.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }
    
    /// Delete character before cursor
    fn delete_char_before(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            self.input_buffer.remove(self.cursor_position);
        }
    }
    
    /// Delete character at cursor
    fn delete_char_at(&mut self) {
        if self.cursor_position < self.input_buffer.len() {
            self.input_buffer.remove(self.cursor_position);
        }
    }
}

#[async_trait]
impl Page for ChatPage {
    fn id(&self) -> &PageId {
        &self.id
    }
    
    fn title(&self) -> &str {
        "Chat"
    }
    
    async fn handle_key_event(&mut self, event: KeyEvent) -> Result<()> {
        match event.code {
            KeyCode::Enter => {
                self.send_input();
            }
            KeyCode::Char(c) => {
                self.insert_char(c);
            }
            KeyCode::Backspace => {
                self.delete_char_before();
            }
            KeyCode::Delete => {
                self.delete_char_at();
            }
            KeyCode::Left => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_position < self.input_buffer.len() {
                    self.cursor_position += 1;
                }
            }
            KeyCode::Home => {
                self.cursor_position = 0;
            }
            KeyCode::End => {
                self.cursor_position = self.input_buffer.len();
            }
            KeyCode::Up => {
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                }
            }
            KeyCode::Down => {
                if self.scroll_offset < self.messages.len().saturating_sub(10) {
                    self.scroll_offset += 1;
                }
            }
            _ => {}
        }
        Ok(())
    }
    
    async fn handle_mouse_event(&mut self, _event: MouseEvent) -> Result<()> {
        // TODO: Handle mouse events for scrolling and selection
        Ok(())
    }
    
    async fn tick(&mut self) -> Result<()> {
        // TODO: Handle periodic updates
        Ok(())
    }
    
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Create layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),      // Messages area
                Constraint::Length(3),   // Input area
            ])
            .split(area);
        
        // Render messages
        self.render_messages(frame, chunks[0], theme);
        
        // Render input
        self.render_input(frame, chunks[1], theme);
    }
    
    fn set_focus(&mut self, focus: bool) {
        self.has_focus = focus;
    }
    
    fn has_focus(&self) -> bool {
        self.has_focus
    }
}

impl ChatPage {
    /// Render the messages area
    fn render_messages(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let styles = &theme.styles;

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Messages")
            .border_style(if self.has_focus {
                styles.dialog_border
            } else {
                styles.base
            });
        
        let inner = block.inner(area);
        frame.render_widget(block, area);
        
        // Create message items
        let items: Vec<ListItem> = self
            .messages
            .iter()
            .skip(self.scroll_offset)
            .map(|msg| {
                let style = match msg.role.as_str() {
                    "You" => styles.chat_user_message,
                    "Assistant" => styles.chat_assistant_message,
                    "System" => styles.chat_system_message,
                    _ => styles.text,
                };
                
                let content = vec![
                    Line::from(vec![
                        Span::styled(&msg.role, style.add_modifier(Modifier::BOLD)),
                        Span::styled(" ", Style::default()),
                        Span::styled(&msg.timestamp, styles.chat_timestamp),
                    ]),
                    Line::from(Span::styled(&msg.content, style)),
                    Line::from(""), // Empty line for spacing
                ];
                
                ListItem::new(content)
            })
            .collect();
        
        let list = List::new(items);
        frame.render_widget(list, inner);
    }
    
    /// Render the input area
    fn render_input(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let styles = &theme.styles;

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Input (Press Enter to send)")
            .border_style(if self.has_focus {
                styles.text_input_focused
            } else {
                styles.text_input_blurred
            });
        
        let inner = block.inner(area);
        frame.render_widget(block, area);
        
        // Render input text with cursor
        let mut display_text = self.input_buffer.clone();
        if self.has_focus {
            display_text.insert(self.cursor_position, '│');
        }
        
        let paragraph = Paragraph::new(display_text)
            .style(styles.text)
            .wrap(Wrap { trim: false });
        
        frame.render_widget(paragraph, inner);
    }
}

impl Default for ChatPage {
    fn default() -> Self {
        Self::new()
    }
}