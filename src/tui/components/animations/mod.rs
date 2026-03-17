//! Animation Framework for Goofy TUI
//!
//! This module provides a comprehensive animation system for the Goofy terminal interface,
//! including loading spinners, smooth transitions, and visual polish effects.
//!
//! ## Features
//!
//! - High-performance animation engine with frame-based timing
//! - Multiple easing functions for smooth transitions
//! - Loading spinners with customizable styles
//! - Fade, slide, bounce, and glow effects
//! - Timeline-based animation sequencing
//! - Integration with existing component system

pub mod animation_engine;
pub mod timeline;
pub mod transitions;
pub mod interpolation;

// Loading states and visual feedback
pub mod spinners;
pub mod progress;
pub mod loading;
pub mod pulse;

// Visual effects
pub mod fade;
pub mod slide;
pub mod bounce;
pub mod glow;

// Component integrations
pub mod animated_text;
pub mod animated_list;
pub mod animated_dialog;
pub mod animated_input;

use crate::tui::themes::{Theme, Styles};
use anyhow::Result;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

// Re-exports for easy access from other modules
pub use animation_engine::*;
pub use timeline::Timeline;
pub use transitions::*;

// Alias for compatibility
pub type Easing = EasingType;

/// Animation events for inter-component communication
#[derive(Debug, Clone)]
pub enum AnimationEvent {
    /// Start an animation with the given ID
    Start { animation_id: String },
    /// Stop an animation with the given ID
    Stop { animation_id: String },
    /// Update animation frame
    Frame { animation_id: String, frame: u32 },
    /// Animation completed
    Complete { animation_id: String },
    /// Animation error
    Error { animation_id: String, error: String },
}

/// Animation state for components
#[derive(Debug, Clone, PartialEq)]
pub enum AnimationState {
    /// Animation has not started
    Idle,
    /// Animation is currently running
    Running { start_time: Instant, current_frame: u32 },
    /// Animation is paused
    Paused { pause_time: Instant, elapsed: Duration },
    /// Animation has completed
    Complete,
    /// Animation encountered an error
    Error(String),
}

impl Default for AnimationState {
    fn default() -> Self {
        Self::Idle
    }
}

impl AnimationState {
    /// Create a new animation state in Idle state
    pub fn new() -> Self {
        Self::Idle
    }
    
    /// Check if the animation is currently active
    pub fn is_active(&self) -> bool {
        matches!(self, AnimationState::Running { .. })
    }
    
    /// Update the animation state with elapsed time
    pub fn update(&mut self, delta_time: Duration) {
        if let AnimationState::Running { current_frame, .. } = self {
            *current_frame += 1;
        }
    }
    
    /// Get the current progress (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        match self {
            AnimationState::Idle => 0.0,
            AnimationState::Running { current_frame, .. } => {
                // Simple progress calculation - could be improved
                (*current_frame as f32 / 60.0).min(1.0)
            }
            AnimationState::Paused { .. } => 0.5, // Arbitrary value for paused state
            AnimationState::Complete => 1.0,
            AnimationState::Error(_) => 0.0,
        }
    }
}

/// Easing functions for smooth animations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EasingType {
    /// Linear interpolation (constant speed)
    Linear,
    /// Ease in (slow start)
    EaseIn,
    /// Ease out (slow end)
    EaseOut,
    /// Ease in-out (slow start and end)
    EaseInOut,
    /// Bounce effect
    Bounce,
    /// Elastic effect
    Elastic,
    /// Back effect (overshoot)
    Back,
    // Extended easing variants
    EaseInBack,
    EaseOutBack,
    EaseInOutBack,
    EaseInQuad,
    EaseOutQuad,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    EaseInQuart,
    EaseOutQuart,
    EaseOutBounce,
    EaseOutElastic,
}

