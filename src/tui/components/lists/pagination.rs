//! Pagination support for list components with flexible page management.
//!
//! This module provides pagination capabilities that can work with both
//! virtual lists and regular lists, supporting various pagination styles
//! and navigation patterns.

use super::ListItem;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Pagination manager for list components
pub struct PaginationManager<T: ListItem> {
    /// Current page (0-based)
    current_page: usize,
    
    /// Items per page
    page_size: usize,
    
    /// Total number of items
    total_items: usize,
    
    /// Pagination configuration
    config: PaginationConfig,
    
    /// Page cache for performance
    page_cache: HashMap<usize, PageCache<T>>,
    
    /// Navigation state
    navigation_state: NavigationState,
    
    /// Performance metrics
    metrics: PaginationMetrics,
    
    /// Event callbacks
    callbacks: Vec<Box<dyn Fn(PaginationEvent) + Send + Sync>>,
}

impl<T: ListItem> std::fmt::Debug for PaginationManager<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PaginationManager")
            .field("current_page", &self.current_page)
            .field("page_size", &self.page_size)
            .field("total_items", &self.total_items)
            .field("config", &self.config)
            .field("metrics", &self.metrics)
            .field("callbacks", &format!("[{} callbacks]", self.callbacks.len()))
            .finish()
    }
}

/// Pagination configuration
#[derive(Debug, Clone)]
pub struct PaginationConfig {
    /// Default page size
    pub default_page_size: usize,
    
    /// Minimum page size
    pub min_page_size: usize,
    
    /// Maximum page size
    pub max_page_size: usize,
    
    /// Available page sizes for user selection
    pub available_page_sizes: Vec<usize>,
    
    /// Whether to show page numbers
    pub show_page_numbers: bool,
    
    /// Whether to show page size selector
    pub show_page_size_selector: bool,
    
    /// Whether to show total items count
    pub show_total_count: bool,
    
    /// Whether to show "Go to page" input
    pub show_goto_page: bool,
    
    /// Maximum number of page numbers to show
    pub max_page_numbers: usize,
    
    /// Whether to enable page caching
    pub enable_caching: bool,
    
    /// Maximum pages to cache
    pub max_cached_pages: usize,
    
    /// Page cache TTL
    pub cache_ttl: Duration,
    
    /// Pagination display style
    pub display_style: PaginationStyle,
    
    /// Styling configuration
    pub styling: PaginationStyling,
}

/// Pagination display styles
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaginationStyle {
    /// Compact: "Page 1 of 10"
    Compact,
    /// Full: "Page 1 of 10 (1-20 of 200 items)"
    Full,
    /// Numbers: "< 1 2 [3] 4 5 >"
    Numbers,
    /// Google: "< Previous 1 2 [3] 4 5 Next >"
    Google,
    /// Custom: User-defined format
    Custom,
}

/// Styling configuration for pagination
#[derive(Debug, Clone)]
pub struct PaginationStyling {
    /// Style for current page
    pub current_page_style: Style,
    
    /// Style for other pages
    pub page_style: Style,
    
    /// Style for navigation arrows
    pub navigation_style: Style,
    
    /// Style for disabled elements
    pub disabled_style: Style,
    
    /// Style for text elements
    pub text_style: Style,
    
    /// Style for input elements
    pub input_style: Style,
}

impl Default for PaginationConfig {
    fn default() -> Self {
        Self {
            default_page_size: 20,
            min_page_size: 5,
            max_page_size: 1000,
            available_page_sizes: vec![10, 20, 50, 100, 200],
            show_page_numbers: true,
            show_page_size_selector: true,
            show_total_count: true,
            show_goto_page: false,
            max_page_numbers: 7,
            enable_caching: true,
            max_cached_pages: 10,
            cache_ttl: Duration::from_secs(300),
            display_style: PaginationStyle::Full,
            styling: PaginationStyling {
                current_page_style: Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
                page_style: Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::UNDERLINED),
                navigation_style: Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
                disabled_style: Style::default()
                    .fg(Color::DarkGray),
                text_style: Style::default()
                    .fg(Color::White),
                input_style: Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::White),
            },
        }
    }
}

/// Cached page data
#[derive(Debug, Clone)]
struct PageCache<T: ListItem> {
    items: Vec<T>,
    cached_at: Instant,
    last_accessed: Instant,
    access_count: usize,
}

