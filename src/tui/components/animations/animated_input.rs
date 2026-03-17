//! Animated input components with focus and validation effects.
//!
//! This module provides input field components that can animate
//! focus states, validation feedback, and typing effects.

use super::{Animation, AnimationConfig, AnimationState, EasingType};
use super::pulse::{PulseAnimation, PulseConfig, PulseStyle};
use super::fade::{FadeAnimation, FadeConfig, FadeDirection};
use super::interpolation::RgbColor;
use crate::tui::themes::Theme;
use anyhow::Result;
use ratatui::{
    layout::Rect,
    style::{Color, Style, Modifier},
    text::{Line, Span},
};
use std::time::{Duration, Instant};

/// Input field animation styles
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputAnimationStyle {
    /// Simple focus highlight
    Focus,
    /// Pulsing border when focused
    Pulse,
    /// Glow effect around input
    Glow,
    /// Shake animation for validation errors
    Shake,
    /// Typewriter effect for placeholder
    Typewriter,
    /// Smooth transitions
    Smooth,
    /// Material design style
    Material,
}

/// Input validation state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValidationState {
    /// No validation performed yet
    None,
    /// Input is valid
    Valid,
    /// Input has errors
    Invalid,
    /// Input is being validated
    Validating,
}

/// Input field configuration
#[derive(Debug, Clone)]
pub struct InputConfig {
    pub placeholder: String,
    pub label: Option<String>,
    pub animation_style: InputAnimationStyle,
    pub focus_duration: Duration,
    pub validation_feedback: bool,
    pub show_character_count: bool,
    pub max_length: Option<usize>,
    pub mask_character: Option<char>, // For password fields
    pub multiline: bool,
    pub auto_resize: bool,
    pub border_style: InputBorderStyle,
    pub validation_rules: Vec<ValidationRule>,
}

/// Input border styling
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputBorderStyle {
    None,
    Single,
    Double,
    Rounded,
    Underline,
}

/// Validation rule for input
#[derive(Debug, Clone)]
pub struct ValidationRule {
    pub name: String,
    pub rule_type: ValidationRuleType,
    pub message: String,
}

/// Types of validation rules
#[derive(Debug, Clone)]
pub enum ValidationRuleType {
    MinLength(usize),
    MaxLength(usize),
    Required,
    Email,
    Number,
    Custom(fn(&str) -> bool),
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            placeholder: "Enter text...".to_string(),
            label: None,
            animation_style: InputAnimationStyle::Focus,
            focus_duration: Duration::from_millis(200),
            validation_feedback: true,
            show_character_count: false,
            max_length: None,
            mask_character: None,
            multiline: false,
            auto_resize: false,
            border_style: InputBorderStyle::Single,
            validation_rules: Vec::new(),
        }
    }
}

impl InputConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_placeholder(mut self, placeholder: String) -> Self {
        self.placeholder = placeholder;
        self
    }

    pub fn with_label(mut self, label: String) -> Self {
        self.label = Some(label);
        self
    }

    pub fn with_animation(mut self, style: InputAnimationStyle) -> Self {
        self.animation_style = style;
        self
    }

    pub fn with_max_length(mut self, max_length: usize) -> Self {
        self.max_length = Some(max_length);
        self.show_character_count = true;
        self
    }

    pub fn password(mut self) -> Self {
        self.mask_character = Some('*');
        self
    }

    pub fn multiline(mut self) -> Self {
        self.multiline = true;
        self.auto_resize = true;
        self
    }

    pub fn with_validation(mut self, rule: ValidationRule) -> Self {
        self.validation_rules.push(rule);
        self.validation_feedback = true;
        self
    }

    pub fn with_border(mut self, style: InputBorderStyle) -> Self {
        self.border_style = style;
        self
    }

    /// Quick presets for common input types
    pub fn text_field(placeholder: String) -> Self {
        Self::new()
            .with_placeholder(placeholder)
            .with_animation(InputAnimationStyle::Focus)
    }

    pub fn password_field() -> Self {
        Self::new()
            .with_placeholder("Password".to_string())
            .password()
            .with_animation(InputAnimationStyle::Glow)
    }

    pub fn email_field() -> Self {
        Self::new()
            .with_placeholder("Email address".to_string())
            .with_validation(ValidationRule::email())
            .with_animation(InputAnimationStyle::Material)
    }

    pub fn search_field() -> Self {
        Self::new()
            .with_placeholder("Search...".to_string())
            .with_animation(InputAnimationStyle::Pulse)
    }

    pub fn text_area(placeholder: String) -> Self {
        Self::new()
            .with_placeholder(placeholder)
            .multiline()
            .with_animation(InputAnimationStyle::Smooth)
    }
}

