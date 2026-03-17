//! Fade in/out animation effects
//!
//! This module provides smooth fade animations for dialogs, messages, and other UI elements.

use super::{Animation, AnimationConfig, AnimationState, Animatable, EasingType};
use crate::tui::themes::Theme;
use anyhow::Result;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
};
use std::time::{Duration, Instant};

/// Direction of the fade animation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FadeDirection {
    /// Fade from transparent to opaque
    In,
    /// Fade from opaque to transparent
    Out,
    /// Fade in then out (pulse effect)
    InOut,
}

/// Configuration for fade animations
#[derive(Debug, Clone)]
pub struct FadeConfig {
    /// Direction of the fade
    pub direction: FadeDirection,
    /// Starting opacity (0.0 to 1.0)
    pub start_opacity: f32,
    /// Ending opacity (0.0 to 1.0)
    pub end_opacity: f32,
    /// Animation configuration
    pub animation: AnimationConfig,
    /// Whether to preserve original colors
    pub preserve_colors: bool,
}

impl Default for FadeConfig {
    fn default() -> Self {
        Self {
            direction: FadeDirection::In,
            start_opacity: 0.0,
            end_opacity: 1.0,
            animation: AnimationConfig::new()
                .duration(Duration::from_millis(300))
                .easing(EasingType::EaseInOut),
            preserve_colors: true,
        }
    }
}

impl FadeConfig {
    /// Create a new fade configuration
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set the fade direction
    pub fn direction(mut self, direction: FadeDirection) -> Self {
        self.direction = direction;
        
        // Set appropriate opacity values based on direction
        match direction {
            FadeDirection::In => {
                self.start_opacity = 0.0;
                self.end_opacity = 1.0;
            }
            FadeDirection::Out => {
                self.start_opacity = 1.0;
                self.end_opacity = 0.0;
            }
            FadeDirection::InOut => {
                self.start_opacity = 0.0;
                self.end_opacity = 0.0;
                // For in-out, we'll handle the special case in the animation
            }
        }
        
        self
    }
    
    /// Set custom opacity range
    pub fn opacity_range(mut self, start: f32, end: f32) -> Self {
        self.start_opacity = start.clamp(0.0, 1.0);
        self.end_opacity = end.clamp(0.0, 1.0);
        self
    }
    
    /// Set animation configuration
    pub fn animation(mut self, config: AnimationConfig) -> Self {
        self.animation = config;
        self
    }
    
    /// Enable/disable color preservation
    pub fn preserve_colors(mut self, preserve: bool) -> Self {
        self.preserve_colors = preserve;
        self
    }
}

/// A fade animation that can be applied to any content
#[derive(Debug)]
pub struct FadeAnimation {
    /// Fade configuration
    config: FadeConfig,
    /// Animation state
    state: AnimationState,
    /// Content to fade
    content: Vec<Line<'static>>,
    /// Original styles for restoration
    original_styles: Vec<Vec<Style>>,
    /// Current opacity
    current_opacity: f32,
    /// Start time of animation
    start_time: Option<Instant>,
}

impl FadeAnimation {
    /// Create a new fade animation
    pub fn new(config: FadeConfig) -> Self {
        Self {
            config,
            state: AnimationState::Idle,
            content: Vec::new(),
            original_styles: Vec::new(),
            current_opacity: 0.0,
            start_time: None,
        }
    }
    
    /// Create a fade-in animation
    pub fn fade_in(duration: Duration) -> Self {
        Self::new(
            FadeConfig::new()
                .direction(FadeDirection::In)
                .animation(AnimationConfig::new().duration(duration))
        )
    }
    
    /// Create a fade-out animation
    pub fn fade_out(duration: Duration) -> Self {
        Self::new(
            FadeConfig::new()
                .direction(FadeDirection::Out)
                .animation(AnimationConfig::new().duration(duration))
        )
    }
    
    /// Create a pulse animation (fade in then out)
    pub fn pulse(duration: Duration) -> Self {
        Self::new(
            FadeConfig::new()
                .direction(FadeDirection::InOut)
                .animation(AnimationConfig::new().duration(duration))
        )
    }
    
    /// Set the content to be animated
    pub fn set_content(&mut self, content: Vec<Line<'static>>) {
        // Store original styles
        self.original_styles = content
            .iter()
            .map(|line| line.spans.iter().map(|span| span.style).collect())
            .collect();
            
        self.content = content;
    }
    
