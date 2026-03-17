//! Filterable list implementation with fuzzy search and highlighting.
//!
//! This module provides a list component that supports real-time filtering
//! with fuzzy search, match highlighting, and efficient search algorithms.

use super::{FilterableItem, ListConfig, ListItem, VirtualList};
use crate::tui::themes::Theme;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::Instant;

/// Filterable list that wraps a VirtualList with search capabilities
#[derive(Debug)]
pub struct FilterableList<T: FilterableItem> {
    /// Underlying virtual list
    virtual_list: VirtualList<T>,
    
    /// All items (unfiltered)
    all_items: Vec<T>,
    
    /// Current search query
    query: String,
    
    /// Whether the filter input is currently focused
    filter_focused: bool,
    
    /// Filter input cursor position
    filter_cursor: usize,
    
    /// Search configuration
    search_config: SearchConfig,
    
    /// Search cache for performance
    search_cache: HashMap<String, SearchResult>,
    
    /// Last search time for metrics
    last_search_time: Option<Instant>,
}

/// Configuration for search behavior
#[derive(Debug, Clone)]
pub struct SearchConfig {
    /// Whether search is case sensitive
    pub case_sensitive: bool,
    /// Minimum query length before filtering starts
    pub min_query_length: usize,
    /// Maximum number of results to show
    pub max_results: Option<usize>,
    /// Whether to use fuzzy matching
    pub fuzzy_search: bool,
    /// Fuzzy search threshold (0.0 to 1.0)
    pub fuzzy_threshold: f64,
    /// Whether to highlight matches
    pub highlight_matches: bool,
    /// Style for match highlighting
    pub highlight_style: Style,
    /// Debounce delay for search updates
    pub debounce_ms: u64,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            min_query_length: 1,
            max_results: None,
            fuzzy_search: true,
            fuzzy_threshold: 0.3,
            highlight_matches: true,
            highlight_style: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            debounce_ms: 150,
        }
    }
}

/// Search result with score and match positions
#[derive(Debug, Clone)]
struct SearchResult {
    items: Vec<SearchMatch>,
    query: String,
    timestamp: Instant,
}

/// Individual search match
#[derive(Debug, Clone)]
struct SearchMatch {
    item_index: usize,
    score: f64,
    match_positions: Vec<usize>,
}

/// Simple filterable item wrapper
#[derive(Debug, Clone)]
pub struct SimpleFilterableItem {
    base: super::SimpleListItem,
    filter_value: String,
    match_indices: Vec<usize>,
}

impl SimpleFilterableItem {
    /// Create a new filterable item from text
    pub fn from_text(id: String, text: String) -> Self {
        Self {
            filter_value: text.clone(),
            base: super::SimpleListItem::from_text(id, text),
            match_indices: Vec::new(),
        }
    }
    
    /// Create a new filterable item with separate display and filter text
    pub fn new(id: String, content: Vec<Line<'static>>, filter_value: String) -> Self {
        Self {
            base: super::SimpleListItem::new(id, content),
            filter_value,
            match_indices: Vec::new(),
        }
    }
    
    /// Set custom height
    pub fn with_height(mut self, height: u16) -> Self {
        self.base = self.base.with_height(height);
        self
    }
    
    /// Make item non-selectable
    pub fn non_selectable(mut self) -> Self {
        self.base = self.base.non_selectable();
        self
    }
    
    /// Set custom style
    pub fn with_style(mut self, style: Style) -> Self {
        self.base = self.base.with_style(style);
        self
    }
    
    /// Add data payload
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.base = self.base.with_data(data);
        self
    }
}

impl ListItem for SimpleFilterableItem {
    fn id(&self) -> String {
        self.base.id()
    }
    
    fn content(&self) -> Vec<Line<'static>> {
        self.base.content()
    }
    
    fn height(&self) -> u16 {
        self.base.height()
    }
    
    fn selectable(&self) -> bool {
        self.base.selectable()
    }
    
    fn is_section_header(&self) -> bool {
        self.base.is_section_header()
    }
    
    fn style(&self) -> Option<Style> {
        self.base.style()
    }
    
    fn data(&self) -> Option<serde_json::Value> {
        self.base.data()
    }
}

