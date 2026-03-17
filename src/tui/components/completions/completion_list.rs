//! Dropdown/popup completion display component

use super::{CompletionItem, CompletionEvent, CompletionMessage, MAX_POPUP_HEIGHT};
use crate::tui::{
    components::{Component, ComponentState},
    themes::Theme,
    Frame,
};
use anyhow::Result;
use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame as RatatuiFrame,
};
use std::cmp::min;
use tokio::sync::mpsc;
use tracing::debug;

/// Completion list component for displaying completion options
pub struct CompletionList {
    state: ComponentState,
    items: Vec<CompletionItem>,
    list_state: ListState,
    visible: bool,
    position: Rect,
    pub query: String,
    event_sender: Option<mpsc::UnboundedSender<CompletionEvent>>,
    selected_index: usize,
    scroll_offset: usize,
    max_visible_items: usize,
    show_descriptions: bool,
    highlight_matches: bool,
}

impl CompletionList {
    /// Create a new completion list
    pub fn new() -> Self {
        Self {
            state: ComponentState::new(),
            items: Vec::new(),
            list_state: ListState::default(),
            visible: false,
            position: Rect::default(),
            query: String::new(),
            event_sender: None,
            selected_index: 0,
            scroll_offset: 0,
            max_visible_items: MAX_POPUP_HEIGHT as usize,
            show_descriptions: true,
            highlight_matches: true,
        }
    }

    /// Set event sender for completion events
    pub fn with_event_sender(mut self, sender: mpsc::UnboundedSender<CompletionEvent>) -> Self {
        self.event_sender = Some(sender);
        self
    }

    /// Enable or disable description display
    pub fn with_descriptions(mut self, show: bool) -> Self {
        self.show_descriptions = show;
        self
    }

    /// Enable or disable match highlighting
    pub fn with_highlight_matches(mut self, highlight: bool) -> Self {
        self.highlight_matches = highlight;
        self
    }

    /// Set maximum visible items
    pub fn with_max_visible_items(mut self, max: usize) -> Self {
        self.max_visible_items = max;
        self
    }

    /// Open the completion list with items
    pub fn open(&mut self, items: Vec<CompletionItem>, position: Rect, query: String) {
        debug!("Opening completion list with {} items at {:?}", items.len(), position);
        
        self.items = items;
        self.position = position;
        self.query = query;
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.visible = true;
        
        // Update list state
        if !self.items.is_empty() {
            self.list_state.select(Some(0));
        }

        // Send opened event
        if let Some(ref sender) = self.event_sender {
            let _ = sender.send(CompletionEvent::Opened {
                items: self.items.clone(),
                x: position.x,
                y: position.y,
            });
        }
    }

    /// Close the completion list
    pub fn close(&mut self) {
        debug!("Closing completion list");
        
        self.visible = false;
        self.items.clear();
        self.list_state.select(None);
        
        // Send closed event
        if let Some(ref sender) = self.event_sender {
            let _ = sender.send(CompletionEvent::Closed);
        }
    }

    /// Filter items with new query
    pub fn filter(&mut self, items: Vec<CompletionItem>, query: String) {
        debug!("Filtering completion list with {} items, query: '{}'", items.len(), query);
        
        self.items = items;
        self.query = query;
        self.selected_index = 0;
        self.scroll_offset = 0;
        
        if self.items.is_empty() {
            self.close();
        } else {
            self.list_state.select(Some(0));
            
            // Send filtered event
            if let Some(ref sender) = self.event_sender {
                let _ = sender.send(CompletionEvent::Filtered {
                    query: self.query.clone(),
                    items: self.items.clone(),
                });
            }
        }
    }

    /// Move selection up
    pub fn move_up(&mut self) {
        if self.items.is_empty() {
            return;
        }

        if self.selected_index > 0 {
            self.selected_index -= 1;
        } else {
            self.selected_index = self.items.len() - 1;
        }

        self.update_scroll();
        self.list_state.select(Some(self.selected_index));
    }

