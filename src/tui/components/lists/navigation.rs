//! Advanced navigation capabilities for lists including search, pagination, and bookmarks.
//!
//! This module provides sophisticated navigation features like global search,
//! pagination controls, bookmarking, and history tracking for list components.

use super::ListItem;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use std::collections::{HashMap, VecDeque};
use std::marker::PhantomData;
use std::time::{Duration, Instant};

/// Navigation state and capabilities for lists
#[derive(Debug)]
pub struct ListNavigator<T: ListItem> {
    /// Current page number (0-based)
    current_page: usize,

    /// Items per page
    page_size: usize,

    /// Total number of items
    total_items: usize,

    /// Navigation history
    history: VecDeque<NavigationEntry>,

    /// Current position in history
    history_position: usize,

    /// Bookmarks by name
    bookmarks: HashMap<String, Bookmark>,

    /// Search state
    search_state: SearchState,

    /// Quick jump state
    jump_state: JumpState,

    /// Navigation configuration
    config: NavigationConfig,

    /// Phantom data for type parameter T
    _phantom: PhantomData<T>,
}

/// Navigation history entry
#[derive(Debug, Clone)]
struct NavigationEntry {
    page: usize,
    selected_id: Option<String>,
    scroll_offset: usize,
    timestamp: Instant,
    description: String,
}

/// Bookmark entry
#[derive(Debug, Clone)]
pub struct Bookmark {
    pub name: String,
    pub page: usize,
    pub selected_id: Option<String>,
    pub scroll_offset: usize,
    pub created_at: Instant,
    pub description: Option<String>,
}

/// Search state for navigation
#[derive(Debug, Clone)]
struct SearchState {
    query: String,
    results: Vec<SearchResult>,
    current_result_index: usize,
    is_active: bool,
    last_search: Option<Instant>,
}

/// Search result entry
#[derive(Debug, Clone)]
struct SearchResult {
    item_id: String,
    page: usize,
    score: f64,
    snippet: String,
}

/// Quick jump state for going to specific pages/items
#[derive(Debug, Clone)]
struct JumpState {
    input: String,
    mode: JumpMode,
    is_active: bool,
}

/// Jump mode types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JumpMode {
    /// Jump to specific page number
    Page,
    /// Jump to item by ID
    ItemId,
    /// Jump to item by index
    ItemIndex,
    /// Jump to bookmark
    Bookmark,
}

/// Navigation configuration
#[derive(Debug, Clone)]
pub struct NavigationConfig {
    /// Maximum history entries to keep
    pub max_history_entries: usize,
    /// Default page size
    pub default_page_size: usize,
    /// Maximum page size
    pub max_page_size: usize,
    /// Whether to enable search highlighting
    pub enable_search_highlighting: bool,
    /// Search result preview length
    pub search_snippet_length: usize,
    /// History retention duration
    pub history_retention: Duration,
    /// Whether to auto-save navigation state
    pub auto_save_state: bool,
}

impl Default for NavigationConfig {
    fn default() -> Self {
        Self {
            max_history_entries: 100,
            default_page_size: 50,
            max_page_size: 1000,
            enable_search_highlighting: true,
            search_snippet_length: 100,
            history_retention: Duration::from_secs(3600), // 1 hour
            auto_save_state: true,
        }
    }
}

impl<T: ListItem> ListNavigator<T> {
    /// Create a new list navigator
    pub fn new() -> Self {
        Self::with_config(NavigationConfig::default())
    }
    
    /// Create a new list navigator with custom configuration
    pub fn with_config(config: NavigationConfig) -> Self {
        Self {
            current_page: 0,
            page_size: config.default_page_size,
            total_items: 0,
            history: VecDeque::new(),
            history_position: 0,
            bookmarks: HashMap::new(),
            search_state: SearchState {
                query: String::new(),
                results: Vec::new(),
                current_result_index: 0,
                is_active: false,
                last_search: None,
            },
            jump_state: JumpState {
                input: String::new(),
                mode: JumpMode::Page,
                is_active: false,
            },
            config,
            _phantom: PhantomData,
        }
    }

    /// Set the total number of items
    pub fn set_total_items(&mut self, total: usize) {
        self.total_items = total;
        
        // Ensure current page is valid
        let max_page = self.max_page();
        if self.current_page > max_page {
            self.current_page = max_page;
        }
    }
    
    /// Get the current page number (0-based)
    pub fn current_page(&self) -> usize {
        self.current_page
    }
    
    /// Get the page size
    pub fn page_size(&self) -> usize {
        self.page_size
    }
    
    /// Get the total number of pages
    pub fn total_pages(&self) -> usize {
        if self.total_items == 0 {
            0
        } else {
            (self.total_items + self.page_size - 1) / self.page_size
        }
    }
    
