//! Enhanced chat interface for Goofy
//!
//! This module provides a comprehensive chat interface with rich message rendering,
//! multiline editing, real-time streaming, and session management.

pub mod message_types;
pub mod message_renderer;
pub mod editor;
pub mod streaming;
pub mod header;
pub mod sidebar;
pub mod formatting;


use super::{Component, ComponentState};
use crate::{
    llm::types::{ProviderEvent, MessageRole},
    session::{Session, SessionManager},
    tui::{
        themes::{Theme, ThemeManager},
        Frame,
    },
};
use anyhow::Result;
use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Instant,
};
use tokio::sync::mpsc;

// Re-export the new types for backward compatibility
pub use message_types::{
    ChatMessage, MessageAttachment, MessageDisplayOptions, StreamingState, FinishReason,
    ToolResult, ToolArtifact,
};
pub use message_renderer::{MessageRenderer, RenderedMessage};
pub use editor::{ChatEditor, EditorMode, CompletionItem, CompletionKind, CursorDirection};
pub use streaming::{
    StreamingManager, StreamingUpdate, StreamingSubscription, StreamingStats, TypingIndicator,
};
pub use header::{ChatHeader, HeaderConfig};
pub use sidebar::{ChatSidebar, SidebarMode, SidebarConfig, SidebarAction};
pub use formatting::{MessageFormatter, FormatOptions, FormattedText};

/// Enhanced chat interface component
pub struct EnhancedChatInterface {
    state: ComponentState,
    
    // Core components
    message_renderer: MessageRenderer,
    editor: ChatEditor,
    header: ChatHeader,
    sidebar: ChatSidebar,
    
    // Message management
    messages: VecDeque<ChatMessage>,
    max_messages: usize,
    
    // Streaming support
    streaming_manager: Arc<Mutex<StreamingManager>>,
    streaming_subscription: Option<StreamingSubscription>,
    
    // Session management
    current_session: Option<Session>,
    // TODO: Re-enable when SessionManager is Send+Sync
    // session_manager: Option<Arc<Mutex<SessionManager>>>,
    
    // Layout configuration
    layout_config: ChatLayoutConfig,
    
    // Event handling
    event_sender: Option<mpsc::UnboundedSender<ChatEvent>>,
    event_receiver: Option<mpsc::UnboundedReceiver<ChatEvent>>,
    
    // Performance optimization
    last_render: Instant,
    render_cache: RenderCache,
    
    // Configuration
    display_options: MessageDisplayOptions,
    
    // Focus management
    focused_component: FocusedComponent,
}

/// Chat layout configuration
#[derive(Debug, Clone)]
pub struct ChatLayoutConfig {
    pub show_sidebar: bool,
    pub show_header: bool,
    pub sidebar_width: u16,
    pub header_height: u16,
    pub min_editor_height: u16,
    pub max_editor_height: u16,
    pub compact_mode: bool,
}

impl Default for ChatLayoutConfig {
    fn default() -> Self {
        Self {
            show_sidebar: true,
            show_header: true,
            sidebar_width: 30,
            header_height: 3,
            min_editor_height: 3,
            max_editor_height: 10,
            compact_mode: false,
        }
    }
}

/// Currently focused component
#[derive(Debug, Clone, PartialEq)]
pub enum FocusedComponent {
    Editor,
    Messages,
    Sidebar,
    Header,
}

/// Chat events
#[derive(Debug, Clone)]
pub enum ChatEvent {
    // Message events
    MessageSent { content: String, attachments: Vec<MessageAttachment> },
    MessageReceived(ChatMessage),
    MessageUpdated { id: String, content: String },
    MessageDeleted(String),
    
    // Streaming events
    StreamingStarted { message_id: String },
    StreamingUpdate { message_id: String, delta: String },
    StreamingCompleted { message_id: String },
    StreamingFailed { message_id: String, error: String },
    
    // Session events
    SessionChanged(Session),
    SessionCreated(Session),
    SessionDeleted(String),
    
    // UI events
    FocusChanged(FocusedComponent),
    LayoutChanged(ChatLayoutConfig),
    ThemeChanged(String),
    
    // Tool events
    ToolCallStarted { message_id: String, tool_name: String },
    ToolCallCompleted { message_id: String, result: String },
    ToolCallFailed { message_id: String, error: String },
}

