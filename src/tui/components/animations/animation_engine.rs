//! Core animation engine with easing functions and timing control.
//! 
//! This module provides the fundamental animation primitives for smooth UI transitions
//! and effects. It supports various easing functions, frame-based timing, and value
//! interpolation for creating fluid animations.

use anyhow::Result;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Frames per second for animations
pub const DEFAULT_FPS: u8 = 60;
pub const DEFAULT_LOADING_FPS: u8 = 20;

/// Animation state tracking
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimationState {
    Idle,
    Running,
    Paused,
    Completed,
}

/// Easing function types for smooth animations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EasingType {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    EaseInQuart,
    EaseOutQuart,
    EaseInOutQuart,
    EaseInBounce,
    EaseOutBounce,
    EaseInOutBounce,
    EaseInElastic,
    EaseOutElastic,
    EaseInOutElastic,
    Bounce,
    Elastic,
    Back,
    EaseInBack,
    EaseOutBack,
    EaseInOutBack,
}

impl From<super::EasingType> for EasingType {
    fn from(e: super::EasingType) -> Self {
        match e {
            super::EasingType::Linear => EasingType::Linear,
            super::EasingType::EaseIn => EasingType::EaseIn,
            super::EasingType::EaseOut => EasingType::EaseOut,
            super::EasingType::EaseInOut => EasingType::EaseInOut,
            super::EasingType::Bounce => EasingType::Bounce,
            super::EasingType::Elastic => EasingType::Elastic,
            super::EasingType::Back | super::EasingType::EaseInBack => EasingType::EaseInBack,
            super::EasingType::EaseOutBack => EasingType::EaseOutBack,
            super::EasingType::EaseInOutBack => EasingType::EaseInOutBack,
            super::EasingType::EaseInQuad => EasingType::EaseInQuad,
            super::EasingType::EaseOutQuad => EasingType::EaseOutQuad,
            super::EasingType::EaseInCubic => EasingType::EaseInCubic,
            super::EasingType::EaseOutCubic => EasingType::EaseOutCubic,
            super::EasingType::EaseInOutCubic => EasingType::EaseInOutCubic,
            super::EasingType::EaseInQuart => EasingType::EaseInQuart,
            super::EasingType::EaseOutQuart => EasingType::EaseOutQuart,
            super::EasingType::EaseOutBounce => EasingType::EaseOutBounce,
            super::EasingType::EaseOutElastic => EasingType::EaseOutElastic,
        }
    }
}

/// Animation configuration
#[derive(Debug, Clone)]
pub struct AnimationConfig {
    pub duration: Duration,
    pub easing: EasingType,
    pub fps: u8,
    pub loop_count: Option<u32>, // None = infinite loop
    pub reverse: bool,
    pub delay: Duration,
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            duration: Duration::from_millis(300),
            easing: EasingType::EaseInOut,
            fps: DEFAULT_FPS,
            loop_count: None,
            reverse: false,
            delay: Duration::from_millis(0),
        }
    }
}

impl AnimationConfig {
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            ..Default::default()
        }
    }

    pub fn with_easing(mut self, easing: EasingType) -> Self {
        self.easing = easing;
        self
    }

    pub fn with_fps(mut self, fps: u8) -> Self {
        self.fps = fps;
        self
    }

    pub fn with_loop_count(mut self, count: u32) -> Self {
        self.loop_count = Some(count);
        self
    }

    pub fn infinite(mut self) -> Self {
        self.loop_count = None;
        self
    }

    pub fn with_reverse(mut self) -> Self {
        self.reverse = true;
        self
    }

    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// Quick configurations for common use cases
    pub fn fade_in() -> Self {
        Self::new(Duration::from_millis(300))
            .with_easing(EasingType::EaseOut)
    }

    pub fn fade_out() -> Self {
        Self::new(Duration::from_millis(200))
            .with_easing(EasingType::EaseIn)
    }

    pub fn slide_in() -> Self {
        Self::new(Duration::from_millis(400))
            .with_easing(EasingType::EaseOutQuad)
    }

    pub fn slide_out() -> Self {
        Self::new(Duration::from_millis(250))
            .with_easing(EasingType::EaseInQuad)
    }

    pub fn bounce() -> Self {
        Self::new(Duration::from_millis(600))
            .with_easing(EasingType::EaseOutBounce)
    }

    pub fn pulse() -> Self {
        Self::new(Duration::from_millis(1000))
            .with_easing(EasingType::EaseInOut)
            .infinite()
            .with_reverse()
    }

    pub fn spinner() -> Self {
        Self::new(Duration::from_millis(50))
            .with_easing(EasingType::Linear)
            .with_fps(DEFAULT_LOADING_FPS)
            .infinite()
    }
}

