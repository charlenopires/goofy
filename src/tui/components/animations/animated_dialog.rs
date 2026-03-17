//! Animated dialog components with entrance and exit effects.
//!
//! This module provides dialog components that can animate in and out
//! with various effects like fade, scale, slide, and bounce.

use super::{Animation, AnimationConfig, AnimationState, EasingType};
use super::fade::{FadeAnimation, FadeConfig, FadeDirection};
use super::slide::{SlideAnimation, SlideConfig, SlideDirection};
use super::pulse::{PulseAnimation, PulseConfig};
use super::interpolation::RgbColor;
use crate::tui::themes::Theme;
use anyhow::Result;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use std::time::{Duration, Instant};

/// Dialog animation styles
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DialogAnimationStyle {
    /// Simple fade in/out
    Fade,
    /// Scale animation (zoom in/out)
    Scale,
    /// Slide from specific direction
    Slide,
    /// Bounce entrance
    Bounce,
    /// Elastic scale effect
    Elastic,
    /// Flip animation
    Flip,
    /// Typewriter content reveal
    Typewriter,
    /// Glow entrance
    Glow,
}

/// Dialog configuration
#[derive(Debug, Clone)]
pub struct DialogConfig {
    pub title: Option<String>,
    pub content: Vec<Line<'static>>,
    pub buttons: Vec<DialogButton>,
    pub style: DialogAnimationStyle,
    pub entrance_duration: Duration,
    pub exit_duration: Duration,
    pub show_backdrop: bool,
    pub backdrop_opacity: f32,
    pub modal: bool, // Blocks interaction with background
    pub closable: bool,
    pub auto_close_duration: Option<Duration>,
    pub border_style: BorderStyle,
    pub slide_direction: SlideDirection,
}

/// Dialog button configuration
#[derive(Debug, Clone)]
pub struct DialogButton {
    pub label: String,
    pub action: String, // Identifier for the action
    pub style: ButtonStyle,
    pub is_default: bool, // Default/focused button
}

/// Button styling options
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ButtonStyle {
    Primary,
    Secondary,
    Success,
    Warning,
    Danger,
    Ghost,
}

/// Border styling options
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BorderStyle {
    None,
    Single,
    Double,
    Rounded,
    Thick,
}

impl Default for DialogConfig {
    fn default() -> Self {
        Self {
            title: None,
            content: Vec::new(),
            buttons: Vec::new(),
            style: DialogAnimationStyle::Fade,
            entrance_duration: Duration::from_millis(300),
            exit_duration: Duration::from_millis(200),
            show_backdrop: true,
            backdrop_opacity: 0.5,
            modal: true,
            closable: true,
            auto_close_duration: None,
            border_style: BorderStyle::Single,
            slide_direction: SlideDirection::FromTop,
        }
    }
}

impl DialogConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }

    pub fn with_content(mut self, content: Vec<Line<'static>>) -> Self {
        self.content = content;
        self
    }

    pub fn with_text(mut self, text: String) -> Self {
        self.content = vec![Line::from(text)];
        self
    }

    pub fn with_button(mut self, button: DialogButton) -> Self {
        self.buttons.push(button);
        self
    }

    pub fn with_style(mut self, style: DialogAnimationStyle) -> Self {
        self.style = style;
        self
    }

    pub fn with_durations(mut self, entrance: Duration, exit: Duration) -> Self {
        self.entrance_duration = entrance;
        self.exit_duration = exit;
        self
    }

    pub fn with_slide_direction(mut self, direction: SlideDirection) -> Self {
        self.slide_direction = direction;
        self
    }

    pub fn no_backdrop(mut self) -> Self {
        self.show_backdrop = false;
        self
    }

    pub fn non_modal(mut self) -> Self {
        self.modal = false;
        self
    }

    pub fn auto_close(mut self, duration: Duration) -> Self {
        self.auto_close_duration = Some(duration);
        self
    }

    pub fn with_border(mut self, style: BorderStyle) -> Self {
        self.border_style = style;
        self
    }

    /// Quick presets for common dialog types
    pub fn confirmation(title: String, message: String) -> Self {
        Self::new()
            .with_title(title)
            .with_text(message)
            .with_button(DialogButton::new("Cancel".to_string(), "cancel".to_string(), ButtonStyle::Secondary))
            .with_button(DialogButton::new("OK".to_string(), "ok".to_string(), ButtonStyle::Primary).as_default())
            .with_style(DialogAnimationStyle::Scale)
    }

    pub fn alert(title: String, message: String) -> Self {
        Self::new()
            .with_title(title)
            .with_text(message)
            .with_button(DialogButton::new("OK".to_string(), "ok".to_string(), ButtonStyle::Primary).as_default())
            .with_style(DialogAnimationStyle::Bounce)
    }

    pub fn error(title: String, message: String) -> Self {
        Self::new()
            .with_title(title)
            .with_text(message)
            .with_button(DialogButton::new("Close".to_string(), "close".to_string(), ButtonStyle::Danger).as_default())
            .with_style(DialogAnimationStyle::Slide)
            .with_slide_direction(SlideDirection::FromTop)
    }

    pub fn loading(message: String) -> Self {
        Self::new()
            .with_text(message)
            .with_style(DialogAnimationStyle::Glow)
            .no_backdrop()
            .non_modal()
            .auto_close(Duration::from_secs(3))
    }

    pub fn notification(message: String) -> Self {
        Self::new()
            .with_text(message)
            .with_style(DialogAnimationStyle::Slide)
            .with_slide_direction(SlideDirection::FromBottom)
            .auto_close(Duration::from_secs(2))
            .with_border(BorderStyle::Rounded)
    }
}

