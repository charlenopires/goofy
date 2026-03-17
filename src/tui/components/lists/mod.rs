//! Advanced list components with virtual scrolling and filtering capabilities.
//!
//! This module provides sophisticated list implementations optimized for
//! performance with large datasets, including virtual scrolling, filtering,
//! multi-selection, and lazy loading capabilities.

pub mod virtual_list;
pub mod filterable_list;
pub mod navigation;
pub mod selection;
pub mod lazy_loading;
pub mod pagination;

pub use virtual_list::*;
pub use filterable_list::*;
pub use navigation::*;
pub use selection::*;
pub use lazy_loading::*;
pub use pagination::*;

use ratatui::{
    style::Style,
    text::Line,
};
use std::fmt::Debug;

/// Base trait for list items that can be displayed in advanced lists
pub trait ListItem: Debug + Clone + Send + Sync {
    /// Unique identifier for the item
    fn id(&self) -> String;
    
    /// Get the display content for this item
    fn content(&self) -> Vec<Line<'static>>;
    
    /// Get the height in lines that this item requires
    fn height(&self) -> u16;
    
    /// Whether this item can be selected
    fn selectable(&self) -> bool {
        true
    }
    
    /// Whether this item acts as a section header
    fn is_section_header(&self) -> bool {
        false
    }
    
    /// Custom styling for this item
    fn style(&self) -> Option<Style> {
        None
    }
    
    /// Optional data payload for the item
    fn data(&self) -> Option<serde_json::Value> {
        None
    }
}

/// Trait for items that can be filtered
pub trait FilterableItem: ListItem {
    /// Get the text content used for filtering
    fn filter_value(&self) -> String;
    
    /// Get fuzzy search match indices for highlighting
    fn match_indices(&self) -> &[usize];
    
    /// Set fuzzy search match indices
    fn set_match_indices(&mut self, indices: Vec<usize>);
}

/// Trait for items that can be indexed for performance
pub trait IndexableItem: ListItem {
    /// Get the current index in the list
    fn index(&self) -> usize;
    
    /// Set the index for this item
    fn set_index(&mut self, index: usize);
}

/// Direction for list navigation and scrolling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Forward direction (top to bottom)
    Forward,
    /// Backward direction (bottom to top)
    Backward,
}

/// List operation types for batch updates
#[derive(Debug, Clone, PartialEq)]
pub enum ListOperation<T: ListItem> {
    /// Add item at specific index
    Insert { index: usize, item: T },
    /// Remove item at index
    Remove { index: usize },
    /// Move item from one index to another
    Move { from: usize, to: usize },
    /// Update item at index
    Update { index: usize, item: T },
    /// Replace all items
    Replace { items: Vec<T> },
    /// Clear all items
    Clear,
    /// Batch multiple operations
    Batch(Vec<ListOperation<T>>),
}

/// List event types for notifications
#[derive(Debug, Clone)]
pub enum ListEvent<T: ListItem> {
    /// Selection changed
    SelectionChanged {
        previous: Option<String>,
        current: Option<String>,
    },
    /// Item was activated (e.g., double-clicked or pressed Enter)
    ItemActivated {
        item_id: String,
        item: T,
    },
    /// Items were filtered
    ItemsFiltered {
        query: String,
        matched_count: usize,
        total_count: usize,
    },
    /// Scroll position changed
    ScrollChanged {
        offset: usize,
        total_height: usize,
    },
}

/// Performance metrics for list operations
#[derive(Debug, Clone, Default)]
pub struct ListMetrics {
    /// Number of items currently rendered
    pub rendered_items: usize,
    /// Total number of items in the list
    pub total_items: usize,
    /// Number of visible items in viewport
    pub visible_items: usize,
    /// Current scroll offset
    pub scroll_offset: usize,
    /// Time spent on last render (microseconds)
    pub render_time_us: u64,
    /// Memory usage estimate (bytes)
    pub memory_usage_bytes: usize,
}

/// Configuration for list behavior
#[derive(Debug, Clone)]
pub struct ListConfig {
    /// Whether to enable wrap-around navigation
    pub wrap_navigation: bool,
    /// Whether to enable mouse interactions
    pub enable_mouse: bool,
    /// Whether to enable smooth scrolling animations
    pub smooth_scrolling: bool,
    /// Page size for page up/down operations
    pub page_size: Option<usize>,
    /// Gap between items in lines
    pub item_gap: u16,
    /// Whether to auto-resize items to fit content
    pub auto_resize_items: bool,
    /// Maximum items to render at once (virtual scrolling)
    pub max_rendered_items: usize,
    /// Buffer size for virtual scrolling (items above/below visible area)
    pub virtual_buffer_size: usize,
}

