//! Chat sidebar component for session management and navigation
//!
//! This module provides a sidebar component that displays session lists,
//! file trees, tool status, and other navigation elements.

use super::message_types::ChatMessage;
use crate::{
    session::{Session, SessionManager},
    tui::{
        components::{Component, ComponentState, ListView, Scrollable},
        themes::{Theme, ThemeManager},
        Frame,
    },
};
use anyhow::Result;
use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

/// Sidebar component for session and file management
pub struct ChatSidebar {
    state: ComponentState,
    theme_manager: ThemeManager,
    
    // Display modes
    mode: SidebarMode,
    compact_mode: bool,
    auto_hide: bool,
    
    // Session management
    sessions: Vec<Session>,
    session_list_state: ListState,
    selected_session_id: Option<String>,
    
    // File tree
    file_tree: FileTree,
    file_tree_expanded: bool,
    
    // Tool status
    tool_statuses: HashMap<String, ToolStatus>,
    
    // Search functionality
    search_mode: bool,
    search_query: String,
    filtered_sessions: Vec<usize>, // Indices into sessions vec
    
    // Animation state
    last_update: Instant,
    scroll_animation: ScrollAnimation,
    
    // Configuration
    config: SidebarConfig,
}

/// Sidebar display modes
#[derive(Debug, Clone, PartialEq)]
pub enum SidebarMode {
    /// Show session list
    Sessions,
    /// Show file tree
    Files,
    /// Show tool status
    Tools,
    /// Show both sessions and files
    Mixed,
}

/// File tree structure
#[derive(Debug, Clone)]
pub struct FileTree {
    root: PathBuf,
    nodes: Vec<FileNode>,
    expanded_paths: std::collections::HashSet<PathBuf>,
    selected_path: Option<PathBuf>,
}

/// File tree node
#[derive(Debug, Clone)]
pub struct FileNode {
    path: PathBuf,
    name: String,
    is_directory: bool,
    is_expanded: bool,
    depth: usize,
    children: Vec<usize>, // Indices into the nodes vec
    modified_time: Option<std::time::SystemTime>,
    size: Option<u64>,
}

/// Tool status information
#[derive(Debug, Clone)]
pub struct ToolStatus {
    name: String,
    status: ToolState,
    last_used: Option<Instant>,
    error_count: usize,
    success_count: usize,
}

/// Tool states
#[derive(Debug, Clone, PartialEq)]
pub enum ToolState {
    Available,
    Running,
    Error(String),
    Disabled,
}

/// Sidebar configuration
#[derive(Debug, Clone)]
pub struct SidebarConfig {
    pub default_mode: SidebarMode,
    pub show_session_count: bool,
    pub show_file_tree: bool,
    pub show_tool_status: bool,
    pub max_sessions_displayed: usize,
    pub max_files_displayed: usize,
    pub auto_refresh_interval: Duration,
    pub show_recent_files: bool,
    pub show_git_status: bool,
    pub enable_file_preview: bool,
}

impl Default for SidebarConfig {
    fn default() -> Self {
        Self {
            default_mode: SidebarMode::Sessions,
            show_session_count: true,
            show_file_tree: true,
            show_tool_status: true,
            max_sessions_displayed: 20,
            max_files_displayed: 15,
            auto_refresh_interval: Duration::from_secs(30),
            show_recent_files: true,
            show_git_status: false,
            enable_file_preview: false,
        }
    }
}

/// Scroll animation state
#[derive(Debug, Clone)]
struct ScrollAnimation {
    target_scroll: usize,
    current_scroll: f32,
    animation_speed: f32,
    is_animating: bool,
}

impl ChatSidebar {
    /// Create a new chat sidebar
    pub fn new() -> Self {
        Self {
            state: ComponentState::new(),
            theme_manager: ThemeManager::new(),
            mode: SidebarMode::Sessions,
            compact_mode: false,
            auto_hide: false,
            sessions: Vec::new(),
            session_list_state: ListState::default(),
            selected_session_id: None,
            file_tree: FileTree::new(PathBuf::from(".")),
            file_tree_expanded: false,
            tool_statuses: HashMap::new(),
            search_mode: false,
            search_query: String::new(),
            filtered_sessions: Vec::new(),
            last_update: Instant::now(),
            scroll_animation: ScrollAnimation::new(),
            config: SidebarConfig::default(),
        }
    }