impl ValidationRule {
    pub fn required() -> Self {
        Self {
            name: "required".to_string(),
            rule_type: ValidationRuleType::Required,
            message: "This field is required".to_string(),
        }
    }

    pub fn min_length(length: usize) -> Self {
        Self {
            name: "min_length".to_string(),
            rule_type: ValidationRuleType::MinLength(length),
            message: format!("Must be at least {} characters", length),
        }
    }

    pub fn max_length(length: usize) -> Self {
        Self {
            name: "max_length".to_string(),
            rule_type: ValidationRuleType::MaxLength(length),
            message: format!("Must be no more than {} characters", length),
        }
    }

    pub fn email() -> Self {
        Self {
            name: "email".to_string(),
            rule_type: ValidationRuleType::Email,
            message: "Must be a valid email address".to_string(),
        }
    }

    pub fn number() -> Self {
        Self {
            name: "number".to_string(),
            rule_type: ValidationRuleType::Number,
            message: "Must be a valid number".to_string(),
        }
    }

    pub fn custom(name: String, rule: fn(&str) -> bool, message: String) -> Self {
        Self {
            name,
            rule_type: ValidationRuleType::Custom(rule),
            message,
        }
    }

    /// Validate input against this rule
    pub fn validate(&self, input: &str) -> bool {
        match &self.rule_type {
            ValidationRuleType::Required => !input.is_empty(),
            ValidationRuleType::MinLength(len) => input.chars().count() >= *len,
            ValidationRuleType::MaxLength(len) => input.chars().count() <= *len,
            ValidationRuleType::Email => {
                // Simple email validation
                input.contains('@') && input.contains('.') && input.len() > 5
            }
            ValidationRuleType::Number => input.parse::<f64>().is_ok(),
            ValidationRuleType::Custom(func) => func(input),
        }
    }
}

/// Animated input field component
#[derive(Debug)]
pub struct AnimatedInput {
    config: InputConfig,
    state: AnimationState,
    value: String,
    cursor_position: usize,
    is_focused: bool,
    validation_state: ValidationState,
    validation_errors: Vec<String>,
    focus_animation: Option<Box<dyn Animation + Send + Sync>>,
    validation_animation: Option<Box<dyn Animation + Send + Sync>>,
    placeholder_animation: Option<Box<dyn Animation + Send + Sync>>,
    area: Rect,
    scroll_offset: usize,
    selection_start: Option<usize>,
    last_focus_time: Option<Instant>,
    typing_indicator: bool,
}

impl AnimatedInput {
    pub fn new(config: InputConfig) -> Self {
        Self {
            config,
            state: AnimationState::Idle,
            value: String::new(),
            cursor_position: 0,
            is_focused: false,
            validation_state: ValidationState::None,
            validation_errors: Vec::new(),
            focus_animation: None,
            validation_animation: None,
            placeholder_animation: None,
            area: Rect::default(),
            scroll_offset: 0,
            selection_start: None,
            last_focus_time: None,
            typing_indicator: false,
        }
    }

    /// Set the input area
    pub fn set_area(&mut self, area: Rect) {
        self.area = area;
    }

    /// Get the current value
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Set the value
    pub fn set_value(&mut self, value: String) {
        if let Some(max_len) = self.config.max_length {
            if value.chars().count() > max_len {
                return;
            }
        }
        
        self.value = value;
        self.cursor_position = self.value.chars().count();
        self.validate_input();
    }

    /// Insert character at cursor position
    pub fn insert_char(&mut self, ch: char) {
        if let Some(max_len) = self.config.max_length {
            if self.value.chars().count() >= max_len {
                return;
            }
        }

        let byte_index = self.value
            .char_indices()
            .nth(self.cursor_position)
            .map(|(i, _)| i)
            .unwrap_or(self.value.len());
        
        self.value.insert(byte_index, ch);
        self.cursor_position += 1;
        self.validate_input();
    }

    /// Delete character before cursor
    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            let byte_index = self.value
                .char_indices()
                .nth(self.cursor_position)
                .map(|(i, _)| i)
                .unwrap_or(self.value.len());
            
