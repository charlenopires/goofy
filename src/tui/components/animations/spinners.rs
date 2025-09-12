//! Loading spinners and rotating indicators
//!
//! This module provides various spinner styles for loading states and progress indication.

use super::{Animation, AnimationConfig, AnimationState, EasingType};
use crate::tui::themes::Theme;
use anyhow::Result;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::time::{Duration, Instant};

/// Different spinner styles available
#[derive(Debug, Clone, PartialEq)]
pub enum SpinnerStyle {
    /// Classic dots spinner: ⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏
    Dots,
    /// Line spinner: |/-\
    Line,
    /// Arrow spinner: ←↖↑↗→↘↓↙
    Arrow,
    /// Bouncing ball: ⠁⠂⠄⠂
    BouncingBall,
    /// Growing dots: ⠋⠙⠚⠞⠖⠦⠴⠲⠳⠓
    GrowingDots,
    /// Clock spinner: 🕐🕑🕒🕓🕔🕕🕖🕗🕘🕙🕚🕛
    Clock,
    /// Pulse effect: ●○●○
    Pulse,
    /// Custom with user-defined frames
    Custom(Vec<String>),
}

impl SpinnerStyle {
    /// Get the frames for this spinner style
    pub fn frames(self) -> Vec<String> {
        match self {
            SpinnerStyle::Dots => vec![
                "⠋".to_string(), "⠙".to_string(), "⠹".to_string(), "⠸".to_string(),
                "⠼".to_string(), "⠴".to_string(), "⠦".to_string(), "⠧".to_string(),
                "⠇".to_string(), "⠏".to_string(),
            ],
            SpinnerStyle::Line => vec![
                "|".to_string(), "/".to_string(), "-".to_string(), "\\".to_string(),
            ],
            SpinnerStyle::Arrow => vec![
                "←".to_string(), "↖".to_string(), "↑".to_string(), "↗".to_string(),
                "→".to_string(), "↘".to_string(), "↓".to_string(), "↙".to_string(),
            ],
            SpinnerStyle::BouncingBall => vec![
                "⠁".to_string(), "⠂".to_string(), "⠄".to_string(), "⠂".to_string(),
            ],
            SpinnerStyle::GrowingDots => vec![
                "⠋".to_string(), "⠙".to_string(), "⠚".to_string(), "⠞".to_string(),
                "⠖".to_string(), "⠦".to_string(), "⠴".to_string(), "⠲".to_string(),
                "⠳".to_string(), "⠓".to_string(),
            ],
            SpinnerStyle::Clock => vec![
                "🕐".to_string(), "🕑".to_string(), "🕒".to_string(), "🕓".to_string(),
                "🕔".to_string(), "🕕".to_string(), "🕖".to_string(), "🕗".to_string(),
                "🕘".to_string(), "🕙".to_string(), "🕚".to_string(), "🕛".to_string(),
            ],
            SpinnerStyle::Pulse => vec![
                "●".to_string(), "○".to_string(),
            ],
            SpinnerStyle::Custom(frames) => frames,
        }
    }
    
    /// Get the recommended frame duration for this spinner style
    pub fn frame_duration(self) -> Duration {
        match self {
            SpinnerStyle::Dots => Duration::from_millis(80),
            SpinnerStyle::Line => Duration::from_millis(100),
            SpinnerStyle::Arrow => Duration::from_millis(120),
            SpinnerStyle::BouncingBall => Duration::from_millis(200),
            SpinnerStyle::GrowingDots => Duration::from_millis(100),
            SpinnerStyle::Clock => Duration::from_millis(1000),
            SpinnerStyle::Pulse => Duration::from_millis(500),
            SpinnerStyle::Custom(_) => Duration::from_millis(100),
        }
    }
}

