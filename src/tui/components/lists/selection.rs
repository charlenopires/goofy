//! Multi-selection capabilities for lists with various selection modes.
//!
//! This module provides sophisticated selection functionality including
//! single selection, multi-selection, range selection, and custom selection
//! modes with keyboard and mouse support.

use super::ListItem;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};
use std::collections::{BTreeSet, HashMap};
use std::time::Instant;

/// Selection manager for list components
pub struct SelectionManager<T: ListItem> {
    /// Current selection mode
    mode: SelectionMode,

    /// Currently selected items
    selected_items: BTreeSet<String>,

    /// Primary selected item (for keyboard navigation)
    primary_selection: Option<String>,

    /// Last selected item (for range selection)
    last_selected: Option<String>,

    /// Anchor item for range selection
    range_anchor: Option<String>,

    /// Selection history for undo/redo
    selection_history: Vec<SelectionSnapshot>,

    /// Current position in history
    history_position: usize,

    /// Selection configuration
    config: SelectionConfig,

    /// Selection metadata
    metadata: HashMap<String, SelectionMetadata>,

    /// Event callbacks
    callbacks: Vec<Box<dyn Fn(SelectionEvent) + Send + Sync>>,

    /// Phantom data for type parameter T
    _phantom: std::marker::PhantomData<T>,
}

impl<T: ListItem> std::fmt::Debug for SelectionManager<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectionManager")
            .field("mode", &self.mode)
            .field("selected_items", &self.selected_items)
            .field("primary_selection", &self.primary_selection)
            .field("history_position", &self.history_position)
            .field("config", &self.config)
            .field("callbacks", &format!("[{} callbacks]", self.callbacks.len()))
            .finish()
    }
}

/// Selection modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    /// No selection allowed
    None,
    /// Single selection only
    Single,
    /// Multiple selection allowed
    Multiple,
    /// Range selection (select continuous ranges)
    Range,
    /// Custom selection mode
    Custom,
}

/// Selection configuration
#[derive(Debug, Clone)]
pub struct SelectionConfig {
    /// Whether to enable keyboard selection
    pub enable_keyboard: bool,
    
    /// Whether to enable mouse selection
    pub enable_mouse: bool,
    
    /// Whether to preserve selection when items are removed
    pub preserve_on_remove: bool,
    
    /// Maximum number of selected items (None = unlimited)
    pub max_selected: Option<usize>,
    
    /// Whether to enable selection history (undo/redo)
    pub enable_history: bool,
    
    /// Maximum history entries
    pub max_history_entries: usize,
    
    /// Whether to allow empty selection
    pub allow_empty_selection: bool,
    
    /// Style for selected items
    pub selected_style: Style,
    
    /// Style for primary selected item
    pub primary_style: Style,
    
    /// Style for range selection
    pub range_style: Style,
    
    /// Whether to show selection indicators
    pub show_indicators: bool,
    
    /// Selection indicator characters
    pub indicators: SelectionIndicators,
}

/// Selection indicator characters
#[derive(Debug, Clone)]
pub struct SelectionIndicators {
    pub selected: String,
    pub primary: String,
    pub range: String,
    pub unselected: String,
}

impl Default for SelectionConfig {
    fn default() -> Self {
        Self {
            enable_keyboard: true,
            enable_mouse: true,
            preserve_on_remove: true,
            max_selected: None,
            enable_history: true,
            max_history_entries: 50,
            allow_empty_selection: true,
            selected_style: Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            primary_style: Style::default()
                .bg(Color::LightBlue)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            range_style: Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
            show_indicators: true,
            indicators: SelectionIndicators {
                selected: "●".to_string(),
                primary: "◉".to_string(),
                range: "◐".to_string(),
                unselected: "○".to_string(),
            },
        }
    }
}

/// Selection snapshot for history
#[derive(Debug, Clone)]
struct SelectionSnapshot {
    selected_items: BTreeSet<String>,
    primary_selection: Option<String>,
    timestamp: Instant,
    description: String,
}