    /// Create sidebar with configuration
    pub fn with_config(config: SidebarConfig) -> Self {
        let mut sidebar = Self::new();
        sidebar.config = config;
        sidebar.mode = sidebar.config.default_mode.clone();
        sidebar
    }

    /// Set sidebar mode
    pub fn set_mode(&mut self, mode: SidebarMode) {
        self.mode = mode;
    }

    /// Get current mode
    pub fn get_mode(&self) -> &SidebarMode {
        &self.mode
    }

    /// Set compact mode
    pub fn set_compact_mode(&mut self, compact: bool) {
        self.compact_mode = compact;
    }

    /// Set sessions list
    pub fn set_sessions(&mut self, sessions: Vec<Session>) {
        self.sessions = sessions;
        self.update_filtered_sessions();
    }

    /// Add a session
    pub fn add_session(&mut self, session: Session) {
        self.sessions.push(session);
        self.update_filtered_sessions();
    }

    /// Remove a session
    pub fn remove_session(&mut self, session_id: &str) {
        self.sessions.retain(|s| s.id != session_id);
        self.update_filtered_sessions();
        
        if self.selected_session_id.as_ref() == Some(&session_id.to_string()) {
            self.selected_session_id = None;
        }
    }

    /// Select a session
    pub fn select_session(&mut self, session_id: Option<String>) {
        self.selected_session_id = session_id;
        
        // Update list state to reflect selection
        if let Some(ref id) = self.selected_session_id {
            if let Some(index) = self.sessions.iter().position(|s| &s.id == id) {
                self.session_list_state.select(Some(index));
            }
        } else {
            self.session_list_state.select(None);
        }
    }

    /// Get selected session
    pub fn get_selected_session(&self) -> Option<&Session> {
        self.selected_session_id.as_ref()
            .and_then(|id| self.sessions.iter().find(|s| &s.id == id))
    }

    /// Set file tree root
    pub fn set_file_tree_root(&mut self, root: PathBuf) {
        self.file_tree = FileTree::new(root);
    }

    /// Refresh file tree
    pub fn refresh_file_tree(&mut self) -> Result<()> {
        self.file_tree.refresh()
    }

    /// Update tool status
    pub fn update_tool_status(&mut self, name: String, status: ToolStatus) {
        self.tool_statuses.insert(name, status);
    }

    /// Start search mode
    pub fn start_search(&mut self) {
        self.search_mode = true;
        self.search_query.clear();
        self.update_filtered_sessions();
    }

    /// Stop search mode
    pub fn stop_search(&mut self) {
        self.search_mode = false;
        self.search_query.clear();
        self.update_filtered_sessions();
    }

    /// Update search query
    pub fn update_search_query(&mut self, query: String) {
        self.search_query = query;
        self.update_filtered_sessions();
    }

    /// Navigate up in current list
    pub fn navigate_up(&mut self) {
        match self.mode {
            SidebarMode::Sessions | SidebarMode::Mixed => {
                let selected = self.session_list_state.selected().unwrap_or(0);
                if selected > 0 {
                    self.session_list_state.select(Some(selected - 1));
                    self.update_selected_session();
                }
            }
            SidebarMode::Files => {
                self.file_tree.navigate_up();
            }
            SidebarMode::Tools => {
                // TODO: Implement tool navigation
            }
        }
    }

    /// Navigate down in current list
    pub fn navigate_down(&mut self) {
        match self.mode {
            SidebarMode::Sessions | SidebarMode::Mixed => {
                let max_index = if self.search_mode {
                    self.filtered_sessions.len().saturating_sub(1)
                } else {
                    self.sessions.len().saturating_sub(1)
                };
                
                let selected = self.session_list_state.selected().unwrap_or(0);
                if selected < max_index {
                    self.session_list_state.select(Some(selected + 1));
                    self.update_selected_session();
                }
            }
            SidebarMode::Files => {
                self.file_tree.navigate_down();
            }
            SidebarMode::Tools => {
                // TODO: Implement tool navigation
            }
        }
    }