/// Core animation engine
#[derive(Debug)]
pub struct AnimationEngine {
    config: AnimationConfig,
    state: AnimationState,
    start_time: Option<Instant>,
    pause_time: Option<Instant>,
    paused_duration: Duration,
    current_loop: u32,
    last_frame_time: Option<Instant>,
}

impl AnimationEngine {
    pub fn new(config: AnimationConfig) -> Self {
        Self {
            config,
            state: AnimationState::Idle,
            start_time: None,
            pause_time: None,
            paused_duration: Duration::from_millis(0),
            current_loop: 0,
            last_frame_time: None,
        }
    }

    /// Start the animation
    pub fn start(&mut self) {
        match self.state {
            AnimationState::Idle => {
                self.start_time = Some(Instant::now() + self.config.delay);
                self.state = AnimationState::Running;
                self.current_loop = 0;
                self.paused_duration = Duration::from_millis(0);
            }
            AnimationState::Paused => {
                if let Some(pause_time) = self.pause_time {
                    self.paused_duration += pause_time.elapsed();
                    self.pause_time = None;
                }
                self.state = AnimationState::Running;
            }
            _ => {} // Already running or completed
        }
    }

    /// Pause the animation
    pub fn pause(&mut self) {
        if self.state == AnimationState::Running {
            self.pause_time = Some(Instant::now());
            self.state = AnimationState::Paused;
        }
    }

    /// Stop and reset the animation
    pub fn stop(&mut self) {
        self.state = AnimationState::Idle;
        self.start_time = None;
        self.pause_time = None;
        self.paused_duration = Duration::from_millis(0);
        self.current_loop = 0;
        self.last_frame_time = None;
    }

    /// Reset the animation to start position
    pub fn reset(&mut self) {
        let was_running = self.state == AnimationState::Running;
        self.stop();
        if was_running {
            self.start();
        }
    }

    /// Get current animation progress (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        match self.state {
            AnimationState::Idle => 0.0,
            AnimationState::Completed => 1.0,
            AnimationState::Running | AnimationState::Paused => {
                if let Some(start_time) = self.start_time {
                    let now = Instant::now();
                    if now < start_time {
                        // Still in delay period
                        return 0.0;
                    }

                    let elapsed = if self.state == AnimationState::Paused {
                        if let Some(pause_time) = self.pause_time {
                            pause_time.duration_since(start_time) - self.paused_duration
                        } else {
                            Duration::from_millis(0)
                        }
                    } else {
                        now.duration_since(start_time) - self.paused_duration
                    };

                    let progress = elapsed.as_secs_f32() / self.config.duration.as_secs_f32();
                    progress.clamp(0.0, 1.0)
                } else {
                    0.0
                }
            }
        }
    }

    /// Get eased progress value using the configured easing function
    pub fn eased_progress(&self) -> f32 {
        let progress = self.progress();
        ease(progress, self.config.easing)
    }

    /// Check if animation should continue running
    pub fn should_update(&mut self) -> bool {
        if self.state != AnimationState::Running {
            return false;
        }

        let now = Instant::now();
        
        // Check frame rate limiting
        if let Some(last_frame) = self.last_frame_time {
            let frame_duration = Duration::from_secs_f32(1.0 / self.config.fps as f32);
            if now.duration_since(last_frame) < frame_duration {
                return false;
            }
        }

        self.last_frame_time = Some(now);

        // Check if animation is complete
        let progress = self.progress();
        if progress >= 1.0 {
            self.current_loop += 1;
            
            // Check if we should continue looping
            if let Some(max_loops) = self.config.loop_count {
                if self.current_loop >= max_loops {
                    self.state = AnimationState::Completed;
                    return false;
                }
            }
            
            // Reset for next loop
            self.start_time = Some(now);
            self.paused_duration = Duration::from_millis(0);
        }

        true
    }

    /// Get current state
    pub fn state(&self) -> AnimationState {
        self.state
    }

    /// Check if animation is running
    pub fn is_running(&self) -> bool {
        self.state == AnimationState::Running
    }

    /// Check if animation is completed
    pub fn is_completed(&self) -> bool {
        self.state == AnimationState::Completed
    }

    /// Get current loop count
    pub fn current_loop(&self) -> u32 {
        self.current_loop
    }

    /// Get frame interval for the current FPS
    pub fn frame_interval(&self) -> Duration {
        Duration::from_secs_f32(1.0 / self.config.fps as f32)
    }

    /// Sleep until next frame
    pub async fn wait_for_next_frame(&self) {
        sleep(self.frame_interval()).await;
    }
}

