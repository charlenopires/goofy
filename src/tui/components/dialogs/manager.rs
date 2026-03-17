//! Dialog manager for handling dialog stack and lifecycle
//! 
//! The dialog manager is responsible for:
//! - Managing a stack of open dialogs
//! - Handling dialog lifecycle (open, close, focus)
//! - Routing events to the appropriate dialog
//! - Managing modal behavior and focus
//! - Rendering dialogs in the correct order (z-index)

use super::{
    types::*,
    layer::DialogLayer,
    navigation::DialogNavigation,
};
use crate::tui::{
    components::Component,
    events::Event,
    themes::Theme,
    Frame,
};
use anyhow::Result;
use async_trait::async_trait;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use std::collections::HashMap;
use tokio::sync::mpsc;

/// Dialog manager handles the dialog stack and lifecycle
pub struct DialogManager {
    /// Stack of open dialogs (last = topmost)
    dialogs: Vec<Box<dyn Dialog>>,
    
    /// Map of dialog IDs to their position in the stack
    id_map: HashMap<DialogId, usize>,
    
    /// Currently focused dialog index
    focused_index: Option<usize>,
    
    /// Event sender for dialog events
    event_sender: Option<mpsc::UnboundedSender<Event>>,
    
    /// Dialog callbacks
    callbacks: Vec<Box<dyn DialogCallback>>,
    
    /// Dialog navigation handler
    navigation: DialogNavigation,
    
    /// Background dimming for modal dialogs
    background_dim: bool,
    
    /// Animation support
    animations_enabled: bool,
    
    /// Dialog layers for rendering
    layers: Vec<DialogLayer>,
    
    /// Last known terminal size
    terminal_size: Rect,
    
    /// Manager state
    state: ManagerState,
}

#[derive(Debug, Clone, PartialEq)]
enum ManagerState {
    Active,
    Suspended,
    Destroyed,
}

impl DialogManager {
    /// Create a new dialog manager
    pub fn new() -> Self {
        Self {
            dialogs: Vec::new(),
            id_map: HashMap::new(),
            focused_index: None,
            event_sender: None,
            callbacks: Vec::new(),
            navigation: DialogNavigation::new(),
            background_dim: true,
            animations_enabled: true,
            layers: Vec::new(),
            terminal_size: Rect::default(),
            state: ManagerState::Active,
        }
    }
    
    /// Set the event sender for dialog events
    pub fn set_event_sender(&mut self, sender: mpsc::UnboundedSender<Event>) {
        self.event_sender = Some(sender);
    }
    
    /// Add a dialog callback
    pub fn add_callback(&mut self, callback: Box<dyn DialogCallback>) {
        self.callbacks.push(callback);
    }
    
    /// Enable or disable background dimming for modal dialogs
    pub fn set_background_dim(&mut self, enabled: bool) {
        self.background_dim = enabled;
    }
    
    /// Enable or disable dialog animations
    pub fn set_animations_enabled(&mut self, enabled: bool) {
        self.animations_enabled = enabled;
    }
    
    /// Open a new dialog
    pub async fn open_dialog(&mut self, mut dialog: Box<dyn Dialog>) -> DialogResult<()> {
        let dialog_id = dialog.id().clone();
        
        // Check if dialog already exists
        if self.id_map.contains_key(&dialog_id) {
            return Err(DialogError::AlreadyExists(dialog_id));
        }
        
        // Don't allow opening dialogs on top of quit dialog
        if let Some(focused_idx) = self.focused_index {
            if let Some(focused_dialog) = self.dialogs.get(focused_idx) {
                if focused_dialog.id().as_str() == dialog_ids::QUIT {
                    return Ok(()); // Silently ignore
                }
            }
        }
        
        // Call callbacks
        for callback in &mut self.callbacks {
            callback.on_opening(&dialog_id).await?;
        }
        
        // Initialize the dialog
        dialog.on_open().await?;
        
        // If dialog already exists in stack, move it to top
        if let Some(&existing_index) = self.id_map.get(&dialog_id) {
            let existing_dialog = self.dialogs.remove(existing_index);
            self.update_id_map_after_removal(existing_index);
            
            // Re-add at top
            let new_index = self.dialogs.len();
            self.dialogs.push(existing_dialog);
            self.id_map.insert(dialog_id.clone(), new_index);
            self.focused_index = Some(new_index);
        } else {
            // Add new dialog to stack
            let index = self.dialogs.len();
            self.dialogs.push(dialog);
            self.id_map.insert(dialog_id.clone(), index);
            self.focused_index = Some(index);
        }
        
        // Update layers
        self.update_layers();
        
        // Send window size event to new dialog
        if let Some(dialog) = self.dialogs.last_mut() {
            let _ = dialog.handle_key_event(crossterm::event::KeyEvent::from(
                crossterm::event::KeyCode::Null
            )).await; // Trigger resize handling
        }
        
        // Call callbacks
        for callback in &mut self.callbacks {
            callback.on_opened(&dialog_id).await?;
        }
        
        // Send dialog event
        self.send_event(Event::Custom(
            "dialog_opened".to_string(),
            serde_json::json!({"dialog_id": dialog_id.as_str()}),
        ));
        
        Ok(())
    }
    