    /// Activate selected item
    pub fn activate_selected(&mut self) -> Option<SidebarAction> {
        match self.mode {
            SidebarMode::Sessions | SidebarMode::Mixed => {
                if let Some(session) = self.get_selected_session() {
                    Some(SidebarAction::SessionSelected(session.id.clone()))
                } else {
                    None
                }
            }
            SidebarMode::Files => {
                if let Some(path) = self.file_tree.get_selected_path().cloned() {
                    if path.is_file() {
                        Some(SidebarAction::FileSelected(path))
                    } else {
                        self.file_tree.toggle_expanded(&path);
                        None
                    }
                } else {
                    None
                }
            }
            SidebarMode::Tools => {
                // TODO: Implement tool activation
                None
            }
        }
    }

    /// Update filtered sessions based on search query
    fn update_filtered_sessions(&mut self) {
        if self.search_mode && !self.search_query.is_empty() {
            let query = self.search_query.to_lowercase();
            self.filtered_sessions = self.sessions
                .iter()
                .enumerate()
                .filter(|(_, session)| {
                    session.title.to_lowercase().contains(&query) ||
                    session.id.to_lowercase().contains(&query)
                })
                .map(|(index, _)| index)
                .collect();
        } else {
            self.filtered_sessions = (0..self.sessions.len()).collect();
        }
    }

    /// Update selected session based on list state
    fn update_selected_session(&mut self) {
        if let Some(selected_index) = self.session_list_state.selected() {
            let actual_index = if self.search_mode {
                self.filtered_sessions.get(selected_index).copied()
            } else {
                Some(selected_index)
            };
            
            if let Some(index) = actual_index {
                if let Some(session) = self.sessions.get(index) {
                    self.selected_session_id = Some(session.id.clone());
                }
            }
        }
    }

    /// Render sessions list
    fn render_sessions_list(&mut self, frame: &mut Frame, area: Rect) {
        let theme = self.theme_manager.current_theme();
        
        let title = if self.search_mode {
            format!("Sessions ({})", self.filtered_sessions.len())
        } else {
            format!("Sessions ({})", self.sessions.len())
        };
        
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(if self.state.has_focus {
                theme.styles.dialog_border.add_modifier(Modifier::BOLD)
            } else {
                theme.styles.dialog_border
            });

        let selected_session_id = self.selected_session_id.clone();
        let compact_mode = self.compact_mode;
        let items: Vec<ListItem> = if self.search_mode {
            self.filtered_sessions.iter()
                .filter_map(|&index| self.sessions.get(index))
                .map(|session| Self::create_session_list_item_static(session, theme, &selected_session_id, compact_mode))
                .collect()
        } else {
            self.sessions.iter()
                .map(|session| Self::create_session_list_item_static(session, theme, &selected_session_id, compact_mode))
                .collect()
        };

        let list = List::new(items)
            .block(block)
            .highlight_style(theme.styles.selected_base)
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, area, &mut self.session_list_state);
        