/// Apply easing function to a progress value (0.0 to 1.0)
pub fn ease(t: f32, easing: EasingType) -> f32 {
    let t = t.clamp(0.0, 1.0);
    
    match easing {
        EasingType::Linear => t,
        EasingType::EaseIn => t * t,
        EasingType::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
        EasingType::EaseInOut => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                1.0 - 2.0 * (1.0 - t) * (1.0 - t)
            }
        }
        EasingType::EaseInQuad => t * t,
        EasingType::EaseOutQuad => 1.0 - (1.0 - t) * (1.0 - t),
        EasingType::EaseInOutQuad => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                1.0 - 2.0 * (1.0 - t) * (1.0 - t)
            }
        }
        EasingType::EaseInCubic => t * t * t,
        EasingType::EaseOutCubic => 1.0 - (1.0 - t).powi(3),
        EasingType::EaseInOutCubic => {
            if t < 0.5 {
                4.0 * t * t * t
            } else {
                1.0 - 4.0 * (1.0 - t).powi(3)
            }
        }
        EasingType::EaseInQuart => t.powi(4),
        EasingType::EaseOutQuart => 1.0 - (1.0 - t).powi(4),
        EasingType::EaseInOutQuart => {
            if t < 0.5 {
                8.0 * t.powi(4)
            } else {
                1.0 - 8.0 * (1.0 - t).powi(4)
            }
        }
        EasingType::EaseInBounce => 1.0 - ease_out_bounce(1.0 - t),
        EasingType::EaseOutBounce => ease_out_bounce(t),
        EasingType::EaseInOutBounce => {
            if t < 0.5 {
                (1.0 - ease_out_bounce(1.0 - 2.0 * t)) / 2.0
            } else {
                (1.0 + ease_out_bounce(2.0 * t - 1.0)) / 2.0
            }
        }
        EasingType::EaseInElastic => {
            if t == 0.0 || t == 1.0 {
                t
            } else {
                let p = 0.3;
                let s = p / 4.0;
                -(2.0_f32.powf(10.0 * (t - 1.0)) * ((t - 1.0 - s) * (2.0 * std::f32::consts::PI) / p).sin())
            }
        }
        EasingType::EaseOutElastic => {
            if t == 0.0 || t == 1.0 {
                t
            } else {
                let p = 0.3;
                let s = p / 4.0;
                2.0_f32.powf(-10.0 * t) * ((t - s) * (2.0 * std::f32::consts::PI) / p).sin() + 1.0
            }
        }
        EasingType::EaseInOutElastic => {
            if t == 0.0 || t == 1.0 {
                t
            } else if t < 0.5 {
                let p = 0.3 * 1.5;
                let s = p / 4.0;
                -0.5 * (2.0_f32.powf(10.0 * (2.0 * t - 1.0)) * ((2.0 * t - 1.0 - s) * (2.0 * std::f32::consts::PI) / p).sin())
            } else {
                let p = 0.3 * 1.5;
                let s = p / 4.0;
                0.5 * 2.0_f32.powf(-10.0 * (2.0 * t - 1.0)) * ((2.0 * t - 1.0 - s) * (2.0 * std::f32::consts::PI) / p).sin() + 1.0
            }
        }
        EasingType::Bounce => ease_out_bounce(t),
        EasingType::Elastic => {
            if t == 0.0 || t == 1.0 {
                t
            } else {
                let p = 0.3;
                let s = p / 4.0;
                2.0_f32.powf(-10.0 * t) * ((t - s) * (2.0 * std::f32::consts::PI) / p).sin() + 1.0
            }
        }
        EasingType::Back | EasingType::EaseInBack => {
            let c1 = 1.70158;
            let c3 = c1 + 1.0;
            c3 * t * t * t - c1 * t * t
        }
        EasingType::EaseOutBack => {
            let c1 = 1.70158;
            let c3 = c1 + 1.0;
            let p = t - 1.0;
            1.0 + c3 * p * p * p + c1 * p * p
        }
        EasingType::EaseInOutBack => {
            let c1 = 1.70158;
            let c2 = c1 * 1.525;
            if t < 0.5 {
                (2.0 * t).powi(2) * ((c2 + 1.0) * 2.0 * t - c2) / 2.0
            } else {
                ((2.0 * t - 2.0).powi(2) * ((c2 + 1.0) * (t * 2.0 - 2.0) + c2) + 2.0) / 2.0
            }
        }
    }
}

