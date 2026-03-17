//! Animated list components for smooth list operations.
//!
//! This module provides animated list components that can handle
//! adding, removing, and reordering items with smooth transitions.

use super::{Animation, AnimationConfig, AnimationState, EasingType};
use super::slide::{SlideAnimation, SlideConfig, SlideDirection};
use super::fade::{FadeAnimation, FadeConfig, FadeDirection};
use super::interpolation::RgbColor;
use crate::tui::themes::Theme;
use anyhow::Result;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// List animation operation types
#[derive(Debug, Clone, PartialEq)]
pub enum ListOperation {
    /// Add item to the list
    Add { index: usize, item: ListItem },
    /// Remove item from the list
    Remove { index: usize },
    /// Move item from one position to another
    Move { from: usize, to: usize },
    /// Update an existing item
    Update { index: usize, item: ListItem },
    /// Clear all items
    Clear,
    /// Batch operations
    Batch(Vec<ListOperation>),
}

/// Individual list item with content and metadata
#[derive(Debug, Clone, PartialEq)]
pub struct ListItem {
    pub id: String,
    pub content: Vec<Line<'static>>,
    pub height: u16,
    pub selectable: bool,
    pub style: Option<Style>,
    pub data: Option<serde_json::Value>, // Optional custom data
}

impl ListItem {
    pub fn new(id: String, content: Vec<Line<'static>>) -> Self {
        Self {
            id,
            height: content.len() as u16,
            content,
            selectable: true,
            style: None,
            data: None,
        }
    }

    pub fn from_text(id: String, text: String) -> Self {
        Self::new(id, vec![Line::from(text)])
    }

    pub fn with_height(mut self, height: u16) -> Self {
        self.height = height;
        self
    }

    pub fn with_style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    pub fn non_selectable(mut self) -> Self {
        self.selectable = false;
        self
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }
}

/// Animation configuration for list operations
#[derive(Debug, Clone)]
pub struct ListAnimationConfig {
    pub add_animation: AnimationConfig,
    pub remove_animation: AnimationConfig,
    pub move_animation: AnimationConfig,
    pub update_animation: AnimationConfig,
    pub stagger_delay: Duration,
    pub parallel_animations: bool,
    pub bounce_on_add: bool,
    pub fade_on_remove: bool,
    pub slide_direction: SlideDirection,
}

impl Default for ListAnimationConfig {
    fn default() -> Self {
        Self {
            add_animation: AnimationConfig::new().duration(Duration::from_millis(300))
                .with_easing(EasingType::EaseOutBack),
            remove_animation: AnimationConfig::new().duration(Duration::from_millis(250))
                .with_easing(EasingType::EaseInBack),
            move_animation: AnimationConfig::new().duration(Duration::from_millis(400))
                .with_easing(EasingType::EaseInOutCubic),
            update_animation: AnimationConfig::new().duration(Duration::from_millis(200))
                .with_easing(EasingType::EaseInOut),
            stagger_delay: Duration::from_millis(50),
            parallel_animations: false,
            bounce_on_add: true,
            fade_on_remove: true,
            slide_direction: SlideDirection::FromLeft,
        }
    }
}