        // Render search box if in search mode
        if self.search_mode {
            self.render_search_box(frame, area);
        }
    }

    /// Create a list item for a session
    fn create_session_list_item(&self, session: &Session) -> ListItem<'static> {
        let theme = self.theme_manager.current_theme();
        Self::create_session_list_item_static(session, theme, &self.selected_session_id, self.compact_mode)
    }

    /// Create a list item for a session (static helper to avoid borrow issues)
    fn create_session_list_item_static(session: &Session, theme: &Theme, selected_session_id: &Option<String>, compact_mode: bool) -> ListItem<'static> {
        let mut spans = vec![
            Span::styled("📝 ".to_string(), theme.styles.info),
        ];

        // Session title
        let title = if session.title.len() > 30 {
            format!("{}...", &session.title[..27])
        } else {
            session.title.clone()
        };

        let title_style = if Some(&session.id) == selected_session_id.as_ref() {
            theme.styles.text.add_modifier(Modifier::BOLD)
        } else {
            theme.styles.text
        };

        spans.push(Span::styled(title, title_style));

        // Add time info if not compact
        if !compact_mode {
            let time_ago = format_time_ago(session.updated_at);
            spans.push(Span::raw(" "));
            spans.push(Span::styled(time_ago, theme.styles.muted));
        }

        ListItem::new(Line::from(spans))
    }

    /// Render file tree
    fn render_file_tree(&mut self, frame: &mut Frame, area: Rect) {
        let theme = self.theme_manager.current_theme();
        
        let block = Block::default()
            .title("Files")
            .borders(Borders::ALL)
            .border_style(theme.styles.dialog_border);

        let items: Vec<ListItem> = self.file_tree.get_visible_nodes()
            .iter()
            .map(|node| self.create_file_list_item(node))
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(theme.styles.selected_base)
            .highlight_symbol("▶ ");

        frame.render_widget(list, area);
    }

    /// Create a list item for a file node
    fn create_file_list_item(&self, node: &FileNode) -> ListItem<'static> {
        let theme = self.theme_manager.current_theme();

        let mut spans = Vec::new();

        // Indentation
        for _ in 0..node.depth {
            spans.push(Span::raw("  "));
        }

        // Icon and name
        if node.is_directory {
            let icon = if node.is_expanded { "📂" } else { "📁" };
            spans.push(Span::styled(icon.to_string(), theme.styles.info));
        } else {
            let icon = get_file_icon(&node.name);
            spans.push(Span::styled(icon.to_string(), theme.styles.info));
        }

        spans.push(Span::raw(" "));
        spans.push(Span::styled(node.name.clone(), theme.styles.text));

        // Size info for files
        if !node.is_directory && !self.compact_mode {
            if let Some(size) = node.size {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    format!("({})", format_file_size(size)),
                    theme.styles.muted,
                ));
            }
        }

        ListItem::new(Line::from(spans))
    }

    /// Render tool status
    fn render_tool_status(&self, frame: &mut Frame, area: Rect) {
        let theme = self.theme_manager.current_theme();
        
        let block = Block::default()
            .title("Tools")
            .borders(Borders::ALL)
            .border_style(theme.styles.dialog_border);

        let items: Vec<ListItem> = self.tool_statuses
            .values()
            .map(|tool| self.create_tool_list_item(tool))
            .collect();

        let list = List::new(items).block(block);
        frame.render_widget(list, area);
    }

    /// Create a list item for a tool status
    fn create_tool_list_item(&self, tool: &ToolStatus) -> ListItem<'static> {
        let theme = self.theme_manager.current_theme();

        let (icon, style) = match &tool.status {
            ToolState::Available => ("✅", theme.styles.success),
            ToolState::Running => ("⏳", theme.styles.info),
            ToolState::Error(_) => ("❌", theme.styles.error),
            ToolState::Disabled => ("⚫", theme.styles.muted),
        };

        let mut spans = vec![
            Span::styled(icon.to_string(), style),
            Span::raw(" "),
            Span::styled(tool.name.clone(), theme.styles.text),
        ];

        if !self.compact_mode {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                format!("({}/{})", tool.success_count, tool.success_count + tool.error_count),
                theme.styles.muted,
            ));
        }

        ListItem::new(Line::from(spans))
    }

    /// Render search box
    fn render_search_box(&self, frame: &mut Frame, area: Rect) {
        let theme = self.theme_manager.current_theme();
        
        // Position search box at the bottom of the area
        let search_area = Rect {
            x: area.x + 1,
            y: area.y + area.height - 2,
            width: area.width - 2,
            height: 1,
        };
        
        // Clear the area
        frame.render_widget(Clear, search_area);
        
        let search_content = format!("Search: {}", self.search_query);
        let search_widget = Paragraph::new(search_content)
            .style(theme.styles.text)
            .wrap(Wrap { trim: true });
        
        frame.render_widget(search_widget, search_area);
    }

    /// Render mixed mode (sessions and files)
    fn render_mixed_mode(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        self.render_sessions_list(frame, chunks[0]);
        
        if self.config.show_file_tree {
            self.render_file_tree(frame, chunks[1]);
        } else if self.config.show_tool_status {
            self.render_tool_status(frame, chunks[1]);
        }
    }

    /// Calculate minimum width needed
    pub fn min_width(&self) -> u16 {
        if self.compact_mode { 20 } else { 30 }
    }

    /// Calculate preferred width
    pub fn preferred_width(&self) -> u16 {
        if self.compact_mode { 25 } else { 40 }
    }
}

/// Actions that can be triggered from the sidebar
#[derive(Debug, Clone)]
pub enum SidebarAction {
    SessionSelected(String),
    SessionDeleted(String),
    FileSelected(PathBuf),
    ToolActivated(String),
    SearchActivated,
    ModeChanged(SidebarMode),
}

