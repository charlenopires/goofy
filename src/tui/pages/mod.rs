// Page modules
pub mod chat;
// TODO: Re-enable when components are fixed
// pub mod home;
// pub mod settings;

use crate::tui::{components::Component, themes::Theme, Frame};
use anyhow::Result;
use async_trait::async_trait;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use std::collections::HashMap;

/// Page identifier type
pub type PageId = String;

/// Base trait for all pages
#[async_trait]
pub trait Page: Send + Sync {
    /// Get the page ID
    fn id(&self) -> &PageId;
    
    /// Get the page title
    fn title(&self) -> &str;
    
    /// Handle keyboard input
    async fn handle_key_event(&mut self, event: KeyEvent) -> Result<()>;
    
    /// Handle mouse input
    async fn handle_mouse_event(&mut self, event: MouseEvent) -> Result<()>;
    
    /// Handle periodic updates
    async fn tick(&mut self) -> Result<()>;
    
    /// Render the page
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme);
    
    /// Called when the page becomes active
    async fn on_enter(&mut self) -> Result<()> {
        Ok(())
    }
    
    /// Called when the page becomes inactive
    async fn on_exit(&mut self) -> Result<()> {
        Ok(())
    }
    
    /// Called when the page is resized
    fn on_resize(&mut self, _area: Rect) {
        // Default implementation
    }
    
    /// Check if the page can be closed
    fn can_close(&self) -> bool {
        true
    }
    
    /// Get page-specific help text
    fn help_text(&self) -> Vec<(&str, &str)> {
        vec![]
    }
    
    /// Set focus state for the page
    fn set_focus(&mut self, focus: bool);
    
    /// Check if the page has focus
    fn has_focus(&self) -> bool;
}

/// Page manager for handling navigation between pages
pub struct PageManager {
    /// All registered pages
    pages: HashMap<PageId, Box<dyn Page>>,
    
    /// Current active page
    current_page: Option<PageId>,
    
    /// Page history for navigation
    history: Vec<PageId>,
    
    /// Maximum history size
    max_history: usize,
}

impl PageManager {
    /// Create a new page manager
    pub fn new() -> Self {
        Self {
            pages: HashMap::new(),
            current_page: None,
            history: Vec::new(),
            max_history: 10,
        }
    }
    
    /// Register a page
    pub fn register_page(&mut self, page: Box<dyn Page>) {
        let id = page.id().clone();
        self.pages.insert(id, page);
    }
    
    /// Navigate to a page
    pub fn navigate_to(&mut self, page_id: PageId) -> Result<()> {
        if !self.pages.contains_key(&page_id) {
            return Err(anyhow::anyhow!("Page '{}' not found", page_id));
        }
        
        // Add current page to history
        if let Some(current_id) = &self.current_page {
            self.add_to_history(current_id.clone());
        }
        
        // Set new current page
        self.current_page = Some(page_id.clone());
        
        Ok(())
    }
    
    /// Get the current page
    pub fn current_page(&self) -> Option<&dyn Page> {
        if let Some(current_id) = &self.current_page {
            self.pages.get(current_id).map(|p| p.as_ref())
        } else {
            None
        }
    }
    
    /// Get the current page mutably
    pub fn current_page_mut(&mut self) -> Option<&mut dyn Page> {
        if let Some(ref current_id) = self.current_page {
            if let Some(page) = self.pages.get_mut(current_id) {
                Some(page.as_mut())
            } else {
                None
            }
        } else {
            None
        }
    }
    
    /// Get the current page ID
    pub fn current_page_id(&self) -> Option<&PageId> {
        self.current_page.as_ref()
    }
    
    /// Resize all pages
    pub fn resize(&mut self, area: Rect) {
        for page in self.pages.values_mut() {
            page.on_resize(area);
        }
    }
    
    /// Add page to history
    fn add_to_history(&mut self, page_id: PageId) {
        // Don't add duplicate consecutive entries
        if self.history.last() != Some(&page_id) {
            self.history.push(page_id);
            
            // Limit history size
            if self.history.len() > self.max_history {
                self.history.remove(0);
            }
        }
    }
}

impl Default for PageManager {
    fn default() -> Self {
        Self::new()
    }
}