/// Render cache for performance optimization
#[derive(Debug, Default)]
struct RenderCache {
    cached_messages: Vec<(String, RenderedMessage)>, // (message_id, rendered)
    cache_valid: bool,
    last_message_count: usize,
}

impl EnhancedChatInterface {
    /// Create a new enhanced chat interface
    pub fn new() -> Self {
        let streaming_manager = Arc::new(Mutex::new(StreamingManager::new()));
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        
        Self {
            state: ComponentState::new(),
            message_renderer: MessageRenderer::new(),
            editor: ChatEditor::new(),
            header: ChatHeader::new(),
            sidebar: ChatSidebar::new(),
            messages: VecDeque::new(),
            max_messages: 1000,
            streaming_manager,
            streaming_subscription: None,
            current_session: None,
            // session_manager: None,
            layout_config: ChatLayoutConfig::default(),
            event_sender: Some(event_sender),
            event_receiver: Some(event_receiver),
            last_render: Instant::now(),
            render_cache: RenderCache::default(),
            display_options: MessageDisplayOptions::default(),
            focused_component: FocusedComponent::Editor,
        }
    }

    /// Create chat interface with configuration
    pub fn with_config(layout_config: ChatLayoutConfig, display_options: MessageDisplayOptions) -> Self {
        let mut interface = Self::new();
        interface.layout_config = layout_config;
        interface.display_options = display_options;
        interface
    }

    /// Set session manager
    // TODO: Re-enable when SessionManager is Send+Sync
    // pub fn set_session_manager(&mut self, session_manager: Arc<Mutex<SessionManager>>) {
    //     self.session_manager = Some(session_manager);
    // }

    /// Set current session
    pub async fn set_session(&mut self, session: Session) -> Result<()> {
        self.current_session = Some(session.clone());
        self.header.set_session(Some(session.clone()));
        
        // Load session messages
        self.load_session_messages(&session).await?;
        
        // Update sidebar
        if let Some(sessions) = self.get_all_sessions().await? {
            self.sidebar.set_sessions(sessions);
        }
        self.sidebar.select_session(Some(session.id.clone()));
        
        // Emit event
        if let Some(ref sender) = self.event_sender {
            let _ = sender.send(ChatEvent::SessionChanged(session));
        }
        
        Ok(())
    }

    /// Add a message to the interface
    pub async fn add_message(&mut self, message: ChatMessage) -> Result<()> {
        self.messages.push_back(message.clone());
        
        // Maintain maximum message limit
        while self.messages.len() > self.max_messages {
            self.messages.pop_front();
        }
        
        // Invalidate render cache
        self.render_cache.cache_valid = false;
        
        // Emit event
        if let Some(ref sender) = self.event_sender {
            let _ = sender.send(ChatEvent::MessageReceived(message));
        }
        
        Ok(())
    }

    /// Send a message
    pub async fn send_message(&mut self, content: String, attachments: Vec<MessageAttachment>) -> Result<()> {
        if content.trim().is_empty() && attachments.is_empty() {
            return Ok(());
        }

        // Create user message
        let mut message = ChatMessage::new_user_text(content.clone());
        for attachment in &attachments {
            message.add_attachment(attachment.clone());
        }

        // Add to interface
        self.add_message(message).await?;

        // Clear editor
        self.editor.clear();

        // Add to history
        self.editor.add_to_history(content.clone());

        // Emit event
        if let Some(ref sender) = self.event_sender {
            let _ = sender.send(ChatEvent::MessageSent { content, attachments });
        }

        Ok(())
    }

    /// Start streaming for a message
    pub async fn start_streaming(&mut self, message_id: String, role: crate::llm::types::MessageRole) -> Result<()> {
        if let Ok(manager) = self.streaming_manager.lock() {
            manager.start_stream(message_id.clone(), role).await?;
        }
        
        if let Some(ref sender) = self.event_sender {
            let _ = sender.send(ChatEvent::StreamingStarted { message_id });
        }
        
        Ok(())
    }

    /// Process streaming event
    pub async fn process_streaming_event(&mut self, message_id: String, event: ProviderEvent) -> Result<()> {
        if let Ok(manager) = self.streaming_manager.lock() {
            manager.process_event(message_id.clone(), event).await?;
        }
        Ok(())
    }

