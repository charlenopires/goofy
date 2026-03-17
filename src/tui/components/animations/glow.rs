//! Glow effects for highlighting and visual emphasis.
//!
//! This module provides various glow animation styles for creating
//! luminous effects, highlighting important elements, and adding
//! atmospheric lighting to the terminal UI.

use super::{Animation, AnimationConfig, AnimationState, EasingType};
use super::interpolation::{RgbColor, Interpolatable};
use crate::tui::themes::Theme;
use anyhow::Result;
use ratatui::{
    layout::Rect,
    style::{Color, Style, Modifier},
    text::{Line, Span},
};
use std::time::{Duration, Instant};

/// Glow animation styles
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GlowStyle {
    /// Simple outer glow effect
    Outer,
    /// Inner glow effect
    Inner,
    /// Pulsing glow
    Pulse,
    /// Breathing glow (slow pulse)
    Breathe,
    /// Halo effect around text
    Halo,
    /// Neon-style glow
    Neon,
    /// Soft ambient glow
    Ambient,
    /// Laser-like sharp glow
    Laser,
}

/// Glow configuration
#[derive(Debug, Clone)]
pub struct GlowConfig {
    pub style: GlowStyle,
    pub duration: Duration,
    pub intensity: f32, // 0.0 to 2.0
    pub radius: f32,    // Glow radius in character units
    pub base_color: RgbColor,
    pub glow_color: RgbColor,
    pub spread: f32,    // How far the glow spreads (0.0 to 1.0)
    pub softness: f32,  // Edge softness (0.0 to 1.0)
    pub flicker: bool,  // Add random flicker effect
    pub reverse: bool,  // Reverse animation direction
    pub loop_count: Option<u32>, // None = infinite
}

impl Default for GlowConfig {
    fn default() -> Self {
        Self {
            style: GlowStyle::Outer,
            duration: Duration::from_millis(1000),
            intensity: 1.0,
            radius: 2.0,
            base_color: RgbColor::new(200, 200, 200),
            glow_color: RgbColor::new(100, 150, 255),
            spread: 0.8,
            softness: 0.6,
            flicker: false,
            reverse: false,
            loop_count: None,
        }
    }
}

impl GlowConfig {
    pub fn new(style: GlowStyle) -> Self {
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
        self.intensity = intensity.clamp(0.0, 2.0);
        self
    }

    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius.max(0.0);
        self
    }

    pub fn with_colors(mut self, base: RgbColor, glow: RgbColor) -> Self {
        self.base_color = base;
        self.glow_color = glow;
        self
    }

    pub fn with_spread(mut self, spread: f32) -> Self {
        self.spread = spread.clamp(0.0, 1.0);
        self
    }

    pub fn with_softness(mut self, softness: f32) -> Self {
        self.softness = softness.clamp(0.0, 1.0);
        self
    }

    pub fn with_flicker(mut self, enable: bool) -> Self {
        self.flicker = enable;
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

    /// Quick configurations for common scenarios
    pub fn notification() -> Self {
        Self::new(GlowStyle::Pulse)
            .with_duration(Duration::from_millis(800))
            .with_colors(
                RgbColor::new(200, 200, 200),
                RgbColor::new(255, 200, 100),
            )
            .with_intensity(1.2)
            .with_radius(1.5)
    }

    pub fn error_glow() -> Self {
        Self::new(GlowStyle::Outer)
            .with_duration(Duration::from_millis(300))
            .with_colors(
                RgbColor::new(200, 200, 200),
                RgbColor::new(255, 100, 100),
            )
            .with_intensity(1.5)
            .with_radius(2.0)
            .with_flicker(true)
    }

    pub fn success_glow() -> Self {
        Self::new(GlowStyle::Halo)
            .with_duration(Duration::from_millis(600))
            .with_colors(
                RgbColor::new(200, 200, 200),
                RgbColor::new(100, 255, 100),
            )
            .with_intensity(1.0)
            .with_radius(1.0)
    }

    pub fn focus_highlight() -> Self {
        Self::new(GlowStyle::Neon)
            .with_duration(Duration::from_millis(1200))
            .with_colors(
                RgbColor::new(150, 150, 150),
                RgbColor::new(100, 150, 255),
            )
            .with_intensity(0.8)
            .infinite()
    }

    pub fn ambient_lighting() -> Self {
        Self::new(GlowStyle::Ambient)
            .with_duration(Duration::from_millis(3000))
            .with_colors(
                RgbColor::new(150, 150, 150),
                RgbColor::new(120, 120, 180),
            )
            .with_intensity(0.4)
            .with_radius(3.0)
            .infinite()
    }

    pub fn laser_focus() -> Self {
        Self::new(GlowStyle::Laser)
            .with_duration(Duration::from_millis(400))
            .with_colors(
                RgbColor::new(255, 255, 255),
                RgbColor::new(255, 100, 100),
            )
            .with_intensity(1.8)
            .with_radius(0.5)
    }

    pub fn breathing() -> Self {
        Self::new(GlowStyle::Breathe)
            .with_duration(Duration::from_millis(2000))
            .with_colors(
                RgbColor::new(180, 180, 180),
                RgbColor::new(200, 200, 255),
            )
            .with_intensity(0.6)
            .infinite()
    }
}

