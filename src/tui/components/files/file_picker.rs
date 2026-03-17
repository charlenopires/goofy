//! File picker component with image preview and directory navigation.
//!
//! This module provides a comprehensive file picker interface that supports:
//! - Directory navigation with breadcrumbs
//! - File filtering by type/extension
//! - Image preview for supported formats
//! - Keyboard and mouse navigation
//! - File size and permission validation

use super::{FileEvent, FileItem, StandardFileItem, validate_file_path, is_file_too_large};
use crate::tui::{
    components::{Component, lists::VirtualList},
    themes::Theme,
    Frame,
};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Maximum file size for attachments (5MB)
pub const MAX_ATTACHMENT_SIZE: u64 = 5 * 1024 * 1024;

/// Supported image extensions
pub const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "gif", "svg"];

/// File picker component
pub struct FilePicker {
    /// Current directory
    current_directory: PathBuf,
    
    /// Available files and directories
    items: Vec<StandardFileItem>,
    
    /// Selected item index
    selected_index: usize,
    
    /// Virtual list for efficient rendering
    virtual_list: VirtualList<StandardFileItem>,
    
    /// File picker configuration
    config: FilePickerConfig,
    
    /// Current preview content
    preview_content: Option<PreviewContent>,
    
    /// Loading state
    is_loading: bool,
    
    /// Error message
    error_message: Option<String>,
    
    /// Event callbacks
    callbacks: Vec<Box<dyn Fn(FileEvent) + Send + Sync>>,
    
    /// Component state
    state: FilePickerState,
    
    /// Size and position
    area: Rect,
    
    /// Whether component has focus
    has_focus: bool,
}

/// File picker configuration
#[derive(Debug, Clone)]
pub struct FilePickerConfig {
    /// Allowed file extensions (None = all files)
    pub allowed_extensions: Option<Vec<String>>,
    
    /// Maximum file size in bytes
    pub max_file_size: u64,
    
    /// Whether to show hidden files
    pub show_hidden: bool,
    
    /// Whether to show file permissions
    pub show_permissions: bool,
    
    /// Whether to show file sizes
    pub show_sizes: bool,
    
    /// Whether to enable image preview
    pub enable_preview: bool,
    
    /// Starting directory
    pub start_directory: Option<PathBuf>,
    
    /// Whether to allow directory selection
    pub allow_directory_selection: bool,
    
    /// Whether to show breadcrumbs
    pub show_breadcrumbs: bool,
    
    /// Preview panel width percentage (0-100)
    pub preview_width_percent: u16,
}

impl Default for FilePickerConfig {
    fn default() -> Self {
        Self {
            allowed_extensions: Some(IMAGE_EXTENSIONS.iter().map(|&s| s.to_string()).collect()),
            max_file_size: MAX_ATTACHMENT_SIZE,
            show_hidden: false,
            show_permissions: false,
            show_sizes: true,
            enable_preview: true,
            start_directory: None,
            allow_directory_selection: false,
            show_breadcrumbs: true,
            preview_width_percent: 40,
        }
    }
}

/// Preview content for files
#[derive(Debug, Clone)]
enum PreviewContent {
    /// Image preview (width, height, content)
    Image { width: u16, height: u16, content: String },
    
    /// Text preview
    Text { content: String },
    
    /// Binary file info
    Binary { size: u64, mime_type: String },
    
    /// Loading indicator
    Loading,
    
    /// Error message
    Error { message: String },
}

/// File picker internal state
#[derive(Debug, Clone, Copy, PartialEq)]
enum FilePickerState {
    /// Normal browsing mode
    Browse,
    
    /// Loading directory contents
    Loading,
    
    /// Error state
    Error,
}

impl std::fmt::Debug for FilePicker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilePicker")
            .field("current_directory", &self.current_directory)
            .field("items", &self.items.len())
            .field("selected_index", &self.selected_index)
            .field("config", &self.config)
            .field("is_loading", &self.is_loading)
            .field("error_message", &self.error_message)
            .field("callbacks", &format!("[{} callbacks]", self.callbacks.len()))
            .field("state", &self.state)
            .field("area", &self.area)
            .field("has_focus", &self.has_focus)
            .finish()
    }
}

