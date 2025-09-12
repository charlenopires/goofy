use crate::tui::{events::Event, keys::KeyMap, pages::{Page, PageId, PageManager, /* chat::ChatPage, home::HomePage, settings::SettingsPage */}, themes::{Theme, presets}, Frame};
use anyhow::Result;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::style::{Color, Style};
use std::collections::HashMap;
use tokio::sync::mpsc;

/// Main application state and controller
pub struct App {
    /// Whether the application should quit
    pub should_quit: bool,
    
    /// Current application dimensions
    pub size: Rect,
    
    /// Key mappings for the application
    pub key_map: KeyMap,
    
    /// Page manager for handling different screens
    pub page_manager: PageManager,
    
    /// Current theme for styling
    pub theme: Theme,
    
    /// Status message to display
    pub status_message: Option<String>,
    
    /// Application configuration
    pub config: AppConfig,
    
    /// Event sender for internal communication
    pub event_sender: mpsc::UnboundedSender<Event>,
    
    /// Event receiver for internal communication
    pub event_receiver: mpsc::UnboundedReceiver<Event>,
}

/// Application configuration
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Show help text
    pub show_help: bool,
    
    /// Enable mouse support
    pub mouse_enabled: bool,
    
    /// Maximum number of messages to keep in memory
    pub max_messages: usize,
    
    /// Auto-save interval in seconds
    pub auto_save_interval: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            show_help: false,
            mouse_enabled: true,
            max_messages: 1000,
            auto_save_interval: 30,
        }
    }
}

impl App {
    /// Create a new application instance
    pub async fn new() -> Result<Self> {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        
        let mut page_manager = PageManager::new();
        
        // Register default pages
        // TODO: Re-enable when pages are fixed
        // page_manager.register_page(Box::new(HomePage::new()));
        // page_manager.register_page(Box::new(ChatPage::new()));
        // page_manager.register_page(Box::new(SettingsPage::new()));
        
        // Navigate to home page by default
        // TODO: Fix when pages are available
        // page_manager.navigate_to("home".to_string())?;
        
        Ok(Self {
            should_quit: false,
            size: Rect::default(),
            key_map: KeyMap::default(),
            page_manager,
            theme: presets::goofy_dark(),
            status_message: None,
            config: AppConfig::default(),
            event_sender,
            event_receiver,
        })
    }
    
    /// Create a new TUI app connected to an existing backend App
    pub async fn new_with_backend(backend: &crate::app::App) -> Result<Self> {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        
        let mut page_manager = PageManager::new();
        
        // Register chat page
        use crate::tui::pages::chat::ChatPage;
        page_manager.register_page(Box::new(ChatPage::new()));
        page_manager.navigate_to("chat".to_string())?;
        
        let status = format!(
            "Connected to {} ({})",
            backend.llm_provider().name(),
            backend.llm_provider().model()
        );
        
        Ok(Self {
            should_quit: false,
            size: Rect::default(),
            key_map: KeyMap::default(),
            page_manager,
            theme: presets::goofy_dark(),
            status_message: Some(status),
            config: AppConfig::default(),
            event_sender,
            event_receiver,
        })
    }
    
    /// Handle incoming events
    pub async fn handle_event(&mut self, event: Event) -> Result<bool> {
        match event {
            Event::Key(key_event) => {
                if self.key_map.should_quit(&key_event) {
                    self.should_quit = true;
                    return Ok(true);
                }
                
                if self.key_map.should_show_help(&key_event) {
                    self.config.show_help = !self.config.show_help;
                    return Ok(false);
                }
                
                // Forward key events to current page
                if let Some(current_page) = self.page_manager.current_page_mut() {
                    current_page.handle_key_event(key_event).await?;
                }
            }
            
            Event::Mouse(mouse_event) => {
                if self.config.mouse_enabled {
                    if let Some(current_page) = self.page_manager.current_page_mut() {
                        current_page.handle_mouse_event(mouse_event).await?;
                    }
                }
            }
            
            Event::Resize(width, height) => {
                self.size = Rect::new(0, 0, width, height);
                self.page_manager.resize(self.size);
            }
            
            Event::Tick => {
                // Handle periodic updates
                if let Some(current_page) = self.page_manager.current_page_mut() {
                    current_page.tick().await?;
                }
            },
            
            Event::Custom(_, _) => {
                // Handle custom events
            },
            
            Event::PageChange(page_id) => {
                self.page_manager.navigate_to(page_id)?;
            }
            
            Event::StatusMessage(message) => {
                self.status_message = Some(message);
            }
            
            Event::ClearStatus => {
                self.status_message = None;
            }
        }
        
        // Process any internal events
        while let Ok(_internal_event) = self.event_receiver.try_recv() {
            // TODO: Handle internal events without recursion
        }
        
        Ok(self.should_quit)
    }
    
    /// Render the application UI
    /// Helper method to get theme styles
    fn theme_styles(&mut self) -> crate::tui::themes::Styles {
        self.theme.styles().clone()
    }
    
    pub fn render(&mut self, frame: &mut Frame) {
        self.size = frame.size();
        
        // Create main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),      // Main content
                Constraint::Length(1),   // Status bar
            ])
            .split(frame.size());
        
        // Render current page
        if let Some(current_page) = self.page_manager.current_page_mut() {
            current_page.render(frame, chunks[0], &self.theme);
        } else {
            // Render empty state
            let empty_block = Block::default()
                .borders(Borders::ALL)
                .title("Crush Terminal")
                .style(self.theme_styles().base);
            
            let empty_text = Paragraph::new("No active page")
                .block(empty_block)
                .style(self.theme_styles().text);
                
            frame.render_widget(empty_text, chunks[0]);
        }
        
        // Render status bar
        self.render_status_bar(frame, chunks[1]);
        
        // Render help overlay if enabled
        if self.config.show_help {
            self.render_help_overlay(frame);
        }
    }
    
    /// Render the status bar
    fn render_status_bar(&mut self, frame: &mut Frame, area: Rect) {
        let status_text = if let Some(ref message) = self.status_message {
            message.clone()
        } else {
            format!(
                "Page: {} | Press Ctrl+G for help | Ctrl+C to quit",
                self.page_manager.current_page_id().map_or("None", |v| v)
            )
        };
        
        let status_paragraph = Paragraph::new(status_text)
            .style(self.theme_styles().base);
            
        frame.render_widget(status_paragraph, area);
    }
    
    /// Render help overlay
    fn render_help_overlay(&mut self, frame: &mut Frame) {
        let help_area = centered_rect(60, 50, frame.size());
        
        let help_text = self.key_map.help_text();
        let help_block = Block::default()
            .borders(Borders::ALL)
            .title("Help")
            .style(self.theme_styles().base);
            
        let help_paragraph = Paragraph::new(help_text)
            .block(help_block)
            .style(self.theme_styles().text);
            
        frame.render_widget(help_paragraph, help_area);
    }
    
    /// Get a sender for internal events
    pub fn event_sender(&self) -> mpsc::UnboundedSender<Event> {
        self.event_sender.clone()
    }
}

/// Create a centered rectangle with given percentage of the screen
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}