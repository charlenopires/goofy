//! Pulse animations for highlighting and attention-grabbing effects.
//! 
//! This module provides various pulse animation styles for drawing attention
//! to UI elements, notifications, and interactive components.

use super::animation_engine::{AnimationEngine, AnimationConfig, EasingType};
use super::interpolation::{RgbColor, Interpolatable};
use super::{Animation, AnimationState};
use crate::tui::themes::Theme;
use anyhow::Result;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Modifier};
use ratatui::text::{Span, Line};
use std::time::{Duration, Instant};

/// Pulse animation styles
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PulseStyle {
    /// Simple opacity fade in/out
    Fade,
    /// Color intensity change
    Glow,
    /// Size scaling effect (simulated with characters)
    Scale,
    /// Border/outline pulse
    Border,
    /// Breathing effect (slow fade)
    Breathe,
    /// Heartbeat-like double pulse
    Heartbeat,
    /// Rainbow color cycling
    Rainbow,
    /// Attention-grabbing flash
    Flash,
}

/// Pulse animation configuration
#[derive(Debug, Clone)]
pub struct PulseConfig {
    pub style: PulseStyle,
    pub duration: Duration,
    pub intensity: f32, // 0.0 to 1.0
    pub base_color: RgbColor,
    pub pulse_color: RgbColor,
    pub reverse: bool,
    pub loop_count: Option<u32>, // None = infinite
}

impl Default for PulseConfig {
    fn default() -> Self {
        Self {
            style: PulseStyle::Fade,
            duration: Duration::from_millis(1000),
            intensity: 0.8,
            base_color: RgbColor::new(150, 150, 150),
            pulse_color: RgbColor::new(255, 255, 255),
            reverse: false,
            loop_count: None,
        }
    }
}

impl PulseConfig {
    pub fn new(style: PulseStyle) -> Self {
        Self {
            style,
            ..Default::default()
        }
    }

    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity.clamp(0.0, 1.0);
        self
    }

    pub fn with_colors(mut self, base: RgbColor, pulse: RgbColor) -> Self {
        self.base_color = base;
        self.pulse_color = pulse;
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

    pub fn reversed(mut self) -> Self {
        self.reverse = true;
        self
    }

    /// Quick configurations for common use cases
    pub fn notification() -> Self {
        Self::new(PulseStyle::Glow)
            .with_duration(Duration::from_millis(800))
            .with_colors(
                RgbColor::new(100, 100, 100),
                RgbColor::new(255, 200, 100),
            )
            .with_intensity(0.9)
    }

    pub fn error_flash() -> Self {
        Self::new(PulseStyle::Flash)
            .with_duration(Duration::from_millis(150))
            .with_colors(
                RgbColor::new(100, 100, 100),
                RgbColor::new(255, 100, 100),
            )
            .with_loop_count(3)
            .with_intensity(1.0)
    }

    pub fn success_glow() -> Self {
        Self::new(PulseStyle::Glow)
            .with_duration(Duration::from_millis(600))
            .with_colors(
                RgbColor::new(100, 150, 100),
                RgbColor::new(150, 255, 150),
            )
            .with_loop_count(2)
    }

    pub fn focus_highlight() -> Self {
        Self::new(PulseStyle::Border)
            .with_duration(Duration::from_millis(1200))
            .with_colors(
                RgbColor::new(100, 100, 200),
                RgbColor::new(150, 150, 255),
            )
            .infinite()
    }

    pub fn breathing() -> Self {
        Self::new(PulseStyle::Breathe)
            .with_duration(Duration::from_millis(2000))
            .with_colors(
                RgbColor::new(120, 120, 120),
                RgbColor::new(200, 200, 200),
            )
            .infinite()
    }

    pub fn rainbow_cycle() -> Self {
        Self::new(PulseStyle::Rainbow)
            .with_duration(Duration::from_millis(3000))
            .infinite()
    }
}

/// Pulse animation component
#[derive(Debug)]
pub struct PulseAnimation {
    config: PulseConfig,
    animation: AnimationEngine,
    content: String,
    is_active: bool,
    anim_state: AnimationState,
}