/// Helper function for bounce easing
fn ease_out_bounce(t: f32) -> f32 {
    if t < 1.0 / 2.75 {
        7.5625 * t * t
    } else if t < 2.0 / 2.75 {
        let t = t - 1.5 / 2.75;
        7.5625 * t * t + 0.75
    } else if t < 2.5 / 2.75 {
        let t = t - 2.25 / 2.75;
        7.5625 * t * t + 0.9375
    } else {
        let t = t - 2.625 / 2.75;
        7.5625 * t * t + 0.984375
    }
}

/// Utility function to interpolate between two values
pub fn interpolate<T>(start: T, end: T, progress: f32) -> T
where
    T: std::ops::Add<Output = T> + std::ops::Mul<f32, Output = T> + std::ops::Sub<Output = T> + Copy,
{
    start + (end - start) * progress
}

/// Utility function to create smooth color transitions
pub fn interpolate_color(start_rgb: (u8, u8, u8), end_rgb: (u8, u8, u8), progress: f32) -> (u8, u8, u8) {
    let progress = progress.clamp(0.0, 1.0);
    (
        (start_rgb.0 as f32 + (end_rgb.0 as f32 - start_rgb.0 as f32) * progress) as u8,
        (start_rgb.1 as f32 + (end_rgb.1 as f32 - start_rgb.1 as f32) * progress) as u8,
        (start_rgb.2 as f32 + (end_rgb.2 as f32 - start_rgb.2 as f32) * progress) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_animation_config_creation() {
        let config = AnimationConfig::new(Duration::from_millis(500))
            .with_easing(EasingType::EaseInOut)
            .with_fps(30);
        
        assert_eq!(config.duration, Duration::from_millis(500));
        assert_eq!(config.easing, EasingType::EaseInOut);
        assert_eq!(config.fps, 30);
    }

    #[test]
    fn test_animation_engine_lifecycle() {
        let config = AnimationConfig::new(Duration::from_millis(100));
        let mut engine = AnimationEngine::new(config);
        
        assert_eq!(engine.state(), AnimationState::Idle);
        assert_eq!(engine.progress(), 0.0);
        
        engine.start();
        assert_eq!(engine.state(), AnimationState::Running);
        
        engine.pause();
        assert_eq!(engine.state(), AnimationState::Paused);
        
        engine.stop();
        assert_eq!(engine.state(), AnimationState::Idle);
    }

    #[test]
    fn test_easing_functions() {
        // Test linear easing
        assert_eq!(ease(0.0, EasingType::Linear), 0.0);
        assert_eq!(ease(0.5, EasingType::Linear), 0.5);
        assert_eq!(ease(1.0, EasingType::Linear), 1.0);
        
        // Test ease in/out
        let ease_in_half = ease(0.5, EasingType::EaseIn);
        assert!(ease_in_half < 0.5); // Should be slower at start
        
        let ease_out_half = ease(0.5, EasingType::EaseOut);
        assert!(ease_out_half > 0.5); // Should be faster at start
    }

    #[test]
    fn test_interpolation() {
        assert_eq!(interpolate(0.0, 10.0, 0.0), 0.0);
        assert_eq!(interpolate(0.0, 10.0, 0.5), 5.0);
        assert_eq!(interpolate(0.0, 10.0, 1.0), 10.0);
    }

    #[test]
    fn test_color_interpolation() {
        let start = (255, 0, 0); // Red
        let end = (0, 255, 0);   // Green
        let middle = interpolate_color(start, end, 0.5);
        
        assert_eq!(middle, (127, 127, 0)); // Should be yellowish
    }
}