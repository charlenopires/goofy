//! Virtual scrolling list implementation for high-performance rendering of large datasets.
//!
//! This module provides a virtual list component that only renders visible items,
//! enabling smooth performance with lists containing hundreds of thousands of items.

use super::{Direction, ListConfig, ListEvent, ListItem, ListMetrics};
use crate::tui::themes::Theme;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use ratatui::{
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
};
use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, Instant};

/// Virtual list component that efficiently handles large datasets
pub struct VirtualList<T: ListItem> {
    /// Configuration settings
    config: ListConfig,
    
    /// All items in the list
    items: Vec<T>,
    
    /// Currently selected item ID
    selected_id: Option<String>,
    
    /// Current scroll offset (in lines)
    scroll_offset: usize,
    
    /// Viewport dimensions
    area: Rect,
    
    /// Direction of the list (forward/backward)
    direction: Direction,
    
    /// Whether the list is focused
    focused: bool,
    
    /// Cached rendered items for performance
    rendered_cache: HashMap<String, RenderedItem>,
    
    /// Virtual scrolling state
    virtual_state: VirtualState,
    
    /// Performance metrics
    metrics: ListMetrics,
    
    /// Event listeners
    event_listeners: Vec<Box<dyn Fn(ListEvent<T>) + Send + Sync>>,
    
    /// Animation state for smooth scrolling
    scroll_animation: Option<ScrollAnimation>,
}

impl<T: ListItem> fmt::Debug for VirtualList<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VirtualList")
            .field("config", &self.config)
            .field("items_count", &self.items.len())
            .field("selected_id", &self.selected_id)
            .field("scroll_offset", &self.scroll_offset)
            .field("area", &self.area)
            .field("direction", &self.direction)
            .field("focused", &self.focused)
            .field("virtual_state", &self.virtual_state)
            .field("metrics", &self.metrics)
            .field("event_listeners", &format!("[{} listeners]", self.event_listeners.len()))
            .finish()
    }
}

/// Cached rendered item
#[derive(Debug, Clone)]
struct RenderedItem {
    id: String,
    lines: Vec<Line<'static>>,
    height: u16,
    last_rendered: Instant,
}

/// Virtual scrolling state tracking
#[derive(Debug, Clone)]
struct VirtualState {
    /// Index of first visible item
    first_visible_index: usize,
    /// Index of last visible item
    last_visible_index: usize,
    /// Total height of all items
    total_height: usize,
    /// Height of items before first visible
    height_before_visible: usize,
    /// Height of items after last visible
    height_after_visible: usize,
    /// Dirty flag for recalculation
    needs_recalc: bool,
}

/// Smooth scrolling animation state
#[derive(Debug, Clone)]
struct ScrollAnimation {
    start_offset: usize,
    target_offset: usize,
    start_time: Instant,
    duration: Duration,
}

impl<T: ListItem> VirtualList<T> {
    /// Create a new virtual list with default configuration
    pub fn new() -> Self {
        Self::with_config(ListConfig::default())
    }
    
    /// Create a new virtual list with custom configuration
    pub fn with_config(config: ListConfig) -> Self {
        Self {
            config,
            items: Vec::new(),
            selected_id: None,
            scroll_offset: 0,
            area: Rect::default(),
            direction: Direction::Forward,
            focused: true,
            rendered_cache: HashMap::new(),
            virtual_state: VirtualState {
                first_visible_index: 0,
                last_visible_index: 0,
                total_height: 0,
                height_before_visible: 0,
                height_after_visible: 0,
                needs_recalc: true,
            },
            metrics: ListMetrics::default(),
            event_listeners: Vec::new(),
            scroll_animation: None,
        }
    }
    
    /// Create a virtual list optimized for large datasets
    pub fn for_large_dataset() -> Self {
        Self::with_config(ListConfig::large_list_preset())
    }
    
    /// Create a virtual list optimized for chat/messages
    pub fn for_chat() -> Self {
        Self::with_config(ListConfig::chat_list_preset())
    }
    
    /// Create a virtual list optimized for file browsing
    pub fn for_files() -> Self {
        Self::with_config(ListConfig::file_list_preset())
    }
    