impl FilePicker {
    /// Create a new file picker with default configuration
    pub fn new() -> Self {
        Self::with_config(FilePickerConfig::default())
    }
    
    /// Create a new file picker with custom configuration
    pub fn with_config(config: FilePickerConfig) -> Self {
        let start_dir = config.start_directory.clone()
            .or_else(|| std::env::current_dir().ok())
            .or_else(|| dirs::home_dir())
            .unwrap_or_else(|| PathBuf::from("/"));
        
        let mut picker = Self {
            current_directory: start_dir,
            items: Vec::new(),
            selected_index: 0,
            virtual_list: VirtualList::default(),
            config,
            preview_content: None,
            is_loading: false,
            error_message: None,
            callbacks: Vec::new(),
            state: FilePickerState::Browse,
            area: Rect::default(),
            has_focus: false,
        };
        
        picker.load_directory();
        picker
    }
    
    /// Add an event callback
    pub fn add_callback<F>(&mut self, callback: F)
    where
        F: Fn(FileEvent) + Send + Sync + 'static,
    {
        self.callbacks.push(Box::new(callback));
    }
    
    /// Load the current directory contents
    fn load_directory(&mut self) {
        self.state = FilePickerState::Loading;
        self.is_loading = true;
        self.error_message = None;
        
        match self.read_directory(&self.current_directory) {
            Ok(items) => {
                self.items = items;
                self.selected_index = 0;
                self.virtual_list.set_items(self.items.clone());
                self.state = FilePickerState::Browse;
                self.update_preview();
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to read directory: {}", e));
                self.state = FilePickerState::Error;
                self.emit_event(FileEvent::Error {
                    message: e.to_string(),
                });
            }
        }
        
        self.is_loading = false;
    }
    
    /// Read directory contents and create file items
    fn read_directory(&self, path: &Path) -> Result<Vec<StandardFileItem>> {
        if let Err(e) = validate_file_path(path) {
            return Err(e);
        }
        
        let mut items = Vec::new();
        
        // Add parent directory entry if not at root
        if path.parent().is_some() {
            let mut parent = StandardFileItem::from_path(path.parent().unwrap())?;
            parent.name = "..".to_string();
            items.push(parent);
        }
        
        // Read directory entries
        let entries = std::fs::read_dir(path)?;
        
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            // Skip hidden files if not configured to show them
            if !self.config.show_hidden {
                if let Some(name) = path.file_name() {
                    if name.to_string_lossy().starts_with('.') {
                        continue;
                    }
                }
            }
            
            match StandardFileItem::from_path(&path) {
                Ok(item) => {
                    // Filter by allowed extensions for files
                    if item.is_file() {
                        if let Some(ref allowed) = self.config.allowed_extensions {
                            if let Some(ext) = item.extension() {
                                if !allowed.contains(&ext.to_lowercase()) {
                                    continue;
                                }
                            } else {
                                // No extension, skip if we have extension filters
                                continue;
                            }
                        }
                    }
                    
                    items.push(item);
                }
                Err(e) => {
                    eprintln!("Failed to read file item {}: {}", path.display(), e);
                }
            }
        }
        