    /// Get the maximum valid page index
    pub fn max_page(&self) -> usize {
        self.total_pages().saturating_sub(1)
    }
    
    /// Get the range of items for the current page
    pub fn current_page_range(&self) -> std::ops::Range<usize> {
        let start = self.current_page * self.page_size;
        let end = (start + self.page_size).min(self.total_items);
        start..end
    }
    
    /// Go to the next page
    pub fn next_page(&mut self) -> Result<bool> {
        if self.current_page < self.max_page() {
            self.goto_page(self.current_page + 1)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Go to the previous page
    pub fn previous_page(&mut self) -> Result<bool> {
        if self.current_page > 0 {
            self.goto_page(self.current_page - 1)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Go to the first page
    pub fn first_page(&mut self) -> Result<()> {
        self.goto_page(0)
    }
    
    /// Go to the last page
    pub fn last_page(&mut self) -> Result<()> {
        self.goto_page(self.max_page())
    }
    
    /// Go to a specific page
    pub fn goto_page(&mut self, page: usize) -> Result<()> {
        let clamped_page = page.min(self.max_page());
        
        if clamped_page != self.current_page {
            self.add_history_entry(
                self.current_page,
                None,
                0,
                format!("Navigated to page {}", clamped_page + 1),
            );
            self.current_page = clamped_page;
        }
        
        Ok(())
    }
    
    /// Set the page size
    pub fn set_page_size(&mut self, size: usize) -> Result<()> {
        let new_size = size.clamp(1, self.config.max_page_size);
        
        if new_size != self.page_size {
            // Calculate new page to maintain roughly the same position
            let current_item_index = self.current_page * self.page_size;
            self.page_size = new_size;
            self.current_page = current_item_index / new_size;
            
            // Ensure page is valid
            self.current_page = self.current_page.min(self.max_page());
        }
        
        Ok(())
    }
    
    /// Add a bookmark
    pub fn add_bookmark(&mut self, name: String, description: Option<String>) -> Result<()> {
        let bookmark = Bookmark {
            name: name.clone(),
            page: self.current_page,
            selected_id: None, // Could be passed as parameter
            scroll_offset: 0,  // Could be passed as parameter
            created_at: Instant::now(),
            description,
        };
        
        self.bookmarks.insert(name, bookmark);
        Ok(())
    }
    
    /// Remove a bookmark
    pub fn remove_bookmark(&mut self, name: &str) -> Option<Bookmark> {
        self.bookmarks.remove(name)
    }
    
    /// Get all bookmarks
    pub fn bookmarks(&self) -> impl Iterator<Item = (&String, &Bookmark)> {
        self.bookmarks.iter()
    }
    
    /// Go to a bookmark
    pub fn goto_bookmark(&mut self, name: &str) -> Result<bool> {
        if let Some(bookmark) = self.bookmarks.get(name) {
            let bookmark_page = bookmark.page;
            self.add_history_entry(
                self.current_page,
                None,
                0,
                format!("Navigated to bookmark '{}'", name),
            );
            self.current_page = bookmark_page.min(self.max_page());
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Start a search
    pub fn start_search(&mut self, query: String) -> Result<()> {
        self.search_state.query = query;
        self.search_state.is_active = true;
        self.search_state.current_result_index = 0;
        // Note: Actual search implementation would depend on the list items
        // This is a placeholder for the search mechanism
        Ok(())
    }
    
    /// Clear the current search
    pub fn clear_search(&mut self) {
        self.search_state.query.clear();
        self.search_state.results.clear();
        self.search_state.current_result_index = 0;
        self.search_state.is_active = false;
    }
    
    /// Go to the next search result
    pub fn next_search_result(&mut self) -> Result<bool> {
        if !self.search_state.results.is_empty() {
            self.search_state.current_result_index = 
                (self.search_state.current_result_index + 1) % self.search_state.results.len();
                
            if let Some(result) = self.search_state.results.get(self.search_state.current_result_index) {
                self.goto_page(result.page)?;
                return Ok(true);
            }
        }
        Ok(false)
    }
    
    /// Go to the previous search result
    pub fn previous_search_result(&mut self) -> Result<bool> {
        if !self.search_state.results.is_empty() {
            if self.search_state.current_result_index == 0 {
                self.search_state.current_result_index = self.search_state.results.len() - 1;
            } else {
                self.search_state.current_result_index -= 1;
            }
            
            if let Some(result) = self.search_state.results.get(self.search_state.current_result_index) {
                self.goto_page(result.page)?;
                return Ok(true);
            }
        }
        Ok(false)
    }
    
    /// Start quick jump mode
    pub fn start_quick_jump(&mut self, mode: JumpMode) {
        self.jump_state.mode = mode;
        self.jump_state.is_active = true;
        self.jump_state.input.clear();
    }
    
    /// Add character to quick jump input
    pub fn quick_jump_input(&mut self, ch: char) {
        if self.jump_state.is_active {
            self.jump_state.input.push(ch);
        }
    }
    
    /// Execute quick jump
    pub fn execute_quick_jump(&mut self) -> Result<bool> {
        if !self.jump_state.is_active {
            return Ok(false);
        }
        
        let success = match self.jump_state.mode {
            JumpMode::Page => {
                if let Ok(page) = self.jump_state.input.parse::<usize>() {
                    self.goto_page(page.saturating_sub(1))?; // Convert to 0-based
                    true
                } else {
                    false
                }
            }
            JumpMode::ItemIndex => {
                if let Ok(index) = self.jump_state.input.parse::<usize>() {
                    let page = index / self.page_size;
                    self.goto_page(page)?;
                    true
                } else {
                    false
                }
            }
            JumpMode::Bookmark => {
                let bookmark_name = self.jump_state.input.clone();
                self.goto_bookmark(&bookmark_name)?
            }
            JumpMode::ItemId => {
                // This would require access to the actual items
                // Placeholder implementation
                false
            }
        };
        
        self.jump_state.is_active = false;
        self.jump_state.input.clear();
        
        Ok(success)
    }
    
    /// Cancel quick jump
    pub fn cancel_quick_jump(&mut self) {
        self.jump_state.is_active = false;
        self.jump_state.input.clear();
    }
    
    /// Go back in history
    pub fn go_back(&mut self) -> Result<bool> {
        if self.history_position < self.history.len().saturating_sub(1) {
            self.history_position += 1;
            if let Some(entry) = self.history.get(self.history.len() - 1 - self.history_position) {
                self.current_page = entry.page;
                return Ok(true);
            }
        }
        Ok(false)
    }
    
    /// Go forward in history
    pub fn go_forward(&mut self) -> Result<bool> {
        if self.history_position > 0 {
            self.history_position -= 1;
            if let Some(entry) = self.history.get(self.history.len() - 1 - self.history_position) {
                self.current_page = entry.page;
                return Ok(true);
            }
        }
        Ok(false)
    }
    
    /// Handle keyboard input for navigation
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<bool> {
        if self.jump_state.is_active {
            return self.handle_jump_key_event(key);
        }
        
        match key.code {
            KeyCode::Char('n') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                self.next_page()?;
                Ok(true)
            }
            KeyCode::Char('p') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                self.previous_page()?;
                Ok(true)
            }
            KeyCode::Char('g') => {
                self.start_quick_jump(JumpMode::Page);
                Ok(true)
            }
            KeyCode::Char('b') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                self.start_quick_jump(JumpMode::Bookmark);
                Ok(true)
            }
            KeyCode::Char('[') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                self.go_back()?;
                Ok(true)
            }
            KeyCode::Char(']') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                self.go_forward()?;
                Ok(true)
            }
            KeyCode::F(3) => {
                self.next_search_result()?;
                Ok(true)
            }
            KeyCode::F(15) => { // Shift+F3
                self.previous_search_result()?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }
    
    /// Handle keyboard input for quick jump mode
    fn handle_jump_key_event(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char(c) => {
                self.quick_jump_input(c);
                Ok(true)
            }
            KeyCode::Backspace => {
                self.jump_state.input.pop();
                Ok(true)
            }
            KeyCode::Enter => {
                self.execute_quick_jump()?;
                Ok(true)
            }
            KeyCode::Esc => {
                self.cancel_quick_jump();
                Ok(true)
            }
            _ => Ok(false),
        }
    }
    
    /// Add an entry to navigation history
    fn add_history_entry(&mut self, page: usize, selected_id: Option<String>, scroll_offset: usize, description: String) {
        let entry = NavigationEntry {
            page,
            selected_id,
            scroll_offset,
            timestamp: Instant::now(),
            description,
        };
        
        self.history.push_back(entry);
        
        // Trim history if it exceeds max entries
        if self.history.len() > self.config.max_history_entries {
            self.history.pop_front();
        }
        
        // Reset history position when adding new entry
        self.history_position = 0;
    }
    
    /// Clean up old history entries
    pub fn cleanup_history(&mut self) {
        let cutoff = Instant::now() - self.config.history_retention;
        self.history.retain(|entry| entry.timestamp > cutoff);
    }
    
    /// Render navigation status line
    pub fn render_status_line(&self, theme: &crate::tui::themes::Theme) -> Line<'static> {
        let mut spans = Vec::new();
        
        // Page info
        spans.push(Span::styled(
            format!("Page {}/{}", self.current_page + 1, self.total_pages()),
            Style::default().fg(theme.colors.text),
        ));
        
        // Item range
        let range = self.current_page_range();
        spans.push(Span::styled(
            format!(" ({}-{} of {})", range.start + 1, range.end, self.total_items),
            Style::default().fg(theme.colors.muted),
        ));
        
        // Search info
        if self.search_state.is_active && !self.search_state.results.is_empty() {
            spans.push(Span::raw(" | "));
            spans.push(Span::styled(
                format!("Search: {}/{} matches", 
                    self.search_state.current_result_index + 1,
                    self.search_state.results.len()),
                Style::default().fg(theme.colors.primary),
            ));
        }
        
        // Quick jump mode
        if self.jump_state.is_active {
            spans.push(Span::raw(" | "));
            let mode_text = match self.jump_state.mode {
                JumpMode::Page => "Go to page:",
                JumpMode::ItemId => "Go to item:",
                JumpMode::ItemIndex => "Go to index:",
                JumpMode::Bookmark => "Go to bookmark:",
            };
            spans.push(Span::styled(
                format!("{} {}", mode_text, self.jump_state.input),
                Style::default()
                    .fg(theme.colors.primary)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        
        Line::from(spans)
    }
    
    /// Render bookmark list
    pub fn render_bookmark_list(&self, theme: &crate::tui::themes::Theme) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        
        if self.bookmarks.is_empty() {
            lines.push(Line::from(Span::styled(
                "No bookmarks",
                Style::default().fg(theme.colors.muted),
            )));
            return lines;
        }
        
        for (name, bookmark) in &self.bookmarks {
            let mut spans = Vec::new();

            spans.push(Span::styled(
                name.clone(),
                Style::default()
                    .fg(theme.colors.primary)
                    .add_modifier(Modifier::BOLD),
            ));

            spans.push(Span::styled(
                format!(" (page {})", bookmark.page + 1),
                Style::default().fg(theme.colors.text),
            ));

            if let Some(description) = &bookmark.description {
                spans.push(Span::raw(" - "));
                spans.push(Span::styled(
                    description.clone(),
                    Style::default().fg(theme.colors.muted),
                ));
            }

            lines.push(Line::from(spans));
        }
        
        lines
    }
    
    /// Get navigation help text
    pub fn help_text() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Ctrl+N", "Next page"),
            ("Ctrl+P", "Previous page"),
            ("g", "Go to page"),
            ("Ctrl+B", "Go to bookmark"),
            ("Ctrl+[", "Go back"),
            ("Ctrl+]", "Go forward"),
            ("F3", "Next search result"),
            ("Shift+F3", "Previous search result"),
        ]
    }
}

impl<T: ListItem> Default for ListNavigator<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::components::lists::SimpleListItem;
    