    /// Set focused component
    pub fn set_focus(&mut self, component: FocusedComponent) {
        // Remove focus from current component
        match self.focused_component {
            FocusedComponent::Editor => self.editor.set_focus(false),
            FocusedComponent::Messages => {}, // Messages don't have focus state
            FocusedComponent::Sidebar => self.sidebar.set_focus(false),
            FocusedComponent::Header => self.header.set_focus(false),
        }

        // Set focus to new component
        match component {
            FocusedComponent::Editor => self.editor.set_focus(true),
            FocusedComponent::Messages => {},
            FocusedComponent::Sidebar => self.sidebar.set_focus(true),
            FocusedComponent::Header => self.header.set_focus(true),
        }

        self.focused_component = component.clone();

        // Emit event
        if let Some(ref sender) = self.event_sender {
            let _ = sender.send(ChatEvent::FocusChanged(component));
        }
    }

    /// Toggle sidebar visibility
    pub fn toggle_sidebar(&mut self) {
        self.layout_config.show_sidebar = !self.layout_config.show_sidebar;
        self.sidebar.set_visible(self.layout_config.show_sidebar);
    }

    /// Toggle header visibility
    pub fn toggle_header(&mut self) {
        self.layout_config.show_header = !self.layout_config.show_header;
        self.header.set_visible(self.layout_config.show_header);
    }

    /// Set display options
    pub fn set_display_options(&mut self, options: MessageDisplayOptions) {
        self.display_options = options.clone();
        self.message_renderer.set_display_options(options);
        self.render_cache.cache_valid = false;
    }

    /// Get messages in current session
    pub fn get_messages(&self) -> &VecDeque<ChatMessage> {
        &self.messages
    }

    /// Get current session
    pub fn get_current_session(&self) -> Option<&Session> {
        self.current_session.as_ref()
    }

    /// Load session messages
    async fn load_session_messages(&mut self, _session: &Session) -> Result<()> {
        // In a real implementation, this would load messages from the session manager
        // For now, we'll just clear the current messages
        self.messages.clear();
        self.render_cache.cache_valid = false;
        Ok(())
    }

    /// Get all sessions
    async fn get_all_sessions(&self) -> Result<Option<Vec<Session>>> {
        // In a real implementation, this would fetch from the session manager
        Ok(None)
    }

    /// Process pending events
    async fn process_events(&mut self) -> Result<()> {
        // Collect events first to avoid double mutable borrow
        let events: Vec<ChatEvent> = if let Some(ref mut receiver) = self.event_receiver {
            let mut collected = Vec::new();
            while let Ok(event) = receiver.try_recv() {
                collected.push(event);
            }
            collected
        } else {
            Vec::new()
        };

        for event in events {
            self.handle_event(event).await?;
        }
        Ok(())
    }

    /// Handle a chat event
    async fn handle_event(&mut self, event: ChatEvent) -> Result<()> {
        match event {
            ChatEvent::MessageSent { content, attachments } => {
                // Handle message sending logic
                self.send_message(content, attachments).await?;
            }
            ChatEvent::MessageReceived(message) => {
                self.add_message(message).await?;
            }
            ChatEvent::SessionChanged(session) => {
                self.set_session(session).await?;
            }
            ChatEvent::FocusChanged(component) => {
                self.set_focus(component);
            }
            ChatEvent::ThemeChanged(_theme_name) => {
                // Theme changes are handled through the theme manager in each component
                // No direct action needed here as components get theme via render() calls
            }
            _ => {
                // Handle other events as needed
            }
        }
        Ok(())
    }

    /// Calculate layout constraints
    fn calculate_layout(&self, area: Rect) -> Vec<Constraint> {
        let mut constraints = Vec::new();
        
        // Header
        if self.layout_config.show_header {
            constraints.push(Constraint::Length(self.layout_config.header_height));
        }
        
        // Main content area
        constraints.push(Constraint::Min(1));
        
        // Editor area
        let editor_height = if self.layout_config.compact_mode {
            self.layout_config.min_editor_height
        } else {
            self.layout_config.max_editor_height.min(area.height / 4)
        };
        constraints.push(Constraint::Length(editor_height));
        
        constraints
    }