    /// Set the items in the list
    pub fn set_items(&mut self, items: Vec<T>) -> Result<()> {
        self.items = items;
        self.virtual_state.needs_recalc = true;
        self.rendered_cache.clear();
        
        // Update selection if current selection is no longer valid
        if let Some(selected_id) = &self.selected_id {
            if !self.items.iter().any(|item| item.id() == *selected_id) {
                self.selected_id = None;
                self.select_first_selectable();
            }
        } else {
            self.select_first_selectable();
        }
        
        self.recalculate_virtual_state()?;
        Ok(())
    }
    
    /// Get all items
    pub fn items(&self) -> &[T] {
        &self.items
    }
    
    /// Get the currently selected item
    pub fn selected_item(&self) -> Option<&T> {
        self.selected_id.as_ref()
            .and_then(|id| self.items.iter().find(|item| item.id() == *id))
    }
    
    /// Get the currently selected item ID
    pub fn selected_id(&self) -> Option<&String> {
        self.selected_id.as_ref()
    }
    
    /// Set the selected item by ID
    pub fn set_selected(&mut self, item_id: Option<String>) -> Result<()> {
        let previous = self.selected_id.clone();
        
        if let Some(id) = &item_id {
            if self.items.iter().any(|item| item.id() == *id && item.selectable()) {
                self.selected_id = Some(id.clone());
                self.scroll_to_selected()?;
            }
        } else {
            self.selected_id = None;
        }
        
        // Emit selection changed event
        if previous != self.selected_id {
            self.emit_event(ListEvent::SelectionChanged {
                previous,
                current: self.selected_id.clone(),
            });
        }
        
        Ok(())
    }
    
    /// Add an event listener
    pub fn add_event_listener<F>(&mut self, listener: F)
    where
        F: Fn(ListEvent<T>) + Send + Sync + 'static,
    {
        self.event_listeners.push(Box::new(listener));
    }
    
    /// Set the area for the list
    pub fn set_area(&mut self, area: Rect) -> Result<()> {
        if self.area != area {
            self.area = area;
            self.virtual_state.needs_recalc = true;
            self.recalculate_virtual_state()?;
        }
        Ok(())
    }
    
    /// Set the direction of the list
    pub fn set_direction(&mut self, direction: Direction) -> Result<()> {
        if self.direction != direction {
            self.direction = direction;
            self.virtual_state.needs_recalc = true;
            self.recalculate_virtual_state()?;
        }
        Ok(())
    }
    
    /// Set focus state
    pub fn set_focus(&mut self, focused: bool) {
        self.focused = focused;
    }
    
    /// Check if the list is focused
    pub fn is_focused(&self) -> bool {
        self.focused
    }
    
    /// Get performance metrics
    pub fn metrics(&self) -> &ListMetrics {
        &self.metrics
    }
    
    /// Select the next item
    pub fn select_next(&mut self) -> Result<()> {
        if let Some(current_id) = &self.selected_id {
            if let Some(current_index) = self.find_item_index(current_id) {
                let next_index = self.find_next_selectable_index(current_index);
                if let Some(index) = next_index {
                    let next_id = self.items[index].id();
                    self.set_selected(Some(next_id))?;
                } else if self.config.wrap_navigation {
                    self.select_first_selectable();
                }
            }
        } else {
            self.select_first_selectable();
        }
        Ok(())
    }
    
    /// Select the previous item
    pub fn select_previous(&mut self) -> Result<()> {
        if let Some(current_id) = &self.selected_id {
            if let Some(current_index) = self.find_item_index(current_id) {
                let prev_index = self.find_previous_selectable_index(current_index);
                if let Some(index) = prev_index {
                    let prev_id = self.items[index].id();
                    self.set_selected(Some(prev_id))?;
                } else if self.config.wrap_navigation {
                    self.select_last_selectable();
                }
            }
        } else {
            self.select_last_selectable();
        }
        Ok(())
    }
    
    /// Scroll down by specified number of lines
    pub fn scroll_down(&mut self, lines: usize) -> Result<()> {
        let new_offset = self.scroll_offset.saturating_add(lines);
        self.set_scroll_offset(new_offset)
    }
    