    /// Close the topmost dialog
    pub async fn close_dialog(&mut self) -> DialogResult<()> {
        if self.dialogs.is_empty() {
            return Ok(());
        }
        
        let index = self.dialogs.len() - 1;
        let dialog_id = self.dialogs[index].id().clone();
        
        self.close_dialog_by_id(&dialog_id).await
    }
    
    /// Close a specific dialog by ID
    pub async fn close_dialog_by_id(&mut self, dialog_id: &DialogId) -> DialogResult<()> {
        let index = self.id_map.get(dialog_id)
            .copied()
            .ok_or_else(|| DialogError::NotFound(dialog_id.clone()))?;
        
        let dialog = &self.dialogs[index];
        
        // Check if dialog can be closed
        if !dialog.can_close().await? {
            return Ok(()); // Dialog refused to close
        }
        
        // Call callbacks
        for callback in &mut self.callbacks {
            if !callback.on_closing(dialog_id).await? {
                return Ok(()); // Callback prevented close
            }
        }
        
        // Remove dialog from stack
        let mut dialog = self.dialogs.remove(index);
        self.id_map.remove(dialog_id);
        self.update_id_map_after_removal(index);
        
        // Update focused index
        if let Some(focused) = self.focused_index {
            if focused == index {
                // Focused dialog was closed, focus the new topmost
                self.focused_index = if self.dialogs.is_empty() {
                    None
                } else {
                    Some(self.dialogs.len() - 1)
                };
            } else if focused > index {
                // Adjust focused index after removal
                self.focused_index = Some(focused - 1);
            }
        }
        
        // Call dialog's close handler
        dialog.on_close().await?;
        
        // Update layers
        self.update_layers();
        
        // Call callbacks
        for callback in &mut self.callbacks {
            callback.on_closed(dialog_id).await?;
        }
        
        // Send dialog event
        self.send_event(Event::Custom(
            "dialog_closed".to_string(),
            serde_json::json!({"dialog_id": dialog_id.as_str()}),
        ));
        
        Ok(())
    }
    
    /// Close all dialogs
    pub async fn close_all_dialogs(&mut self) -> DialogResult<()> {
        while !self.dialogs.is_empty() {
            self.close_dialog().await?;
        }
        Ok(())
    }
    
    /// Get the currently focused dialog
    pub fn focused_dialog(&self) -> Option<&dyn Dialog> {
        self.focused_index
            .and_then(|idx| self.dialogs.get(idx))
            .map(|dialog| dialog.as_ref())
    }
    
    /// Get the currently focused dialog (mutable)
    pub fn focused_dialog_mut(&mut self) -> Option<&mut (dyn Dialog + '_)> {
        let idx = self.focused_index?;
        self.dialogs.get_mut(idx).map(|dialog| &mut **dialog as &mut dyn Dialog)
    }
    
    /// Get dialog by ID
    pub fn get_dialog(&self, dialog_id: &DialogId) -> Option<&dyn Dialog> {
        self.id_map.get(dialog_id)
            .and_then(|&idx| self.dialogs.get(idx))
            .map(|dialog| dialog.as_ref())
    }
    