impl EasingType {
    /// Apply the easing function to a progress value (0.0 to 1.0)
    pub fn apply(self, progress: f32) -> f32 {
        match self {
            EasingType::Linear => progress,
            EasingType::EaseIn => progress * progress,
            EasingType::EaseOut => 1.0 - (1.0 - progress) * (1.0 - progress),
            EasingType::EaseInOut => {
                if progress < 0.5 {
                    2.0 * progress * progress
                } else {
                    1.0 - 2.0 * (1.0 - progress) * (1.0 - progress)
                }
            }
            EasingType::Bounce => {
                let n1 = 7.5625;
                let d1 = 2.75;
                
                if progress < 1.0 / d1 {
                    n1 * progress * progress
                } else if progress < 2.0 / d1 {
                    let progress = progress - 1.5 / d1;
                    n1 * progress * progress + 0.75
                } else if progress < 2.5 / d1 {
                    let progress = progress - 2.25 / d1;
                    n1 * progress * progress + 0.9375
                } else {
                    let progress = progress - 2.625 / d1;
                    n1 * progress * progress + 0.984375
                }
            }
            EasingType::Elastic => {
                if progress == 0.0 || progress == 1.0 {
                    progress
                } else {
                    let c4 = (2.0 * std::f32::consts::PI) / 3.0;
                    -(2.0_f32.powf(10.0 * progress - 10.0)) * ((progress * 10.0 - 10.75) * c4).sin()
                }
            }
            EasingType::Back | EasingType::EaseInBack => {
                let c1 = 1.70158;
                let c3 = c1 + 1.0;
                c3 * progress * progress * progress - c1 * progress * progress
            }
            EasingType::EaseOutBack => {
                let c1 = 1.70158;
                let c3 = c1 + 1.0;
                let p = progress - 1.0;
                1.0 + c3 * p * p * p + c1 * p * p
            }
            EasingType::EaseInOutBack => {
                let c1 = 1.70158;
                let c2 = c1 * 1.525;
                if progress < 0.5 {
                    (2.0 * progress).powi(2) * ((c2 + 1.0) * 2.0 * progress - c2) / 2.0
                } else {
                    ((2.0 * progress - 2.0).powi(2) * ((c2 + 1.0) * (progress * 2.0 - 2.0) + c2) + 2.0) / 2.0
                }
            }
            EasingType::EaseInQuad => progress * progress,
            EasingType::EaseOutQuad => 1.0 - (1.0 - progress) * (1.0 - progress),
            EasingType::EaseInCubic => progress * progress * progress,
            EasingType::EaseOutCubic => 1.0 - (1.0 - progress).powi(3),
            EasingType::EaseInOutCubic => {
                if progress < 0.5 {
                    4.0 * progress * progress * progress
                } else {
                    1.0 - (-2.0 * progress + 2.0).powi(3) / 2.0
                }
            }
            EasingType::EaseInQuart => progress * progress * progress * progress,
            EasingType::EaseOutQuart => 1.0 - (1.0 - progress).powi(4),
            EasingType::EaseOutBounce => {
                let n1 = 7.5625;
                let d1 = 2.75;
                if progress < 1.0 / d1 {
                    n1 * progress * progress
                } else if progress < 2.0 / d1 {
                    let p = progress - 1.5 / d1;
                    n1 * p * p + 0.75
                } else if progress < 2.5 / d1 {
                    let p = progress - 2.25 / d1;
                    n1 * p * p + 0.9375
                } else {
                    let p = progress - 2.625 / d1;
                    n1 * p * p + 0.984375
                }
            }
            EasingType::EaseOutElastic => {
                if progress == 0.0 || progress == 1.0 {
                    progress
                } else {
                    let c4 = (2.0 * std::f32::consts::PI) / 3.0;
                    2.0_f32.powf(-10.0 * progress) * ((progress * 10.0 - 0.75) * c4).sin() + 1.0
                }
            }
        }
    }
}