/// Navigation state for "Go to page" functionality
#[derive(Debug, Clone)]
struct NavigationState {
    goto_input: String,
    goto_active: bool,
    page_size_input: String,
    page_size_active: bool,
}

impl Default for NavigationState {
    fn default() -> Self {
        Self {
            goto_input: String::new(),
            goto_active: false,
            page_size_input: String::new(),
            page_size_active: false,
        }
    }
}

/// Pagination events
#[derive(Debug, Clone)]
pub enum PaginationEvent {
    /// Page changed
    PageChanged {
        old_page: usize,
        new_page: usize,
        page_size: usize,
    },
    
    /// Page size changed
    PageSizeChanged {
        old_size: usize,
        new_size: usize,
        new_total_pages: usize,
    },
    
    /// Navigation requested (go to page input)
    NavigationRequested {
        target_page: usize,
    },
    
    /// Page cache updated
    CacheUpdated {
        page: usize,
        cache_size: usize,
    },
}

/// Performance metrics for pagination
#[derive(Debug, Clone, Default)]
pub struct PaginationMetrics {
    /// Total page navigation operations
    pub total_navigations: u64,
    
    /// Cache hits
    pub cache_hits: u64,
    
    /// Cache misses
    pub cache_misses: u64,
    
    /// Average page load time
    pub avg_page_load_time_ms: f64,
    
    /// Current cache size
    pub cache_size: usize,
    
    /// Most frequently accessed page
    pub most_accessed_page: Option<usize>,
    
    /// Navigation pattern stats
    pub navigation_patterns: HashMap<String, u64>,
}

impl<T: ListItem> PaginationManager<T> {
    /// Create a new pagination manager
    pub fn new() -> Self {
        Self::with_config(PaginationConfig::default())
    }
    