        // Sort items: directories first, then files, both alphabetically
        items.sort_by(|a, b| {
            match (a.is_directory(), b.is_directory()) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name().to_lowercase().cmp(&b.name().to_lowercase()),
            }
        });
        
        Ok(items)
    }
    
    /// Update preview for the current selection
    fn update_preview(&mut self) {
        if !self.config.enable_preview || self.items.is_empty() {
            self.preview_content = None;
            return;
        }
        
        if let Some(item) = self.items.get(self.selected_index) {
            if item.is_file() {
                self.preview_content = Some(PreviewContent::Loading);
                
                match self.generate_preview(item) {
                    Ok(content) => self.preview_content = Some(content),
                    Err(e) => {
                        self.preview_content = Some(PreviewContent::Error {
                            message: format!("Preview error: {}", e),
                        });
                    }
                }
            } else {
                self.preview_content = None;
            }
        }
    }
    
    /// Generate preview content for a file item
    fn generate_preview(&self, item: &StandardFileItem) -> Result<PreviewContent> {
        let path = item.path();
        
        // Check file size
        if let Some(size) = item.size() {
            if size > self.config.max_file_size {
                return Ok(PreviewContent::Binary {
                    size,
                    mime_type: item.mime_type().unwrap_or_else(|| "application/octet-stream".to_string()),
                });
            }
        }
        
        // Generate preview based on file type
        match item.extension() {
            Some("jpg") | Some("jpeg") | Some("png") | Some("gif") => {
                self.generate_image_preview(path)
            }
            Some("txt") | Some("md") | Some("rs") | Some("go") | Some("py") | Some("js") | Some("html") | Some("css") | Some("json") | Some("yaml") | Some("yml") | Some("toml") => {
                self.generate_text_preview(path)
            }
            _ => {
                Ok(PreviewContent::Binary {
                    size: item.size().unwrap_or(0),
                    mime_type: item.mime_type().unwrap_or_else(|| "application/octet-stream".to_string()),
                })
            }
        }
    }
    
    /// Generate image preview
    fn generate_image_preview(&self, path: &Path) -> Result<PreviewContent> {
        // For now, just show image info
        // In a full implementation, you'd use an image library to render ASCII art or similar
        let metadata = std::fs::metadata(path)?;
        Ok(PreviewContent::Image {
            width: 0,
            height: 0,
            content: format!("Image: {}\nSize: {} bytes", path.display(), metadata.len()),
        })
    }
    
    /// Generate text preview
    fn generate_text_preview(&self, path: &Path) -> Result<PreviewContent> {
        let content = std::fs::read_to_string(path)?;
        let preview = if content.len() > 1000 {
            format!("{}...", &content[..1000])
        } else {
            content
        };
        
        Ok(PreviewContent::Text { content: preview })
    }
    
    /// Navigate to a directory
    fn navigate_to(&mut self, path: PathBuf) -> Result<()> {
        self.current_directory = path.canonicalize()?;
        self.load_directory();
        self.emit_event(FileEvent::DirectoryOpened {
            path: self.current_directory.clone(),
        });
        Ok(())
    }
    
    /// Select the current item
    fn select_current_item(&mut self) -> Result<()> {
        if let Some(item) = self.items.get(self.selected_index) {
            if item.is_directory() {
                if self.config.allow_directory_selection {
                    self.emit_event(FileEvent::FileSelected {
                        path: item.path().to_path_buf(),
                    });
                } else {
                    self.navigate_to(item.path().to_path_buf())?;
                }
            } else {
                // Validate file before selection
                if let Err(e) = is_file_too_large(item.path(), self.config.max_file_size) {
                    self.error_message = Some(format!("File too large: {}", e));
                    return Ok(());
                }
                
                self.emit_event(FileEvent::FileSelected {
                    path: item.path().to_path_buf(),
                });
            }
        }
        Ok(())
    }
    
    /// Get the item ID for a given index
    fn item_id_at(&self, index: usize) -> Option<String> {
        use crate::tui::components::lists::ListItem;
        self.items.get(index).map(|item| item.id())
    }

    /// Select the item at the given index in the virtual list
    fn select_virtual_list_item(&mut self, index: usize) {
        if let Some(id) = self.item_id_at(index) {
            let _ = self.virtual_list.set_selected(Some(id));
        }
    }

    /// Move selection up
    fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.select_virtual_list_item(self.selected_index);
            self.update_preview();
        }
    }
    
    /// Move selection down
    fn move_selection_down(&mut self) {
        if self.selected_index < self.items.len().saturating_sub(1) {
            self.selected_index += 1;
            self.select_virtual_list_item(self.selected_index);
            self.update_preview();
        }
    }
    
    /// Navigate to parent directory
    fn go_to_parent(&mut self) -> Result<()> {
        if let Some(parent) = self.current_directory.parent() {
            self.navigate_to(parent.to_path_buf())
        } else {
            Ok(())
        }
    }
    
    /// Emit an event to all callbacks
    fn emit_event(&self, event: FileEvent) {
        for callback in &self.callbacks {
            callback(event.clone());
        }
    }
    
    /// Render breadcrumbs
    fn render_breadcrumbs(&self, area: Rect, theme: &Theme) -> Paragraph {
        let mut spans = Vec::new();
        
        // Home icon
        spans.push(Span::styled("🏠 ", Style::default().fg(theme.colors.primary)));
        
        // Path components
        let components: Vec<_> = self.current_directory.components().collect();
        for (i, component) in components.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" / ", Style::default().fg(theme.colors.muted)));
            }
            
            let name = match component {
                std::path::Component::RootDir => "".to_string(),
                std::path::Component::Normal(name) => name.to_string_lossy().to_string(),
                _ => component.as_os_str().to_string_lossy().to_string(),
            };
            
            if !name.is_empty() {
                spans.push(Span::styled(name, Style::default().fg(theme.colors.text)));
            }
        }
        
        Paragraph::new(Line::from(spans))
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: true })
    }
    
    /// Render file list
    fn render_file_list(&mut self, area: Rect, theme: &Theme) {
        self.virtual_list.set_area(area);
        
        // Update virtual list selection
        if !self.items.is_empty() {
            self.select_virtual_list_item(self.selected_index);
        }
    }
    
    /// Render preview panel
    fn render_preview(&self, area: Rect, theme: &Theme) -> Block {
        let mut block = Block::default()
            .title("Preview")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.border));
        
        if let Some(ref content) = self.preview_content {
            match content {
                PreviewContent::Image { content, .. } => {
                    block = block.title("Image Preview");
                }
                PreviewContent::Text { .. } => {
                    block = block.title("Text Preview");
                }
                PreviewContent::Binary { size, mime_type } => {
                    block = block.title(format!("Binary File ({}, {})", 
                        super::format_file_size(*size), mime_type));
                }
                PreviewContent::Loading => {
                    block = block.title("Loading...");
                }
                PreviewContent::Error { .. } => {
                    block = block.title("Preview Error").border_style(Style::default().fg(Color::Red));
                }
            }
        }
        
        block
    }
    
    /// Get current file path
    pub fn current_path(&self) -> &Path {
        &self.current_directory
    }
    
    /// Get selected item
    pub fn selected_item(&self) -> Option<&StandardFileItem> {
        self.items.get(self.selected_index)
    }
}