    /// Get the current opacity value
    pub fn opacity(&self) -> f32 {
        self.current_opacity
    }
    
    /// Calculate opacity based on animation progress
    fn calculate_opacity(&self, progress: f32) -> f32 {
        match self.config.direction {
            FadeDirection::In => {
                self.config.start_opacity.interpolate(&self.config.end_opacity, progress)
            }
            FadeDirection::Out => {
                self.config.start_opacity.interpolate(&self.config.end_opacity, progress)
            }
            FadeDirection::InOut => {
                // Fade in for first half, fade out for second half
                if progress <= 0.5 {
                    let fade_in_progress = progress * 2.0;
                    0.0_f32.interpolate(&1.0, fade_in_progress)
                } else {
                    let fade_out_progress = (progress - 0.5) * 2.0;
                    1.0_f32.interpolate(&0.0, fade_out_progress)
                }
            }
        }
    }
    
    /// Apply opacity to a color
    fn apply_opacity_to_color(&self, color: Color, opacity: f32) -> Color {
        if !self.config.preserve_colors {
            return color;
        }
        
        match color {
            Color::Rgb(r, g, b) => {
                // Blend with background (assuming black background)
                let r = (r as f32 * opacity) as u8;
                let g = (g as f32 * opacity) as u8;
                let b = (b as f32 * opacity) as u8;
                Color::Rgb(r, g, b)
            }
            _ => {
                // For other color types, we can't easily apply opacity
                // so we return the original color
                color
            }
        }
    }
    
    /// Apply fade effect to content
    fn apply_fade_to_content(&self) -> Vec<Line<'static>> {
        if self.content.is_empty() {
            return Vec::new();
        }
        
        let opacity = self.current_opacity;
        
        self.content
            .iter()
            .enumerate()
            .map(|(line_idx, line)| {
                let spans: Vec<Span> = line
                    .spans
                    .iter()
                    .enumerate()
                    .map(|(span_idx, span)| {
                        let mut style = span.style;
                        
                        // Apply opacity to foreground color
                        if let Some(fg) = style.fg {
                            style.fg = Some(self.apply_opacity_to_color(fg, opacity));
                        }
                        
                        // Apply opacity to background color
                        if let Some(bg) = style.bg {
                            style.bg = Some(self.apply_opacity_to_color(bg, opacity));
                        }
                        
                        // For very low opacity, make text invisible
                        if opacity < 0.1 {
                            style.fg = Some(Color::Reset);
                        }
                        
                        Span::styled(span.content.clone(), style)
                    })
                    .collect();
                    
                Line::from(spans)
            })
            .collect()
    }
}

impl Animation for FadeAnimation {
    fn start(&mut self) -> Result<()> {
        let now = Instant::now();
        self.state = AnimationState::Running {
            start_time: now,
            current_frame: 0,
        };
        self.start_time = Some(now);
        self.current_opacity = self.config.start_opacity;
        Ok(())
    }
    
    fn stop(&mut self) -> Result<()> {
        self.state = AnimationState::Complete;
        self.current_opacity = self.config.end_opacity;
        Ok(())
    }
    
    fn update(&mut self) -> Result<()> {
        if let AnimationState::Running { start_time, .. } = &self.state {
            let elapsed = start_time.elapsed();
            
            if elapsed >= self.config.animation.duration {
                // Animation complete
                self.state = AnimationState::Complete;
                self.current_opacity = match self.config.direction {
                    FadeDirection::InOut => 0.0, // End transparent for pulse
                    _ => self.config.end_opacity,
                };
            } else {
                // Calculate progress and opacity
                let progress = elapsed.as_secs_f32() / self.config.animation.duration.as_secs_f32();
                let eased_progress = self.config.animation.easing.apply(progress);
                self.current_opacity = self.calculate_opacity(eased_progress);
                
                // Update frame count
                let frame_duration = self.config.animation.frame_duration();
                let frame_count = (elapsed.as_nanos() / frame_duration.as_nanos()) as u32;
                
                self.state = AnimationState::Running {
                    start_time: *start_time,
                    current_frame: frame_count,
                };
            }
        }
        
        Ok(())
    }
    
    fn is_complete(&self) -> bool {
        matches!(self.state, AnimationState::Complete)
    }
    
    fn state(&self) -> &AnimationState {
        &self.state
    }
    
    fn render(&self, _area: Rect, _theme: &Theme) -> Vec<Line> {
        self.apply_fade_to_content()
    }
}