    /// Create a new pagination manager with custom configuration
    pub fn with_config(config: PaginationConfig) -> Self {
        Self {
            current_page: 0,
            page_size: config.default_page_size,
            total_items: 0,
            config,
            page_cache: HashMap::new(),
            navigation_state: NavigationState::default(),
            metrics: PaginationMetrics::default(),
            callbacks: Vec::new(),
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
    
    /// Get the current page (0-based)
    pub fn current_page(&self) -> usize {
        self.current_page
    }
    
    /// Get the current page size
    pub fn page_size(&self) -> usize {
        self.page_size
    }
    
    /// Get the total number of items
    pub fn total_items(&self) -> usize {
        self.total_items
    }
    
    /// Get the total number of pages
    pub fn total_pages(&self) -> usize {
        if self.total_items == 0 {
            1
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
    
    /// Get the range of items for a specific page
    pub fn page_range(&self, page: usize) -> std::ops::Range<usize> {
        let start = page * self.page_size;
        let end = (start + self.page_size).min(self.total_items);
        start..end
    }
    
    /// Add an event callback
    pub fn add_callback<F>(&mut self, callback: F)
    where
        F: Fn(PaginationEvent) + Send + Sync + 'static,
    {
        self.callbacks.push(Box::new(callback));
    }
    
    /// Go to the next page
    pub fn next_page(&mut self) -> Result<bool> {
        if self.current_page < self.max_page() {
            self.goto_page(self.current_page + 1)?;
            self.record_navigation("next");
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Go to the previous page
    pub fn previous_page(&mut self) -> Result<bool> {
        if self.current_page > 0 {
            self.goto_page(self.current_page - 1)?;
            self.record_navigation("previous");
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Go to the first page
    pub fn first_page(&mut self) -> Result<()> {
        self.goto_page(0)?;
        self.record_navigation("first");
        Ok(())
    }
    
    /// Go to the last page
    pub fn last_page(&mut self) -> Result<()> {
        self.goto_page(self.max_page())?;
        self.record_navigation("last");
        Ok(())
    }
    
    /// Go to a specific page
    pub fn goto_page(&mut self, page: usize) -> Result<()> {
        let old_page = self.current_page;
        let new_page = page.min(self.max_page());
        
        if old_page != new_page {
            self.current_page = new_page;
            self.emit_event(PaginationEvent::PageChanged {
                old_page,
                new_page,
                page_size: self.page_size,
            });
            
            self.metrics.total_navigations += 1;
        }
        
        Ok(())
    }
    
    /// Set the page size
    pub fn set_page_size(&mut self, size: usize) -> Result<()> {
        let new_size = size.clamp(self.config.min_page_size, self.config.max_page_size);
        let old_size = self.page_size;
        
        if old_size != new_size {
            // Calculate new page to maintain roughly the same position
            let current_item_index = self.current_page * self.page_size;
            self.page_size = new_size;
            self.current_page = current_item_index / new_size;
            
            // Ensure page is valid
            self.current_page = self.current_page.min(self.max_page());
            
            // Clear cache as page boundaries have changed
            if self.config.enable_caching {
                self.page_cache.clear();
            }
            
            self.emit_event(PaginationEvent::PageSizeChanged {
                old_size,
                new_size,
                new_total_pages: self.total_pages(),
            });
        }
        
        Ok(())
    }
    
    /// Start "Go to page" input mode
    pub fn start_goto_page(&mut self) {
        if self.config.show_goto_page {
            self.navigation_state.goto_active = true;
            self.navigation_state.goto_input.clear();
        }
    }
    
    /// Add character to "Go to page" input
    pub fn goto_page_input(&mut self, ch: char) {
        if self.navigation_state.goto_active && ch.is_ascii_digit() {
            self.navigation_state.goto_input.push(ch);
        }
    }
    
    /// Execute "Go to page" command
    pub fn execute_goto_page(&mut self) -> Result<bool> {
        if !self.navigation_state.goto_active {
            return Ok(false);
        }
        
        let success = if let Ok(page) = self.navigation_state.goto_input.parse::<usize>() {
            if page > 0 {
                self.emit_event(PaginationEvent::NavigationRequested {
                    target_page: page - 1, // Convert to 0-based
                });
                self.goto_page(page - 1)?;
                true
            } else {
                false
            }
        } else {
            false
        };
        
        self.navigation_state.goto_active = false;
        self.navigation_state.goto_input.clear();
        
        if success {
            self.record_navigation("goto");
        }
        
        Ok(success)
    }
    
    /// Cancel "Go to page" input
    pub fn cancel_goto_page(&mut self) {
        self.navigation_state.goto_active = false;
        self.navigation_state.goto_input.clear();
    }
    
    /// Start page size input mode
    pub fn start_page_size_input(&mut self) {
        if self.config.show_page_size_selector {
            self.navigation_state.page_size_active = true;
            self.navigation_state.page_size_input = self.page_size.to_string();
        }
    }
    
    /// Add character to page size input
    pub fn page_size_input(&mut self, ch: char) {
        if self.navigation_state.page_size_active {
            if ch.is_ascii_digit() {
                self.navigation_state.page_size_input.push(ch);
            } else if ch == '\u{8}' { // Backspace
                self.navigation_state.page_size_input.pop();
            }
        }
    }
    
    /// Execute page size change
    pub fn execute_page_size_change(&mut self) -> Result<bool> {
        if !self.navigation_state.page_size_active {
            return Ok(false);
        }
        
        let success = if let Ok(size) = self.navigation_state.page_size_input.parse::<usize>() {
            self.set_page_size(size)?;
            true
        } else {
            false
        };
        
        self.navigation_state.page_size_active = false;
        self.navigation_state.page_size_input.clear();
        
        Ok(success)
    }
    
    /// Cancel page size input
    pub fn cancel_page_size_input(&mut self) {
        self.navigation_state.page_size_active = false;
        self.navigation_state.page_size_input.clear();
    }
    
    /// Handle keyboard input
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<bool> {
        if self.navigation_state.goto_active {
            return self.handle_goto_key_event(key);
        }
        
        if self.navigation_state.page_size_active {
            return self.handle_page_size_key_event(key);
        }
        
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                self.previous_page()?;
                Ok(true)
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.next_page()?;
                Ok(true)
            }
            KeyCode::Home => {
                self.first_page()?;
                Ok(true)
            }
            KeyCode::End => {
                self.last_page()?;
                Ok(true)
            }
            KeyCode::Char('g') => {
                self.start_goto_page();
                Ok(true)
            }
            KeyCode::Char('G') => {
                self.last_page()?;
                Ok(true)
            }
            KeyCode::Char('s') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                self.start_page_size_input();
                Ok(true)
            }
            KeyCode::PageUp => {
                // Jump back multiple pages
                let jump_pages = 5;
                if self.current_page >= jump_pages {
                    self.goto_page(self.current_page - jump_pages)?;
                } else {
                    self.first_page()?;
                }
                Ok(true)
            }
            KeyCode::PageDown => {
                // Jump forward multiple pages
                let jump_pages = 5;
                let target = self.current_page + jump_pages;
                if target <= self.max_page() {
                    self.goto_page(target)?;
                } else {
                    self.last_page()?;
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
    
    /// Handle keyboard input for "Go to page" mode
    fn handle_goto_key_event(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char(c) if c.is_ascii_digit() => {
                self.goto_page_input(c);
                Ok(true)
            }
            KeyCode::Backspace => {
                self.navigation_state.goto_input.pop();
                Ok(true)
            }
            KeyCode::Enter => {
                self.execute_goto_page()?;
                Ok(true)
            }
            KeyCode::Esc => {
                self.cancel_goto_page();
                Ok(true)
            }
            _ => Ok(false),
        }
    }
    
    /// Handle keyboard input for page size mode
    fn handle_page_size_key_event(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char(c) if c.is_ascii_digit() => {
                self.page_size_input(c);
                Ok(true)
            }
            KeyCode::Backspace => {
                self.page_size_input('\u{8}');
                Ok(true)
            }
            KeyCode::Enter => {
                self.execute_page_size_change()?;
                Ok(true)
            }
            KeyCode::Esc => {
                self.cancel_page_size_input();
                Ok(true)
            }
            _ => Ok(false),
        }
    }
    
    /// Handle mouse input
    pub fn handle_mouse_event(&mut self, event: MouseEvent, area: Rect) -> Result<bool> {
        match event.kind {
            MouseEventKind::Down(button) => {
                match button {
                    crossterm::event::MouseButton::Left => {
                        // This would need specific area calculations for clickable elements
                        // For now, just handle basic navigation
                        if event.column < area.width / 2 {
                            self.previous_page()?;
                        } else {
                            self.next_page()?;
                        }
                        Ok(true)
                    }
                    _ => Ok(false),
                }
            }
            MouseEventKind::ScrollUp => {
                self.previous_page()?;
                Ok(true)
            }
            MouseEventKind::ScrollDown => {
                self.next_page()?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }
    
    /// Cache page data
    pub fn cache_page(&mut self, page: usize, items: Vec<T>) {
        if !self.config.enable_caching {
            return;
        }
        
        let cache_entry = PageCache {
            items,
            cached_at: Instant::now(),
            last_accessed: Instant::now(),
            access_count: 1,
        };
        
        self.page_cache.insert(page, cache_entry);
        
        // Clean up old cache entries
        if self.page_cache.len() > self.config.max_cached_pages {
            self.cleanup_cache();
        }
        
        self.emit_event(PaginationEvent::CacheUpdated {
            page,
            cache_size: self.page_cache.len(),
        });
    }
    
    /// Get cached page data
    pub fn get_cached_page(&mut self, page: usize) -> Option<&Vec<T>> {
        if let Some(cache_entry) = self.page_cache.get_mut(&page) {
            cache_entry.last_accessed = Instant::now();
            cache_entry.access_count += 1;
            self.metrics.cache_hits += 1;
            Some(&cache_entry.items)
        } else {
            self.metrics.cache_misses += 1;
            None
        }
    }
    
    /// Clean up old cache entries
    fn cleanup_cache(&mut self) {
        let cutoff = Instant::now() - self.config.cache_ttl;
        
        // Remove expired entries
        self.page_cache.retain(|_, entry| entry.cached_at > cutoff);
        
        // If still too many, remove least recently used
        if self.page_cache.len() > self.config.max_cached_pages {
            let entries: Vec<_> = self.page_cache.iter().map(|(k, v)| (k.clone(), v.last_accessed)).collect();
            let mut sorted_entries = entries;
            sorted_entries.sort_by_key(|(_, accessed)| *accessed);
            
            let excess = self.page_cache.len() - self.config.max_cached_pages;
            for (page, _) in sorted_entries.iter().take(excess) {
                self.page_cache.remove(page);
            }
        }
    }
    
    /// Record navigation pattern for metrics
    fn record_navigation(&mut self, pattern: &str) {
        *self.metrics.navigation_patterns.entry(pattern.to_string()).or_insert(0) += 1;
    }
    
    /// Emit an event to all callbacks
    fn emit_event(&self, event: PaginationEvent) {
        for callback in &self.callbacks {
            callback(event.clone());
        }
    }
    
    /// Render pagination controls
    pub fn render(&self, _area: Rect, theme: &crate::tui::themes::Theme) -> Result<Vec<Line<'static>>> {
        let mut lines = Vec::new();
        
        match self.config.display_style {
            PaginationStyle::Compact => {
                lines.push(self.render_compact_style(theme));
            }
            PaginationStyle::Full => {
                lines.push(self.render_full_style(theme));
            }
            PaginationStyle::Numbers => {
                lines.push(self.render_numbers_style(theme));
            }
            PaginationStyle::Google => {
                lines.push(self.render_google_style(theme));
            }
            PaginationStyle::Custom => {
                lines.push(self.render_custom_style(theme));
            }
        }
        
        // Add input line if active
        if self.navigation_state.goto_active {
            lines.push(Line::from(vec![
                Span::styled("Go to page: ", self.config.styling.text_style),
                Span::styled(self.navigation_state.goto_input.clone(), self.config.styling.input_style),
                Span::styled("_", self.config.styling.input_style),
            ]));
        }

        if self.navigation_state.page_size_active {
            lines.push(Line::from(vec![
                Span::styled("Items per page: ", self.config.styling.text_style),
                Span::styled(self.navigation_state.page_size_input.clone(), self.config.styling.input_style),
                Span::styled("_", self.config.styling.input_style),
            ]));
        }
        
        Ok(lines)
    }
    
    /// Render compact pagination style
    fn render_compact_style(&self, _theme: &crate::tui::themes::Theme) -> Line<'static> {
        Line::from(vec![
            Span::styled(
                format!("Page {} of {}", self.current_page + 1, self.total_pages()),
                self.config.styling.text_style,
            ),
        ])
    }
    
    /// Render full pagination style
    fn render_full_style(&self, _theme: &crate::tui::themes::Theme) -> Line<'static> {
        let range = self.current_page_range();
        Line::from(vec![
            Span::styled(
                format!("Page {} of {}", self.current_page + 1, self.total_pages()),
                self.config.styling.text_style,
            ),
            Span::styled(
                format!(" ({}-{} of {} items)", 
                    range.start + 1, 
                    range.end, 
                    self.total_items),
                self.config.styling.text_style,
            ),
        ])
    }
    
    /// Render numbers pagination style
    fn render_numbers_style(&self, _theme: &crate::tui::themes::Theme) -> Line<'static> {
        let mut spans = Vec::new();
        
        // Previous arrow
        if self.current_page > 0 {
            spans.push(Span::styled("< ", self.config.styling.navigation_style));
        } else {
            spans.push(Span::styled("< ", self.config.styling.disabled_style));
        }
        
        // Page numbers
        let start_page = self.current_page.saturating_sub(self.config.max_page_numbers / 2);
        let end_page = (start_page + self.config.max_page_numbers).min(self.total_pages());
        
        for page in start_page..end_page {
            if page == self.current_page {
                spans.push(Span::styled(
                    format!("[{}] ", page + 1),
                    self.config.styling.current_page_style,
                ));
            } else {
                spans.push(Span::styled(
                    format!("{} ", page + 1),
                    self.config.styling.page_style,
                ));
            }
        }
        
        // Next arrow
        if self.current_page < self.max_page() {
            spans.push(Span::styled(">", self.config.styling.navigation_style));
        } else {
            spans.push(Span::styled(">", self.config.styling.disabled_style));
        }
        
        Line::from(spans)
    }
    
    /// Render Google-style pagination
    fn render_google_style(&self, _theme: &crate::tui::themes::Theme) -> Line<'static> {
        let mut spans = Vec::new();
        
        // Previous link
        if self.current_page > 0 {
            spans.push(Span::styled("< Previous ", self.config.styling.navigation_style));
        }
        
        // Page numbers (similar to numbers style but with "Previous"/"Next")
        let start_page = self.current_page.saturating_sub(self.config.max_page_numbers / 2);
        let end_page = (start_page + self.config.max_page_numbers).min(self.total_pages());
        
        for page in start_page..end_page {
            if page == self.current_page {
                spans.push(Span::styled(
                    format!("[{}] ", page + 1),
                    self.config.styling.current_page_style,
                ));
            } else {
                spans.push(Span::styled(
                    format!("{} ", page + 1),
                    self.config.styling.page_style,
                ));
            }
        }
        
        // Next link
        if self.current_page < self.max_page() {
            spans.push(Span::styled("Next >", self.config.styling.navigation_style));
        }
        
        Line::from(spans)
    }
    
    /// Render custom pagination style
    fn render_custom_style(&self, theme: &crate::tui::themes::Theme) -> Line<'static> {
        // Fallback to full style for now
        self.render_full_style(theme)
    }
    
    /// Get performance metrics
    pub fn metrics(&self) -> &PaginationMetrics {
        &self.metrics
    }
    
    /// Get pagination statistics
    pub fn stats(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();
        
        stats.insert("current_page".to_string(), serde_json::Value::from(self.current_page));
        stats.insert("total_pages".to_string(), serde_json::Value::from(self.total_pages()));
        stats.insert("page_size".to_string(), serde_json::Value::from(self.page_size));
        stats.insert("total_items".to_string(), serde_json::Value::from(self.total_items));
        stats.insert("cache_size".to_string(), serde_json::Value::from(self.page_cache.len()));
        stats.insert("total_navigations".to_string(), serde_json::Value::from(self.metrics.total_navigations));
        stats.insert("cache_hit_rate".to_string(), {
            let total_requests = self.metrics.cache_hits + self.metrics.cache_misses;
            if total_requests > 0 {
                serde_json::Value::from(self.metrics.cache_hits as f64 / total_requests as f64)
            } else {
                serde_json::Value::from(0.0)
            }
        });
        
        stats
    }
}

impl<T: ListItem> Default for PaginationManager<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::components::lists::SimpleListItem;
    
    #[test]
    fn test_pagination_manager_creation() {
        let manager: PaginationManager<SimpleListItem> = PaginationManager::new();
        assert_eq!(manager.current_page(), 0);
        assert_eq!(manager.total_pages(), 1);
    }
    
    #[test]
    fn test_page_navigation() {
        let mut manager: PaginationManager<SimpleListItem> = PaginationManager::new();
        manager.set_total_items(100);
        
        assert_eq!(manager.total_pages(), 5); // 20 items per page by default
        assert_eq!(manager.current_page(), 0);
        
        manager.next_page().unwrap();
        assert_eq!(manager.current_page(), 1);
        
        manager.previous_page().unwrap();
        assert_eq!(manager.current_page(), 0);
        
        manager.last_page().unwrap();
        assert_eq!(manager.current_page(), 4);
        
        manager.first_page().unwrap();
        assert_eq!(manager.current_page(), 0);
    }
    
    #[test]
    fn test_page_size_change() {
        let mut manager: PaginationManager<SimpleListItem> = PaginationManager::new();
        manager.set_total_items(100);
        
        assert_eq!(manager.total_pages(), 5); // 20 items per page
        
        manager.set_page_size(10).unwrap();
        assert_eq!(manager.page_size(), 10);
        assert_eq!(manager.total_pages(), 10);
    }
    
    #[test]
    fn test_page_ranges() {
        let mut manager: PaginationManager<SimpleListItem> = PaginationManager::new();
        manager.set_total_items(100);
        manager.set_page_size(10);
        
        let range = manager.current_page_range();
        assert_eq!(range, 0..10);
        
        manager.goto_page(5).unwrap();
        let range = manager.current_page_range();
        assert_eq!(range, 50..60);
        
        manager.last_page().unwrap();
        let range = manager.current_page_range();
        assert_eq!(range, 90..100); // Last page
    }
    
    #[test]
    fn test_goto_page_functionality() {
        let mut config = PaginationConfig::default();
        config.show_goto_page = true;
        let mut manager: PaginationManager<SimpleListItem> = PaginationManager::with_config(config);
        manager.set_total_items(100);

        manager.start_goto_page();
        manager.goto_page_input('5');
        manager.execute_goto_page().unwrap();

        assert_eq!(manager.current_page(), 4); // Page 5 is index 4
    }
    
    #[test]
    fn test_page_caching() {
        let mut manager = PaginationManager::new();
        
        let items = vec![
            SimpleListItem::from_text("1".to_string(), "Item 1".to_string()),
            SimpleListItem::from_text("2".to_string(), "Item 2".to_string()),
        ];
        
        manager.cache_page(0, items);
        
        let cached = manager.get_cached_page(0);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().len(), 2);
        
        // Test cache miss
        let not_cached = manager.get_cached_page(1);
        assert!(not_cached.is_none());
    }
}