#[async_trait::async_trait]
impl Component for FilePicker {
    async fn handle_key_event(&mut self, event: KeyEvent) -> Result<()> {
        if !self.has_focus {
            return Ok(());
        }
        
        match event.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_selection_up();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_selection_down();
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.select_current_item()?;
            }
            KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => {
                self.go_to_parent()?;
            }
            KeyCode::Char('r') => {
                self.load_directory();
            }
            KeyCode::Char('H') => {
                self.config.show_hidden = !self.config.show_hidden;
                self.load_directory();
            }
            KeyCode::Home => {
                self.selected_index = 0;
                if !self.items.is_empty() {
                    self.select_virtual_list_item(0);
                    self.update_preview();
                }
            }
            KeyCode::End => {
                if !self.items.is_empty() {
                    self.selected_index = self.items.len() - 1;
                    self.select_virtual_list_item(self.selected_index);
                    self.update_preview();
                }
            }
            KeyCode::PageUp => {
                let page_size = self.area.height as usize / 2;
                self.selected_index = self.selected_index.saturating_sub(page_size);
                self.select_virtual_list_item(self.selected_index);
                self.update_preview();
            }
            KeyCode::PageDown => {
                let page_size = self.area.height as usize / 2;
                self.selected_index = (self.selected_index + page_size).min(self.items.len().saturating_sub(1));
                self.select_virtual_list_item(self.selected_index);
                self.update_preview();
            }
            _ => {}
        }
        
        Ok(())
    }
    
    async fn handle_mouse_event(&mut self, event: MouseEvent) -> Result<()> {
        // Mouse support for clicking on files
        // Implementation would depend on exact mouse coordinates
        Ok(())
    }
    
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.area = area;
        
        // Clear the area
        frame.render_widget(Clear, area);
        
        let main_block = Block::default()
            .title("File Picker")
            .borders(Borders::ALL)
            .border_style(if self.has_focus {
                Style::default().fg(theme.colors.primary)
            } else {
                Style::default().fg(theme.colors.border)
            });
        
        frame.render_widget(main_block, area);
        
        let inner = area.inner(&ratatui::layout::Margin { horizontal: 1, vertical: 1 });
        
        // Layout: breadcrumbs, file list, and optional preview
        let (list_area, preview_area) = if self.config.enable_preview {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(100 - self.config.preview_width_percent),
                    Constraint::Percentage(self.config.preview_width_percent),
                ])
                .split(inner);
            (chunks[0], Some(chunks[1]))
        } else {
            (inner, None)
        };
        
        let list_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(if self.config.show_breadcrumbs { 3 } else { 0 }),
                Constraint::Min(0),
            ])
            .split(list_area);
        
        // Render breadcrumbs
        if self.config.show_breadcrumbs {
            let breadcrumbs = self.render_breadcrumbs(list_chunks[0], theme);
            frame.render_widget(breadcrumbs, list_chunks[0]);
        }
        
        // Render file list
        let list_area = if self.config.show_breadcrumbs {
            list_chunks[1]
        } else {
            list_chunks[0]
        };
        
        self.render_file_list(list_area, theme);
        if let Ok(lines) = self.virtual_list.render(theme) {
            let text = ratatui::text::Text::from(lines);
            let paragraph = ratatui::widgets::Paragraph::new(text);
            frame.render_widget(paragraph, list_area);
        }
        
        // Render preview panel
        if let Some(preview_area) = preview_area {
            let preview_block = self.render_preview(preview_area, theme);
            frame.render_widget(preview_block, preview_area);
            
            // Render preview content
            if let Some(ref content) = self.preview_content {
                let content_area = preview_area.inner(&ratatui::layout::Margin { horizontal: 1, vertical: 1 });
                
                let preview_widget = match content {
                    PreviewContent::Text { content } => {
                        Paragraph::new(content.as_str())
                            .wrap(Wrap { trim: true })
                            .style(Style::default().fg(theme.colors.text))
                    }
                    PreviewContent::Image { content, .. } => {
                        Paragraph::new(content.as_str())
                            .wrap(Wrap { trim: true })
                            .style(Style::default().fg(theme.colors.text))
                    }
                    PreviewContent::Binary { size, mime_type } => {
                        Paragraph::new(format!("Binary file\nType: {}\nSize: {}", 
                            mime_type, super::format_file_size(*size)))
                            .style(Style::default().fg(theme.colors.muted))
                    }
                    PreviewContent::Loading => {
                        Paragraph::new("Loading preview...")
                            .style(Style::default().fg(theme.colors.muted))
                    }
                    PreviewContent::Error { message } => {
                        Paragraph::new(message.as_str())
                            .style(Style::default().fg(Color::Red))
                    }
                };
                
                frame.render_widget(preview_widget, content_area);
            }
        }
        
        // Render error message if present
        if let Some(ref error) = self.error_message {
            let error_area = Rect {
                x: area.x + 2,
                y: area.y + area.height - 3,
                width: area.width - 4,
                height: 1,
            };
            
            let error_widget = Paragraph::new(error.as_str())
                .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));
            
            frame.render_widget(error_widget, error_area);
        }
    }
    
    fn size(&self) -> Rect {
        self.area
    }
    
    fn set_size(&mut self, size: Rect) {
        self.area = size;
    }
    
    fn has_focus(&self) -> bool {
        self.has_focus
    }
    
    fn set_focus(&mut self, focus: bool) {
        self.has_focus = focus;
    }
}

impl Default for FilePicker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_file_picker_creation() {
        let picker = FilePicker::new();
        assert!(!picker.items.is_empty() || picker.state == FilePickerState::Error);
    }
    
    #[test]
    fn test_file_picker_with_temp_dir() {
        let temp_dir = TempDir::new().unwrap();
        let config = FilePickerConfig {
            start_directory: Some(temp_dir.path().to_path_buf()),
            ..Default::default()
        };
        
        let picker = FilePicker::with_config(config);
        assert_eq!(picker.current_directory, temp_dir.path());
    }
    
    #[test]
    fn test_allowed_extensions() {
        let config = FilePickerConfig {
            allowed_extensions: Some(vec!["txt".to_string(), "md".to_string()]),
            ..Default::default()
        };
        
        let _picker = FilePicker::with_config(config);
        // Test would require creating actual files to verify filtering
    }
}