/// Metadata for selected items
#[derive(Debug, Clone)]
struct SelectionMetadata {
    selected_at: Instant,
    selection_order: usize,
    is_primary: bool,
}

/// Selection events
#[derive(Debug, Clone)]
pub enum SelectionEvent {
    /// Selection changed
    SelectionChanged {
        selected: Vec<String>,
        primary: Option<String>,
    },
    
    /// Item was selected
    ItemSelected {
        item_id: String,
        is_primary: bool,
    },
    
    /// Item was deselected
    ItemDeselected {
        item_id: String,
    },
    
    /// Range selection changed
    RangeSelectionChanged {
        start: String,
        end: String,
        selected: Vec<String>,
    },
    
    /// Selection was cleared
    SelectionCleared,
    
    /// Selection was inverted
    SelectionInverted {
        newly_selected: Vec<String>,
        newly_deselected: Vec<String>,
    },
    
    /// Maximum selection limit reached
    MaxSelectionReached {
        limit: usize,
        attempted_item: String,
    },
}

impl<T: ListItem> SelectionManager<T> {
    /// Create a new selection manager
    pub fn new(mode: SelectionMode) -> Self {
        Self::with_config(mode, SelectionConfig::default())
    }
    
    /// Create a new selection manager with custom configuration
    pub fn with_config(mode: SelectionMode, config: SelectionConfig) -> Self {
        Self {
            mode,
            selected_items: BTreeSet::new(),
            primary_selection: None,
            last_selected: None,
            range_anchor: None,
            selection_history: Vec::new(),
            history_position: 0,
            config,
            metadata: HashMap::new(),
            callbacks: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Set the selection mode
    pub fn set_mode(&mut self, mode: SelectionMode) -> Result<()> {
        if self.mode != mode {
            match mode {
                SelectionMode::None => {
                    self.clear_selection()?;
                }
                SelectionMode::Single => {
                    // Keep only primary selection
                    if let Some(primary) = &self.primary_selection {
                        let primary_id = primary.clone();
                        self.clear_selection()?;
                        self.select_item(&primary_id, true)?;
                    } else if !self.selected_items.is_empty() {
                        let first = self.selected_items.iter().next().unwrap().clone();
                        self.clear_selection()?;
                        self.select_item(&first, true)?;
                    }
                }
                _ => {
                    // Other modes allow current selection
                }
            }
            self.mode = mode;
        }
        Ok(())
    }
    
    /// Get the current selection mode
    pub fn mode(&self) -> SelectionMode {
        self.mode
    }
    
    /// Add an event callback
    pub fn add_callback<F>(&mut self, callback: F)
    where
        F: Fn(SelectionEvent) + Send + Sync + 'static,
    {
        self.callbacks.push(Box::new(callback));
    }
    
    /// Select an item
    pub fn select_item(&mut self, item_id: &str, make_primary: bool) -> Result<bool> {
        if self.mode == SelectionMode::None {
            return Ok(false);
        }
        
        // Check if already selected
        if self.selected_items.contains(item_id) {
            if make_primary {
                self.set_primary_selection(Some(item_id.to_string()))?;
            }
            return Ok(false);
        }
        
        // Check selection limit
        if let Some(max) = self.config.max_selected {
            if self.selected_items.len() >= max {
                self.emit_event(SelectionEvent::MaxSelectionReached {
                    limit: max,
                    attempted_item: item_id.to_string(),
                });
                return Ok(false);
            }
        }
        
        // Handle different selection modes
        match self.mode {
            SelectionMode::Single => {
                self.clear_selection()?;
                self.add_to_selection(item_id, true)?;
            }
            SelectionMode::Multiple | SelectionMode::Range | SelectionMode::Custom => {
                self.add_to_selection(item_id, make_primary)?;
            }
            SelectionMode::None => return Ok(false),
        }

        self.save_selection_state(format!("Select item {}", item_id));
        Ok(true)
    }
    
    /// Deselect an item
    pub fn deselect_item(&mut self, item_id: &str) -> Result<bool> {
        if !self.selected_items.contains(item_id) {
            return Ok(false);
        }
        
        self.remove_from_selection(item_id)?;
        Ok(true)
    }
    
    /// Toggle selection of an item
    pub fn toggle_item(&mut self, item_id: &str, make_primary: bool) -> Result<bool> {
        if self.selected_items.contains(item_id) {
            self.deselect_item(item_id)
        } else {
            self.select_item(item_id, make_primary)
        }
    }
    
    /// Select a range of items
    pub fn select_range(&mut self, start_id: &str, end_id: &str, item_list: &[T]) -> Result<()> {
        if self.mode == SelectionMode::None {
            return Ok(());
        }
        
        let start_index = item_list.iter().position(|item| item.id() == start_id);
        let end_index = item_list.iter().position(|item| item.id() == end_id);
        
        if let (Some(start), Some(end)) = (start_index, end_index) {
            let (range_start, range_end) = if start <= end {
                (start, end)
            } else {
                (end, start)
            };
            
            self.save_selection_state("Range selection".to_string());
            
            let mut selected_in_range = Vec::new();
            
            for i in range_start..=range_end {
                if let Some(item) = item_list.get(i) {
                    if item.selectable() {
                        let item_id = item.id();
                        if !self.selected_items.contains(&item_id) {
                            self.add_to_selection(&item_id, false)?;
                        }
                        selected_in_range.push(item_id);
                    }
                }
            }
            
            self.emit_event(SelectionEvent::RangeSelectionChanged {
                start: start_id.to_string(),
                end: end_id.to_string(),
                selected: selected_in_range,
            });
        }
        
        Ok(())
    }
    
    /// Select all items
    pub fn select_all(&mut self, item_list: &[T]) -> Result<()> {
        if self.mode == SelectionMode::None || self.mode == SelectionMode::Single {
            return Ok(());
        }
        
        self.save_selection_state("Select all".to_string());
        
        for item in item_list {
            if item.selectable() {
                let item_id = item.id();
                if !self.selected_items.contains(&item_id) {
                    // Check selection limit
                    if let Some(max) = self.config.max_selected {
                        if self.selected_items.len() >= max {
                            break;
                        }
                    }
                    self.add_to_selection(&item_id, false)?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Clear all selection
    pub fn clear_selection(&mut self) -> Result<()> {
        if self.selected_items.is_empty() {
            return Ok(());
        }
        
        self.save_selection_state("Clear selection".to_string());
        
        self.selected_items.clear();
        self.primary_selection = None;
        self.last_selected = None;
        self.range_anchor = None;
        self.metadata.clear();
        
        self.emit_event(SelectionEvent::SelectionCleared);
        self.emit_selection_changed();
        
        Ok(())
    }
    
    /// Invert selection
    pub fn invert_selection(&mut self, item_list: &[T]) -> Result<()> {
        if self.mode == SelectionMode::None || self.mode == SelectionMode::Single {
            return Ok(());
        }
        
        self.save_selection_state("Invert selection".to_string());
        
        let mut newly_selected = Vec::new();
        let mut newly_deselected = Vec::new();
        
        for item in item_list {
            if item.selectable() {
                let item_id = item.id();
                if self.selected_items.contains(&item_id) {
                    self.remove_from_selection(&item_id)?;
                    newly_deselected.push(item_id);
                } else {
                    // Check selection limit
                    if let Some(max) = self.config.max_selected {
                        if self.selected_items.len() >= max {
                            continue;
                        }
                    }
                    self.add_to_selection(&item_id, false)?;
                    newly_selected.push(item_id);
                }
            }
        }
        
        self.emit_event(SelectionEvent::SelectionInverted {
            newly_selected,
            newly_deselected,
        });
        
        Ok(())
    }
    
    /// Get selected item IDs
    pub fn selected_items(&self) -> Vec<String> {
        self.selected_items.iter().cloned().collect()
    }
    
    /// Get the primary selected item
    pub fn primary_selection(&self) -> Option<&String> {
        self.primary_selection.as_ref()
    }
    
    /// Set the primary selection
    pub fn set_primary_selection(&mut self, item_id: Option<String>) -> Result<()> {
        if let Some(id) = &item_id {
            if !self.selected_items.contains(id) {
                return Err(anyhow::anyhow!("Item must be selected to be primary"));
            }
        }
        
        // Update metadata
        if let Some(old_primary) = &self.primary_selection {
            if let Some(meta) = self.metadata.get_mut(old_primary) {
                meta.is_primary = false;
            }
        }
        
        if let Some(new_primary) = &item_id {
            if let Some(meta) = self.metadata.get_mut(new_primary) {
                meta.is_primary = true;
            }
        }
        
        self.primary_selection = item_id;
        self.emit_selection_changed();
        
        Ok(())
    }
    
    /// Check if an item is selected
    pub fn is_selected(&self, item_id: &str) -> bool {
        self.selected_items.contains(item_id)
    }
    
    /// Check if an item is the primary selection
    pub fn is_primary(&self, item_id: &str) -> bool {
        self.primary_selection.as_ref() == Some(&item_id.to_string())
    }
    
    /// Get selection count
    pub fn selection_count(&self) -> usize {
        self.selected_items.len()
    }
    
    /// Check if selection is empty
    pub fn is_empty(&self) -> bool {
        self.selected_items.is_empty()
    }
    
    /// Handle keyboard input
    pub fn handle_key_event(&mut self, key: KeyEvent, item_list: &[T]) -> Result<bool> {
        if !self.config.enable_keyboard || self.mode == SelectionMode::None {
            return Ok(false);
        }
        
        match (key.code, key.modifiers) {
            (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                self.select_all(item_list)?;
                Ok(true)
            }
            (KeyCode::Char('i'), KeyModifiers::CONTROL) => {
                self.invert_selection(item_list)?;
                Ok(true)
            }
            (KeyCode::Esc, _) => {
                self.clear_selection()?;
                Ok(true)
            }
            (KeyCode::Char('z'), KeyModifiers::CONTROL) => {
                self.undo()?;
                Ok(true)
            }
            (KeyCode::Char('y'), KeyModifiers::CONTROL) => {
                self.redo()?;
                Ok(true)
            }
            (KeyCode::Char('z'), modifiers) if modifiers.contains(KeyModifiers::CONTROL) && modifiers.contains(KeyModifiers::SHIFT) => {
                self.redo()?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }
    
    /// Handle mouse input
    pub fn handle_mouse_event(&mut self, event: MouseEvent, item_at_position: Option<&str>) -> Result<bool> {
        if !self.config.enable_mouse || self.mode == SelectionMode::None {
            return Ok(false);
        }
        
        if let Some(item_id) = item_at_position {
            match event.kind {
                MouseEventKind::Down(button) => {
                    match button {
                        crossterm::event::MouseButton::Left => {
                            if event.modifiers.contains(KeyModifiers::CONTROL) {
                                // Ctrl+click: toggle selection
                                self.toggle_item(item_id, true)?;
                            } else if event.modifiers.contains(KeyModifiers::SHIFT) {
                                // Shift+click: range selection
                                if let Some(_anchor) = &self.range_anchor.clone() {
                                    // Note: This requires access to the full item list
                                    // For now, just select the item
                                    self.select_item(item_id, true)?;
                                } else {
                                    self.select_item(item_id, true)?;
                                }
                            } else {
                                // Normal click: single selection or clear and select
                                match self.mode {
                                    SelectionMode::Single => {
                                        self.select_item(item_id, true)?;
                                    }
                                    SelectionMode::Multiple => {
                                        if !self.is_selected(item_id) {
                                            self.clear_selection()?;
                                            self.select_item(item_id, true)?;
                                        } else {
                                            self.set_primary_selection(Some(item_id.to_string()))?;
                                        }
                                    }
                                    _ => {
                                        self.select_item(item_id, true)?;
                                    }
                                }
                            }
                            self.range_anchor = Some(item_id.to_string());
                            Ok(true)
                        }
                        _ => Ok(false),
                    }
                }
                _ => Ok(false),
            }
        } else {
            Ok(false)
        }
    }
    
    /// Undo last selection change
    pub fn undo(&mut self) -> Result<bool> {
        if !self.config.enable_history || self.history_position + 1 >= self.selection_history.len() {
            return Ok(false);
        }

        self.history_position += 1;
        let index = self.selection_history.len() - 1 - self.history_position;
        if let Some(snapshot) = self.selection_history.get(index) {
            self.restore_selection_state(snapshot.clone())?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Redo last undone selection change
    pub fn redo(&mut self) -> Result<bool> {
        if !self.config.enable_history || self.history_position == 0 {
            return Ok(false);
        }

        self.history_position -= 1;
        let index = self.selection_history.len() - 1 - self.history_position;
        if let Some(snapshot) = self.selection_history.get(index) {
            self.restore_selection_state(snapshot.clone())?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Add an item to selection
    fn add_to_selection(&mut self, item_id: &str, make_primary: bool) -> Result<()> {
        let order = self.metadata.len();
        
        self.selected_items.insert(item_id.to_string());
        self.metadata.insert(item_id.to_string(), SelectionMetadata {
            selected_at: Instant::now(),
            selection_order: order,
            is_primary: make_primary,
        });
        
        if make_primary {
            self.primary_selection = Some(item_id.to_string());
        }
        
        self.last_selected = Some(item_id.to_string());
        
        self.emit_event(SelectionEvent::ItemSelected {
            item_id: item_id.to_string(),
            is_primary: make_primary,
        });
        
        self.emit_selection_changed();
        
        Ok(())
    }
    
    /// Remove an item from selection
    fn remove_from_selection(&mut self, item_id: &str) -> Result<()> {
        self.selected_items.remove(item_id);
        self.metadata.remove(item_id);
        
        if self.primary_selection.as_ref() == Some(&item_id.to_string()) {
            // Choose new primary selection
            self.primary_selection = self.selected_items.iter().next().cloned();
        }
        
        if self.last_selected.as_ref() == Some(&item_id.to_string()) {
            self.last_selected = self.selected_items.iter().next().cloned();
        }
        
        if self.range_anchor.as_ref() == Some(&item_id.to_string()) {
            self.range_anchor = None;
        }
        
        self.emit_event(SelectionEvent::ItemDeselected {
            item_id: item_id.to_string(),
        });
        
        self.emit_selection_changed();
        
        Ok(())
    }
    
    /// Save current selection state to history
    fn save_selection_state(&mut self, description: String) {
        if !self.config.enable_history {
            return;
        }
        
        let snapshot = SelectionSnapshot {
            selected_items: self.selected_items.clone(),
            primary_selection: self.primary_selection.clone(),
            timestamp: Instant::now(),
            description,
        };
        
        // Remove any redo history when adding new state
        if self.history_position > 0 {
            let remove_count = self.history_position;
            for _ in 0..remove_count {
                self.selection_history.pop();
            }
            self.history_position = 0;
        }
        
        self.selection_history.push(snapshot);
        
        // Limit history size
        if self.selection_history.len() > self.config.max_history_entries {
            self.selection_history.remove(0);
        }
    }
    
    /// Restore selection state from snapshot
    fn restore_selection_state(&mut self, snapshot: SelectionSnapshot) -> Result<()> {
        self.selected_items = snapshot.selected_items;
        self.primary_selection = snapshot.primary_selection;
        
        // Rebuild metadata
        self.metadata.clear();
        for (order, item_id) in self.selected_items.iter().enumerate() {
            let is_primary = self.primary_selection.as_ref() == Some(item_id);
            self.metadata.insert(item_id.clone(), SelectionMetadata {
                selected_at: snapshot.timestamp,
                selection_order: order,
                is_primary,
            });
        }
        
        self.emit_selection_changed();
        Ok(())
    }
    
    /// Emit selection changed event
    fn emit_selection_changed(&self) {
        self.emit_event(SelectionEvent::SelectionChanged {
            selected: self.selected_items.iter().cloned().collect(),
            primary: self.primary_selection.clone(),
        });
    }
    
    /// Emit an event to all callbacks
    fn emit_event(&self, event: SelectionEvent) {
        for callback in &self.callbacks {
            callback(event.clone());
        }
    }
    
    /// Apply item styling based on selection state
    pub fn apply_selection_style(&self, item_id: &str, base_style: Style) -> Style {
        if !self.is_selected(item_id) {
            return base_style;
        }
        
        let selection_style = if self.is_primary(item_id) {
            self.config.primary_style
        } else {
            self.config.selected_style
        };
        
        // Merge styles
        let mut style = base_style;
        if let Some(fg) = selection_style.fg {
            style = style.fg(fg);
        }
        if let Some(bg) = selection_style.bg {
            style = style.bg(bg);
        }
        style = style.add_modifier(selection_style.add_modifier);
        style = style.remove_modifier(selection_style.sub_modifier);
        
        style
    }
    
    /// Render selection indicator
    pub fn render_selection_indicator(&self, item_id: &str) -> Option<Span<'static>> {
        if !self.config.show_indicators {
            return None;
        }
        
        let indicator = if self.is_primary(item_id) {
            &self.config.indicators.primary
        } else if self.is_selected(item_id) {
            &self.config.indicators.selected
        } else {
            &self.config.indicators.unselected
        };
        
        Some(Span::styled(
            indicator.clone(),
            if self.is_selected(item_id) {
                self.config.selected_style
            } else {
                Style::default()
            },
        ))
    }
    
    /// Get selection statistics
    pub fn selection_stats(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();
        
        stats.insert("total_selected".to_string(), 
            serde_json::Value::from(self.selected_items.len()));
        stats.insert("has_primary".to_string(), 
            serde_json::Value::from(self.primary_selection.is_some()));
        stats.insert("mode".to_string(), 
            serde_json::Value::from(format!("{:?}", self.mode)));
        
        if !self.selected_items.is_empty() {
            let avg_selection_age = self.metadata.values()
                .map(|meta| meta.selected_at.elapsed().as_secs())
                .sum::<u64>() / self.metadata.len() as u64;
            stats.insert("avg_selection_age_seconds".to_string(),
                serde_json::Value::from(avg_selection_age));
        }
        
        stats.insert("history_entries".to_string(),
            serde_json::Value::from(self.selection_history.len()));
        stats.insert("history_position".to_string(),
            serde_json::Value::from(self.history_position));
        
        stats
    }
}

impl<T: ListItem> Default for SelectionManager<T> {
    fn default() -> Self {
        Self::new(SelectionMode::Single)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::components::lists::SimpleListItem;
    
    fn create_test_items() -> Vec<SimpleListItem> {
        vec![
            SimpleListItem::from_text("1".to_string(), "Item 1".to_string()),
            SimpleListItem::from_text("2".to_string(), "Item 2".to_string()),
            SimpleListItem::from_text("3".to_string(), "Item 3".to_string()),
            SimpleListItem::from_text("4".to_string(), "Item 4".to_string()),
            SimpleListItem::from_text("5".to_string(), "Item 5".to_string()),
        ]
    }
    
    #[test]
    fn test_single_selection() {
        let mut manager: SelectionManager<SimpleListItem> = SelectionManager::new(SelectionMode::Single);
        
        manager.select_item("1", true).unwrap();
        assert_eq!(manager.selection_count(), 1);
        assert!(manager.is_selected("1"));
        assert!(manager.is_primary("1"));
        
        // Selecting another item should clear the first
        manager.select_item("2", true).unwrap();
        assert_eq!(manager.selection_count(), 1);
        assert!(!manager.is_selected("1"));
        assert!(manager.is_selected("2"));
    }
    
    #[test]
    fn test_multi_selection() {
        let mut manager: SelectionManager<SimpleListItem> = SelectionManager::new(SelectionMode::Multiple);
        
        manager.select_item("1", true).unwrap();
        manager.select_item("2", false).unwrap();
        manager.select_item("3", false).unwrap();
        
        assert_eq!(manager.selection_count(), 3);
        assert!(manager.is_selected("1"));
        assert!(manager.is_selected("2"));
        assert!(manager.is_selected("3"));
        assert!(manager.is_primary("1"));
    }
    
    #[test]
    fn test_range_selection() {
        let mut manager = SelectionManager::new(SelectionMode::Range);
        let items = create_test_items();
        
        manager.select_range("2", "4", &items).unwrap();
        
        assert!(manager.is_selected("2"));
        assert!(manager.is_selected("3"));
        assert!(manager.is_selected("4"));
        assert!(!manager.is_selected("1"));
        assert!(!manager.is_selected("5"));
    }
    
    #[test]
    fn test_select_all() {
        let mut manager = SelectionManager::new(SelectionMode::Multiple);
        let items = create_test_items();
        
        manager.select_all(&items).unwrap();
        
        assert_eq!(manager.selection_count(), 5);
        for item in &items {
            assert!(manager.is_selected(&item.id()));
        }
    }
    
    #[test]
    fn test_clear_selection() {
        let mut manager = SelectionManager::new(SelectionMode::Multiple);
        let items = create_test_items();
        
        manager.select_all(&items).unwrap();
        assert_eq!(manager.selection_count(), 5);
        
        manager.clear_selection().unwrap();
        assert_eq!(manager.selection_count(), 0);
        assert!(manager.is_empty());
    }
    
    #[test]
    fn test_toggle_selection() {
        let mut manager: SelectionManager<SimpleListItem> = SelectionManager::new(SelectionMode::Multiple);
        
        // Toggle on
        manager.toggle_item("1", true).unwrap();
        assert!(manager.is_selected("1"));
        
        // Toggle off
        manager.toggle_item("1", false).unwrap();
        assert!(!manager.is_selected("1"));
    }
    
    #[test]
    fn test_selection_history() {
        let mut manager: SelectionManager<SimpleListItem> = SelectionManager::with_config(
            SelectionMode::Multiple,
            SelectionConfig::default(),
        );
        
        manager.select_item("1", true).unwrap();
        manager.select_item("2", false).unwrap();
        manager.clear_selection().unwrap();
        
        // Undo clear
        manager.undo().unwrap();
        assert_eq!(manager.selection_count(), 2);
        
        // Undo second selection
        manager.undo().unwrap();
        assert_eq!(manager.selection_count(), 1);
        assert!(manager.is_selected("1"));
        
        // Redo
        manager.redo().unwrap();
        assert_eq!(manager.selection_count(), 2);
    }
    
    #[test]
    fn test_selection_limit() {
        let mut config = SelectionConfig::default();
        config.max_selected = Some(2);
        let mut manager: SelectionManager<SimpleListItem> = SelectionManager::with_config(SelectionMode::Multiple, config);
        
        manager.select_item("1", true).unwrap();
        manager.select_item("2", false).unwrap();
        
        // This should fail due to limit
        let result = manager.select_item("3", false).unwrap();
        assert!(!result);
        assert_eq!(manager.selection_count(), 2);
    }
}