    /// Move selection down
    pub fn move_down(&mut self) {
        if self.items.is_empty() {
            return;
        }

        if self.selected_index < self.items.len() - 1 {
            self.selected_index += 1;
        } else {
            self.selected_index = 0;
        }

        self.update_scroll();
        self.list_state.select(Some(self.selected_index));
    }

    /// Update scroll offset based on selection
    fn update_scroll(&mut self) {
        let visible_items = min(self.max_visible_items, self.items.len());
        
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + visible_items {
            self.scroll_offset = self.selected_index - visible_items + 1;
        }
    }

    /// Get currently selected item
    pub fn selected_item(&self) -> Option<&CompletionItem> {
        self.items.get(self.selected_index)
    }

    /// Select the current item
    pub fn select_current(&mut self, insert: bool) {
        if let Some(item) = self.selected_item() {
            let selected_item = item.clone();
            
            // Send selection event
            if let Some(ref sender) = self.event_sender {
                let _ = sender.send(CompletionEvent::Selected {
                    item: selected_item,
                    insert,
                });
            }
            
            if !insert {
                self.close();
            }
        }
    }

    /// Reposition the completion list
    pub fn reposition(&mut self, position: Rect) {
        self.position = position;
        
        // Send repositioned event
        if let Some(ref sender) = self.event_sender {
            let _ = sender.send(CompletionEvent::Repositioned {
                x: position.x,
                y: position.y,
            });
        }
    }

    /// Calculate the display area for the completion list
    fn calculate_display_area(&self, area: Rect) -> Rect {
        let items_count = min(self.items.len(), self.max_visible_items);
        let height = min(items_count as u16 + 2, MAX_POPUP_HEIGHT); // +2 for borders
        let width = self.calculate_width();

        let x = min(self.position.x, area.width.saturating_sub(width));
        let y = if self.position.y + height > area.height {
            // Show above if not enough space below
            self.position.y.saturating_sub(height)
        } else {
            self.position.y
        };

        Rect {
            x,
            y,
            width,
            height,
        }
    }

    /// Calculate the optimal width for the completion list
    fn calculate_width(&self) -> u16 {
        let mut max_width = 20u16; // Minimum width

        for item in &self.items {
            let item_width = if self.show_descriptions && item.description.is_some() {
                item.title.len() + item.description.as_ref().unwrap().len() + 3 // " - "
            } else {
                item.title.len()
            };
            max_width = max_width.max(item_width as u16);
        }

        min(max_width + 4, 80) // +4 for borders and padding, max 80 chars
    }

    /// Create list items for rendering
    fn create_list_items(&self, theme: &Theme) -> Vec<ListItem<'static>> {
        let visible_items = self.items
            .iter()
            .skip(self.scroll_offset)
            .take(self.max_visible_items);

        visible_items
            .enumerate()
            .map(|(i, item)| {
                let is_selected = self.scroll_offset + i == self.selected_index;
                self.create_list_item(item, is_selected, theme)
            })
            .collect()
    }

    /// Create a single list item
    fn create_list_item(&self, item: &CompletionItem, is_selected: bool, theme: &Theme) -> ListItem<'static> {
        let mut spans = Vec::new();

        // Highlight matching characters in title
        if self.highlight_matches && !self.query.is_empty() {
            spans.extend(self.highlight_text(&item.title, &self.query, theme));
        } else {
            spans.push(Span::raw(item.title.clone()));
        }

        // Add description if enabled
        if self.show_descriptions {
            if let Some(ref description) = item.description {
                spans.push(Span::styled(
                    format!(" - {}", description),
                    Style::default().fg(theme.colors.fg_muted),
                ));
            }
        }

        // Add provider indicator
        spans.push(Span::styled(
            format!(" [{}]", item.provider),
            Style::default()
                .fg(theme.colors.accent)
                .add_modifier(Modifier::DIM),
        ));

        let style = if is_selected {
            Style::default()
                .bg(theme.colors.accent)
                .fg(theme.colors.bg_base)
        } else {
            Style::default().fg(theme.colors.fg_base)
        };

        ListItem::new(Line::from(spans)).style(style)
    }

    /// Highlight matching characters in text
    fn highlight_text(&self, text: &str, query: &str, theme: &Theme) -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        let text_lower = text.to_lowercase();
        let query_lower = query.to_lowercase();

        let mut last_end = 0;
        let mut pos = 0;

        while let Some(found) = text_lower[pos..].find(&query_lower) {
            let absolute_pos = pos + found;

            // Add text before match
            if absolute_pos > last_end {
                spans.push(Span::raw(text[last_end..absolute_pos].to_string()));
            }

            // Add highlighted match
            spans.push(Span::styled(
                text[absolute_pos..absolute_pos + query.len()].to_string(),
                Style::default()
                    .fg(theme.colors.accent)
                    .add_modifier(Modifier::BOLD),
            ));

            last_end = absolute_pos + query.len();
            pos = last_end;
        }

        // Add remaining text
        if last_end < text.len() {
            spans.push(Span::raw(text[last_end..].to_string()));
        }

        spans
    }
}