            self.value.remove(byte_index);
            self.validate_input();
        }
    }

    /// Move cursor left
    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    /// Move cursor right
    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.value.chars().count() {
            self.cursor_position += 1;
        }
    }

    /// Set focus state
    pub fn set_focus(&mut self, focused: bool) -> Result<()> {
        if self.is_focused != focused {
            self.is_focused = focused;
            self.last_focus_time = Some(Instant::now());

            if focused {
                self.start_focus_animation()?;
            } else {
                self.start_blur_animation()?;
            }
        }
        Ok(())
    }

    /// Check if input is focused
    pub fn is_focused(&self) -> bool {
        self.is_focused
    }

    /// Get validation state
    pub fn validation_state(&self) -> ValidationState {
        self.validation_state
    }

    /// Get validation errors
    pub fn validation_errors(&self) -> &[String] {
        &self.validation_errors
    }

    /// Validate the current input
    fn validate_input(&mut self) {
        self.validation_errors.clear();
        
        for rule in &self.config.validation_rules {
            if !rule.validate(&self.value) {
                self.validation_errors.push(rule.message.clone());
            }
        }

        let new_state = if self.validation_errors.is_empty() {
            ValidationState::Valid
        } else {
            ValidationState::Invalid
        };

        if new_state != self.validation_state {
            self.validation_state = new_state;
            if self.config.validation_feedback {
                let _ = self.start_validation_animation();
            }
        }
    }

    /// Start focus animation
    fn start_focus_animation(&mut self) -> Result<()> {
        match self.config.animation_style {
            InputAnimationStyle::Focus => {
                let fade_config = FadeConfig::new()
                    .direction(FadeDirection::In)
                    .animation(AnimationConfig::new().duration(self.config.focus_duration)
                        .with_easing(EasingType::EaseOut));
                self.focus_animation = Some(Box::new(FadeAnimation::new(fade_config)));
            }
            InputAnimationStyle::Pulse => {
                let pulse_config = PulseConfig::focus_highlight();
                self.focus_animation = Some(Box::new(PulseAnimation::new(pulse_config, "".to_string())));
            }
            InputAnimationStyle::Glow => {
                let pulse_config = PulseConfig::new(PulseStyle::Glow)
                    .with_colors(
                        RgbColor::new(100, 100, 100),
                        RgbColor::new(100, 150, 255),
                    );
                self.focus_animation = Some(Box::new(PulseAnimation::new(pulse_config, "".to_string())));
            }
            _ => {
                let fade_config = FadeConfig::new().direction(FadeDirection::In);
                self.focus_animation = Some(Box::new(FadeAnimation::new(fade_config)));
            }
        }

        if let Some(animation) = &mut self.focus_animation {
            animation.start()?;
        }

        Ok(())
    }

    /// Start blur animation
    fn start_blur_animation(&mut self) -> Result<()> {
        if let Some(animation) = &mut self.focus_animation {
            animation.stop()?;
        }
        Ok(())
    }

    /// Start validation animation
    fn start_validation_animation(&mut self) -> Result<()> {
        match self.validation_state {
            ValidationState::Invalid => {
                let pulse_config = PulseConfig::error_flash();
                self.validation_animation = Some(Box::new(PulseAnimation::new(pulse_config, "".to_string())));
            }
            ValidationState::Valid => {
                let pulse_config = PulseConfig::success_glow();
                self.validation_animation = Some(Box::new(PulseAnimation::new(pulse_config, "".to_string())));
            }
            _ => return Ok(()),
        }

        if let Some(animation) = &mut self.validation_animation {
            animation.start()?;
        }

        Ok(())
    }

    /// Get the display text (with masking if needed)
    fn display_text(&self) -> String {
        if let Some(mask_char) = self.config.mask_character {
            mask_char.to_string().repeat(self.value.chars().count())
        } else {
            self.value.clone()
        }
    }

    /// Render the input border
    fn render_border(&self, theme: &Theme) -> Style {
        let base_color = match self.validation_state {
            ValidationState::Valid => Color::Green,
            ValidationState::Invalid => Color::Red,
            ValidationState::Validating => Color::Yellow,
            _ => theme.colors.border,
        };

        let mut style = Style::default().fg(base_color);

        // Apply focus effects
        if self.is_focused {
            style = style.add_modifier(Modifier::BOLD);
            
            // Apply animation effects
            if let Some(focus_animation) = &self.focus_animation {
                // Focus animation would modify the style here
                style = style.fg(theme.colors.primary);
            }
        }

        style
    }

    /// Render cursor
    fn render_cursor(&self) -> (usize, char) {
        let display_pos = if self.cursor_position > self.scroll_offset {
            self.cursor_position - self.scroll_offset
        } else {
            0
        };
        
        (display_pos, if self.is_focused { '|' } else { ' ' })
    }
}