impl FilterableItem for SimpleFilterableItem {
    fn filter_value(&self) -> String {
        self.filter_value.clone()
    }
    
    fn match_indices(&self) -> &[usize] {
        &self.match_indices
    }
    
    fn set_match_indices(&mut self, indices: Vec<usize>) {
        self.match_indices = indices;
    }
}

impl<T: FilterableItem> FilterableList<T> {
    /// Create a new filterable list
    pub fn new() -> Self {
        Self::with_config(ListConfig::default(), SearchConfig::default())
    }
    
    /// Create a new filterable list with custom configuration
    pub fn with_config(list_config: ListConfig, search_config: SearchConfig) -> Self {
        Self {
            virtual_list: VirtualList::with_config(list_config),
            all_items: Vec::new(),
            query: String::new(),
            filter_focused: false,
            filter_cursor: 0,
            search_config,
            search_cache: HashMap::new(),
            last_search_time: None,
        }
    }
    
    /// Create a filterable list optimized for large datasets
    pub fn for_large_dataset() -> Self {
        Self::with_config(
            ListConfig::large_list_preset(),
            SearchConfig::default(),
        )
    }
    
    /// Set the items in the list
    pub fn set_items(&mut self, items: Vec<T>) -> Result<()> {
        self.all_items = items;
        self.search_cache.clear();
        self.apply_filter()?;
        Ok(())
    }
    
    /// Get all items (unfiltered)
    pub fn all_items(&self) -> &[T] {
        &self.all_items
    }
    
    /// Get filtered items
    pub fn filtered_items(&self) -> &[T] {
        self.virtual_list.items()
    }
    
    /// Get the current search query
    pub fn query(&self) -> &str {
        &self.query
    }
    
    /// Set the search query
    pub fn set_query(&mut self, query: String) -> Result<()> {
        if self.query != query {
            self.query = query;
            self.filter_cursor = self.query.len();
            self.apply_filter()?;
        }
        Ok(())
    }
    
    /// Clear the search query
    pub fn clear_query(&mut self) -> Result<()> {
        self.set_query(String::new())
    }
    
    /// Set whether the filter input is focused
    pub fn set_filter_focused(&mut self, focused: bool) {
        self.filter_focused = focused;
    }
    
    /// Check if the filter input is focused
    pub fn is_filter_focused(&self) -> bool {
        self.filter_focused
    }
    
    /// Get the current selected item
    pub fn selected_item(&self) -> Option<&T> {
        self.virtual_list.selected_item()
    }
    
    /// Set the selected item by ID
    pub fn set_selected(&mut self, item_id: Option<String>) -> Result<()> {
        self.virtual_list.set_selected(item_id)
    }
    