impl PulseAnimation {
    pub fn new(config: PulseConfig, content: String) -> Self {
        let easing = match config.style {
            PulseStyle::Fade | PulseStyle::Glow | PulseStyle::Breathe => EasingType::EaseInOut,
            PulseStyle::Scale => EasingType::EaseOutElastic,
            PulseStyle::Border => EasingType::EaseInOutQuad,
            PulseStyle::Heartbeat => EasingType::EaseOutBounce,
            PulseStyle::Rainbow => EasingType::Linear,
            PulseStyle::Flash => EasingType::EaseInOutQuart,
        };

        let mut animation_config = AnimationConfig::new(config.duration)
            .with_easing(easing);

        if config.reverse {
            animation_config = animation_config.with_reverse();
        }

        let animation_config = if let Some(count) = config.loop_count {
            animation_config.with_loop_count(count)
        } else {
            animation_config.infinite()
        };

        Self {
            config,
            animation: AnimationEngine::new(animation_config),
            content,
            is_active: false,
            anim_state: AnimationState::Idle,
        }
    }

    /// Start the pulse animation
    pub fn start(&mut self) {
        self.animation.start();
        self.is_active = true;
        self.anim_state = AnimationState::Running {
            start_time: Instant::now(),
            current_frame: 0,
        };
    }

    /// Stop the pulse animation
    pub fn stop(&mut self) {
        self.animation.stop();
        self.is_active = false;
        self.anim_state = AnimationState::Complete;
    }

    /// Pause the pulse animation
    pub fn pause(&mut self) {
        self.animation.pause();
    }

    /// Resume the pulse animation
    pub fn resume(&mut self) {
        self.animation.start(); // This resumes from pause
    }

    /// Update the animation
    pub fn update(&mut self) -> Result<bool> {
        if self.is_active {
            Ok(self.animation.should_update())
        } else {
            Ok(false)
        }
    }

    /// Update the content text
    pub fn set_content(&mut self, content: String) {
        self.content = content;
    }

    /// Render the pulsing content
    pub fn render(&self) -> Line {
        if !self.is_active {
            return Line::from(Span::styled(
                &self.content,
                Style::default().fg(self.config.base_color.to_color()),
            ));
        }

        let progress = self.animation.eased_progress();
        let style = self.calculate_style(progress);

        Line::from(Span::styled(&self.content, style))
    }

    /// Calculate the style based on animation progress
    fn calculate_style(&self, progress: f32) -> Style {
        match self.config.style {
            PulseStyle::Fade => self.fade_style(progress),
            PulseStyle::Glow => self.glow_style(progress),
            PulseStyle::Scale => self.scale_style(progress),
            PulseStyle::Border => self.border_style(progress),
            PulseStyle::Breathe => self.breathe_style(progress),
            PulseStyle::Heartbeat => self.heartbeat_style(progress),
            PulseStyle::Rainbow => self.rainbow_style(progress),
            PulseStyle::Flash => self.flash_style(progress),
        }
    }

    /// Fade style: interpolate opacity (simulated with color intensity)
    fn fade_style(&self, progress: f32) -> Style {
        let intensity = 0.3 + 0.7 * progress * self.config.intensity;
        let color = self.config.base_color.interpolate(&self.config.pulse_color, intensity);
        Style::default().fg(color.to_color())
    }

    /// Glow style: color brightness variation
    fn glow_style(&self, progress: f32) -> Style {
        let intensity = progress * self.config.intensity;
        let color = self.config.base_color.interpolate(&self.config.pulse_color, intensity);
        Style::default().fg(color.to_color())
    }

    /// Scale style: simulate size changes with modifiers
    fn scale_style(&self, progress: f32) -> Style {
        let intensity = progress * self.config.intensity;
        let color = self.config.base_color.interpolate(&self.config.pulse_color, intensity);
        
        let mut style = Style::default().fg(color.to_color());
        
        if intensity > 0.7 {
            style = style.add_modifier(Modifier::BOLD);
        }
        if intensity > 0.9 {
            style = style.add_modifier(Modifier::UNDERLINED);
        }
        
        style
    }

    /// Border style: simulate border highlighting
    fn border_style(&self, progress: f32) -> Style {
        let intensity = progress * self.config.intensity;
        let color = self.config.base_color.interpolate(&self.config.pulse_color, intensity);
        
        let mut style = Style::default().fg(color.to_color());
        
        if intensity > 0.5 {
            style = style.add_modifier(Modifier::UNDERLINED);
        }
        
        style
    }