impl Default for CompletionList {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Component for CompletionList {
    async fn handle_key_event(&mut self, event: KeyEvent) -> Result<()> {
        if !self.visible {
            return Ok(());
        }

        match event.code {
            KeyCode::Up => {
                self.move_up();
            }
            KeyCode::Down => {
                self.move_down();
            }
            KeyCode::Enter | KeyCode::Tab => {
                self.select_current(false);
            }
            KeyCode::Char('n') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_down();
                self.select_current(true); // Insert and continue
            }
            KeyCode::Char('p') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_up();
                self.select_current(true); // Insert and continue
            }
            KeyCode::Char('y') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.select_current(false);
            }
            KeyCode::Esc => {
                self.close();
            }
            _ => {}
        }

        Ok(())
    }

    async fn handle_mouse_event(&mut self, _event: MouseEvent) -> Result<()> {
        // TODO: Implement mouse selection
        Ok(())
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.visible || self.items.is_empty() {
            return;
        }

        let display_area = self.calculate_display_area(area);
        
        // Clear the area behind the popup
        frame.render_widget(Clear, display_area);

        // Create the list widget (collect items first to release immutable borrow)
        let items = self.create_list_items(theme);
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Completions")
                    .border_style(Style::default().fg(theme.colors.border))
                    .title_style(Style::default().fg(theme.colors.fg_base).add_modifier(Modifier::BOLD)),
            )
            .highlight_style(
                Style::default()
                    .bg(theme.colors.accent)
                    .fg(theme.colors.bg_base)
                    .add_modifier(Modifier::BOLD),
            );

        // Render the list using a separate mutable borrow of list_state
        let list_state = &mut self.list_state;
        frame.render_stateful_widget(list, display_area, list_state);

        // Show scroll indicator if needed
        if self.items.len() > self.max_visible_items {
            self.render_scroll_indicator(frame, display_area, theme);
        }
    }

    fn size(&self) -> Rect {
        self.position
    }

    fn set_size(&mut self, size: Rect) {
        self.position = size;
    }

    fn has_focus(&self) -> bool {
        self.visible
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn set_visible(&mut self, visible: bool) {
        if !visible {
            self.close();
        }
        self.visible = visible;
    }
}

impl CompletionList {
    /// Render scroll indicator
    fn render_scroll_indicator(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if area.width < 3 {
            return;
        }

        let scroll_area = Rect {
            x: area.x + area.width - 1,
            y: area.y + 1,
            width: 1,
            height: area.height.saturating_sub(2),
        };

        let total_items = self.items.len();
        let visible_items = self.max_visible_items;
        let scroll_progress = if total_items > visible_items {
            self.scroll_offset as f64 / (total_items - visible_items) as f64
        } else {
            0.0
        };

        let scroll_pos = (scroll_progress * scroll_area.height as f64) as u16;
        let scroll_char = if scroll_pos < scroll_area.height {
            "▌"
        } else {
            "▄"
        };

        let scroll_indicator = Paragraph::new(scroll_char)
            .style(Style::default().fg(theme.colors.accent));

        let indicator_area = Rect {
            x: scroll_area.x,
            y: scroll_area.y + scroll_pos,
            width: 1,
            height: 1,
        };

        frame.render_widget(scroll_indicator, indicator_area);
    }
}