    /// Scroll up by specified number of lines
    pub fn scroll_up(&mut self, lines: usize) -> Result<()> {
        let new_offset = self.scroll_offset.saturating_sub(lines);
        self.set_scroll_offset(new_offset)
    }
    
    /// Scroll to the top of the list
    pub fn scroll_to_top(&mut self) -> Result<()> {
        self.set_scroll_offset(0)
    }
    
    /// Scroll to the bottom of the list
    pub fn scroll_to_bottom(&mut self) -> Result<()> {
        let max_offset = self.virtual_state.total_height.saturating_sub(self.area.height as usize);
        self.set_scroll_offset(max_offset)
    }
    
    /// Page down
    pub fn page_down(&mut self) -> Result<()> {
        let page_size = self.config.page_size.unwrap_or(self.area.height as usize);
        self.scroll_down(page_size)
    }
    
    /// Page up
    pub fn page_up(&mut self) -> Result<()> {
        let page_size = self.config.page_size.unwrap_or(self.area.height as usize);
        self.scroll_up(page_size)
    }
    
    /// Handle keyboard input
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<bool> {
        if !self.focused {
            return Ok(false);
        }
        
        match key.code {
            KeyCode::Up => {
                self.select_previous()?;
                Ok(true)
            }
            KeyCode::Down => {
                self.select_next()?;
                Ok(true)
            }
            KeyCode::PageUp => {
                self.page_up()?;
                Ok(true)
            }
            KeyCode::PageDown => {
                self.page_down()?;
                Ok(true)
            }
            KeyCode::Home => {
                self.scroll_to_top()?;
                self.select_first_selectable();
                Ok(true)
            }
            KeyCode::End => {
                self.scroll_to_bottom()?;
                self.select_last_selectable();
                Ok(true)
            }
            KeyCode::Enter => {
                if let Some(selected) = self.selected_item().cloned() {
                    self.emit_event(ListEvent::ItemActivated {
                        item_id: selected.id(),
                        item: selected,
                    });
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
    
    /// Handle mouse input
    pub fn handle_mouse_event(&mut self, event: MouseEvent) -> Result<bool> {
        if !self.config.enable_mouse || !self.focused {
            return Ok(false);
        }
        
        match event.kind {
            MouseEventKind::ScrollUp => {
                self.scroll_up(3)?;
                Ok(true)
            }
            MouseEventKind::ScrollDown => {
                self.scroll_down(3)?;
                Ok(true)
            }
            MouseEventKind::Down(_) => {
                // Handle item selection by click position
                if let Some(item_id) = self.get_item_at_position(event.row, event.column) {
                    self.set_selected(Some(item_id))?;
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
    
    /// Update animations and state
    pub fn update(&mut self, delta_time: Duration) -> Result<()> {
        // Update scroll animation
        if let Some(animation) = &self.scroll_animation {
            let elapsed = animation.start_time.elapsed();
            if elapsed >= animation.duration {
                // Animation complete
                self.scroll_offset = animation.target_offset;
                self.scroll_animation = None;
            } else {
                // Interpolate scroll position
                let progress = elapsed.as_secs_f64() / animation.duration.as_secs_f64();
                let eased_progress = Self::ease_out_cubic(progress);
                let current_offset = animation.start_offset as f64 + 
                    (animation.target_offset as f64 - animation.start_offset as f64) * eased_progress;
                self.scroll_offset = current_offset as usize;
            }
            
            self.virtual_state.needs_recalc = true;
        }
        
        // Clean up old cache entries
        self.cleanup_cache();
        
        // Recalculate virtual state if needed
        if self.virtual_state.needs_recalc {
            self.recalculate_virtual_state()?;
        }
        
        Ok(())
    }
    
    /// Render the list
    pub fn render(&mut self, theme: &Theme) -> Result<Vec<Line<'static>>> {
        let start_time = Instant::now();
        
        if self.area.width == 0 || self.area.height == 0 {
            return Ok(Vec::new());
        }
        
        // Ensure virtual state is up to date
        if self.virtual_state.needs_recalc {
            self.recalculate_virtual_state()?;
        }
        
        let mut lines = Vec::new();
        let viewport_height = self.area.height as usize;

        // Render visible items
        let visible_range = self.get_visible_item_range();
        let range_start = visible_range.start;
        let range_end = visible_range.end;

        // Collect item data needed for rendering to avoid borrow conflicts
        let items_to_render: Vec<(usize, T, bool)> = (range_start..=range_end.min(self.items.len().saturating_sub(1)))
            .map(|index| {
                let item = self.items[index].clone();
                let is_selected = self.selected_id.as_ref() == Some(&item.id());
                (index, item, is_selected)
            })
            .collect();

        for (index, item, is_selected) in &items_to_render {
            let rendered_lines = self.render_item_lines(item, *is_selected, theme);

            // Add gap if configured
            if *index > range_start && self.config.item_gap > 0 {
                for _ in 0..self.config.item_gap {
                    lines.push(Line::from(""));
                }
            }

            lines.extend(rendered_lines);

            // Stop if we've filled the viewport
            if lines.len() >= viewport_height {
                lines.truncate(viewport_height);
                break;
            }
        }

        // Fill remaining space if needed
        while lines.len() < viewport_height {
            lines.push(Line::from(""));
        }

        // Update metrics
        self.metrics.rendered_items = range_end.saturating_sub(range_start) + 1;
        self.metrics.total_items = self.items.len();
        self.metrics.visible_items = self.metrics.rendered_items;
        self.metrics.scroll_offset = self.scroll_offset;
        self.metrics.render_time_us = start_time.elapsed().as_micros() as u64;
        self.metrics.memory_usage_bytes = self.estimate_memory_usage();
        
        Ok(lines)
    }
    
    /// Get the range of visible items
    fn get_visible_item_range(&self) -> std::ops::Range<usize> {
        let start = self.virtual_state.first_visible_index;
        let end = self.virtual_state.last_visible_index;
        start..end.saturating_add(1)
    }
    
    /// Render an item to styled lines
    fn render_item_lines(&self, item: &T, is_selected: bool, theme: &Theme) -> Vec<Line<'static>> {
        let content_lines = item.content();
        let mut rendered_lines = Vec::new();

        for line in content_lines {
            let mut styled_line = line;

            // Apply selection styling
            if is_selected {
                let spans: Vec<Span> = styled_line.spans.into_iter()
                    .map(|span| {
                        let mut style = span.style;
                        style = style.bg(theme.colors.selection);
                        if style.fg.is_none() {
                            style = style.fg(theme.colors.text);
                        }
                        style = style.add_modifier(Modifier::BOLD);
                        Span::styled(span.content, style)
                    })
                    .collect();
                styled_line = Line::from(spans);
            }

            // Apply item-specific styling
            if let Some(item_style) = item.style() {
                let spans: Vec<Span> = styled_line.spans.into_iter()
                    .map(|span| {
                        let mut style = span.style;
                        if let Some(fg) = item_style.fg {
                            style = style.fg(fg);
                        }
                        if let Some(bg) = item_style.bg {
                            style = style.bg(bg);
                        }
                        style = style.add_modifier(item_style.add_modifier);
                        style = style.remove_modifier(item_style.sub_modifier);
                        Span::styled(span.content, style)
                    })
                    .collect();
                styled_line = Line::from(spans);
            }

            rendered_lines.push(styled_line);
        }

        rendered_lines
    }
    
    /// Recalculate virtual scrolling state
    fn recalculate_virtual_state(&mut self) -> Result<()> {
        let viewport_height = self.area.height as usize;
        
        if self.items.is_empty() {
            self.virtual_state = VirtualState {
                first_visible_index: 0,
                last_visible_index: 0,
                total_height: 0,
                height_before_visible: 0,
                height_after_visible: 0,
                needs_recalc: false,
            };
            return Ok(());
        }
        
        // Calculate total height
        let mut total_height = 0;
        for (i, item) in self.items.iter().enumerate() {
            total_height += item.height() as usize;
            if i > 0 {
                total_height += self.config.item_gap as usize;
            }
        }
        
        // Find visible range
        let mut current_height = 0;
        let mut first_visible_index = 0;
        let mut last_visible_index = 0;
        let mut height_before_visible = 0;
        let mut found_first = false;
        
        for (i, item) in self.items.iter().enumerate() {
            let item_height = item.height() as usize;
            let gap_height = if i > 0 { self.config.item_gap as usize } else { 0 };
            
            if !found_first && current_height + item_height > self.scroll_offset {
                first_visible_index = i;
                height_before_visible = current_height;
                found_first = true;
            }
            
            if found_first {
                if current_height - height_before_visible >= viewport_height {
                    last_visible_index = i.saturating_sub(1);
                    break;
                }
                last_visible_index = i;
            }
            
            current_height += gap_height + item_height;
        }
        
        let height_after_visible = total_height.saturating_sub(
            height_before_visible + 
            self.items[first_visible_index..=last_visible_index]
                .iter()
                .map(|item| item.height() as usize)
                .sum::<usize>()
        );
        
        self.virtual_state = VirtualState {
            first_visible_index,
            last_visible_index,
            total_height,
            height_before_visible,
            height_after_visible,
            needs_recalc: false,
        };
        
        Ok(())
    }
    
    /// Set scroll offset with optional animation
    fn set_scroll_offset(&mut self, offset: usize) -> Result<()> {
        let max_offset = self.virtual_state.total_height.saturating_sub(self.area.height as usize);
        let clamped_offset = offset.min(max_offset);
        
        if self.config.smooth_scrolling && (clamped_offset as i32 - self.scroll_offset as i32).abs() > 10 {
            // Start smooth scrolling animation
            self.scroll_animation = Some(ScrollAnimation {
                start_offset: self.scroll_offset,
                target_offset: clamped_offset,
                start_time: Instant::now(),
                duration: Duration::from_millis(200),
            });
        } else {
            // Immediate scroll
            self.scroll_offset = clamped_offset;
            self.virtual_state.needs_recalc = true;
        }
        
        Ok(())
    }
    
    /// Scroll to the currently selected item
    fn scroll_to_selected(&mut self) -> Result<()> {
        if let Some(selected_id) = &self.selected_id {
            if let Some(index) = self.find_item_index(selected_id) {
                let item_top = self.get_item_top_position(index);
                let item_height = self.items[index].height() as usize;
                let viewport_height = self.area.height as usize;
                
                // Check if item is already visible
                if item_top >= self.scroll_offset && 
                   item_top + item_height <= self.scroll_offset + viewport_height {
                    return Ok(());
                }
                
                // Scroll to make item visible
                if item_top < self.scroll_offset {
                    // Item is above viewport
                    self.set_scroll_offset(item_top)?;
                } else {
                    // Item is below viewport
                    let new_offset = item_top + item_height - viewport_height;
                    self.set_scroll_offset(new_offset)?;
                }
            }
        }
        Ok(())
    }
    
    /// Get the top position of an item in the virtual space
    fn get_item_top_position(&self, index: usize) -> usize {
        let mut position = 0;
        for i in 0..index {
            position += self.items[i].height() as usize;
            if i > 0 {
                position += self.config.item_gap as usize;
            }
        }
        position
    }
    
    /// Find the index of an item by ID
    fn find_item_index(&self, item_id: &str) -> Option<usize> {
        self.items.iter().position(|item| item.id() == item_id)
    }
    
    /// Find the next selectable item index
    fn find_next_selectable_index(&self, current: usize) -> Option<usize> {
        for i in (current + 1)..self.items.len() {
            if self.items[i].selectable() {
                return Some(i);
            }
        }
        None
    }
    
    /// Find the previous selectable item index
    fn find_previous_selectable_index(&self, current: usize) -> Option<usize> {
        if current == 0 {
            return None;
        }
        for i in (0..current).rev() {
            if self.items[i].selectable() {
                return Some(i);
            }
        }
        None
    }
    
    /// Select the first selectable item
    fn select_first_selectable(&mut self) {
        for item in &self.items {
            if item.selectable() {
                self.selected_id = Some(item.id());
                break;
            }
        }
    }
    
    /// Select the last selectable item
    fn select_last_selectable(&mut self) {
        for item in self.items.iter().rev() {
            if item.selectable() {
                self.selected_id = Some(item.id());
                break;
            }
        }
    }
    
    /// Get the item at a specific screen position
    fn get_item_at_position(&self, row: u16, _column: u16) -> Option<String> {
        let local_row = row.saturating_sub(self.area.y) as usize;
        
        let mut current_height = 0;
        let visible_range = self.get_visible_item_range();
        let range_start = visible_range.start;

        for index in visible_range {
            if index >= self.items.len() {
                break;
            }

            let item = &self.items[index];
            let item_height = item.height() as usize;

            if local_row >= current_height && local_row < current_height + item_height {
                return Some(item.id());
            }

            current_height += item_height;
            if index > range_start && self.config.item_gap > 0 {
                current_height += self.config.item_gap as usize;
            }
        }
        
        None
    }
    
    /// Emit an event to all listeners
    fn emit_event(&self, event: ListEvent<T>) {
        for listener in &self.event_listeners {
            listener(event.clone());
        }
    }
    
    /// Clean up old cache entries
    fn cleanup_cache(&mut self) {
        let cutoff = Instant::now() - Duration::from_secs(10);
        self.rendered_cache.retain(|_, item| item.last_rendered > cutoff);
    }
    
    /// Estimate memory usage in bytes
    fn estimate_memory_usage(&self) -> usize {
        let items_size = self.items.len() * std::mem::size_of::<T>();
        let cache_size = self.rendered_cache.len() * 1024; // Rough estimate
        items_size + cache_size
    }
    
    /// Easing function for smooth scrolling
    fn ease_out_cubic(t: f64) -> f64 {
        1.0 - (1.0 - t).powi(3)
    }
}

impl<T: ListItem> Default for VirtualList<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::components::lists::SimpleListItem;
    
    #[test]
    fn test_virtual_list_creation() {
        let list: VirtualList<SimpleListItem> = VirtualList::new();
        assert_eq!(list.items().len(), 0);
        assert!(list.selected_id().is_none());
        assert_eq!(list.scroll_offset, 0);
    }
    
    #[test]
    fn test_set_items() {
        let mut list = VirtualList::new();
        let items = vec![
            SimpleListItem::from_text("1".to_string(), "Item 1".to_string()),
            SimpleListItem::from_text("2".to_string(), "Item 2".to_string()),
            SimpleListItem::from_text("3".to_string(), "Item 3".to_string()),
        ];
        
        list.set_items(items).unwrap();
        assert_eq!(list.items().len(), 3);
        assert_eq!(list.selected_id(), Some(&"1".to_string()));
    }
    
    #[test]
    fn test_selection_navigation() {
        let mut list = VirtualList::new();
        let items = vec![
            SimpleListItem::from_text("1".to_string(), "Item 1".to_string()),
            SimpleListItem::from_text("2".to_string(), "Item 2".to_string()),
            SimpleListItem::from_text("3".to_string(), "Item 3".to_string()),
        ];
        
        list.set_items(items).unwrap();
        
        // Test next selection
        list.select_next().unwrap();
        assert_eq!(list.selected_id(), Some(&"2".to_string()));
        
        list.select_next().unwrap();
        assert_eq!(list.selected_id(), Some(&"3".to_string()));
        
        // Test previous selection
        list.select_previous().unwrap();
        assert_eq!(list.selected_id(), Some(&"2".to_string()));
        
        list.select_previous().unwrap();
        assert_eq!(list.selected_id(), Some(&"1".to_string()));
    }
    
    #[test]
    fn test_scrolling() {
        let mut list = VirtualList::new();
        list.set_area(Rect::new(0, 0, 50, 10)).unwrap();
        
        let items: Vec<SimpleListItem> = (0..100)
            .map(|i| SimpleListItem::from_text(i.to_string(), format!("Item {}", i)))
            .collect();
        
        list.set_items(items).unwrap();
        
        // Test scrolling down
        list.scroll_down(5).unwrap();
        assert_eq!(list.scroll_offset, 5);
        
        // Test scrolling up
        list.scroll_up(3).unwrap();
        assert_eq!(list.scroll_offset, 2);
        
        // Test scroll to top
        list.scroll_to_top().unwrap();
        assert_eq!(list.scroll_offset, 0);
    }
}