    /// Breathe style: slow, gentle fade
    fn breathe_style(&self, progress: f32) -> Style {
        // Use sine wave for smooth breathing effect
        let breathing_progress = (progress * 2.0 * std::f32::consts::PI).sin().abs();
        let intensity = 0.4 + 0.6 * breathing_progress * self.config.intensity;
        let color = self.config.base_color.interpolate(&self.config.pulse_color, intensity);
        Style::default().fg(color.to_color())
    }

    /// Heartbeat style: double pulse pattern
    fn heartbeat_style(&self, progress: f32) -> Style {
        let intensity = if progress < 0.15 {
            // First beat
            (progress / 0.15) * self.config.intensity
        } else if progress < 0.25 {
            // Rest between beats
            ((0.25 - progress) / 0.1) * self.config.intensity * 0.3
        } else if progress < 0.4 {
            // Second beat
            ((progress - 0.25) / 0.15) * self.config.intensity
        } else {
            // Long rest
            ((1.0 - progress) / 0.6) * self.config.intensity * 0.1
        };

        let color = self.config.base_color.interpolate(&self.config.pulse_color, intensity);
        Style::default().fg(color.to_color())
    }

    /// Rainbow style: cycle through colors
    fn rainbow_style(&self, progress: f32) -> Style {
        let hue = progress * 360.0;
        let color = hsl_to_rgb(hue, 0.8, 0.6);
        Style::default().fg(color.to_color())
    }

    /// Flash style: rapid on/off
    fn flash_style(&self, progress: f32) -> Style {
        let intensity = if progress < 0.5 {
            self.config.intensity
        } else {
            0.0
        };

        let color = self.config.base_color.interpolate(&self.config.pulse_color, intensity);
        Style::default().fg(color.to_color())
    }

    /// Check if animation is running
    pub fn is_running(&self) -> bool {
        self.is_active && self.animation.is_running()
    }

    /// Check if animation is completed
    pub fn is_completed(&self) -> bool {
        self.animation.is_completed()
    }

    /// Get current animation progress
    pub fn progress(&self) -> f32 {
        self.animation.progress()
    }
}

impl Animation for PulseAnimation {
    fn start(&mut self) -> Result<()> {
        self.animation.start();
        self.is_active = true;
        self.anim_state = AnimationState::Running {
            start_time: Instant::now(),
            current_frame: 0,
        };
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        self.animation.stop();
        self.is_active = false;
        self.anim_state = AnimationState::Complete;
        Ok(())
    }

    fn update(&mut self) -> Result<()> {
        if self.is_active {
            self.animation.should_update();
            if self.is_completed() {
                self.anim_state = AnimationState::Complete;
            }
        }
        Ok(())
    }

    fn is_complete(&self) -> bool {
        self.is_completed()
    }

    fn state(&self) -> &AnimationState {
        &self.anim_state
    }

    fn render(&self, _area: Rect, _theme: &Theme) -> Vec<Line> {
        vec![PulseAnimation::render(self)]
    }
}

/// Multi-element pulse coordinator for synchronized effects
#[derive(Debug)]
pub struct PulseCoordinator {
    pulses: std::collections::HashMap<String, PulseAnimation>,
    global_sync: bool,
    sync_offset: Duration,
}

impl PulseCoordinator {
    pub fn new() -> Self {
        Self {
            pulses: std::collections::HashMap::new(),
            global_sync: false,
            sync_offset: Duration::from_millis(0),
        }
    }

    /// Enable global synchronization
    pub fn enable_sync(mut self, offset: Duration) -> Self {
        self.global_sync = true;
        self.sync_offset = offset;
        self
    }

    /// Add a pulse animation
    pub fn add_pulse(&mut self, id: String, mut pulse: PulseAnimation) {
        if self.global_sync {
            // Note: synchronization offset is tracked but pulse is started immediately.
            // For true delayed start, the caller should handle timing externally.
            pulse.start();
        }
        self.pulses.insert(id, pulse);
    }

    /// Remove a pulse animation
    pub fn remove_pulse(&mut self, id: &str) {
        self.pulses.remove(id);
    }

    /// Start all pulse animations
    pub fn start_all(&mut self) {
        for pulse in self.pulses.values_mut() {
            pulse.start();
        }
    }

    /// Stop all pulse animations
    pub fn stop_all(&mut self) {
        for pulse in self.pulses.values_mut() {
            pulse.stop();
        }
    }