impl ListAnimationConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_add_animation(mut self, config: AnimationConfig) -> Self {
        self.add_animation = config;
        self
    }

    pub fn with_remove_animation(mut self, config: AnimationConfig) -> Self {
        self.remove_animation = config;
        self
    }

    pub fn with_stagger_delay(mut self, delay: Duration) -> Self {
        self.stagger_delay = delay;
        self
    }

    pub fn parallel(mut self) -> Self {
        self.parallel_animations = true;
        self
    }

    pub fn with_slide_direction(mut self, direction: SlideDirection) -> Self {
        self.slide_direction = direction;
        self
    }

    /// Quick presets for common list animation styles
    pub fn smooth_ios() -> Self {
        Self {
            add_animation: AnimationConfig::new().duration(Duration::from_millis(300))
                .with_easing(EasingType::EaseOutCubic),
            remove_animation: AnimationConfig::new().duration(Duration::from_millis(250))
                .with_easing(EasingType::EaseInCubic),
            stagger_delay: Duration::from_millis(30),
            bounce_on_add: false,
            fade_on_remove: true,
            slide_direction: SlideDirection::FromLeft,
            ..Default::default()
        }
    }

    pub fn material_design() -> Self {
        Self {
            add_animation: AnimationConfig::new().duration(Duration::from_millis(225))
                .with_easing(EasingType::EaseOutQuart),
            remove_animation: AnimationConfig::new().duration(Duration::from_millis(195))
                .with_easing(EasingType::EaseInQuart),
            stagger_delay: Duration::from_millis(25),
            bounce_on_add: false,
            fade_on_remove: true,
            ..Default::default()
        }
    }

    pub fn playful_bounce() -> Self {
        Self {
            add_animation: AnimationConfig::new().duration(Duration::from_millis(500))
                .with_easing(EasingType::EaseOutBounce),
            remove_animation: AnimationConfig::new().duration(Duration::from_millis(400))
                .with_easing(EasingType::EaseInBack),
            stagger_delay: Duration::from_millis(80),
            bounce_on_add: true,
            fade_on_remove: true,
            ..Default::default()
        }
    }
}

/// Animated list item with its current state
#[derive(Debug)]
struct AnimatedListItem {
    item: ListItem,
    animation: Option<Box<dyn Animation + Send + Sync>>,
    current_rect: Rect,
    target_rect: Rect,
    is_animating: bool,
    operation: Option<ListOperation>,
}

impl AnimatedListItem {
    fn new(item: ListItem) -> Self {
        Self {
            item,
            animation: None,
            current_rect: Rect::default(),
            target_rect: Rect::default(),
            is_animating: false,
            operation: None,
        }
    }
}

/// Animated list component
#[derive(Debug)]
pub struct AnimatedList {
    config: ListAnimationConfig,
    items: Vec<AnimatedListItem>,
    pending_operations: Vec<ListOperation>,
    state: AnimationState,
    selected_index: Option<usize>,
    scroll_offset: usize,
    item_height: u16,
    total_height: u16,
    area: Rect,
}

impl AnimatedList {
    pub fn new(config: ListAnimationConfig) -> Self {
        Self {
            config,
            items: Vec::new(),
            pending_operations: Vec::new(),
            state: AnimationState::Idle,
            selected_index: None,
            scroll_offset: 0,
            item_height: 1,
            total_height: 0,
            area: Rect::default(),
        }
    }

    /// Set the area for the list
    pub fn set_area(&mut self, area: Rect) {
        self.area = area;
        self.recalculate_layout();
    }

    /// Add an item to the list
    pub fn add_item(&mut self, item: ListItem) {
        // Count existing items plus pending adds to get the correct append index
        let pending_add_count = self.pending_operations.iter()
            .filter(|op| matches!(op, ListOperation::Add { .. }))
            .count();
        self.pending_operations.push(ListOperation::Add {
            index: self.items.len() + pending_add_count,
            item,
        });
    }

    /// Insert an item at a specific index
    pub fn insert_item(&mut self, index: usize, item: ListItem) {
        self.pending_operations.push(ListOperation::Add { index, item });
    }

    /// Remove an item by index
    pub fn remove_item(&mut self, index: usize) {
        if index < self.items.len() {
            self.pending_operations.push(ListOperation::Remove { index });
        }
    }

    /// Remove an item by ID
    pub fn remove_item_by_id(&mut self, id: &str) {
        if let Some(index) = self.items.iter().position(|item| item.item.id == id) {
            self.remove_item(index);
        }
    }

    /// Move an item from one position to another
    pub fn move_item(&mut self, from: usize, to: usize) {
        if from < self.items.len() && to < self.items.len() {
            self.pending_operations.push(ListOperation::Move { from, to });
        }
    }

    /// Update an existing item
    pub fn update_item(&mut self, index: usize, item: ListItem) {
        if index < self.items.len() {
            self.pending_operations.push(ListOperation::Update { index, item });
        }
    }