/// Configuration for a spinner
#[derive(Debug, Clone)]
pub struct SpinnerConfig {
    /// The style of spinner to use
    pub style: SpinnerStyle,
    /// Message to display next to the spinner
    pub message: String,
    /// Color for the spinner
    pub color: Option<Color>,
    /// Color for the message
    pub message_color: Option<Color>,
    /// Whether to show a prefix before the spinner
    pub show_prefix: bool,
    /// Custom prefix text
    pub prefix: String,
    /// Animation configuration
    pub animation: AnimationConfig,
}

impl Default for SpinnerConfig {
    fn default() -> Self {
        Self {
            style: SpinnerStyle::Dots,
            message: "Loading...".to_string(),
            color: None,
            message_color: None,
            show_prefix: false,
            prefix: "".to_string(),
            animation: AnimationConfig::new()
                .duration(Duration::from_secs(1))
                .repeat(true),
        }
    }
}

impl SpinnerConfig {
    /// Create a new spinner configuration
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set the spinner style
    pub fn style(mut self, style: SpinnerStyle) -> Self {
        self.style = style;
        self.animation.duration = style.frame_duration() * style.frames().len() as u32;
        self
    }
    
    /// Set the message
    pub fn message<S: Into<String>>(mut self, message: S) -> Self {
        self.message = message.into();
        self
    }
    
    /// Set the spinner color
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
    
    /// Set the message color
    pub fn message_color(mut self, color: Color) -> Self {
        self.message_color = Some(color);
        self
    }
    
    /// Enable prefix display
    pub fn with_prefix<S: Into<String>>(mut self, prefix: S) -> Self {
        self.prefix = prefix.into();
        self.show_prefix = true;
        self
    }
    
    /// Set animation configuration
    pub fn animation(mut self, config: AnimationConfig) -> Self {
        self.animation = config;
        self
    }
}

/// A loading spinner animation
pub struct Spinner {
    /// Spinner configuration
    config: SpinnerConfig,
    /// Animation state
    state: AnimationState,
    /// Spinner frames
    frames: Vec<String>,
    /// Current frame index
    current_frame: usize,
    /// Last frame update time
    last_update: Option<Instant>,
    /// Start time of the animation
    start_time: Option<Instant>,
}

impl Spinner {
    /// Create a new spinner with the given configuration
    pub fn new(config: SpinnerConfig) -> Self {
        let frames = config.style.frames();
        
        Self {
            config,
            state: AnimationState::Idle,
            frames,
            current_frame: 0,
            last_update: None,
            start_time: None,
        }
    }
    
    /// Create a spinner with default configuration and message
    pub fn with_message<S: Into<String>>(message: S) -> Self {
        Self::new(SpinnerConfig::new().message(message))
    }
    
    /// Create a dots spinner
    pub fn dots<S: Into<String>>(message: S) -> Self {
        Self::new(
            SpinnerConfig::new()
                .style(SpinnerStyle::Dots)
                .message(message)
        )
    }
    
    /// Create a line spinner
    pub fn line<S: Into<String>>(message: S) -> Self {
        Self::new(
            SpinnerConfig::new()
                .style(SpinnerStyle::Line)
                .message(message)
        )
    }
    
    /// Create a pulse spinner
    pub fn pulse<S: Into<String>>(message: S) -> Self {
        Self::new(
            SpinnerConfig::new()
                .style(SpinnerStyle::Pulse)
                .message(message)
        )
    }
    
    /// Get the current frame
    pub fn current_frame(&self) -> &str {
        self.frames.get(self.current_frame).unwrap_or(&self.frames[0])
    }
    
    /// Set the message
    pub fn set_message<S: Into<String>>(&mut self, message: S) {
        self.config.message = message.into();
    }
    
    /// Check if enough time has passed to update the frame
    fn should_update_frame(&self) -> bool {
        if let Some(last_update) = self.last_update {
            let frame_duration = self.config.style.frame_duration();
            last_update.elapsed() >= frame_duration
        } else {
            true
        }
    }
}

