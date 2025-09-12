// Core components
pub mod chat;
pub mod core;
pub mod dialogs;
pub mod list;
pub mod input;
pub mod logo;
pub mod splash;
pub mod status;

// Advanced components
pub mod animations;
pub mod completions;
pub mod files;
pub mod lists;
pub mod highlighting;
pub mod image;
pub mod markdown;

use crate::tui::{events::Event, themes::Theme, Frame};
use anyhow::Result;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use async_trait::async_trait;

/// Base trait for all UI components
#[async_trait]
pub trait Component: Send + Sync {
    /// Handle keyboard input
    async fn handle_key_event(&mut self, event: KeyEvent) -> Result<()> {
        let _ = event;
        Ok(())
    }
    
    /// Handle mouse input
    async fn handle_mouse_event(&mut self, event: MouseEvent) -> Result<()> {
        let _ = event;
        Ok(())
    }
    
    /// Handle periodic updates
    async fn tick(&mut self) -> Result<()> {
        Ok(())
    }
    
    /// Render the component
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme);
    
    /// Get component dimensions
    fn size(&self) -> Rect;
    
    /// Set component dimensions
    fn set_size(&mut self, size: Rect);
    
    /// Check if component has focus
    fn has_focus(&self) -> bool {
        false
    }
    
    /// Set component focus
    fn set_focus(&mut self, focus: bool) {
        let _ = focus;
    }
    
    /// Check if component is visible
    fn is_visible(&self) -> bool {
        true
    }
    
    /// Set component visibility
    fn set_visible(&mut self, visible: bool) {
        let _ = visible;
    }
}

/// Trait for components that can be resized
pub trait Resizable {
    fn resize(&mut self, area: Rect);
    fn min_size(&self) -> (u16, u16);
    fn preferred_size(&self) -> (u16, u16);
}

/// Trait for components that can handle text input
#[async_trait]
pub trait TextInput: Component {
    async fn insert_char(&mut self, c: char) -> Result<()>;
    async fn delete_char(&mut self) -> Result<()>;
    async fn delete_previous_char(&mut self) -> Result<()>;
    fn get_text(&self) -> &str;
    fn set_text(&mut self, text: String);
    fn cursor_position(&self) -> usize;
    fn set_cursor_position(&mut self, pos: usize);
}

/// Trait for components that can display a list of items
#[async_trait]
pub trait ListView<T>: Component {
    async fn add_item(&mut self, item: T) -> Result<()>;
    async fn remove_item(&mut self, index: usize) -> Result<()>;
    async fn clear_items(&mut self) -> Result<()>;
    fn get_items(&self) -> &[T];
    fn selected_index(&self) -> Option<usize>;
    fn set_selected_index(&mut self, index: Option<usize>);
    async fn move_selection_up(&mut self) -> Result<()>;
    async fn move_selection_down(&mut self) -> Result<()>;
}

/// Trait for components that can be scrolled
pub trait Scrollable {
    fn scroll_up(&mut self, lines: usize);
    fn scroll_down(&mut self, lines: usize);
    fn scroll_to_top(&mut self);
    fn scroll_to_bottom(&mut self);
    fn scroll_position(&self) -> usize;
    fn can_scroll_up(&self) -> bool;
    fn can_scroll_down(&self) -> bool;
}

/// Base component state
#[derive(Debug, Clone)]
pub struct ComponentState {
    pub size: Rect,
    pub has_focus: bool,
    pub is_visible: bool,
    pub is_enabled: bool,
}

impl Default for ComponentState {
    fn default() -> Self {
        Self {
            size: Rect::default(),
            has_focus: false,
            is_visible: true,
            is_enabled: true,
        }
    }
}

impl ComponentState {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_size(mut self, size: Rect) -> Self {
        self.size = size;
        self
    }
    
    pub fn with_focus(mut self, focus: bool) -> Self {
        self.has_focus = focus;
        self
    }
    
    pub fn with_visibility(mut self, visible: bool) -> Self {
        self.is_visible = visible;
        self
    }
    
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.is_enabled = enabled;
        self
    }
}

/// Component event types
#[derive(Debug, Clone)]
pub enum ComponentEvent {
    /// Component gained focus
    FocusGained,
    
    /// Component lost focus
    FocusLost,
    
    /// Component was resized
    Resized(Rect),
    
    /// Component was shown
    Shown,
    
    /// Component was hidden
    Hidden,
    
    /// Component was enabled
    Enabled,
    
    /// Component was disabled
    Disabled,
    
    /// Custom component event
    Custom(String, serde_json::Value),
}

/// Component manager for handling multiple components
pub struct ComponentManager {
    components: Vec<Box<dyn Component>>,
    focused_index: Option<usize>,
}

impl ComponentManager {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            focused_index: None,
        }
    }
    
    pub fn add_component(&mut self, component: Box<dyn Component>) {
        self.components.push(component);
    }
    
    pub fn remove_component(&mut self, index: usize) {
        if index < self.components.len() {
            self.components.remove(index);
            
            // Adjust focused index if necessary
            if let Some(focused) = self.focused_index {
                if focused == index {
                    self.focused_index = None;
                } else if focused > index {
                    self.focused_index = Some(focused - 1);
                }
            }
        }
    }
    
    pub fn set_focus(&mut self, index: Option<usize>) {
        // Remove focus from current component
        if let Some(current) = self.focused_index {
            if let Some(component) = self.components.get_mut(current) {
                component.set_focus(false);
            }
        }
        
        // Set focus to new component
        self.focused_index = index;
        if let Some(new_index) = index {
            if let Some(component) = self.components.get_mut(new_index) {
                component.set_focus(true);
            }
        }
    }
    
    pub fn focus_next(&mut self) {
        let next_index = match self.focused_index {
            Some(current) => {
                if current + 1 < self.components.len() {
                    Some(current + 1)
                } else {
                    Some(0)
                }
            }
            None => {
                if !self.components.is_empty() {
                    Some(0)
                } else {
                    None
                }
            }
        };
        
        self.set_focus(next_index);
    }
    
    pub fn focus_previous(&mut self) {
        let prev_index = match self.focused_index {
            Some(current) => {
                if current > 0 {
                    Some(current - 1)
                } else if !self.components.is_empty() {
                    Some(self.components.len() - 1)
                } else {
                    None
                }
            }
            None => {
                if !self.components.is_empty() {
                    Some(self.components.len() - 1)
                } else {
                    None
                }
            }
        };
        
        self.set_focus(prev_index);
    }
    
    pub async fn handle_key_event(&mut self, event: KeyEvent) -> Result<()> {
        if let Some(focused) = self.focused_index {
            if let Some(component) = self.components.get_mut(focused) {
                component.handle_key_event(event).await?;
            }
        }
        Ok(())
    }
    
    pub async fn handle_mouse_event(&mut self, event: MouseEvent) -> Result<()> {
        // Handle mouse events for all visible components
        for component in &mut self.components {
            if component.is_visible() {
                component.handle_mouse_event(event).await?;
            }
        }
        Ok(())
    }
    
    pub async fn tick(&mut self) -> Result<()> {
        for component in &mut self.components {
            component.tick().await?;
        }
        Ok(())
    }
    
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        for component in &mut self.components {
            if component.is_visible() {
                component.render(frame, area, theme);
            }
        }
    }
    
    pub fn resize(&mut self, area: Rect) {
        for component in &mut self.components {
            component.set_size(area);
        }
    }
}

impl Default for ComponentManager {
    fn default() -> Self {
        Self::new()
    }
}