impl DialogButton {
    pub fn new(label: String, action: String, style: ButtonStyle) -> Self {
        Self {
            label,
            action,
            style,
            is_default: false,
        }
    }

    pub fn as_default(mut self) -> Self {
        self.is_default = true;
        self
    }
}

/// Dialog state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DialogState {
    /// Dialog is closed
    Closed,
    /// Dialog is animating in
    Opening,
    /// Dialog is fully open
    Open,
    /// Dialog is animating out
    Closing,
}

/// Animated dialog component
#[derive(Debug)]
pub struct AnimatedDialog {
    config: DialogConfig,
    state: DialogState,
    animation_state: AnimationState,
    entrance_animation: Option<Box<dyn Animation + Send + Sync>>,
    exit_animation: Option<Box<dyn Animation + Send + Sync>>,
    backdrop_animation: Option<FadeAnimation>,
    selected_button: usize,
    start_time: Option<Instant>,
    auto_close_start: Option<Instant>,
    area: Rect,
    content_area: Rect,
    result: Option<String>, // Action result when dialog is closed
}

impl AnimatedDialog {
    pub fn new(config: DialogConfig) -> Self {
        Self {
            selected_button: config.buttons.iter()
                .position(|b| b.is_default)
                .unwrap_or(0),
            config,
            state: DialogState::Closed,
            animation_state: AnimationState::Idle,
            entrance_animation: None,
            exit_animation: None,
            backdrop_animation: None,
            start_time: None,
            auto_close_start: None,
            area: Rect::default(),
            content_area: Rect::default(),
            result: None,
        }
    }

    /// Open the dialog
    pub fn open(&mut self) -> Result<()> {
        self.state = DialogState::Opening;
        self.animation_state = AnimationState::Running {
            start_time: Instant::now(),
            current_frame: 0,
        };
        self.start_time = Some(Instant::now());
        self.result = None;

        // Setup entrance animation based on style
        self.entrance_animation = Some(self.create_entrance_animation());

        // Setup backdrop animation if enabled
        if self.config.show_backdrop {
            let fade_config = FadeConfig::new()
                .direction(FadeDirection::In)
                .opacity_range(0.0, self.config.backdrop_opacity);
            self.backdrop_animation = Some(FadeAnimation::new(fade_config));
        }

        // Auto-close timer
        if self.config.auto_close_duration.is_some() {
            self.auto_close_start = Some(Instant::now());
        }

        // Start animations
        if let Some(animation) = &mut self.entrance_animation {
            animation.start()?;
        }
        if let Some(backdrop) = &mut self.backdrop_animation {
            backdrop.start()?;
        }

        Ok(())
    }

    /// Close the dialog
    pub fn close(&mut self) -> Result<()> {
        if matches!(self.state, DialogState::Open | DialogState::Opening) {
            self.state = DialogState::Closing;

            // Setup exit animation
            self.exit_animation = Some(self.create_exit_animation());

            // Setup backdrop fade out
            if let Some(backdrop) = &mut self.backdrop_animation {
                let fade_config = FadeConfig::new()
                    .direction(FadeDirection::Out)
                    .opacity_range(self.config.backdrop_opacity, 0.0);
                *backdrop = FadeAnimation::new(fade_config);
                backdrop.start()?;
            }

            // Start exit animation
            if let Some(animation) = &mut self.exit_animation {
                animation.start()?;
            }
        }
        Ok(())
    }