    /// Calculate main layout (with sidebar)
    fn calculate_main_layout(&self, area: Rect) -> (Option<Rect>, Rect) {
        if self.layout_config.show_sidebar {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(self.layout_config.sidebar_width),
                    Constraint::Min(1),
                ])
                .split(area);
            (Some(chunks[0]), chunks[1])
        } else {
            (None, area)
        }
    }

    /// Render messages area
    fn render_messages(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Create a scrollable message area
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Messages")
            .border_style(theme.styles.dialog_border);

        let inner_area = block.inner(area);
        frame.render_widget(block, area);

        // Render messages using the message renderer
        let mut current_y = inner_area.y;
        let available_height = inner_area.height;
        
        for message in self.messages.iter().rev() {
            if current_y >= inner_area.y + available_height {
                break;
            }
            
            let message_area = Rect {
                x: inner_area.x,
                y: current_y,
                width: inner_area.width,
                height: available_height - (current_y - inner_area.y),
            };
            
            let rendered = self.message_renderer.render_message(message, frame, message_area);
            current_y += rendered.total_height;
        }
    }
}

#[async_trait]
impl Component for EnhancedChatInterface {
    async fn handle_key_event(&mut self, event: KeyEvent) -> Result<()> {
        // Process pending events first
        self.process_events().await?;

        // Handle global shortcuts
        match (event.code, event.modifiers) {
            // Tab between components
            (KeyCode::Tab, KeyModifiers::NONE) => {
                let next_component = match self.focused_component {
                    FocusedComponent::Editor => FocusedComponent::Messages,
                    FocusedComponent::Messages => if self.layout_config.show_sidebar {
                        FocusedComponent::Sidebar
                    } else {
                        FocusedComponent::Editor
                    },
                    FocusedComponent::Sidebar => if self.layout_config.show_header {
                        FocusedComponent::Header
                    } else {
                        FocusedComponent::Editor
                    },
                    FocusedComponent::Header => FocusedComponent::Editor,
                };
                self.set_focus(next_component);
                return Ok(());
            }
            
            // Toggle sidebar
            (KeyCode::F(9), KeyModifiers::NONE) => {
                self.toggle_sidebar();
                return Ok(());
            }
            
            // Toggle header details
            (KeyCode::F(1), KeyModifiers::NONE) => {
                self.header.toggle_details();
                return Ok(());
            }
            
            // Send message (Ctrl+Enter from any component)
            (KeyCode::Enter, KeyModifiers::CONTROL) => {
                if !self.editor.get_content().trim().is_empty() {
                    let content = self.editor.get_content().to_string();
                    let attachments = self.editor.get_attachments().to_vec();
                    self.send_message(content, attachments).await?;
                }
                return Ok(());
            }
            
            _ => {}
        }

        // Delegate to focused component
        match self.focused_component {
            FocusedComponent::Editor => {
                self.editor.handle_key_event(event).await?;
            }
            FocusedComponent::Sidebar => {
                self.sidebar.handle_key_event(event).await?;
            }
            FocusedComponent::Header => {
                self.header.handle_key_event(event).await?;
            }
            FocusedComponent::Messages => {
                // Handle message area navigation
                // TODO: Implement message selection and scrolling
            }
        }

        Ok(())
    }

    async fn handle_mouse_event(&mut self, event: MouseEvent) -> Result<()> {
        // TODO: Implement mouse event handling for all components
        self.editor.handle_mouse_event(event).await?;
        self.sidebar.handle_mouse_event(event).await?;
        self.header.handle_mouse_event(event).await?;
        Ok(())
    }

    async fn tick(&mut self) -> Result<()> {
        // Process events
        self.process_events().await?;
        
        // Tick all components
        self.editor.tick().await?;
        self.sidebar.tick().await?;
        self.header.tick().await?;
        
        Ok(())
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Calculate main layout (sidebar + content)
        let (sidebar_area, content_area) = self.calculate_main_layout(area);
        
        // Render sidebar if visible
        if let Some(sidebar_area) = sidebar_area {
            self.sidebar.render(frame, sidebar_area, theme);
        }
        
        // Calculate content layout (header + messages + editor)
        let constraints = self.calculate_layout(content_area);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(content_area);
        
        let mut chunk_index = 0;
        
        // Render header if visible
        if self.layout_config.show_header {
            self.header.render(frame, chunks[chunk_index], theme);
            chunk_index += 1;
        }
        
        // Render messages area
        self.render_messages(frame, chunks[chunk_index], theme);
        chunk_index += 1;
        
        // Render editor
        self.editor.render(frame, chunks[chunk_index], theme);
        
        // Update render timestamp
        self.last_render = Instant::now();
    }