/// A component that can fade in/out its content
pub struct FadingComponent {
    /// The fade animation
    animation: FadeAnimation,
    /// Whether the component is currently visible
    visible: bool,
    /// Fade in configuration
    fade_in_config: FadeConfig,
    /// Fade out configuration
    fade_out_config: FadeConfig,
}

impl FadingComponent {
    /// Create a new fading component
    pub fn new() -> Self {
        Self {
            animation: FadeAnimation::new(FadeConfig::default()),
            visible: false,
            fade_in_config: FadeConfig::new().direction(FadeDirection::In),
            fade_out_config: FadeConfig::new().direction(FadeDirection::Out),
        }
    }
    
    /// Configure fade in animation
    pub fn fade_in_config(mut self, config: FadeConfig) -> Self {
        self.fade_in_config = config;
        self
    }
    
    /// Configure fade out animation
    pub fn fade_out_config(mut self, config: FadeConfig) -> Self {
        self.fade_out_config = config;
        self
    }
    
    /// Show the component with fade in animation
    pub fn show(&mut self, content: Vec<Line<'static>>) -> Result<()> {
        self.animation = FadeAnimation::new(self.fade_in_config.clone());
        self.animation.set_content(content);
        self.animation.start()?;
        self.visible = true;
        Ok(())
    }
    
    /// Hide the component with fade out animation
    pub fn hide(&mut self) -> Result<()> {
        if self.visible {
            let current_content = self.animation.content.clone();
            self.animation = FadeAnimation::new(self.fade_out_config.clone());
            self.animation.set_content(current_content);
            self.animation.start()?;
        }
        Ok(())
    }
    
    /// Update the animation
    pub fn update(&mut self) -> Result<()> {
        self.animation.update()?;
        
        // If fade out animation is complete, mark as not visible
        if self.animation.is_complete() && 
           self.animation.config.direction == FadeDirection::Out {
            self.visible = false;
        }
        
        Ok(())
    }
    
    /// Check if the component is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    
    /// Get the current opacity
    pub fn opacity(&self) -> f32 {
        self.animation.opacity()
    }
    
    /// Render the fading content
    pub fn render(&self, area: Rect, theme: &Theme) -> Vec<Line> {
        if self.visible || !self.animation.is_complete() {
            self.animation.render(area, theme)
        } else {
            Vec::new()
        }
    }
}

impl Default for FadingComponent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fade_config() {
        let config = FadeConfig::new()
            .direction(FadeDirection::In)
            .opacity_range(0.2, 0.8);
            
        assert_eq!(config.direction, FadeDirection::In);
        assert_eq!(config.start_opacity, 0.2);
        assert_eq!(config.end_opacity, 0.8);
    }
    
    #[test]
    fn test_fade_direction_opacity() {
        let fade_in = FadeConfig::new().direction(FadeDirection::In);
        assert_eq!(fade_in.start_opacity, 0.0);
        assert_eq!(fade_in.end_opacity, 1.0);
        
        let fade_out = FadeConfig::new().direction(FadeDirection::Out);
        assert_eq!(fade_out.start_opacity, 1.0);
        assert_eq!(fade_out.end_opacity, 0.0);
    }
    
    #[test]
    fn test_fade_animation_creation() {
        let fade_in = FadeAnimation::fade_in(Duration::from_millis(500));
        assert_eq!(fade_in.config.direction, FadeDirection::In);
        assert_eq!(fade_in.config.animation.duration, Duration::from_millis(500));
        
        let fade_out = FadeAnimation::fade_out(Duration::from_millis(300));
        assert_eq!(fade_out.config.direction, FadeDirection::Out);
        assert_eq!(fade_out.config.animation.duration, Duration::from_millis(300));
    }
    
    #[test]
    fn test_opacity_calculation() {
        let mut animation = FadeAnimation::fade_in(Duration::from_millis(1000));
        
        // Test at 0% progress
        let opacity = animation.calculate_opacity(0.0);
        assert_eq!(opacity, 0.0);
        
        // Test at 50% progress
        let opacity = animation.calculate_opacity(0.5);
        assert!(opacity > 0.0 && opacity < 1.0);
        
        // Test at 100% progress
        let opacity = animation.calculate_opacity(1.0);
        assert_eq!(opacity, 1.0);
    }
    
    #[test]
    fn test_fading_component() {
        let mut component = FadingComponent::new();
        assert!(!component.is_visible());
        
        let content = vec![Line::from("Test content")];
        component.show(content).unwrap();
        assert!(component.is_visible());
    }
}