#[async_trait]
impl Component for ChatSidebar {
    async fn handle_key_event(&mut self, event: KeyEvent) -> Result<()> {
        if self.search_mode {
            return self.handle_search_key_event(event).await;
        }

        match event.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.navigate_up();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.navigate_down();
            }
            KeyCode::Enter => {
                if let Some(_action) = self.activate_selected() {
                    // TODO: Emit action event
                }
            }
            KeyCode::Char('/') => {
                self.start_search();
            }
            KeyCode::Char('s') => {
                self.set_mode(SidebarMode::Sessions);
            }
            KeyCode::Char('f') => {
                self.set_mode(SidebarMode::Files);
            }
            KeyCode::Char('t') => {
                self.set_mode(SidebarMode::Tools);
            }
            KeyCode::Char('m') => {
                self.set_mode(SidebarMode::Mixed);
            }
            KeyCode::Delete => {
                // TODO: Handle session/file deletion
            }
            _ => {}
        }

        Ok(())
    }

    async fn handle_mouse_event(&mut self, _event: MouseEvent) -> Result<()> {
        // TODO: Handle mouse events for selection, scrolling, etc.
        Ok(())
    }

    async fn tick(&mut self) -> Result<()> {
        // Update scroll animation
        self.scroll_animation.update();
        
        // Auto-refresh file tree if needed
        if self.last_update.elapsed() >= self.config.auto_refresh_interval {
            if matches!(self.mode, SidebarMode::Files | SidebarMode::Mixed) {
                let _ = self.refresh_file_tree();
            }
            self.last_update = Instant::now();
        }
        
        Ok(())
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, _theme: &Theme) {
        match self.mode {
            SidebarMode::Sessions => self.render_sessions_list(frame, area),
            SidebarMode::Files => self.render_file_tree(frame, area),
            SidebarMode::Tools => self.render_tool_status(frame, area),
            SidebarMode::Mixed => self.render_mixed_mode(frame, area),
        }
    }

    fn size(&self) -> Rect {
        self.state.size
    }

    fn set_size(&mut self, size: Rect) {
        self.state.size = size;
    }

    fn has_focus(&self) -> bool {
        self.state.has_focus
    }

    fn set_focus(&mut self, focus: bool) {
        self.state.has_focus = focus;
    }

    fn is_visible(&self) -> bool {
        self.state.is_visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.state.is_visible = visible;
    }
}

impl ChatSidebar {
    async fn handle_search_key_event(&mut self, event: KeyEvent) -> Result<()> {
        match event.code {
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.update_filtered_sessions();
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.update_filtered_sessions();
            }
            KeyCode::Enter => {
                self.stop_search();
            }
            KeyCode::Esc => {
                self.stop_search();
            }
            _ => {}
        }
        Ok(())
    }
}

// FileTree implementation
impl FileTree {
    fn new(root: PathBuf) -> Self {
        let mut tree = Self {
            root: root.clone(),
            nodes: Vec::new(),
            expanded_paths: std::collections::HashSet::new(),
            selected_path: None,
        };
        
        if let Err(_) = tree.refresh() {
            // If refresh fails, create an empty tree
        }
        
        tree
    }

    fn refresh(&mut self) -> Result<()> {
        self.nodes.clear();
        self.load_directory(&self.root.clone(), 0)?;
        Ok(())
    }

    fn load_directory(&mut self, path: &Path, depth: usize) -> Result<()> {
        if depth > 10 { // Prevent infinite recursion
            return Ok(());
        }

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().to_string();
                    let is_directory = path.is_dir();
                    
                    let metadata = entry.metadata().ok();
                    let modified_time = metadata.as_ref().and_then(|m| m.modified().ok());
                    let size = metadata.as_ref().and_then(|m| if m.is_file() { Some(m.len()) } else { None });
                    
                    let node = FileNode {
                        path: path.clone(),
                        name,
                        is_directory,
                        is_expanded: self.expanded_paths.contains(&path),
                        depth,
                        children: Vec::new(),
                        modified_time,
                        size,
                    };
                    
                    self.nodes.push(node);
                    
                    // Load children if directory is expanded
                    if is_directory && self.expanded_paths.contains(&path) {
                        self.load_directory(&path, depth + 1)?;
                    }
                }
            }
        }
        
        Ok(())
    }

    fn get_visible_nodes(&self) -> Vec<&FileNode> {
        self.nodes.iter().collect()
    }

    fn navigate_up(&mut self) {
        // TODO: Implement file tree navigation
    }

    fn navigate_down(&mut self) {
        // TODO: Implement file tree navigation
    }

    fn get_selected_path(&self) -> Option<&PathBuf> {
        self.selected_path.as_ref()
    }

    fn toggle_expanded(&mut self, path: &Path) {
        if self.expanded_paths.contains(path) {
            self.expanded_paths.remove(path);
        } else {
            self.expanded_paths.insert(path.to_path_buf());
        }
        let _ = self.refresh();
    }
}