    /// Clear all items
    pub fn clear(&mut self) {
        self.pending_operations.push(ListOperation::Clear);
    }

    /// Set the selected item index
    pub fn set_selected(&mut self, index: Option<usize>) {
        if let Some(idx) = index {
            if idx < self.items.len() && self.items[idx].item.selectable {
                self.selected_index = Some(idx);
            }
        } else {
            self.selected_index = None;
        }
    }

    /// Get the selected item
    pub fn selected_item(&self) -> Option<&ListItem> {
        self.selected_index
            .and_then(|idx| self.items.get(idx))
            .map(|animated_item| &animated_item.item)
    }

    /// Get all items
    pub fn items(&self) -> Vec<&ListItem> {
        self.items.iter().map(|animated_item| &animated_item.item).collect()
    }

    /// Move selection up
    pub fn select_previous(&mut self) {
        if let Some(current) = self.selected_index {
            if current > 0 {
                self.set_selected(Some(current - 1));
            }
        } else if !self.items.is_empty() {
            self.set_selected(Some(self.items.len() - 1));
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        if let Some(current) = self.selected_index {
            if current + 1 < self.items.len() {
                self.set_selected(Some(current + 1));
            }
        } else if !self.items.is_empty() {
            self.set_selected(Some(0));
        }
    }

    /// Process pending operations
    fn process_operations(&mut self) -> Result<()> {
        let operations = std::mem::take(&mut self.pending_operations);
        
        for operation in operations {
            match operation {
                ListOperation::Add { index, item } => {
                    self.execute_add_operation(index, item)?;
                }
                ListOperation::Remove { index } => {
                    self.execute_remove_operation(index)?;
                }
                ListOperation::Move { from, to } => {
                    self.execute_move_operation(from, to)?;
                }
                ListOperation::Update { index, item } => {
                    self.execute_update_operation(index, item)?;
                }
                ListOperation::Clear => {
                    self.execute_clear_operation()?;
                }
                ListOperation::Batch(batch_ops) => {
                    for op in batch_ops {
                        self.pending_operations.push(op);
                    }
                }
            }
        }

        Ok(())
    }

    /// Execute add operation
    fn execute_add_operation(&mut self, index: usize, item: ListItem) -> Result<()> {
        let mut animated_item = AnimatedListItem::new(item);
        animated_item.operation = Some(ListOperation::Add { index, item: animated_item.item.clone() });

        // Create slide-in animation if configured
        if self.config.bounce_on_add {
            let slide_config = SlideConfig::new(self.config.slide_direction)
                .with_duration(self.config.add_animation.duration)
                .with_easing(self.config.add_animation.easing.into());
            
            animated_item.animation = Some(Box::new(SlideAnimation::new(slide_config, animated_item.target_rect)));
            animated_item.is_animating = true;
        }

        self.items.insert(index.min(self.items.len()), animated_item);
        self.recalculate_layout();
        Ok(())
    }

    /// Execute remove operation
    fn execute_remove_operation(&mut self, index: usize) -> Result<()> {
        if index < self.items.len() {
            if self.config.fade_on_remove {
                let fade_config = FadeConfig::new()
                    .direction(FadeDirection::Out)
                    .animation(self.config.remove_animation.clone());
                
                self.items[index].animation = Some(Box::new(FadeAnimation::new(fade_config)));
                self.items[index].is_animating = true;
                self.items[index].operation = Some(ListOperation::Remove { index });
            } else {
                self.items.remove(index);
                self.adjust_selection_after_removal(index);
            }
        }
        Ok(())
    }

    /// Execute move operation
    fn execute_move_operation(&mut self, from: usize, to: usize) -> Result<()> {
        if from < self.items.len() && to < self.items.len() && from != to {
            let item = self.items.remove(from);
            self.items.insert(to, item);
            
            // Adjust selection if necessary
            if let Some(selected) = self.selected_index {
                if selected == from {
                    self.selected_index = Some(to);
                } else if from < selected && to >= selected {
                    self.selected_index = Some(selected - 1);
                } else if from > selected && to <= selected {
                    self.selected_index = Some(selected + 1);
                }
            }
            
            self.recalculate_layout();
        }
        Ok(())
    }

    /// Execute update operation
    fn execute_update_operation(&mut self, index: usize, item: ListItem) -> Result<()> {
        if index < self.items.len() {
            self.items[index].item = item;
            // Could add fade animation for updates
        }
        Ok(())
    }

    /// Execute clear operation
    fn execute_clear_operation(&mut self) -> Result<()> {
        self.items.clear();
        self.selected_index = None;
        self.scroll_offset = 0;
        Ok(())
    }

    /// Adjust selection after item removal
    fn adjust_selection_after_removal(&mut self, removed_index: usize) {
        if let Some(selected) = self.selected_index {
            if selected == removed_index {
                // Select the next item, or previous if we're at the end
                if selected < self.items.len() {
                    // Keep the same index (item shifted down)
                } else if !self.items.is_empty() {
                    self.selected_index = Some(self.items.len() - 1);
                } else {
                    self.selected_index = None;
                }
            } else if selected > removed_index {
                self.selected_index = Some(selected - 1);
            }
        }
    }

    /// Recalculate layout positions for all items
    fn recalculate_layout(&mut self) {
        let mut y_offset = self.area.y;
        
        for item in &mut self.items {
            item.target_rect = Rect {
                x: self.area.x,
                y: y_offset,
                width: self.area.width,
                height: item.item.height,
            };
            
            if !item.is_animating {
                item.current_rect = item.target_rect;
            }
            
            y_offset += item.item.height;
        }
        
        self.total_height = y_offset.saturating_sub(self.area.y) as u16;
    }

    /// Update all animations
    fn update_animations(&mut self) -> Result<bool> {
        let mut any_updated = false;
        let mut items_to_remove = Vec::new();

        for (index, item) in self.items.iter_mut().enumerate() {
            if let Some(animation) = &mut item.animation {
                animation.update()?;
                any_updated = true;

                if animation.is_complete() {
                    item.is_animating = false;
                    item.animation = None;

                    // Check if this was a remove operation
                    if let Some(ListOperation::Remove { index: remove_index }) = &item.operation {
                        if *remove_index == index {
                            items_to_remove.push(index);
                        }
                    }
                }
            }
        }

        // Remove items that finished their removal animation
        for &index in items_to_remove.iter().rev() {
            self.items.remove(index);
            self.adjust_selection_after_removal(index);
        }

        if !items_to_remove.is_empty() {
            self.recalculate_layout();
        }

        Ok(any_updated)
    }
}

impl Animation for AnimatedList {
    fn start(&mut self) -> Result<()> {
        self.state = AnimationState::Running {
            start_time: Instant::now(),
            current_frame: 0,
        };
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        self.state = AnimationState::Complete;
        
        // Stop all item animations
        for item in &mut self.items {
            if let Some(animation) = &mut item.animation {
                animation.stop()?;
            }
            item.is_animating = false;
            item.animation = None;
        }
        
        Ok(())
    }