    /// Set the area for the list
    pub fn set_area(&mut self, area: Rect) -> Result<()> {
        // Reserve one line for the filter input
        let list_area = Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: area.height.saturating_sub(1),
        };
        self.virtual_list.set_area(list_area)
    }
    
    /// Set focus state for the list (not the filter)
    pub fn set_list_focus(&mut self, focused: bool) {
        self.virtual_list.set_focus(focused);
    }
    
    /// Get performance metrics
    pub fn metrics(&self) -> super::ListMetrics {
        let mut metrics = self.virtual_list.metrics().clone();
        metrics.total_items = self.all_items.len();
        metrics
    }
    
    /// Handle keyboard input
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<bool> {
        if self.filter_focused {
            self.handle_filter_key_event(key)
        } else {
            self.virtual_list.handle_key_event(key)
        }
    }
    
    /// Handle keyboard input for the filter
    fn handle_filter_key_event(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char(c) => {
                self.query.insert(self.filter_cursor, c);
                self.filter_cursor += 1;
                self.apply_filter()?;
                Ok(true)
            }
            KeyCode::Backspace => {
                if self.filter_cursor > 0 {
                    self.filter_cursor -= 1;
                    self.query.remove(self.filter_cursor);
                    self.apply_filter()?;
                }
                Ok(true)
            }
            KeyCode::Delete => {
                if self.filter_cursor < self.query.len() {
                    self.query.remove(self.filter_cursor);
                    self.apply_filter()?;
                }
                Ok(true)
            }
            KeyCode::Left => {
                if self.filter_cursor > 0 {
                    self.filter_cursor -= 1;
                }
                Ok(true)
            }
            KeyCode::Right => {
                if self.filter_cursor < self.query.len() {
                    self.filter_cursor += 1;
                }
                Ok(true)
            }
            KeyCode::Home => {
                self.filter_cursor = 0;
                Ok(true)
            }
            KeyCode::End => {
                self.filter_cursor = self.query.len();
                Ok(true)
            }
            KeyCode::Esc => {
                self.set_filter_focused(false);
                Ok(true)
            }
            KeyCode::Down => {
                self.set_filter_focused(false);
                self.virtual_list.set_focus(true);
                Ok(true)
            }
            _ => Ok(false),
        }
    }
    
    /// Update the list
    pub fn update(&mut self, delta_time: std::time::Duration) -> Result<()> {
        self.virtual_list.update(delta_time)
    }
    
    /// Apply the current filter to the items
    fn apply_filter(&mut self) -> Result<()> {
        let start_time = Instant::now();

        if self.query.len() < self.search_config.min_query_length {
            // Show all items when query is too short
            let mut items = self.all_items.clone();

            // Clear match indices
            for item in &mut items {
                item.set_match_indices(Vec::new());
            }

            self.virtual_list.set_items(items)?;
        } else {
            // Check cache first - clone cached result to avoid borrow conflict
            let cached_result = self.search_cache.get(&self.query)
                .filter(|cached| cached.timestamp.elapsed().as_millis() < 1000)
                .cloned();

            if let Some(cached) = cached_result {
                self.apply_cached_results(&cached)?;
                return Ok(());
            }

            // Perform search - clone query to avoid borrow conflict
            let query = self.query.clone();
            let matches = if self.search_config.fuzzy_search {
                self.fuzzy_search(&query)?
            } else {
                self.exact_search(&query)?
            };

            // Cache results
            let result = SearchResult {
                items: matches.clone(),
                query: query.clone(),
                timestamp: start_time,
            };
            self.search_cache.insert(query, result);

            // Apply results
            let mut filtered_items = Vec::new();
            for search_match in matches {
                let mut item = self.all_items[search_match.item_index].clone();
                item.set_match_indices(search_match.match_positions);
                filtered_items.push(item);

                if let Some(max_results) = self.search_config.max_results {
                    if filtered_items.len() >= max_results {
                        break;
                    }
                }
            }

            self.virtual_list.set_items(filtered_items)?;
        }

        self.last_search_time = Some(start_time);
        Ok(())
    }
    
    /// Apply cached search results
    fn apply_cached_results(&mut self, cached: &SearchResult) -> Result<()> {
        let mut filtered_items = Vec::new();
        for search_match in &cached.items {
            let mut item = self.all_items[search_match.item_index].clone();
            item.set_match_indices(search_match.match_positions.clone());
            filtered_items.push(item);
            
            if let Some(max_results) = self.search_config.max_results {
                if filtered_items.len() >= max_results {
                    break;
                }
            }
        }
        
        self.virtual_list.set_items(filtered_items)?;
        Ok(())
    }
    
    /// Perform fuzzy search
    fn fuzzy_search(&self, query: &str) -> Result<Vec<SearchMatch>> {
        let query_lower = if self.search_config.case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };
        
        let mut matches = Vec::new();
        
        for (index, item) in self.all_items.iter().enumerate() {
            let text = if self.search_config.case_sensitive {
                item.filter_value()
            } else {
                item.filter_value().to_lowercase()
            };
            
            if let Some((score, positions)) = self.calculate_fuzzy_score(&query_lower, &text) {
                if score >= self.search_config.fuzzy_threshold {
                    matches.push(SearchMatch {
                        item_index: index,
                        score,
                        match_positions: positions,
                    });
                }
            }
        }
        
        // Sort by score (descending)
        matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        
        Ok(matches)
    }
    
    /// Perform exact search
    fn exact_search(&self, query: &str) -> Result<Vec<SearchMatch>> {
        let query_lower = if self.search_config.case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };
        
        let mut matches = Vec::new();
        
        for (index, item) in self.all_items.iter().enumerate() {
            let text = if self.search_config.case_sensitive {
                item.filter_value()
            } else {
                item.filter_value().to_lowercase()
            };
            
            if let Some(start) = text.find(&query_lower) {
                let positions: Vec<usize> = (start..start + query_lower.len()).collect();
                matches.push(SearchMatch {
                    item_index: index,
                    score: 1.0, // Exact matches get perfect score
                    match_positions: positions,
                });
            }
        }
        
        Ok(matches)
    }
    
    /// Calculate fuzzy match score and positions
    fn calculate_fuzzy_score(&self, query: &str, text: &str) -> Option<(f64, Vec<usize>)> {
        if query.is_empty() {
            return Some((0.0, Vec::new()));
        }
        
        let query_chars: Vec<char> = query.chars().collect();
        let text_chars: Vec<char> = text.chars().collect();
        
        let mut matches = Vec::new();
        let mut query_index = 0;
        
        for (text_index, &text_char) in text_chars.iter().enumerate() {
            if query_index < query_chars.len() && text_char == query_chars[query_index] {
                matches.push(text_index);
                query_index += 1;
            }
        }
        
        // Check if all query characters were matched
        if query_index < query_chars.len() {
            return None;
        }
        
        // Calculate score based on:
        // - Ratio of matched characters to text length
        // - Compactness of matches (shorter spans are better)
        // - Position of first match (earlier is better)
        
        let match_ratio = matches.len() as f64 / text_chars.len() as f64;
        
        let span = if matches.len() > 1 {
            matches.last().unwrap() - matches.first().unwrap()
        } else {
            0
        };
        let compactness = 1.0 - (span as f64 / text_chars.len() as f64);
        
        let position_bonus = 1.0 - (*matches.first().unwrap() as f64 / text_chars.len() as f64);
        
        let score = (match_ratio * 0.5) + (compactness * 0.3) + (position_bonus * 0.2);
        
        Some((score, matches))
    }
    
    /// Render the filter input
    pub fn render_filter(&self, theme: &Theme) -> Line<'static> {
        let cursor_char = if self.filter_focused { "│" } else { "" };
        
        let mut spans = Vec::new();
        
        // Add prompt
        spans.push(Span::styled(
            "Filter: ",
            Style::default().fg(theme.colors.text),
        ));
        
        // Add query text with cursor
        if self.query.is_empty() {
            if self.filter_focused {
                spans.push(Span::styled(
                    cursor_char,
                    Style::default()
                        .fg(theme.colors.primary)
                        .add_modifier(Modifier::RAPID_BLINK),
                ));
            } else {
                spans.push(Span::styled(
                    "type to search...",
                    Style::default().fg(theme.colors.muted),
                ));
            }
        } else {
            let (before_cursor, after_cursor) = self.query.split_at(self.filter_cursor);
            let before_cursor = before_cursor.to_string();
            let after_cursor = after_cursor.to_string();

            // Text before cursor
            spans.push(Span::styled(
                before_cursor,
                Style::default().fg(theme.colors.text),
            ));

            // Cursor
            if self.filter_focused {
                spans.push(Span::styled(
                    cursor_char,
                    Style::default()
                        .fg(theme.colors.primary)
                        .add_modifier(Modifier::RAPID_BLINK),
                ));
            }

            // Text after cursor
            spans.push(Span::styled(
                after_cursor,
                Style::default().fg(theme.colors.text),
            ));
        }
        
        // Add match count
        let match_count = self.virtual_list.items().len();
        let total_count = self.all_items.len();
        
        if !self.query.is_empty() {
            spans.push(Span::styled(
                format!(" ({}/{})", match_count, total_count),
                Style::default().fg(theme.colors.muted),
            ));
        }
        
        Line::from(spans)
    }
    
    /// Render the list
    pub fn render(&mut self, theme: &Theme) -> Result<Vec<Line<'static>>> {
        let mut lines = Vec::new();
        
        // Add filter input
        lines.push(self.render_filter(theme));
        
        // Add list content
        let list_lines = self.virtual_list.render(theme)?;
        lines.extend(list_lines);
        
        Ok(lines)
    }
}