impl ScrollAnimation {
    fn new() -> Self {
        Self {
            target_scroll: 0,
            current_scroll: 0.0,
            animation_speed: 0.2,
            is_animating: false,
        }
    }

    fn update(&mut self) {
        if self.is_animating {
            let diff = self.target_scroll as f32 - self.current_scroll;
            if diff.abs() < 0.1 {
                self.current_scroll = self.target_scroll as f32;
                self.is_animating = false;
            } else {
                self.current_scroll += diff * self.animation_speed;
            }
        }
    }
}

impl Default for ChatSidebar {
    fn default() -> Self {
        Self::new()
    }
}

// Helper functions

fn format_time_ago(timestamp: chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(timestamp);
    
    if duration.num_days() > 0 {
        format!("{}d", duration.num_days())
    } else if duration.num_hours() > 0 {
        format!("{}h", duration.num_hours())
    } else if duration.num_minutes() > 0 {
        format!("{}m", duration.num_minutes())
    } else {
        "now".to_string()
    }
}

fn format_file_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = size as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{:.0}{}", size, UNITS[unit_index])
    } else {
        format!("{:.1}{}", size, UNITS[unit_index])
    }
}

fn get_file_icon(filename: &str) -> &'static str {
    let extension = Path::new(filename).extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");
    
    match extension {
        "rs" => "🦀",
        "py" => "🐍",
        "js" | "ts" => "📜",
        "json" => "📋",
        "md" => "📝",
        "txt" => "📄",
        "png" | "jpg" | "jpeg" | "gif" => "🖼️",
        "pdf" => "📕",
        "zip" | "tar" | "gz" => "📦",
        _ => "📄",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sidebar_creation() {
        let sidebar = ChatSidebar::new();
        assert_eq!(sidebar.mode, SidebarMode::Sessions);
        assert!(!sidebar.compact_mode);
        assert!(sidebar.sessions.is_empty());
    }

    #[test]
    fn test_search_functionality() {
        let mut sidebar = ChatSidebar::new();
        
        // Add some test sessions
        sidebar.add_session(Session {
            id: "1".to_string(),
            title: "Test Session 1".to_string(),
            parent_session_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            message_count: 0,
            token_usage: crate::llm::TokenUsage::default(),
            total_cost: 0.0,
            metadata: std::collections::HashMap::new(),
        });

        sidebar.add_session(Session {
            id: "2".to_string(),
            title: "Another Session".to_string(),
            parent_session_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            message_count: 0,
            token_usage: crate::llm::TokenUsage::default(),
            total_cost: 0.0,
            metadata: std::collections::HashMap::new(),
        });
        
        // Test search
        sidebar.start_search();
        sidebar.update_search_query("Test".to_string());
        
        assert_eq!(sidebar.filtered_sessions.len(), 1);
        assert!(sidebar.search_mode);
    }

    #[test]
    fn test_file_size_formatting() {
        assert_eq!(format_file_size(512), "512B");
        assert_eq!(format_file_size(1024), "1.0KB");
        assert_eq!(format_file_size(1536), "1.5KB");
        assert_eq!(format_file_size(1024 * 1024), "1.0MB");
    }

    #[test]
    fn test_file_icons() {
        assert_eq!(get_file_icon("test.rs"), "🦀");
        assert_eq!(get_file_icon("script.py"), "🐍");
        assert_eq!(get_file_icon("config.json"), "📋");
        assert_eq!(get_file_icon("unknown.xyz"), "📄");
    }
}