    /// Update all pulse animations
    pub fn update_all(&mut self) -> Result<()> {
        for pulse in self.pulses.values_mut() {
            pulse.update()?;
        }
        Ok(())
    }

    /// Get a specific pulse animation
    pub fn get_pulse(&self, id: &str) -> Option<&PulseAnimation> {
        self.pulses.get(id)
    }

    /// Get a mutable reference to a specific pulse animation
    pub fn get_pulse_mut(&mut self, id: &str) -> Option<&mut PulseAnimation> {
        self.pulses.get_mut(id)
    }

    /// Check if any animations are running
    pub fn has_active_pulses(&self) -> bool {
        self.pulses.values().any(|p| p.is_running())
    }
}

impl Default for PulseCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Collection of pulse presets for common UI scenarios
pub struct PulsePresets;

impl PulsePresets {
    /// Notification pulse
    pub fn notification(content: String) -> PulseAnimation {
        PulseAnimation::new(PulseConfig::notification(), content)
    }

    /// Error flash
    pub fn error(content: String) -> PulseAnimation {
        PulseAnimation::new(PulseConfig::error_flash(), content)
    }

    /// Success glow
    pub fn success(content: String) -> PulseAnimation {
        PulseAnimation::new(PulseConfig::success_glow(), content)
    }

    /// Focus highlight
    pub fn focus(content: String) -> PulseAnimation {
        PulseAnimation::new(PulseConfig::focus_highlight(), content)
    }

    /// Breathing effect
    pub fn breathing(content: String) -> PulseAnimation {
        PulseAnimation::new(PulseConfig::breathing(), content)
    }

    /// Rainbow cycle
    pub fn rainbow(content: String) -> PulseAnimation {
        PulseAnimation::new(PulseConfig::rainbow_cycle(), content)
    }
}

/// Helper function to convert HSL to RGB
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> RgbColor {
    let h = h % 360.0;
    let s = s.clamp(0.0, 1.0);
    let l = l.clamp(0.0, 1.0);

    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r_prime, g_prime, b_prime) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    RgbColor::new(
        ((r_prime + m) * 255.0) as u8,
        ((g_prime + m) * 255.0) as u8,
        ((b_prime + m) * 255.0) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pulse_config_creation() {
        let config = PulseConfig::new(PulseStyle::Glow)
            .with_duration(Duration::from_millis(500))
            .with_intensity(0.8)
            .with_loop_count(3);
        
        assert_eq!(config.style, PulseStyle::Glow);
        assert_eq!(config.duration, Duration::from_millis(500));
        assert_eq!(config.intensity, 0.8);
        assert_eq!(config.loop_count, Some(3));
    }

    #[test]
    fn test_pulse_animation_lifecycle() {
        let config = PulseConfig::default();
        let mut pulse = PulseAnimation::new(config, "Test".to_string());
        
        assert!(!pulse.is_running());
        
        pulse.start();
        assert!(pulse.is_running());
        
        pulse.stop();
        assert!(!pulse.is_running());
    }

    #[test]
    fn test_pulse_coordinator() {
        let mut coordinator = PulseCoordinator::new();
        
        let pulse1 = PulseAnimation::new(PulseConfig::default(), "Pulse 1".to_string());
        let pulse2 = PulseAnimation::new(PulseConfig::default(), "Pulse 2".to_string());
        
        coordinator.add_pulse("pulse1".to_string(), pulse1);
        coordinator.add_pulse("pulse2".to_string(), pulse2);
        
        assert!(!coordinator.has_active_pulses());
        
        coordinator.start_all();
        assert!(coordinator.has_active_pulses());
    }

    #[test]
    fn test_hsl_to_rgb_conversion() {
        let red = hsl_to_rgb(0.0, 1.0, 0.5);
        assert_eq!(red.r, 255);
        assert_eq!(red.g, 0);
        assert_eq!(red.b, 0);
        
        let green = hsl_to_rgb(120.0, 1.0, 0.5);
        assert_eq!(green.r, 0);
        assert_eq!(green.g, 255);
        assert_eq!(green.b, 0);
    }

    #[test]
    fn test_pulse_presets() {
        let notification = PulsePresets::notification("Alert".to_string());
        let error = PulsePresets::error("Error".to_string());
        let success = PulsePresets::success("Success".to_string());
        
        // Just verify they can be created without panicking
        assert!(!notification.is_running());
        assert!(!error.is_running());
        assert!(!success.is_running());
    }
}