/// Glow animation component
#[derive(Debug)]
pub struct GlowAnimation {
    config: GlowConfig,
    state: AnimationState,
    start_time: Option<Instant>,
    content: Vec<Line<'static>>,
    current_intensity: f32,
    flicker_offset: f32,
}

impl GlowAnimation {
    pub fn new(config: GlowConfig) -> Self {
        Self {
            config,
            state: AnimationState::Idle,
            start_time: None,
            content: Vec::new(),
            current_intensity: 0.0,
            flicker_offset: 0.0,
        }
    }

    /// Set the content to be rendered with glow effect
    pub fn set_content(&mut self, content: Vec<Line<'static>>) {
        self.content = content;
    }

    /// Set content from a single string
    pub fn set_text(&mut self, text: String) {
        self.content = vec![Line::from(text)];
    }

    /// Calculate the current glow intensity based on style and progress
    fn calculate_glow_intensity(&self, progress: f32) -> f32 {
        let base_intensity = match self.config.style {
            GlowStyle::Outer | GlowStyle::Inner => {
                // Simple fade in/out
                if progress <= 0.5 {
                    progress * 2.0
                } else {
                    2.0 - progress * 2.0
                }
            }
            GlowStyle::Pulse => {
                // Pulsing effect
                (progress * 2.0 * std::f32::consts::PI).sin().abs()
            }
            GlowStyle::Breathe => {
                // Slow breathing effect
                ((progress * 2.0 * std::f32::consts::PI).sin() + 1.0) / 2.0
            }
            GlowStyle::Halo => {
                // Constant glow with slight variation
                0.8 + 0.2 * (progress * 4.0 * std::f32::consts::PI).sin()
            }
            GlowStyle::Neon => {
                // Flickering neon effect
                let base = (progress * 2.0 * std::f32::consts::PI).sin().abs();
                if self.config.flicker {
                    base * (0.9 + 0.1 * self.flicker_offset)
                } else {
                    base
                }
            }
            GlowStyle::Ambient => {
                // Gentle ambient glow
                0.6 + 0.4 * ((progress * std::f32::consts::PI).sin().abs())
            }
            GlowStyle::Laser => {
                // Sharp, intense glow
                if progress < 0.1 {
                    progress * 10.0
                } else if progress > 0.9 {
                    (1.0 - progress) * 10.0
                } else {
                    1.0
                }
            }
        };

        let intensity = base_intensity * self.config.intensity;
        
        // Apply flicker if enabled
        if self.config.flicker {
            intensity * (0.9 + 0.1 * self.flicker_offset)
        } else {
            intensity
        }
    }

    /// Calculate glow color at a given distance from the center
    fn calculate_glow_color(&self, distance: f32, intensity: f32) -> RgbColor {
        let normalized_distance = (distance / self.config.radius).clamp(0.0, 1.0);
        let distance_falloff = 1.0 - normalized_distance.powf(1.0 + self.config.softness);
        
        let effective_intensity = intensity * distance_falloff * self.config.spread;
        
        match self.config.style {
            GlowStyle::Inner => {
                // Inner glow gets stronger towards center
                let inner_intensity = effective_intensity * (1.0 - normalized_distance);
                self.config.base_color.interpolate(&self.config.glow_color, inner_intensity)
            }
            GlowStyle::Laser => {
                // Sharp laser glow
                if normalized_distance < 0.3 {
                    self.config.glow_color
                } else {
                    self.config.base_color.interpolate(&self.config.glow_color, effective_intensity * 0.5)
                }
            }
            _ => {
                // Outer glow and other styles
                self.config.base_color.interpolate(&self.config.glow_color, effective_intensity)
            }
        }
    }