/// Animation configuration
#[derive(Debug, Clone)]
pub struct AnimationConfig {
    /// Duration of the animation
    pub duration: Duration,
    /// Easing function to use
    pub easing: EasingType,
    /// Whether the animation should loop
    pub repeat: bool,
    /// Number of times to repeat (None for infinite)
    pub repeat_count: Option<u32>,
    /// Delay before starting the animation
    pub delay: Duration,
    /// Target frames per second
    pub fps: u32,
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            duration: Duration::from_millis(300),
            easing: EasingType::EaseInOut,
            repeat: false,
            repeat_count: None,
            delay: Duration::ZERO,
            fps: 60,
        }
    }
}

impl AnimationConfig {
    /// Create a new animation configuration with default values or specific duration
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with a specific duration
    pub fn from_duration(dur: Duration) -> Self {
        Self {
            duration: dur,
            ..Self::default()
        }
    }

    /// Set the duration of the animation
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }
    
    /// Set the easing function
    pub fn easing(mut self, easing: EasingType) -> Self {
        self.easing = easing;
        self
    }

    /// Alias for `easing()` - set the easing function
    pub fn with_easing(self, easing: EasingType) -> Self {
        self.easing(easing)
    }
    
    /// Set whether the animation should repeat
    pub fn repeat(mut self, repeat: bool) -> Self {
        self.repeat = repeat;
        self
    }
    
    /// Set the number of times to repeat
    pub fn repeat_count(mut self, count: u32) -> Self {
        self.repeat_count = Some(count);
        self
    }
    
    /// Set the delay before starting
    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
    
    /// Set the target FPS
    pub fn fps(mut self, fps: u32) -> Self {
        self.fps = fps;
        self
    }
    
    /// Calculate the frame duration based on FPS
    pub fn frame_duration(&self) -> Duration {
        Duration::from_nanos(1_000_000_000 / self.fps as u64)
    }
    
    /// Calculate the total number of frames for this animation
    pub fn total_frames(&self) -> u32 {
        let frame_duration = self.frame_duration();
        (self.duration.as_nanos() / frame_duration.as_nanos()) as u32
    }
}

/// Trait for animated values that can be interpolated
pub trait Animatable {
    /// Interpolate between two values given a progress (0.0 to 1.0)
    fn interpolate(&self, target: &Self, progress: f32) -> Self;
}

impl Animatable for f32 {
    fn interpolate(&self, target: &Self, progress: f32) -> Self {
        self + (target - self) * progress
    }
}

impl Animatable for Color {
    fn interpolate(&self, target: &Self, progress: f32) -> Self {
        match (self, target) {
            (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
                let r = (*r1 as f32).interpolate(&(*r2 as f32), progress) as u8;
                let g = (*g1 as f32).interpolate(&(*g2 as f32), progress) as u8;
                let b = (*b1 as f32).interpolate(&(*b2 as f32), progress) as u8;
                Color::Rgb(r, g, b)
            }
            _ => {
                // For non-RGB colors, just switch at 50% progress
                if progress < 0.5 { *self } else { *target }
            }
        }
    }
}

impl Animatable for Style {
    fn interpolate(&self, target: &Self, progress: f32) -> Self {
        let mut style = *self;
        
        // Interpolate foreground color
        if let (Some(fg1), Some(fg2)) = (self.fg, target.fg) {
            style.fg = Some(fg1.interpolate(&fg2, progress));
        }
        
        // Interpolate background color
        if let (Some(bg1), Some(bg2)) = (self.bg, target.bg) {
            style.bg = Some(bg1.interpolate(&bg2, progress));
        }
        
        style
    }
}

/// Animation manager for coordinating multiple animations
pub struct AnimationManager {
    /// Active animations
    animations: std::collections::HashMap<String, Box<dyn Animation + Send + Sync>>,
    /// Event sender for animation updates
    event_sender: mpsc::UnboundedSender<AnimationEvent>,
}

impl AnimationManager {
    /// Create a new animation manager
    pub fn new(event_sender: mpsc::UnboundedSender<AnimationEvent>) -> Self {
        Self {
            animations: std::collections::HashMap::new(),
            event_sender,
        }
    }
    