impl Animation for AnimatedInput {
    fn start(&mut self) -> Result<()> {
        self.state = AnimationState::Running {
            start_time: Instant::now(),
            current_frame: 0,
        };
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        self.state = AnimationState::Complete;
        
        if let Some(animation) = &mut self.focus_animation {
            animation.stop()?;
        }
        if let Some(animation) = &mut self.validation_animation {
            animation.stop()?;
        }
        
        Ok(())
    }

    fn update(&mut self) -> Result<()> {
        // Update focus animation
        if let Some(animation) = &mut self.focus_animation {
            animation.update()?;
        }

        // Update validation animation
        if let Some(animation) = &mut self.validation_animation {
            animation.update()?;
            if animation.is_complete() {
                self.validation_animation = None;
            }
        }

        // Update frame counter
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
        matches!(self.state, AnimationState::Complete | AnimationState::Idle)
    }

    fn state(&self) -> &AnimationState {
        &self.state
    }

    fn render(&self, _area: Rect, theme: &Theme) -> Vec<Line> {
        let mut lines = Vec::new();

        // Label
        if let Some(label) = &self.config.label {
            lines.push(Line::from(vec![
                Span::styled(
                    label,
                    Style::default()
                        .fg(theme.colors.text)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }

        // Input field
        let border_style = self.render_border(theme);
        let display_text = self.display_text();
        let (cursor_pos, cursor_char) = self.render_cursor();

        // Create input line with border
        let mut input_spans = Vec::new();

        // Left border
        match self.config.border_style {
            InputBorderStyle::Single => input_spans.push(Span::styled("│", border_style)),
            InputBorderStyle::Double => input_spans.push(Span::styled("║", border_style)),
            InputBorderStyle::Rounded => input_spans.push(Span::styled("╰", border_style)),
            _ => input_spans.push(Span::raw(" ")),
        }

        // Content
        let available_width = self.area.width.saturating_sub(4) as usize; // Account for borders and padding
        let visible_text = if display_text.len() > available_width {
            display_text.chars().skip(self.scroll_offset).take(available_width).collect()
        } else {
            display_text
        };

        // Add text with cursor
        if visible_text.is_empty() && !self.is_focused {
            // Show placeholder
            input_spans.push(Span::styled(
                format!(" {} ", self.config.placeholder),
                Style::default().fg(theme.colors.muted),
            ));
        } else {
            let chars: Vec<char> = visible_text.chars().collect();
            input_spans.push(Span::raw(" "));
            
            for (i, &ch) in chars.iter().enumerate() {
                if i == cursor_pos && self.is_focused {
                    input_spans.push(Span::styled(
                        cursor_char.to_string(),
                        Style::default()
                            .fg(theme.colors.primary)
                            .add_modifier(Modifier::RAPID_BLINK),
                    ));
                }
                input_spans.push(Span::raw(ch.to_string()));
            }
            
            // Cursor at end
            if cursor_pos >= chars.len() && self.is_focused {
                input_spans.push(Span::styled(
                    cursor_char.to_string(),
                    Style::default()
                        .fg(theme.colors.primary)
                        .add_modifier(Modifier::RAPID_BLINK),
                ));
            }
            
            input_spans.push(Span::raw(" "));
        }

        // Right border
        match self.config.border_style {
            InputBorderStyle::Single => input_spans.push(Span::styled("│", border_style)),
            InputBorderStyle::Double => input_spans.push(Span::styled("║", border_style)),
            InputBorderStyle::Rounded => input_spans.push(Span::styled("╯", border_style)),
            _ => input_spans.push(Span::raw(" ")),
        }

        lines.push(Line::from(input_spans));

        // Character count
        if self.config.show_character_count {
            let count_text = if let Some(max_len) = self.config.max_length {
                format!("{}/{}", self.value.chars().count(), max_len)
            } else {
                self.value.chars().count().to_string()
            };
            
            lines.push(Line::from(vec![
                Span::styled(
                    count_text,
                    Style::default().fg(theme.colors.muted),
                ),
            ]));
        }

        // Validation errors
        if self.config.validation_feedback && !self.validation_errors.is_empty() {
            for error in &self.validation_errors {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("⚠ {}", error),
                        Style::default().fg(Color::Red),
                    ),
                ]));
            }
        }

        lines
    }
}