impl Animation for Spinner {
    fn start(&mut self) -> Result<()> {
        self.state = AnimationState::Running {
            start_time: Instant::now(),
            current_frame: 0,
        };
        self.start_time = Some(Instant::now());
        self.last_update = Some(Instant::now());
        Ok(())
    }
    
    fn stop(&mut self) -> Result<()> {
        self.state = AnimationState::Complete;
        Ok(())
    }
    
    fn update(&mut self) -> Result<()> {
        match &self.state {
            AnimationState::Running { start_time, .. } => {
                if self.should_update_frame() {
                    self.current_frame = (self.current_frame + 1) % self.frames.len();
                    self.last_update = Some(Instant::now());
                    
                    // Update the state with new frame count
                    let elapsed = start_time.elapsed();
                    let frame_duration = self.config.style.frame_duration();
                    let total_frames = (elapsed.as_nanos() / frame_duration.as_nanos()) as u32;
                    
                    self.state = AnimationState::Running {
                        start_time: *start_time,
                        current_frame: total_frames,
                    };
                }
            }
            _ => {} // No update needed for other states
        }
        Ok(())
    }
    
    fn is_complete(&self) -> bool {
        matches!(self.state, AnimationState::Complete)
    }
    
    fn state(&self) -> &AnimationState {
        &self.state
    }
    
    fn render(&self, _area: Rect, theme: &Theme) -> Vec<Line> {
        let mut spans = Vec::new();
        
        // Add prefix if enabled
        if self.config.show_prefix && !self.config.prefix.is_empty() {
            spans.push(Span::styled(
                format!("{} ", self.config.prefix),
                Style::default().fg(theme.colors.muted),
            ));
        }
        
        // Add spinner frame
        let spinner_color = self.config.color.unwrap_or(theme.colors.primary);
        spans.push(Span::styled(
            self.current_frame().to_string(),
            Style::default()
                .fg(spinner_color)
                .add_modifier(Modifier::BOLD),
        ));
        
        // Add message
        if !self.config.message.is_empty() {
            spans.push(Span::raw(" "));
            let message_color = self.config.message_color.unwrap_or(theme.colors.text);
            spans.push(Span::styled(
                &self.config.message,
                Style::default().fg(message_color),
            ));
        }
        
        vec![Line::from(spans)]
    }
}

/// A multi-spinner that can show multiple concurrent loading operations
pub struct MultiSpinner {
    /// Individual spinners with their IDs
    spinners: std::collections::HashMap<String, Spinner>,
    /// Global animation state
    state: AnimationState,
    /// Maximum number of visible spinners
    max_visible: usize,
}

impl MultiSpinner {
    /// Create a new multi-spinner
    pub fn new() -> Self {
        Self {
            spinners: std::collections::HashMap::new(),
            state: AnimationState::Idle,
            max_visible: 5,
        }
    }
    
    /// Add a spinner with the given ID and configuration
    pub fn add_spinner<S: Into<String>>(&mut self, id: S, config: SpinnerConfig) -> Result<()> {
        let id = id.into();
        let mut spinner = Spinner::new(config);
        
        // Start the spinner immediately if we're running
        if matches!(self.state, AnimationState::Running { .. }) {
            spinner.start()?;
        }
        
        self.spinners.insert(id, spinner);
        Ok(())
    }
    
    /// Remove a spinner by ID
    pub fn remove_spinner(&mut self, id: &str) -> Result<()> {
        self.spinners.remove(id);
        Ok(())
    }
    
    /// Set the maximum number of visible spinners
    pub fn set_max_visible(&mut self, max: usize) {
        self.max_visible = max;
    }
    
    /// Get the number of active spinners
    pub fn spinner_count(&self) -> usize {
        self.spinners.len()
    }
    