    fn update(&mut self) -> Result<()> {
        self.process_operations()?;
        self.update_animations()?;
        
        // Update frame counter if running
        if let AnimationState::Running { start_time, .. } = &self.state {
            let frame_count = (start_time.elapsed().as_millis() / 16) as u32;
            self.state = AnimationState::Running {
                start_time: *start_time,
                current_frame: frame_count,
            };
        }
        
        Ok(())
    }

    fn is_complete(&self) -> bool {
        matches!(self.state, AnimationState::Complete | AnimationState::Idle) &&
        self.items.iter().all(|item| !item.is_animating)
    }

    fn state(&self) -> &AnimationState {
        &self.state
    }

    fn render(&self, _area: Rect, theme: &Theme) -> Vec<Line> {
        let mut lines = Vec::new();
        
        for (index, item) in self.items.iter().enumerate() {
            // Skip items that are outside the visible area
            let item_rect = if item.is_animating {
                if let Some(animation) = &item.animation {
                    // Use animation's current area if available
                    item.current_rect
                } else {
                    item.current_rect
                }
            } else {
                item.target_rect
            };

            // Check if item is visible
            if item_rect.y >= self.area.y + self.area.height ||
               item_rect.y + item_rect.height <= self.area.y {
                continue;
            }

            // Apply selection styling
            let is_selected = self.selected_index == Some(index);
            let item_lines: Vec<Line> = item.item.content
                .iter()
                .map(|line| {
                    if is_selected {
                        let spans: Vec<Span> = line.spans
                            .iter()
                            .map(|span| {
                                let mut style = span.style;
                                style = style.bg(theme.colors.selection);
                                Span::styled(span.content.clone(), style)
                            })
                            .collect();
                        Line::from(spans)
                    } else {
                        line.clone()
                    }
                })
                .collect();

            lines.extend(item_lines);
        }

        lines
    }
}

/// Presets for common animated list scenarios
pub struct AnimatedListPresets;

impl AnimatedListPresets {
    /// Chat message list with smooth additions
    pub fn chat_messages() -> AnimatedList {
        AnimatedList::new(
            ListAnimationConfig::smooth_ios()
                .with_slide_direction(SlideDirection::FromBottom)
        )
    }