    /// Apply glow effect to content
    fn apply_glow_effect(&self) -> Vec<Line> {
        if self.content.is_empty() {
            return Vec::new();
        }

        let intensity = self.current_intensity;
        
        self.content
            .iter()
            .map(|line| {
                let spans: Vec<Span> = line
                    .spans
                    .iter()
                    .map(|span| {
                        // Calculate glow color for this span
                        let glow_color = self.calculate_glow_color(0.0, intensity);
                        let mut style = span.style;
                        
                        // Apply glow color
                        style.fg = Some(glow_color.to_color());
                        
                        // Add visual effects based on style
                        match self.config.style {
                            GlowStyle::Neon => {
                                if intensity > 0.7 {
                                    style = style.add_modifier(Modifier::BOLD);
                                }
                                if intensity > 0.9 {
                                    style = style.add_modifier(Modifier::UNDERLINED);
                                }
                            }
                            GlowStyle::Laser => {
                                if intensity > 0.8 {
                                    style = style.add_modifier(Modifier::BOLD);
                                    style = style.add_modifier(Modifier::UNDERLINED);
                                }
                            }
                            GlowStyle::Halo => {
                                if intensity > 0.5 {
                                    style = style.add_modifier(Modifier::BOLD);
                                }
                            }
                            _ => {}
                        }
                        
                        Span::styled(span.content.clone(), style)
                    })
                    .collect();
                Line::from(spans)
            })
            .collect()
    }

    /// Generate random flicker offset
    fn update_flicker(&mut self) {
        if self.config.flicker {
            // Simple pseudo-random flicker based on time
            let time_factor = self.start_time
                .map(|t| t.elapsed().as_millis() as f32 / 100.0)
                .unwrap_or(0.0);
            self.flicker_offset = (time_factor * 13.7).sin() * 0.5 + 0.5;
        }
    }
}

impl Animation for GlowAnimation {
    fn start(&mut self) -> Result<()> {
        self.state = AnimationState::Running {
            start_time: Instant::now(),
            current_frame: 0,
        };
        self.start_time = Some(Instant::now());
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        self.state = AnimationState::Complete;
        self.current_intensity = 0.0;
        Ok(())
    }