/// Input field presets for common scenarios
pub struct InputPresets;

impl InputPresets {
    /// Basic text input
    pub fn text_input(placeholder: String) -> AnimatedInput {
        AnimatedInput::new(InputConfig::text_field(placeholder))
    }

    /// Password input
    pub fn password_input() -> AnimatedInput {
        AnimatedInput::new(InputConfig::password_field())
    }

    /// Email input with validation
    pub fn email_input() -> AnimatedInput {
        AnimatedInput::new(InputConfig::email_field())
    }

    /// Search input
    pub fn search_input() -> AnimatedInput {
        AnimatedInput::new(InputConfig::search_field())
    }

    /// Multi-line text area
    pub fn text_area(placeholder: String) -> AnimatedInput {
        AnimatedInput::new(InputConfig::text_area(placeholder))
    }

    /// Number input
    pub fn number_input() -> AnimatedInput {
        AnimatedInput::new(
            InputConfig::new()
                .with_placeholder("Enter number".to_string())
                .with_validation(ValidationRule::number())
                .with_animation(InputAnimationStyle::Material)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_config_creation() {
        let config = InputConfig::text_field("Enter name".to_string());
        assert_eq!(config.placeholder, "Enter name");
        assert_eq!(config.animation_style, InputAnimationStyle::Focus);
    }

    #[test]
    fn test_validation_rules() {
        let required_rule = ValidationRule::required();
        assert!(required_rule.validate("hello"));
        assert!(!required_rule.validate(""));

        let email_rule = ValidationRule::email();
        assert!(email_rule.validate("test@example.com"));
        assert!(!email_rule.validate("invalid-email"));

        let min_length_rule = ValidationRule::min_length(5);
        assert!(min_length_rule.validate("hello"));
        assert!(!min_length_rule.validate("hi"));
    }

    #[test]
    fn test_animated_input_value() {
        let config = InputConfig::default();
        let mut input = AnimatedInput::new(config);
        
        assert_eq!(input.value(), "");
        
        input.set_value("test".to_string());
        assert_eq!(input.value(), "test");
        
        input.insert_char('!');
        assert_eq!(input.value(), "test!");
        
        input.delete_char();
        assert_eq!(input.value(), "test");
    }

    #[test]
    fn test_cursor_movement() {
        let config = InputConfig::default();
        let mut input = AnimatedInput::new(config);
        
        input.set_value("hello".to_string());
        assert_eq!(input.cursor_position, 5);
        
        input.move_cursor_left();
        assert_eq!(input.cursor_position, 4);
        
        input.move_cursor_right();
        assert_eq!(input.cursor_position, 5);
    }

    #[test]
    fn test_input_validation() {
        let config = InputConfig::new()
            .with_validation(ValidationRule::required())
            .with_validation(ValidationRule::min_length(3));
        let mut input = AnimatedInput::new(config);
        
        // Empty input should be invalid
        input.set_value("".to_string());
        assert_eq!(input.validation_state(), ValidationState::Invalid);
        assert_eq!(input.validation_errors().len(), 2);
        
        // Short input should be invalid
        input.set_value("hi".to_string());
        assert_eq!(input.validation_state(), ValidationState::Invalid);
        assert_eq!(input.validation_errors().len(), 1);
        
        // Valid input
        input.set_value("hello".to_string());
        assert_eq!(input.validation_state(), ValidationState::Valid);
        assert_eq!(input.validation_errors().len(), 0);
    }

    #[test]
    fn test_input_presets() {
        let text = InputPresets::text_input("Name".to_string());
        let password = InputPresets::password_input();
        let email = InputPresets::email_input();
        let search = InputPresets::search_input();
        
        // Just verify they can be created without panicking
        assert_eq!(text.value(), "");
        assert_eq!(password.value(), "");
        assert_eq!(email.value(), "");
        assert_eq!(search.value(), "");
    }
}