    /// Close dialog with a specific result
    pub fn close_with_result(&mut self, result: String) -> Result<()> {
        self.result = Some(result);
        self.close()
    }

    /// Handle button selection (keyboard navigation)
    pub fn select_previous_button(&mut self) {
        if !self.config.buttons.is_empty() {
            self.selected_button = if self.selected_button == 0 {
                self.config.buttons.len() - 1
            } else {
                self.selected_button - 1
            };
        }
    }

    pub fn select_next_button(&mut self) {
        if !self.config.buttons.is_empty() {
            self.selected_button = (self.selected_button + 1) % self.config.buttons.len();
        }
    }

    /// Activate the currently selected button
    pub fn activate_selected_button(&mut self) -> Result<Option<String>> {
        if self.selected_button < self.config.buttons.len() {
            let action = self.config.buttons[self.selected_button].action.clone();
            self.close_with_result(action.clone())?;
            Ok(Some(action))
        } else {
            Ok(None)
        }
    }

    /// Get the dialog result (available after closing)
    pub fn result(&self) -> Option<&String> {
        self.result.as_ref()
    }

    /// Check if dialog is open
    pub fn is_open(&self) -> bool {
        matches!(self.state, DialogState::Open | DialogState::Opening)
    }

    /// Check if dialog is closed
    pub fn is_closed(&self) -> bool {
        matches!(self.state, DialogState::Closed)
    }

    /// Set the dialog area
    pub fn set_area(&mut self, area: Rect) {
        self.area = area;
        self.calculate_content_area();
    }

    /// Calculate the content area for the dialog
    fn calculate_content_area(&mut self) {
        // Center the dialog in the available area
        let dialog_width = (self.area.width * 3 / 4).min(80);
        let dialog_height = (self.config.content.len() as u16 + 6).min(self.area.height);

        let x = (self.area.width.saturating_sub(dialog_width)) / 2;
        let y = (self.area.height.saturating_sub(dialog_height)) / 2;

        self.content_area = Rect {
            x: self.area.x + x,
            y: self.area.y + y,
            width: dialog_width,
            height: dialog_height,
        };
    }

    /// Create entrance animation based on style
    fn create_entrance_animation(&self) -> Box<dyn Animation + Send + Sync> {
        match self.config.style {
            DialogAnimationStyle::Fade => {
                let fade_config = FadeConfig::new()
                    .direction(FadeDirection::In)
                    .animation(AnimationConfig::new().duration(self.config.entrance_duration)
                        .with_easing(EasingType::EaseOut));
                Box::new(FadeAnimation::new(fade_config))
            }
            DialogAnimationStyle::Scale => {
                let fade_config = FadeConfig::new()
                    .direction(FadeDirection::In)
                    .animation(AnimationConfig::new().duration(self.config.entrance_duration)
                        .with_easing(EasingType::EaseOutBack));
                Box::new(FadeAnimation::new(fade_config))
            }
            DialogAnimationStyle::Slide => {
                let slide_config = SlideConfig::new(self.config.slide_direction)
                    .with_duration(self.config.entrance_duration)
                    .with_easing(EasingType::EaseOutQuad.into());
                Box::new(SlideAnimation::new(slide_config, self.content_area))
            }
            DialogAnimationStyle::Bounce => {
                let fade_config = FadeConfig::new()
                    .direction(FadeDirection::In)
                    .animation(AnimationConfig::new().duration(self.config.entrance_duration)
                        .with_easing(EasingType::EaseOutBounce));
                Box::new(FadeAnimation::new(fade_config))
            }
            DialogAnimationStyle::Elastic => {
                let fade_config = FadeConfig::new()
                    .direction(FadeDirection::In)
                    .animation(AnimationConfig::new().duration(self.config.entrance_duration)
                        .with_easing(EasingType::EaseOutElastic));
                Box::new(FadeAnimation::new(fade_config))
            }
            DialogAnimationStyle::Glow => {
                let pulse_config = PulseConfig::notification();
                Box::new(PulseAnimation::new(pulse_config, "".to_string()))
            }
            _ => {
                // Default to fade for other styles
                let fade_config = FadeConfig::new()
                    .direction(FadeDirection::In);
                Box::new(FadeAnimation::new(fade_config))
            }
        }
    }