    /// Update a spinner's message
    pub fn update_spinner_message<S: Into<String>>(&mut self, id: &str, message: S) -> Result<()> {
        if let Some(spinner) = self.spinners.get_mut(id) {
            spinner.set_message(message);
        }
        Ok(())
    }
}

impl Default for MultiSpinner {
    fn default() -> Self {
        Self::new()
    }
}

impl Animation for MultiSpinner {
    fn start(&mut self) -> Result<()> {
        self.state = AnimationState::Running {
            start_time: Instant::now(),
            current_frame: 0,
        };
        
        // Start all individual spinners
        for spinner in self.spinners.values_mut() {
            spinner.start()?;
        }
        
        Ok(())
    }
    
    fn stop(&mut self) -> Result<()> {
        self.state = AnimationState::Complete;
        
        // Stop all individual spinners
        for spinner in self.spinners.values_mut() {
            spinner.stop()?;
        }
        
        Ok(())
    }
    
    fn update(&mut self) -> Result<()> {
        // Update all individual spinners
        for spinner in self.spinners.values_mut() {
            spinner.update()?;
        }
        
        // Update our own state frame counter
        if let AnimationState::Running { start_time, .. } = &self.state {
            let elapsed = start_time.elapsed();
            let frame_count = (elapsed.as_millis() / 50) as u32; // 20 FPS
            
            self.state = AnimationState::Running {
                start_time: *start_time,
                current_frame: frame_count,
            };
        }
        
        Ok(())
    }
    
    fn is_complete(&self) -> bool {
        matches!(self.state, AnimationState::Complete)
    }
    
    fn state(&self) -> &AnimationState {
        &self.state
    }
    
    fn render(&self, _area: Rect, theme: &Theme) -> Vec<Line> {
        let mut lines = Vec::new();
        
        // Take up to max_visible spinners
        let spinners: Vec<_> = self.spinners.values().take(self.max_visible).collect();
        
        for spinner in spinners {
            let spinner_lines = spinner.render(_area, theme);
            lines.extend(spinner_lines);
        }
        
        // Show count if there are more spinners than visible
        if self.spinners.len() > self.max_visible {
            let hidden_count = self.spinners.len() - self.max_visible;
            lines.push(Line::from(vec![
                Span::styled(
                    format!("... and {} more", hidden_count),
                    Style::default().fg(theme.colors.muted),
                ),
            ]));
        }
        
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_styles() {
        assert!(!SpinnerStyle::Dots.frames().is_empty());
        assert!(!SpinnerStyle::Line.frames().is_empty());
        assert!(!SpinnerStyle::Arrow.frames().is_empty());
        
        // Test custom spinner
        let custom_frames = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let custom_style = SpinnerStyle::Custom(custom_frames.clone());
        assert_eq!(custom_style.frames(), custom_frames);
    }
    
    #[test]
    fn test_spinner_config() {
        let config = SpinnerConfig::new()
            .style(SpinnerStyle::Pulse)
            .message("Testing...")
            .color(Color::Red);
            
        assert_eq!(config.style, SpinnerStyle::Pulse);
        assert_eq!(config.message, "Testing...");
        assert_eq!(config.color, Some(Color::Red));
    }
    
    #[test]
    fn test_spinner_creation() {
        let spinner = Spinner::dots("Loading test");
        assert_eq!(spinner.config.message, "Loading test");
        assert_eq!(spinner.config.style, SpinnerStyle::Dots);
        assert_eq!(spinner.current_frame, 0);
    }
    
    #[test]
    fn test_multi_spinner() {
        let mut multi = MultiSpinner::new();
        
        let config = SpinnerConfig::new().message("Test 1");
        multi.add_spinner("test1", config).unwrap();
        
        let config = SpinnerConfig::new().message("Test 2");
        multi.add_spinner("test2", config).unwrap();
        
        assert_eq!(multi.spinner_count(), 2);
        
        multi.remove_spinner("test1").unwrap();
        assert_eq!(multi.spinner_count(), 1);
    }
}