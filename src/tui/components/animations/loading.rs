//! Loading states for async operations with various visual styles.
//! 
//! This module provides loading state management and visual indicators
//! for async operations, combining spinners and progress indicators
//! with contextual messaging.

use super::spinners::{Spinner, SpinnerConfig, SpinnerStyle};
use super::progress::{ProgressIndicator, ProgressConfig, ProgressStyle};
use super::animation_engine::{AnimationEngine, AnimationConfig, EasingType};
use super::interpolation::RgbColor;
use anyhow::Result;
use ratatui::style::{Color, Style};
use ratatui::text::{Span, Line};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use std::time::{Duration, Instant};

/// Loading state types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoadingState {
    /// Indeterminate loading (spinner)
    Indeterminate,
    /// Determinate loading with progress (progress bar)
    Determinate,
    /// Hybrid with both spinner and progress
    Hybrid,
    /// Text-only loading with animated dots
    TextOnly,
    /// Skeleton loading placeholder
    Skeleton,
}

/// Loading message with optional context
#[derive(Debug, Clone)]
pub struct LoadingMessage {
    pub primary: String,
    pub secondary: Option<String>,
    pub context: Option<String>,
    pub timestamp: Instant,
}

impl LoadingMessage {
    pub fn new(primary: String) -> Self {
        Self {
            primary,
            secondary: None,
            context: None,
            timestamp: Instant::now(),
        }
    }

    pub fn with_secondary(mut self, secondary: String) -> Self {
        self.secondary = Some(secondary);
        self
    }

    pub fn with_context(mut self, context: String) -> Self {
        self.context = Some(context);
        self
    }

    /// Get elapsed time since message was created
    pub fn elapsed(&self) -> Duration {
        self.timestamp.elapsed()
    }
}

/// Configuration for loading indicators
#[derive(Debug, Clone)]
pub struct LoadingConfig {
    pub state: LoadingState,
    pub show_elapsed_time: bool,
    pub show_context: bool,
    pub center_content: bool,
    pub spinner_config: Option<SpinnerConfig>,
    pub progress_config: Option<ProgressConfig>,
    pub text_color: RgbColor,
    pub accent_color: RgbColor,
    pub dim_color: RgbColor,
    pub animation_speed: Duration,
}

impl Default for LoadingConfig {
    fn default() -> Self {
        Self {
            state: LoadingState::Indeterminate,
            show_elapsed_time: false,
            show_context: true,
            center_content: true,
            spinner_config: Some(SpinnerConfig::loading()),
            progress_config: None,
            text_color: RgbColor::new(200, 200, 200),
            accent_color: RgbColor::new(100, 150, 255),
            dim_color: RgbColor::new(120, 120, 120),
            animation_speed: Duration::from_millis(100),
        }
    }
}

impl LoadingConfig {
    pub fn new(state: LoadingState) -> Self {
        Self {
            state,
            ..Default::default()
        }
    }

    pub fn with_spinner(mut self, config: SpinnerConfig) -> Self {
        self.spinner_config = Some(config);
        self
    }

    pub fn with_progress(mut self, config: ProgressConfig) -> Self {
        self.progress_config = Some(config);
        self
    }

    pub fn show_elapsed_time(mut self) -> Self {
        self.show_elapsed_time = true;
        self
    }

    pub fn hide_context(mut self) -> Self {
        self.show_context = false;
        self
    }

    pub fn align_left(mut self) -> Self {
        self.center_content = false;
        self
    }

    pub fn with_colors(mut self, text: RgbColor, accent: RgbColor, dim: RgbColor) -> Self {
        self.text_color = text;
        self.accent_color = accent;
        self.dim_color = dim;
        self
    }

    /// Quick configurations for common scenarios
    pub fn ai_thinking() -> Self {
        Self::new(LoadingState::Indeterminate)
            .with_spinner(SpinnerConfig::thinking())
            .with_colors(
                RgbColor::new(220, 220, 255),
                RgbColor::new(150, 180, 255),
                RgbColor::new(100, 100, 150),
            )
    }

    pub fn file_operation() -> Self {
        Self::new(LoadingState::Determinate)
            .with_progress(ProgressConfig::file_download())
            .show_elapsed_time()
            .with_colors(
                RgbColor::new(220, 255, 220),
                RgbColor::new(100, 255, 150),
                RgbColor::new(100, 150, 100),
            )
    }