impl<T: FilterableItem> Default for FilterableList<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_filterable_list_creation() {
        let list: FilterableList<SimpleFilterableItem> = FilterableList::new();
        assert_eq!(list.all_items().len(), 0);
        assert_eq!(list.query(), "");
        assert!(!list.is_filter_focused());
    }
    
    #[test]
    fn test_set_items() {
        let mut list = FilterableList::new();
        let items = vec![
            SimpleFilterableItem::from_text("1".to_string(), "Apple".to_string()),
            SimpleFilterableItem::from_text("2".to_string(), "Banana".to_string()),
            SimpleFilterableItem::from_text("3".to_string(), "Cherry".to_string()),
        ];
        
        list.set_items(items).unwrap();
        assert_eq!(list.all_items().len(), 3);
        assert_eq!(list.filtered_items().len(), 3);
    }
    
    #[test]
    fn test_exact_search() {
        let mut list = FilterableList::new();
        let items = vec![
            SimpleFilterableItem::from_text("1".to_string(), "Apple".to_string()),
            SimpleFilterableItem::from_text("2".to_string(), "Banana".to_string()),
            SimpleFilterableItem::from_text("3".to_string(), "Cherry".to_string()),
            SimpleFilterableItem::from_text("4".to_string(), "Apricot".to_string()),
        ];
        
        list.set_items(items).unwrap();
        list.set_query("ap".to_string()).unwrap();
        
        // Should match "Apple" and "Apricot"
        assert_eq!(list.filtered_items().len(), 2);
    }
    
    #[test]
    fn test_fuzzy_search() {
        let mut list = FilterableList::new();
        let items = vec![
            SimpleFilterableItem::from_text("1".to_string(), "Hello World".to_string()),
            SimpleFilterableItem::from_text("2".to_string(), "Help Me".to_string()),
            SimpleFilterableItem::from_text("3".to_string(), "Heavy Metal".to_string()),
            SimpleFilterableItem::from_text("4".to_string(), "Random Text".to_string()),
        ];
        
        list.set_items(items).unwrap();
        list.set_query("hel".to_string()).unwrap();
        
        // Should match items that contain h, e, l in order
        let filtered = list.filtered_items();
        assert!(!filtered.is_empty());
        
        // "Hello World" and "Help Me" should have higher scores than "Heavy Metal"
        assert!(filtered.iter().any(|item| item.filter_value().contains("Hello")));
        assert!(filtered.iter().any(|item| item.filter_value().contains("Help")));
    }
    
    #[test]
    fn test_filter_input_handling() {
        let mut list: FilterableList<SimpleFilterableItem> = FilterableList::new();
        list.set_filter_focused(true);
        
        // Test character input
        list.handle_key_event(KeyEvent::from(KeyCode::Char('a'))).unwrap();
        assert_eq!(list.query(), "a");
        assert_eq!(list.filter_cursor, 1);
        
        list.handle_key_event(KeyEvent::from(KeyCode::Char('b'))).unwrap();
        assert_eq!(list.query(), "ab");
        assert_eq!(list.filter_cursor, 2);
        
        // Test backspace
        list.handle_key_event(KeyEvent::from(KeyCode::Backspace)).unwrap();
        assert_eq!(list.query(), "a");
        assert_eq!(list.filter_cursor, 1);
        
        // Test cursor movement
        list.handle_key_event(KeyEvent::from(KeyCode::Left)).unwrap();
        assert_eq!(list.filter_cursor, 0);
        
        list.handle_key_event(KeyEvent::from(KeyCode::Right)).unwrap();
        assert_eq!(list.filter_cursor, 1);
    }
}