    /// Register a new animation
    pub fn register_animation(&mut self, id: String, animation: Box<dyn Animation + Send + Sync>) -> Result<()> {
        self.animations.insert(id, animation);
        Ok(())
    }
    
    /// Start an animation by ID
    pub fn start_animation(&mut self, id: &str) -> Result<()> {
        if let Some(animation) = self.animations.get_mut(id) {
            animation.start()?;
            let _ = self.event_sender.send(AnimationEvent::Start {
                animation_id: id.to_string(),
            });
        }
        Ok(())
    }
    
    /// Stop an animation by ID
    pub fn stop_animation(&mut self, id: &str) -> Result<()> {
        if let Some(animation) = self.animations.get_mut(id) {
            animation.stop()?;
            let _ = self.event_sender.send(AnimationEvent::Stop {
                animation_id: id.to_string(),
            });
        }
        Ok(())
    }
    
    /// Update all animations (call this every frame)
    pub fn update(&mut self) -> Result<()> {
        let mut completed = Vec::new();
        
        for (id, animation) in &mut self.animations {
            animation.update()?;
            
            if animation.is_complete() {
                completed.push(id.clone());
            }
        }
        
        // Send completion events
        for id in completed {
            let _ = self.event_sender.send(AnimationEvent::Complete {
                animation_id: id,
            });
        }
        
        Ok(())
    }
    
}

/// Base trait for all animations
pub trait Animation: std::fmt::Debug {
    /// Start the animation
    fn start(&mut self) -> Result<()>;
    
    /// Stop the animation
    fn stop(&mut self) -> Result<()>;
    
    /// Update the animation (called every frame)
    fn update(&mut self) -> Result<()>;
    
    /// Check if the animation is complete
    fn is_complete(&self) -> bool;
    
    /// Get the current animation state
    fn state(&self) -> &AnimationState;
    
    /// Render the animation to text spans
    fn render(&self, area: Rect, theme: &Theme) -> Vec<Line>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_easing_functions() {
        // Test linear easing
        assert_eq!(EasingType::Linear.apply(0.0), 0.0);
        assert_eq!(EasingType::Linear.apply(0.5), 0.5);
        assert_eq!(EasingType::Linear.apply(1.0), 1.0);
        
        // Test ease in
        assert_eq!(EasingType::EaseIn.apply(0.0), 0.0);
        assert!(EasingType::EaseIn.apply(0.5) < 0.5);
        assert_eq!(EasingType::EaseIn.apply(1.0), 1.0);
        
        // Test ease out
        assert_eq!(EasingType::EaseOut.apply(0.0), 0.0);
        assert!(EasingType::EaseOut.apply(0.5) > 0.5);
        assert_eq!(EasingType::EaseOut.apply(1.0), 1.0);
    }
    
    #[test]
    fn test_animation_config() {
        let config = AnimationConfig::new()
            .duration(Duration::from_millis(500))
            .easing(EasingType::Bounce)
            .repeat(true)
            .fps(30);
            
        assert_eq!(config.duration, Duration::from_millis(500));
        assert_eq!(config.easing, EasingType::Bounce);
        assert!(config.repeat);
        assert_eq!(config.fps, 30);
    }
    
    #[test]
    fn test_color_interpolation() {
        let color1 = Color::Rgb(255, 0, 0);   // Red
        let color2 = Color::Rgb(0, 255, 0);   // Green
        
        let mid = color1.interpolate(&color2, 0.5);
        assert_eq!(mid, Color::Rgb(127, 127, 0));
    }
    
    #[test]
    fn test_animation_state() {
        let state = AnimationState::default();
        assert_eq!(state, AnimationState::Idle);
        
        let running = AnimationState::Running {
            start_time: Instant::now(),
            current_frame: 0,
        };
        assert!(matches!(running, AnimationState::Running { .. }));
    }
}