    fn update(&mut self) -> Result<()> {
        let (elapsed, st) = match &self.state {
            AnimationState::Running { start_time, .. } => {
                (start_time.elapsed(), *start_time)
            }
            _ => return Ok(()),
        };

        if elapsed >= self.config.duration {
            if self.config.loop_count.is_none() {
                // Infinite loop - restart
                self.state = AnimationState::Running {
                    start_time: Instant::now(),
                    current_frame: 0,
                };
                self.start_time = Some(Instant::now());
            } else {
                // Finite loop - complete
                self.state = AnimationState::Complete;
                self.current_intensity = 0.0;
            }
        } else {
            // Calculate progress and intensity
            let progress = elapsed.as_secs_f32() / self.config.duration.as_secs_f32();
            let adjusted_progress = if self.config.reverse {
                1.0 - progress
            } else {
                progress
            };

            self.current_intensity = self.calculate_glow_intensity(adjusted_progress);
            self.update_flicker();

            // Update frame count
            let frame_count = (elapsed.as_millis() / 16) as u32; // ~60 FPS
            self.state = AnimationState::Running {
                start_time: st,
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

    fn render(&self, _area: Rect, _theme: &Theme) -> Vec<Line> {
        self.apply_glow_effect()
    }
}

/// Multi-layer glow effect for complex lighting
#[derive(Debug)]
pub struct LayeredGlow {
    layers: Vec<GlowAnimation>,
    blend_mode: BlendMode,
}

/// Blending modes for layered glow effects
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlendMode {
    /// Add colors together
    Additive,
    /// Multiply colors
    Multiply,
    /// Screen blend
    Screen,
    /// Overlay effect
    Overlay,
}

impl LayeredGlow {
    pub fn new(blend_mode: BlendMode) -> Self {
        Self {
            layers: Vec::new(),
            blend_mode,
        }
    }

    /// Add a glow layer
    pub fn add_layer(&mut self, glow: GlowAnimation) {
        self.layers.push(glow);
    }

    /// Set content for all layers
    pub fn set_content(&mut self, content: Vec<Line<'static>>) {
        for layer in &mut self.layers {
            layer.set_content(content.clone());
        }
    }

    /// Start all glow layers
    pub fn start(&mut self) -> Result<()> {
        for layer in &mut self.layers {
            layer.start()?;
        }
        Ok(())
    }

    /// Stop all glow layers
    pub fn stop(&mut self) -> Result<()> {
        for layer in &mut self.layers {
            layer.stop()?;
        }
        Ok(())
    }

    /// Update all glow layers
    pub fn update(&mut self) -> Result<()> {
        for layer in &mut self.layers {
            layer.update()?;
        }
        Ok(())
    }

    /// Render layered glow effect
    pub fn render(&self, area: Rect, theme: &Theme) -> Vec<Line> {
        if self.layers.is_empty() {
            return Vec::new();
        }

        // Start with the first layer
        let mut result = self.layers[0].render(area, theme);

        // Blend additional layers
        for layer in &self.layers[1..] {
            let layer_lines = layer.render(area, theme);
            result = self.blend_lines(result, layer_lines);
        }

        result
    }

    /// Blend two sets of lines based on blend mode
    fn blend_lines<'a>(&self, base: Vec<Line<'a>>, overlay: Vec<Line<'a>>) -> Vec<Line<'a>> {
        base.into_iter()
            .zip(overlay.into_iter())
            .map(|(base_line, overlay_line)| {
                let spans: Vec<Span> = base_line
                    .spans
                    .into_iter()
                    .zip(overlay_line.spans.into_iter())
                    .map(|(base_span, overlay_span)| {
                        let blended_color = self.blend_colors(
                            base_span.style.fg.unwrap_or(Color::White),
                            overlay_span.style.fg.unwrap_or(Color::White),
                        );

                        Span::styled(
                            base_span.content,
                            base_span.style.fg(blended_color),
                        )
                    })
                    .collect();
                Line::from(spans)
            })
            .collect()
    }

    /// Blend two colors based on blend mode
    fn blend_colors(&self, base: Color, overlay: Color) -> Color {
        match (base, overlay) {
            (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
                match self.blend_mode {
                    BlendMode::Additive => {
                        Color::Rgb(
                            (r1 as u16 + r2 as u16).min(255) as u8,
                            (g1 as u16 + g2 as u16).min(255) as u8,
                            (b1 as u16 + b2 as u16).min(255) as u8,
                        )
                    }
                    BlendMode::Multiply => {
                        Color::Rgb(
                            (r1 as u16 * r2 as u16 / 255) as u8,
                            (g1 as u16 * g2 as u16 / 255) as u8,
                            (b1 as u16 * b2 as u16 / 255) as u8,
                        )
                    }
                    BlendMode::Screen => {
                        Color::Rgb(
                            (255 - (255 - r1 as u16) * (255 - r2 as u16) / 255) as u8,
                            (255 - (255 - g1 as u16) * (255 - g2 as u16) / 255) as u8,
                            (255 - (255 - b1 as u16) * (255 - b2 as u16) / 255) as u8,
                        )
                    }
                    BlendMode::Overlay => {
                        // Simplified overlay blend
                        if r1 < 128 {
                            Color::Rgb(
                                (2 * r1 as u16 * r2 as u16 / 255) as u8,
                                (2 * g1 as u16 * g2 as u16 / 255) as u8,
                                (2 * b1 as u16 * b2 as u16 / 255) as u8,
                            )
                        } else {
                            Color::Rgb(
                                (255 - 2 * (255 - r1 as u16) * (255 - r2 as u16) / 255) as u8,
                                (255 - 2 * (255 - g1 as u16) * (255 - g2 as u16) / 255) as u8,
                                (255 - 2 * (255 - b1 as u16) * (255 - b2 as u16) / 255) as u8,
                            )
                        }
                    }
                }
            }
            _ => overlay, // Fallback to overlay color
        }
    }

    /// Check if any layer is running
    pub fn is_running(&self) -> bool {
        self.layers.iter().any(|layer| !layer.is_complete())
    }
}

/// Collection of glow presets for common UI scenarios
pub struct GlowPresets;

impl GlowPresets {
    /// Notification glow
    pub fn notification(content: String) -> GlowAnimation {
        let mut glow = GlowAnimation::new(GlowConfig::notification());
        glow.set_text(content);
        glow
    }

    /// Error glow
    pub fn error(content: String) -> GlowAnimation {
        let mut glow = GlowAnimation::new(GlowConfig::error_glow());
        glow.set_text(content);
        glow
    }

    /// Success glow
    pub fn success(content: String) -> GlowAnimation {
        let mut glow = GlowAnimation::new(GlowConfig::success_glow());
        glow.set_text(content);
        glow
    }

    /// Focus highlight
    pub fn focus(content: String) -> GlowAnimation {
        let mut glow = GlowAnimation::new(GlowConfig::focus_highlight());
        glow.set_text(content);
        glow
    }

    /// Ambient lighting
    pub fn ambient(content: String) -> GlowAnimation {
        let mut glow = GlowAnimation::new(GlowConfig::ambient_lighting());
        glow.set_text(content);
        glow
    }

    /// Laser focus
    pub fn laser(content: String) -> GlowAnimation {
        let mut glow = GlowAnimation::new(GlowConfig::laser_focus());
        glow.set_text(content);
        glow
    }

    /// Breathing glow
    pub fn breathing(content: String) -> GlowAnimation {
        let mut glow = GlowAnimation::new(GlowConfig::breathing());
        glow.set_text(content);
        glow
    }

    /// Multi-layer atmospheric glow
    pub fn atmospheric(content: String) -> LayeredGlow {
        let mut layered = LayeredGlow::new(BlendMode::Additive);
        
        // Base ambient layer
        let mut ambient = GlowAnimation::new(
            GlowConfig::ambient_lighting()
                .with_intensity(0.3)
                .with_radius(4.0)
        );
        ambient.set_text(content.clone());
        
        // Pulse layer
        let mut pulse = GlowAnimation::new(
            GlowConfig::new(GlowStyle::Pulse)
                .with_duration(Duration::from_millis(2000))
                .with_intensity(0.5)
                .with_colors(
                    RgbColor::new(150, 150, 150),
                    RgbColor::new(200, 150, 255),
                )
        );
        pulse.set_text(content);
        
        layered.add_layer(ambient);
        layered.add_layer(pulse);
        layered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glow_config_creation() {
        let config = GlowConfig::new(GlowStyle::Pulse)
            .with_duration(Duration::from_millis(500))
            .with_intensity(1.5)
            .with_radius(2.0)
            .with_flicker(true);
        
        assert_eq!(config.style, GlowStyle::Pulse);
        assert_eq!(config.duration, Duration::from_millis(500));
        assert_eq!(config.intensity, 1.5);
        assert_eq!(config.radius, 2.0);
        assert!(config.flicker);
    }

    #[test]
    fn test_glow_animation_lifecycle() {
        let config = GlowConfig::notification();
        let mut glow = GlowAnimation::new(config);
        
        assert!(glow.is_complete());
        
        glow.start().unwrap();
        assert!(!glow.is_complete());
        
        glow.stop().unwrap();
        assert!(glow.is_complete());
        assert_eq!(glow.current_intensity, 0.0);
    }

    #[test]
    fn test_layered_glow() {
        let mut layered = LayeredGlow::new(BlendMode::Additive);
        
        let glow1 = GlowAnimation::new(GlowConfig::default());
        let glow2 = GlowAnimation::new(GlowConfig::notification());
        
        layered.add_layer(glow1);
        layered.add_layer(glow2);
        
        assert!(!layered.is_running()); // Not started yet
        
        layered.start().unwrap();
        // Would need more complex testing for running state
    }

    #[test]
    fn test_glow_presets() {
        let notification = GlowPresets::notification("Alert".to_string());
        let error = GlowPresets::error("Error".to_string());
        let success = GlowPresets::success("Success".to_string());
        let focus = GlowPresets::focus("Focus".to_string());
        
        // Just verify they can be created without panicking
        assert!(notification.is_complete());
        assert!(error.is_complete());
        assert!(success.is_complete());
        assert!(focus.is_complete());
    }

    #[test]
    fn test_color_blending() {
        let layered = LayeredGlow::new(BlendMode::Additive);
        
        let base = Color::Rgb(100, 100, 100);
        let overlay = Color::Rgb(50, 50, 50);
        let result = layered.blend_colors(base, overlay);
        
        if let Color::Rgb(r, g, b) = result {
            assert_eq!(r, 150);
            assert_eq!(g, 150);
            assert_eq!(b, 150);
        }
    }
}