    /// Get dialog by ID (mutable)
    pub fn get_dialog_mut(&mut self, dialog_id: &DialogId) -> Option<&mut (dyn Dialog + '_)> {
        let &idx = self.id_map.get(dialog_id)?;
        self.dialogs.get_mut(idx).map(|dialog| &mut **dialog as &mut dyn Dialog)
    }
    
    /// Check if any dialogs are open
    pub fn has_dialogs(&self) -> bool {
        !self.dialogs.is_empty()
    }
    
    /// Check if any modal dialogs are open
    pub fn has_modal_dialogs(&self) -> bool {
        self.dialogs.iter().any(|dialog| dialog.is_modal())
    }
    
    /// Get the number of open dialogs
    pub fn dialog_count(&self) -> usize {
        self.dialogs.len()
    }
    
    /// Get list of open dialog IDs
    pub fn dialog_ids(&self) -> Vec<DialogId> {
        self.dialogs.iter().map(|dialog| dialog.id().clone()).collect()
    }
    
    /// Get the topmost dialog ID
    pub fn topmost_dialog_id(&self) -> Option<DialogId> {
        self.dialogs.last().map(|dialog| dialog.id().clone())
    }
    
    /// Update dialog layers for rendering
    fn update_layers(&mut self) {
        self.layers.clear();
        
        for (index, dialog) in self.dialogs.iter().enumerate() {
            let is_focused = Some(index) == self.focused_index;
            let layout = DialogLayout::calculate(
                dialog.config(),
                self.terminal_size,
                Some(dialog.preferred_size()),
            );
            
            let layer = DialogLayer::new(
                dialog.id().clone(),
                layout,
                is_focused,
                dialog.config().z_index,
            );
            
            self.layers.push(layer);
        }
        
        // Sort layers by z-index
        self.layers.sort_by_key(|layer| layer.z_index());
    }
    
    /// Update ID map after removing a dialog at the given index
    fn update_id_map_after_removal(&mut self, removed_index: usize) {
        // Update indices for dialogs that came after the removed one
        for (_, index) in self.id_map.iter_mut() {
            if *index > removed_index {
                *index -= 1;
            }
        }
    }
    
    /// Send an event if event sender is configured
    fn send_event(&self, event: Event) {
        if let Some(sender) = &self.event_sender {
            let _ = sender.send(event);
        }
    }
    
    /// Handle navigation between dialogs
    pub async fn navigate_to_previous(&mut self) -> Result<()> {
        if self.dialogs.is_empty() {
            return Ok(());
        }
        
        let new_index = if let Some(current) = self.focused_index {
            if current > 0 {
                current - 1
            } else {
                self.dialogs.len() - 1
            }
        } else {
            self.dialogs.len() - 1
        };
        
        self.set_focus(Some(new_index)).await
    }
    
    /// Navigate to next dialog
    pub async fn navigate_to_next(&mut self) -> Result<()> {
        if self.dialogs.is_empty() {
            return Ok(());
        }
        
        let new_index = if let Some(current) = self.focused_index {
            if current + 1 < self.dialogs.len() {
                current + 1
            } else {
                0
            }
        } else {
            0
        };
        
        self.set_focus(Some(new_index)).await
    }
    
    /// Set focus to a specific dialog index
    async fn set_focus(&mut self, new_index: Option<usize>) -> Result<()> {
        // Remove focus from current dialog
        if let Some(current) = self.focused_index {
            if let Some(dialog) = self.dialogs.get_mut(current) {
                dialog.set_focus(false);
                
                // Call callbacks
                for callback in &mut self.callbacks {
                    callback.on_blurred(dialog.id()).await?;
                }
            }
        }
        
        // Set focus to new dialog
        self.focused_index = new_index;
        if let Some(new_idx) = new_index {
            if let Some(dialog) = self.dialogs.get_mut(new_idx) {
                dialog.set_focus(true);
                
                // Call callbacks
                for callback in &mut self.callbacks {
                    callback.on_focused(dialog.id()).await?;
                }
            }
        }
        
        self.update_layers();
        Ok(())
    }
}