    /// File browser with bounce effects
    pub fn file_browser() -> AnimatedList {
        AnimatedList::new(ListAnimationConfig::playful_bounce())
    }

    /// Menu list with material design animations
    pub fn menu_list() -> AnimatedList {
        AnimatedList::new(ListAnimationConfig::material_design())
    }

    /// Todo list with task animations
    pub fn todo_list() -> AnimatedList {
        AnimatedList::new(
            ListAnimationConfig::default()
                .with_slide_direction(SlideDirection::FromRight)
        )
    }

    /// Notification list
    pub fn notification_list() -> AnimatedList {
        AnimatedList::new(
            ListAnimationConfig::default()
                .with_slide_direction(SlideDirection::FromTop)
                .with_stagger_delay(Duration::from_millis(100))
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_item_creation() {
        let item = ListItem::from_text("test1".to_string(), "Hello World".to_string())
            .with_height(2)
            .non_selectable();
        
        assert_eq!(item.id, "test1");
        assert_eq!(item.height, 2);
        assert!(!item.selectable);
    }

    #[test]
    fn test_animated_list_creation() {
        let config = ListAnimationConfig::default();
        let list = AnimatedList::new(config);
        
        assert_eq!(list.items.len(), 0);
        assert!(list.selected_index.is_none());
    }

    #[test]
    fn test_list_operations() {
        let mut list = AnimatedList::new(ListAnimationConfig::default());
        
        let item1 = ListItem::from_text("1".to_string(), "Item 1".to_string());
        let item2 = ListItem::from_text("2".to_string(), "Item 2".to_string());
        
        list.add_item(item1);
        list.add_item(item2);
        
        // Process operations
        list.process_operations().unwrap();
        
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.items[0].item.id, "1");
        assert_eq!(list.items[1].item.id, "2");
    }

    #[test]
    fn test_selection_management() {
        let mut list = AnimatedList::new(ListAnimationConfig::default());
        
        let item1 = ListItem::from_text("1".to_string(), "Item 1".to_string());
        let item2 = ListItem::from_text("2".to_string(), "Item 2".to_string());
        
        list.add_item(item1);
        list.add_item(item2);
        list.process_operations().unwrap();
        
        // Test selection
        list.set_selected(Some(0));
        assert_eq!(list.selected_index, Some(0));
        
        list.select_next();
        assert_eq!(list.selected_index, Some(1));
        
        list.select_previous();
        assert_eq!(list.selected_index, Some(0));
    }

    #[test]
    fn test_list_presets() {
        let chat = AnimatedListPresets::chat_messages();
        let file_browser = AnimatedListPresets::file_browser();
        let menu = AnimatedListPresets::menu_list();
        
        // Just verify they can be created without panicking
        assert_eq!(chat.items.len(), 0);
        assert_eq!(file_browser.items.len(), 0);
        assert_eq!(menu.items.len(), 0);
    }
}