/// Handle completion messages for the list component
pub fn handle_completion_message(
    list: &mut CompletionList,
    message: CompletionMessage,
) -> Result<()> {
    match message {
        CompletionMessage::Request(context) => {
            // This would trigger the completion engine, not handled here
            debug!("Completion request received: {:?}", context);
        }
        CompletionMessage::Filter { query, reopen, x, y } => {
            let position = Rect::new(x, y, 0, 0);
            if list.is_visible() {
                // Filter existing items in the list - we need to update the filter method
                list.query = query;
                list.reposition(position);
            } else if reopen {
                // To reopen, we need items - this suggests the message should include items
                // For now, do nothing if not visible and we don't have items
                debug!("Cannot reopen completion list without items");
            }
        }
        CompletionMessage::Reposition { x, y } => {
            let position = Rect::new(x, y, 0, 0);
            list.reposition(position);
        }
        CompletionMessage::Select { item, insert } => {
            // Selection is typically handled by key events
            debug!("Completion selection: {} (insert: {})", item.title, insert);
        }
        CompletionMessage::Close => {
            list.close();
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_completion_list_creation() {
        let list = CompletionList::new();
        assert!(!list.visible);
        assert!(list.items.is_empty());
        assert_eq!(list.selected_index, 0);
    }

    #[test]
    fn test_completion_list_open_close() {
        let mut list = CompletionList::new();
        
        let items = vec![
            CompletionItem::new("test1", "test1", "provider"),
            CompletionItem::new("test2", "test2", "provider"),
        ];
        
        let position = Rect::new(10, 5, 0, 0);
        list.open(items.clone(), position, "test".to_string());
        
        assert!(list.visible);
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.query, "test");
        assert_eq!(list.position, position);
        
        list.close();
        assert!(!list.visible);
        assert!(list.items.is_empty());
    }

    #[test]
    fn test_completion_list_navigation() {
        let mut list = CompletionList::new();
        
        let items = vec![
            CompletionItem::new("test1", "test1", "provider"),
            CompletionItem::new("test2", "test2", "provider"),
            CompletionItem::new("test3", "test3", "provider"),
        ];
        
        list.open(items, Rect::default(), String::new());
        
        assert_eq!(list.selected_index, 0);
        
        list.move_down();
        assert_eq!(list.selected_index, 1);
        
        list.move_down();
        assert_eq!(list.selected_index, 2);
        
        list.move_down(); // Should wrap to 0
        assert_eq!(list.selected_index, 0);
        
        list.move_up(); // Should wrap to 2
        assert_eq!(list.selected_index, 2);
        
        list.move_up();
        assert_eq!(list.selected_index, 1);
    }

    #[test]
    fn test_completion_list_filter() {
        let mut list = CompletionList::new();
        
        let initial_items = vec![
            CompletionItem::new("test1", "test1", "provider"),
            CompletionItem::new("test2", "test2", "provider"),
        ];
        
        list.open(initial_items, Rect::default(), "te".to_string());
        assert_eq!(list.items.len(), 2);
        
        let filtered_items = vec![
            CompletionItem::new("test1", "test1", "provider"),
        ];
        
        list.filter(filtered_items, "test1".to_string());
        assert_eq!(list.items.len(), 1);
        assert_eq!(list.query, "test1");
        
        // Empty filter should close the list
        list.filter(vec![], "nonexistent".to_string());
        assert!(!list.visible);
    }

    #[test]
    fn test_width_calculation() {
        let mut list = CompletionList::new();
        
        let items = vec![
            CompletionItem::new("short", "short", "provider"),
            CompletionItem::new("very_long_completion_item", "very_long_completion_item", "provider")
                .with_description("This is a long description".to_string()),
        ];
        
        list.items = items;
        let width = list.calculate_width();
        
        // Should accommodate the longest item plus description
        assert!(width > 20);
        assert!(width <= 80);
    }
}