    fn size(&self) -> Rect {
        self.state.size
    }

    fn set_size(&mut self, size: Rect) {
        self.state.size = size;
        
        // Update component sizes based on layout
        let (sidebar_area, content_area) = self.calculate_main_layout(size);
        
        if let Some(sidebar_area) = sidebar_area {
            self.sidebar.set_size(sidebar_area);
        }
        
        let constraints = self.calculate_layout(content_area);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(content_area);
        
        let mut chunk_index = 0;
        
        if self.layout_config.show_header {
            self.header.set_size(chunks[chunk_index]);
            chunk_index += 1;
        }
        
        // Messages area size is handled in render
        chunk_index += 1;
        
        self.editor.set_size(chunks[chunk_index]);
    }

    fn has_focus(&self) -> bool {
        self.state.has_focus
    }

    fn set_focus(&mut self, focus: bool) {
        self.state.has_focus = focus;
        if focus {
            // Focus the currently selected component
            match self.focused_component {
                FocusedComponent::Editor => self.editor.set_focus(true),
                FocusedComponent::Sidebar => self.sidebar.set_focus(true),
                FocusedComponent::Header => self.header.set_focus(true),
                _ => {}
            }
        } else {
            // Remove focus from all components
            self.editor.set_focus(false);
            self.sidebar.set_focus(false);
            self.header.set_focus(false);
        }
    }

    fn is_visible(&self) -> bool {
        self.state.is_visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.state.is_visible = visible;
        self.editor.set_visible(visible);
        self.sidebar.set_visible(visible && self.layout_config.show_sidebar);
        self.header.set_visible(visible && self.layout_config.show_header);
    }
}

impl Default for EnhancedChatInterface {
    fn default() -> Self {
        Self::new()
    }
}

// Legacy compatibility types and implementations
#[deprecated(note = "Use message_types::ChatMessage instead")]
pub type LegacyChatMessage = message_types::ChatMessage;

#[deprecated(note = "Use EnhancedChatInterface instead")]
pub type ChatMessageList = LegacyChatMessageList;

#[deprecated(note = "Use ChatEditor instead")]
pub type ChatInput = LegacyChatInput;

/// Legacy message list for backward compatibility
pub struct LegacyChatMessageList {
    interface: EnhancedChatInterface,
}

impl LegacyChatMessageList {
    pub fn new() -> Self {
        Self {
            interface: EnhancedChatInterface::new(),
        }
    }
}

/// Legacy input for backward compatibility  
pub struct LegacyChatInput {
    editor: ChatEditor,
}

impl LegacyChatInput {
    pub fn new() -> Self {
        Self {
            editor: ChatEditor::new(),
        }
    }
    
    pub fn get_content(&self) -> &str {
        self.editor.get_content()
    }
    
    pub fn clear(&mut self) {
        self.editor.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_chat_interface_creation() {
        let interface = EnhancedChatInterface::new();
        assert_eq!(interface.focused_component, FocusedComponent::Editor);
        assert!(interface.messages.is_empty());
        assert!(interface.layout_config.show_sidebar);
        assert!(interface.layout_config.show_header);
    }

    #[tokio::test]
    async fn test_message_handling() {
        let mut interface = EnhancedChatInterface::new();
        let message = ChatMessage::new_user_text("Test message".to_string());
        
        assert!(interface.add_message(message).await.is_ok());
        assert_eq!(interface.messages.len(), 1);
    }

    #[test]
    fn test_focus_management() {
        let mut interface = EnhancedChatInterface::new();
        
        interface.set_focus(FocusedComponent::Sidebar);
        assert_eq!(interface.focused_component, FocusedComponent::Sidebar);
        assert!(interface.sidebar.has_focus());
        assert!(!interface.editor.has_focus());
    }

    #[test]
    fn test_layout_configuration() {
        let layout_config = ChatLayoutConfig {
            show_sidebar: false,
            show_header: true,
            sidebar_width: 25,
            header_height: 2,
            ..Default::default()
        };
        
        let interface = EnhancedChatInterface::with_config(layout_config.clone(), MessageDisplayOptions::default());
        assert_eq!(interface.layout_config.sidebar_width, 25);
        assert_eq!(interface.layout_config.header_height, 2);
        assert!(!interface.layout_config.show_sidebar);
    }
}