    pub fn network_request() -> Self {
        Self::new(LoadingState::Hybrid)
            .with_spinner(SpinnerConfig::new()
                .style(SpinnerStyle::Dots)
                .message("Connecting"))
            .with_progress(ProgressConfig::new(ProgressStyle::Bar)
                .with_width(15)
                .show_percentage(false))
            .with_colors(
                RgbColor::new(255, 220, 150),
                RgbColor::new(255, 180, 100),
                RgbColor::new(150, 120, 80),
            )
    }

    pub fn text_processing() -> Self {
        Self::new(LoadingState::TextOnly)
            .with_colors(
                RgbColor::new(255, 255, 220),
                RgbColor::new(255, 255, 150),
                RgbColor::new(150, 150, 100),
            )
    }
}

/// Comprehensive loading indicator component
#[derive(Debug)]
pub struct LoadingIndicator {
    config: LoadingConfig,
    message: LoadingMessage,
    spinner: Option<Spinner>,
    progress: Option<ProgressIndicator>,
    text_animation: AnimationEngine,
    start_time: Instant,
    progress_value: f32,
}

impl LoadingIndicator {
    pub fn new(config: LoadingConfig, message: LoadingMessage) -> Self {
        let mut spinner = config.spinner_config.as_ref().map(|cfg| {
            let mut s = Spinner::new(cfg.clone());
            s.start();
            s
        });

        let progress = config.progress_config.as_ref().map(|cfg| {
            ProgressIndicator::new(cfg.clone())
        });

        let text_animation = AnimationEngine::new(
            AnimationConfig::new(Duration::from_millis(1500))
                .with_easing(EasingType::Linear)
                .infinite()
        );

        Self {
            config,
            message,
            spinner,
            progress,
            text_animation,
            start_time: Instant::now(),
            progress_value: 0.0,
        }
    }

    /// Update the loading message
    pub fn set_message(&mut self, message: LoadingMessage) {
        self.message = message;
    }

    /// Set progress for determinate loading (0.0 to 1.0)
    pub fn set_progress(&mut self, progress: f32) {
        self.progress_value = progress.clamp(0.0, 1.0);
        if let Some(progress_indicator) = &mut self.progress {
            progress_indicator.set_progress(self.progress_value);
        }
    }

    /// Update animations
    pub fn update(&mut self) -> Result<bool> {
        let mut updated = false;

        // Update spinner
        if let Some(spinner) = &mut self.spinner {
            if spinner.update()? {
                updated = true;
            }
        }

        // Update progress indicator
        if let Some(progress) = &mut self.progress {
            if progress.update()? {
                updated = true;
            }
        }

        // Update text animation
        if self.text_animation.should_update() {
            updated = true;
        }

        Ok(updated)
    }

    /// Start animations
    pub fn start(&mut self) {
        if let Some(spinner) = &mut self.spinner {
            spinner.start();
        }
        self.text_animation.start();
        self.start_time = Instant::now();
    }

    /// Stop animations
    pub fn stop(&mut self) {
        if let Some(spinner) = &mut self.spinner {
            spinner.stop();
        }
        self.text_animation.stop();
    }

    /// Render the loading indicator
    pub fn render(&self, area: Rect) -> Vec<Line> {
        let mut lines = Vec::new();

        match self.config.state {
            LoadingState::Indeterminate => {
                lines.extend(self.render_indeterminate());
            }
            LoadingState::Determinate => {
                lines.extend(self.render_determinate());
            }
            LoadingState::Hybrid => {
                lines.extend(self.render_hybrid());
            }
            LoadingState::TextOnly => {
                lines.extend(self.render_text_only());
            }
            LoadingState::Skeleton => {
                lines.extend(self.render_skeleton());
            }
        }

        // Add elapsed time if configured
        if self.config.show_elapsed_time {
            lines.push(self.render_elapsed_time());
        }

        lines
    }