    /// Create exit animation based on style
    fn create_exit_animation(&self) -> Box<dyn Animation + Send + Sync> {
        match self.config.style {
            DialogAnimationStyle::Fade => {
                let fade_config = FadeConfig::new()
                    .direction(FadeDirection::Out)
                    .animation(AnimationConfig::new().duration(self.config.exit_duration)
                        .with_easing(EasingType::EaseIn));
                Box::new(FadeAnimation::new(fade_config))
            }
            DialogAnimationStyle::Scale => {
                let fade_config = FadeConfig::new()
                    .direction(FadeDirection::Out)
                    .animation(AnimationConfig::new().duration(self.config.exit_duration)
                        .with_easing(EasingType::EaseInBack));
                Box::new(FadeAnimation::new(fade_config))
            }
            DialogAnimationStyle::Slide => {
                let slide_config = SlideConfig::new(self.config.slide_direction.reverse())
                    .with_duration(self.config.exit_duration)
                    .with_easing(EasingType::EaseInQuad.into());
                Box::new(SlideAnimation::new(slide_config, self.content_area))
            }
            _ => {
                let fade_config = FadeConfig::new()
                    .direction(FadeDirection::Out);
                Box::new(FadeAnimation::new(fade_config))
            }
        }
    }

    /// Check for auto-close timeout
    fn check_auto_close(&mut self) -> Result<()> {
        if let (Some(duration), Some(start_time)) = (self.config.auto_close_duration, self.auto_close_start) {
            if start_time.elapsed() >= duration {
                self.close()?;
                self.auto_close_start = None;
            }
        }
        Ok(())
    }

    /// Render the dialog buttons
    fn render_buttons(&self, theme: &Theme) -> Vec<Line> {
        if self.config.buttons.is_empty() {
            return Vec::new();
        }

        let mut button_spans = Vec::new();
        
        for (index, button) in self.config.buttons.iter().enumerate() {
            let is_selected = index == self.selected_button;
            
            let button_style = match button.style {
                ButtonStyle::Primary => theme.colors.primary,
                ButtonStyle::Secondary => theme.colors.secondary,
                ButtonStyle::Success => Color::Green,
                ButtonStyle::Warning => Color::Yellow,
                ButtonStyle::Danger => Color::Red,
                ButtonStyle::Ghost => theme.colors.muted,
            };

            let style = if is_selected {
                Style::default()
                    .bg(button_style)
                    .fg(theme.colors.background)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(button_style)
                    .add_modifier(if button.is_default { Modifier::BOLD } else { Modifier::empty() })
            };

            if index > 0 {
                button_spans.push(Span::raw("  "));
            }
            
            button_spans.push(Span::raw("["));
            button_spans.push(Span::styled(&button.label, style));
            button_spans.push(Span::raw("]"));
        }

        vec![Line::from(button_spans)]
    }
}

impl Animation for AnimatedDialog {
    fn start(&mut self) -> Result<()> {
        self.open()
    }

    fn stop(&mut self) -> Result<()> {
        self.close()
    }

    fn update(&mut self) -> Result<()> {
        // Check auto-close
        self.check_auto_close()?;

        // Update animations based on current state
        match self.state {
            DialogState::Opening => {
                let mut entrance_complete = false;
                if let Some(animation) = &mut self.entrance_animation {
                    animation.update()?;
                    if animation.is_complete() {
                        entrance_complete = true;
                    }
                }

                if let Some(backdrop) = &mut self.backdrop_animation {
                    backdrop.update()?;
                }

                if entrance_complete {
                    self.state = DialogState::Open;
                }
            }
            DialogState::Closing => {
                let mut exit_complete = false;
                if let Some(animation) = &mut self.exit_animation {
                    animation.update()?;
                    if animation.is_complete() {
                        exit_complete = true;
                    }
                }

                if let Some(backdrop) = &mut self.backdrop_animation {
                    backdrop.update()?;
                }

                if exit_complete {
                    self.state = DialogState::Closed;
                    self.animation_state = AnimationState::Complete;
                }
            }
            _ => {}
        }

        // Update frame counter
        if let AnimationState::Running { start_time, .. } = &self.animation_state {
            let frame_count = (start_time.elapsed().as_millis() / 16) as u32;
            self.animation_state = AnimationState::Running {
                start_time: *start_time,
                current_frame: frame_count,
            };
        }

        Ok(())
    }

    fn is_complete(&self) -> bool {
        matches!(self.animation_state, AnimationState::Complete) &&
        matches!(self.state, DialogState::Closed)
    }

    fn state(&self) -> &AnimationState {
        &self.animation_state
    }