#[async_trait]
impl Component for DialogManager {
    async fn handle_key_event(&mut self, event: KeyEvent) -> Result<()> {
        if self.state != ManagerState::Active {
            return Ok(());
        }
        
        // If we have a focused dialog, give it first chance to handle the event
        if let Some(dialog) = self.focused_dialog_mut() {
            // First, let dialog handle its own key events
            if dialog.handle_dialog_key(event).await? {
                // Dialog handled the event (e.g., close on Escape)
                if dialog.is_closable() && 
                   event.code == crossterm::event::KeyCode::Esc &&
                   event.modifiers.is_empty() {
                    let dialog_id = dialog.id().clone();
                    self.close_dialog_by_id(&dialog_id).await?;
                }
                return Ok(());
            }
            
            // Then let dialog handle normal key events
            dialog.handle_key_event(event).await?;
        }
        
        Ok(())
    }
    
    async fn handle_mouse_event(&mut self, event: MouseEvent) -> Result<()> {
        if self.state != ManagerState::Active {
            return Ok(());
        }
        
        // Route mouse events to the appropriate dialog based on position
        for (index, layer) in self.layers.iter().enumerate().rev() {
            let dialog_area = layer.layout().dialog_area;
            
            if event.column >= dialog_area.x && 
               event.column < dialog_area.x + dialog_area.width &&
               event.row >= dialog_area.y &&
               event.row < dialog_area.y + dialog_area.height {
                
                // Found the dialog under the mouse
                if let Some(dialog) = self.dialogs.get_mut(index) {
                    dialog.handle_mouse_event(event).await?;
                    
                    // Set focus to this dialog if it's not already focused
                    if self.focused_index != Some(index) {
                        self.set_focus(Some(index)).await?;
                    }
                }
                break;
            }
        }
        
        Ok(())
    }
    
    async fn tick(&mut self) -> Result<()> {
        if self.state != ManagerState::Active {
            return Ok(());
        }
        
        // Tick all dialogs
        for dialog in &mut self.dialogs {
            dialog.tick().await?;
        }
        
        Ok(())
    }
    
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if self.state != ManagerState::Active {
            return;
        }
        
        // Update terminal size
        self.terminal_size = area;
        self.update_layers();
        
        // Render background dimming for modal dialogs if enabled
        if self.background_dim && self.has_modal_dialogs() {
            self.render_modal_background(frame, area, theme);
        }
        
        // Render dialogs in z-index order
        for i in 0..self.layers.len() {
            let dialog_id = self.layers[i].dialog_id().clone();
            let layout = self.layers[i].layout().clone();
            
            if let Some(dialog) = self.get_dialog_mut(&dialog_id) {
                // Render dialog chrome (border, title)
                dialog.render_chrome(frame, layout.dialog_area, theme);
                
                // Render dialog content
                dialog.render_content(frame, layout.content_area, theme);
            }
        }
    }
    
    fn size(&self) -> Rect {
        self.terminal_size
    }
    
    fn set_size(&mut self, size: Rect) {
        self.terminal_size = size;
        
        // Propagate size to all dialogs
        for dialog in &mut self.dialogs {
            dialog.set_size(size);
        }
        
        self.update_layers();
    }
    
    fn has_focus(&self) -> bool {
        self.state == ManagerState::Active && self.has_dialogs()
    }
    
    fn set_focus(&mut self, focus: bool) {
        if !focus {
            self.state = ManagerState::Suspended;
        } else if self.state == ManagerState::Suspended {
            self.state = ManagerState::Active;
        }
    }
    
    fn is_visible(&self) -> bool {
        self.has_dialogs()
    }
}

impl DialogManager {
    /// Render modal background dimming
    fn render_modal_background(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        use ratatui::{
            style::{Color, Style},
            widgets::{Block, Clear},
        };
        
        // Clear the background
        frame.render_widget(Clear, area);
        
        // Render dimmed background
        let dim_style = Style::default()
            .bg(Color::Black)
            .fg(theme.text)
            .add_modifier(ratatui::style::Modifier::DIM);
        
        let dim_block = Block::default().style(dim_style);
        frame.render_widget(dim_block, area);
    }
}

impl Default for DialogManager {
    fn default() -> Self {
        Self::new()
    }
}