    /// Render indeterminate loading (spinner only)
    fn render_indeterminate(&self) -> Vec<Line> {
        let mut spans = Vec::new();

        // Add spinner
        if let Some(spinner) = &self.spinner {
            spans.extend(spinner.render());
        }

        // Add primary message
        spans.push(Span::styled(
            format!(" {}", self.message.primary),
            Style::default().fg(self.config.text_color.to_color()),
        ));

        let mut lines = vec![Line::from(spans)];

        // Add secondary message if present
        if let Some(secondary) = &self.message.secondary {
            lines.push(Line::from(vec![
                Span::styled(
                    secondary,
                    Style::default().fg(self.config.dim_color.to_color()),
                ),
            ]));
        }

        // Add context if configured and present
        if self.config.show_context {
            if let Some(context) = &self.message.context {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {}", context),
                        Style::default().fg(self.config.dim_color.to_color()),
                    ),
                ]));
            }
        }

        lines
    }

    /// Render determinate loading (progress bar)
    fn render_determinate(&self) -> Vec<Line> {
        let mut lines = Vec::new();

        // Add primary message
        lines.push(Line::from(vec![
            Span::styled(
                &self.message.primary,
                Style::default().fg(self.config.text_color.to_color()),
            ),
        ]));

        // Add progress bar
        if let Some(progress) = &self.progress {
            lines.extend(progress.render());
        }

        // Add secondary message
        if let Some(secondary) = &self.message.secondary {
            lines.push(Line::from(vec![
                Span::styled(
                    secondary,
                    Style::default().fg(self.config.dim_color.to_color()),
                ),
            ]));
        }

        lines
    }

    /// Render hybrid loading (spinner + progress)
    fn render_hybrid(&self) -> Vec<Line> {
        let mut spans = Vec::new();

        // Add spinner
        if let Some(spinner) = &self.spinner {
            spans.extend(spinner.render());
        }

        // Add primary message
        spans.push(Span::styled(
            format!(" {}", self.message.primary),
            Style::default().fg(self.config.text_color.to_color()),
        ));

        let mut lines = vec![Line::from(spans)];

        // Add progress bar
        if let Some(progress) = &self.progress {
            lines.extend(progress.render());
        }

        lines
    }

    /// Render text-only loading with animated dots
    fn render_text_only(&self) -> Vec<Line> {
        let progress = self.text_animation.progress();
        let dot_count = ((progress * 4.0) as usize % 4) + 1;
        let dots = ".".repeat(dot_count);

        let line = Line::from(vec![
            Span::styled(
                &self.message.primary,
                Style::default().fg(self.config.text_color.to_color()),
            ),
            Span::styled(
                dots,
                Style::default().fg(self.config.accent_color.to_color()),
            ),
        ]);

        vec![line]
    }

    /// Render skeleton loading placeholder
    fn render_skeleton(&self) -> Vec<Line> {
        let pulse_progress = self.text_animation.progress();
        let intensity = (pulse_progress * 2.0 * std::f32::consts::PI).sin().abs();
        
        let skeleton_color = RgbColor::new(
            (self.config.dim_color.r as f32 * (0.3 + 0.3 * intensity)) as u8,
            (self.config.dim_color.g as f32 * (0.3 + 0.3 * intensity)) as u8,
            (self.config.dim_color.b as f32 * (0.3 + 0.3 * intensity)) as u8,
        );

        vec![
            Line::from(vec![
                Span::styled(
                    "████████████████",
                    Style::default().fg(skeleton_color.to_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "██████████",
                    Style::default().fg(skeleton_color.to_color()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "████████████████████",
                    Style::default().fg(skeleton_color.to_color()),
                ),
            ]),
        ]
    }

    /// Render elapsed time
    fn render_elapsed_time(&self) -> Line {
        let elapsed = self.start_time.elapsed();
        let seconds = elapsed.as_secs();
        let time_str = if seconds < 60 {
            format!("{}s", seconds)
        } else {
            let minutes = seconds / 60;
            let remaining_seconds = seconds % 60;
            format!("{}m {}s", minutes, remaining_seconds)
        };

        Line::from(vec![
            Span::styled(
                format!("Elapsed: {}", time_str),
                Style::default().fg(self.config.dim_color.to_color()),
            ),
        ])
    }

    /// Check if any animations are running
    pub fn is_animating(&self) -> bool {
        self.spinner.as_ref().map(|s| s.is_running()).unwrap_or(false) ||
        self.progress.as_ref().map(|p| p.is_animating()).unwrap_or(false) ||
        self.text_animation.is_running()
    }

    /// Get current progress value
    pub fn progress(&self) -> f32 {
        self.progress_value
    }

    /// Get elapsed time since start
    pub fn elapsed_time(&self) -> Duration {
        self.start_time.elapsed()
    }
}

/// Loading state manager for handling multiple concurrent operations
#[derive(Debug)]
pub struct LoadingStateManager {
    indicators: std::collections::HashMap<String, LoadingIndicator>,
    active_count: usize,
}

impl LoadingStateManager {
    pub fn new() -> Self {
        Self {
            indicators: std::collections::HashMap::new(),
            active_count: 0,
        }
    }

    /// Start a new loading operation
    pub fn start_loading(&mut self, id: String, config: LoadingConfig, message: LoadingMessage) {
        let mut indicator = LoadingIndicator::new(config, message);
        indicator.start();
        self.indicators.insert(id, indicator);
        self.active_count += 1;
    }

    /// Update progress for a specific operation
    pub fn update_progress(&mut self, id: &str, progress: f32) -> Result<()> {
        if let Some(indicator) = self.indicators.get_mut(id) {
            indicator.set_progress(progress);
        }
        Ok(())
    }

    /// Update message for a specific operation
    pub fn update_message(&mut self, id: &str, message: LoadingMessage) -> Result<()> {
        if let Some(indicator) = self.indicators.get_mut(id) {
            indicator.set_message(message);
        }
        Ok(())
    }

    /// Complete a loading operation
    pub fn complete_loading(&mut self, id: &str) {
        if let Some(mut indicator) = self.indicators.remove(id) {
            indicator.stop();
            if self.active_count > 0 {
                self.active_count -= 1;
            }
        }
    }

    /// Update all active indicators
    pub fn update_all(&mut self) -> Result<()> {
        for indicator in self.indicators.values_mut() {
            indicator.update()?;
        }
        Ok(())
    }

    /// Get all active indicators
    pub fn active_indicators(&self) -> &std::collections::HashMap<String, LoadingIndicator> {
        &self.indicators
    }

    /// Check if any operations are loading
    pub fn is_loading(&self) -> bool {
        self.active_count > 0
    }

    /// Get count of active operations
    pub fn active_count(&self) -> usize {
        self.active_count
    }

    /// Clear all loading operations
    pub fn clear_all(&mut self) {
        for (_, mut indicator) in self.indicators.drain() {
            indicator.stop();
        }
        self.active_count = 0;
    }
}

impl Default for LoadingStateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loading_message_creation() {
        let message = LoadingMessage::new("Loading data".to_string())
            .with_secondary("Please wait".to_string())
            .with_context("Processing 1000 items".to_string());
        
        assert_eq!(message.primary, "Loading data");
        assert_eq!(message.secondary, Some("Please wait".to_string()));
        assert_eq!(message.context, Some("Processing 1000 items".to_string()));
    }

    #[test]
    fn test_loading_indicator_creation() {
        let config = LoadingConfig::ai_thinking();
        let message = LoadingMessage::new("Thinking".to_string());
        let indicator = LoadingIndicator::new(config, message);
        
        assert!(indicator.is_animating()); // Should be animating after creation
        assert_eq!(indicator.progress(), 0.0);
    }

    #[test]
    fn test_loading_state_manager() {
        let mut manager = LoadingStateManager::new();
        
        let config = LoadingConfig::file_operation();
        let message = LoadingMessage::new("Downloading".to_string());
        
        manager.start_loading("download1".to_string(), config, message);
        assert!(manager.is_loading());
        assert_eq!(manager.active_count(), 1);
        
        manager.update_progress("download1", 0.5).unwrap();
        manager.complete_loading("download1");
        
        assert!(!manager.is_loading());
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn test_progress_setting() {
        let config = LoadingConfig::file_operation();
        let message = LoadingMessage::new("Processing".to_string());
        let mut indicator = LoadingIndicator::new(config, message);
        
        indicator.set_progress(0.75);
        assert_eq!(indicator.progress(), 0.75);
        
        indicator.set_progress(1.5); // Should clamp to 1.0
        assert_eq!(indicator.progress(), 1.0);
    }
}