    #[test]
    fn test_navigator_creation() {
        let navigator: ListNavigator<SimpleListItem> = ListNavigator::new();
        assert_eq!(navigator.current_page(), 0);
        assert_eq!(navigator.total_pages(), 0);
    }
    
    #[test]
    fn test_pagination() {
        let mut navigator: ListNavigator<SimpleListItem> = ListNavigator::new();
        navigator.set_total_items(100);

        assert_eq!(navigator.total_pages(), 2); // 50 items per page by default
        assert_eq!(navigator.current_page(), 0);

        navigator.next_page().unwrap();
        assert_eq!(navigator.current_page(), 1);

        navigator.previous_page().unwrap();
        assert_eq!(navigator.current_page(), 0);
    }

    #[test]
    fn test_bookmarks() {
        let mut navigator: ListNavigator<SimpleListItem> = ListNavigator::new();
        navigator.set_total_items(100);
        navigator.goto_page(1).unwrap();
        
        navigator.add_bookmark("test".to_string(), Some("Test bookmark".to_string())).unwrap();
        assert_eq!(navigator.bookmarks().count(), 1);
        
        navigator.goto_page(0).unwrap();
        assert_eq!(navigator.current_page(), 0);
        
        navigator.goto_bookmark("test").unwrap();
        assert_eq!(navigator.current_page(), 1);
    }
    
    #[test]
    fn test_page_range() {
        let mut navigator: ListNavigator<SimpleListItem> = ListNavigator::new();
        navigator.set_total_items(75); // 75 items, 50 per page = 2 pages
        
        let range = navigator.current_page_range();
        assert_eq!(range, 0..50);
        
        navigator.next_page().unwrap();
        let range = navigator.current_page_range();
        assert_eq!(range, 50..75); // Last page has only 25 items
    }
    
    #[test]
    fn test_quick_jump() {
        let mut navigator: ListNavigator<SimpleListItem> = ListNavigator::new();
        navigator.set_total_items(100);

        navigator.start_quick_jump(JumpMode::Page);
        navigator.quick_jump_input('2');
        navigator.execute_quick_jump().unwrap();
        
        assert_eq!(navigator.current_page(), 1); // Page 2 is index 1
    }
}