    fn render(&self, _area: Rect, theme: &Theme) -> Vec<Line> {
        if matches!(self.state, DialogState::Closed) {
            return Vec::new();
        }

        let mut lines = Vec::new();

        // Render backdrop if enabled
        if self.config.show_backdrop {
            if let Some(backdrop) = &self.backdrop_animation {
                // In a real implementation, this would render a semi-transparent overlay
                // For terminal UI, we can simulate with a dimmed background
                let backdrop_char = if backdrop.opacity() > 0.3 { "░" } else { " " };
                let backdrop_line = backdrop_char.repeat(self.area.width as usize);
                for _ in 0..self.area.height {
                    lines.push(Line::from(Span::styled(backdrop_line.clone(),
                        Style::default().fg(Color::DarkGray))));
                }
            }
        }

        // Render dialog content
        let dialog_area = self.content_area;
        
        // Title
        if let Some(title) = &self.config.title {
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {} ", title),
                    Style::default()
                        .fg(theme.colors.primary)
                        .add_modifier(Modifier::BOLD)
                ),
            ]));
        }

        // Content
        lines.extend(self.config.content.clone());

        // Buttons
        if !self.config.buttons.is_empty() {
            lines.push(Line::from("")); // Separator
            lines.extend(self.render_buttons(theme));
        }

        lines
    }
}

/// Dialog presets for common scenarios
pub struct DialogPresets;

impl DialogPresets {
    /// Simple alert dialog
    pub fn alert(message: String) -> AnimatedDialog {
        AnimatedDialog::new(DialogConfig::alert("Alert".to_string(), message))
    }

    /// Confirmation dialog
    pub fn confirm(message: String) -> AnimatedDialog {
        AnimatedDialog::new(DialogConfig::confirmation("Confirm".to_string(), message))
    }

    /// Error dialog
    pub fn error(message: String) -> AnimatedDialog {
        AnimatedDialog::new(DialogConfig::error("Error".to_string(), message))
    }

    /// Loading dialog
    pub fn loading(message: String) -> AnimatedDialog {
        AnimatedDialog::new(DialogConfig::loading(message))
    }

    /// Notification toast
    pub fn notification(message: String) -> AnimatedDialog {
        AnimatedDialog::new(DialogConfig::notification(message))
    }

    /// Custom dialog with multiple buttons
    pub fn custom(title: String, message: String, buttons: Vec<DialogButton>) -> AnimatedDialog {
        let mut config = DialogConfig::new()
            .with_title(title)
            .with_text(message);
        
        for button in buttons {
            config = config.with_button(button);
        }
        
        AnimatedDialog::new(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialog_config_creation() {
        let config = DialogConfig::confirmation(
            "Test".to_string(),
            "Are you sure?".to_string()
        );
        
        assert_eq!(config.title, Some("Test".to_string()));
        assert_eq!(config.buttons.len(), 2);
        assert_eq!(config.style, DialogAnimationStyle::Scale);
    }

    #[test]
    fn test_dialog_button() {
        let button = DialogButton::new(
            "OK".to_string(),
            "ok".to_string(),
            ButtonStyle::Primary
        ).as_default();
        
        assert_eq!(button.label, "OK");
        assert_eq!(button.action, "ok");
        assert!(button.is_default);
    }

    #[test]
    fn test_animated_dialog_lifecycle() {
        let config = DialogConfig::alert("Test".to_string(), "Message".to_string());
        let mut dialog = AnimatedDialog::new(config);
        
        assert!(dialog.is_closed());
        
        dialog.open().unwrap();
        assert!(!dialog.is_closed());
        
        dialog.close().unwrap();
        // Dialog will be in closing state, not immediately closed due to animation
    }

    #[test]
    fn test_button_navigation() {
        let config = DialogConfig::confirmation(
            "Test".to_string(),
            "Message".to_string()
        );
        let mut dialog = AnimatedDialog::new(config);
        
        assert_eq!(dialog.selected_button, 1); // Default button (OK)
        
        dialog.select_previous_button();
        assert_eq!(dialog.selected_button, 0); // Cancel
        
        dialog.select_next_button();
        assert_eq!(dialog.selected_button, 1); // OK
    }

    #[test]
    fn test_dialog_presets() {
        let alert = DialogPresets::alert("Test alert".to_string());
        let confirm = DialogPresets::confirm("Test confirm".to_string());
        let error = DialogPresets::error("Test error".to_string());
        
        // Just verify they can be created without panicking
        assert!(alert.is_closed());
        assert!(confirm.is_closed());
        assert!(error.is_closed());
    }
}