impl Default for ListConfig {
    fn default() -> Self {
        Self {
            wrap_navigation: false,
            enable_mouse: true,
            smooth_scrolling: true,
            page_size: None, // Will use viewport height
            item_gap: 0,
            auto_resize_items: true,
            max_rendered_items: 1000,
            virtual_buffer_size: 50,
        }
    }
}

impl ListConfig {
    /// Create a new list configuration
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Enable wrap-around navigation
    pub fn with_wrap_navigation(mut self) -> Self {
        self.wrap_navigation = true;
        self
    }
    
    /// Disable mouse interactions
    pub fn without_mouse(mut self) -> Self {
        self.enable_mouse = false;
        self
    }
    
    /// Disable smooth scrolling
    pub fn without_smooth_scrolling(mut self) -> Self {
        self.smooth_scrolling = false;
        self
    }
    
    /// Set custom page size
    pub fn with_page_size(mut self, size: usize) -> Self {
        self.page_size = Some(size);
        self
    }
    
    /// Set item gap
    pub fn with_item_gap(mut self, gap: u16) -> Self {
        self.item_gap = gap;
        self
    }
    
    /// Set maximum rendered items for virtual scrolling
    pub fn with_max_rendered_items(mut self, max: usize) -> Self {
        self.max_rendered_items = max;
        self
    }
    
    /// Set virtual buffer size
    pub fn with_virtual_buffer_size(mut self, size: usize) -> Self {
        self.virtual_buffer_size = size;
        self
    }
    
    /// Performance preset for large lists (100k+ items)
    pub fn large_list_preset() -> Self {
        Self {
            max_rendered_items: 500,
            virtual_buffer_size: 100,
            smooth_scrolling: false,
            auto_resize_items: false,
            ..Default::default()
        }
    }
    
    /// Performance preset for chat/message lists
    pub fn chat_list_preset() -> Self {
        Self {
            wrap_navigation: false,
            smooth_scrolling: true,
            item_gap: 1,
            max_rendered_items: 200,
            virtual_buffer_size: 20,
            ..Default::default()
        }
    }
    
    /// Performance preset for file/directory lists
    pub fn file_list_preset() -> Self {
        Self {
            wrap_navigation: true,
            enable_mouse: true,
            item_gap: 0,
            max_rendered_items: 1000,
            virtual_buffer_size: 50,
            ..Default::default()
        }
    }
}

/// Simple list item implementation for common use cases
#[derive(Debug, Clone)]
pub struct SimpleListItem {
    pub id: String,
    pub content: Vec<Line<'static>>,
    pub height: u16,
    pub selectable: bool,
    pub style: Option<Style>,
    pub data: Option<serde_json::Value>,
}

impl SimpleListItem {
    /// Create a new simple list item from text
    pub fn from_text(id: String, text: String) -> Self {
        Self {
            id,
            content: vec![Line::from(text)],
            height: 1,
            selectable: true,
            style: None,
            data: None,
        }
    }
    
    /// Create a new simple list item with custom content
    pub fn new(id: String, content: Vec<Line<'static>>) -> Self {
        let height = content.len() as u16;
        Self {
            id,
            content,
            height,
            selectable: true,
            style: None,
            data: None,
        }
    }
    
    /// Set custom height
    pub fn with_height(mut self, height: u16) -> Self {
        self.height = height;
        self
    }
    
    /// Make item non-selectable
    pub fn non_selectable(mut self) -> Self {
        self.selectable = false;
        self
    }
    
    /// Set custom style
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }
    
    /// Add data payload
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }
}

impl ListItem for SimpleListItem {
    fn id(&self) -> String {
        self.id.clone()
    }
    
    fn content(&self) -> Vec<Line<'static>> {
        self.content.clone()
    }
    
    fn height(&self) -> u16 {
        self.height
    }
    
    fn selectable(&self) -> bool {
        self.selectable
    }
    
    fn style(&self) -> Option<Style> {
        self.style
    }
    
    fn data(&self) -> Option<serde_json::Value> {
        self.data.clone()
    }
}