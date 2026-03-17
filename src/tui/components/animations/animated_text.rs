//! Animated text components with various text effects.
//!
//! This module provides text components that can be animated with
//! typewriter effects, fading, morphing, and other text-specific animations.

use super::{Animation, AnimationConfig, AnimationState, EasingType};
use super::fade::FadeAnimation;
use super::pulse::PulseAnimation;
use super::interpolation::{RgbColor, Interpolatable};
use crate::tui::themes::Theme;
use anyhow::Result;
use ratatui::{
    layout::Rect,
    style::{Color, Style, Modifier},
    text::{Line, Span},
};
use std::time::{Duration, Instant};

/// Text animation styles
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextAnimationStyle {
    /// Typewriter effect - characters appear one by one
    Typewriter,
    /// Fade in effect for the entire text
    FadeIn,
    /// Fade out effect for the entire text
    FadeOut,
    /// Characters fade in individually
    CharacterFadeIn,
    /// Wave effect - characters animate in a wave pattern
    Wave,
    /// Glow effect with pulsing
    Glow,
    /// Shake effect for emphasis
    Shake,
    /// Matrix-style digital rain effect
    Matrix,
    /// Morphing between two different texts
    Morph,
    /// Flickering text like old CRT monitors
    Flicker,
}

/// Text animation configuration
#[derive(Debug, Clone)]
pub struct TextAnimationConfig {
    pub style: TextAnimationStyle,
    pub duration: Duration,
    pub delay_between_chars: Duration,
    pub color: Option<RgbColor>,
    pub highlight_color: Option<RgbColor>,
    pub loop_animation: bool,
    pub reverse_on_complete: bool,
    pub easing: EasingType,
}

impl Default for TextAnimationConfig {
    fn default() -> Self {
        Self {
            style: TextAnimationStyle::Typewriter,
            duration: Duration::from_millis(1000),
            delay_between_chars: Duration::from_millis(50),
            color: None,
            highlight_color: None,
            loop_animation: false,
            reverse_on_complete: false,
            easing: EasingType::EaseOut,
        }
    }
}

impl TextAnimationConfig {
    pub fn new(style: TextAnimationStyle) -> Self {
        Self {
            style,
            ..Default::default()
        }
    }

    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    pub fn with_char_delay(mut self, delay: Duration) -> Self {
        self.delay_between_chars = delay;
        self
    }

    pub fn with_color(mut self, color: RgbColor) -> Self {
        self.color = Some(color);
        self
    }

    pub fn with_highlight_color(mut self, color: RgbColor) -> Self {
        self.highlight_color = Some(color);
        self
    }

    pub fn looping(mut self) -> Self {
        self.loop_animation = true;
        self
    }

    pub fn with_reverse(mut self) -> Self {
        self.reverse_on_complete = true;
        self
    }

    pub fn with_easing(mut self, easing: EasingType) -> Self {
        self.easing = easing;
        self
    }

    /// Quick presets for common text animations
    pub fn typewriter_fast() -> Self {
        Self::new(TextAnimationStyle::Typewriter)
            .with_char_delay(Duration::from_millis(30))
            .with_duration(Duration::from_millis(800))
    }

    pub fn typewriter_slow() -> Self {
        Self::new(TextAnimationStyle::Typewriter)
            .with_char_delay(Duration::from_millis(100))
            .with_duration(Duration::from_millis(2000))
    }

    pub fn fade_in_smooth() -> Self {
        Self::new(TextAnimationStyle::FadeIn)
            .with_duration(Duration::from_millis(800))
            .with_easing(EasingType::EaseInOut)
    }

    pub fn character_wave() -> Self {
        Self::new(TextAnimationStyle::Wave)
            .with_duration(Duration::from_millis(1200))
            .with_char_delay(Duration::from_millis(80))
    }

    pub fn glow_pulse() -> Self {
        Self::new(TextAnimationStyle::Glow)
            .with_duration(Duration::from_millis(1500))
            .looping()
    }

    pub fn matrix_style() -> Self {
        Self::new(TextAnimationStyle::Matrix)
            .with_duration(Duration::from_millis(2000))
            .with_char_delay(Duration::from_millis(20))
            .with_color(RgbColor::new(0, 255, 0))
    }

    pub fn error_shake() -> Self {
        Self::new(TextAnimationStyle::Shake)
            .with_duration(Duration::from_millis(400))
            .with_color(RgbColor::new(255, 100, 100))
    }

    pub fn crt_flicker() -> Self {
        Self::new(TextAnimationStyle::Flicker)
            .with_duration(Duration::from_millis(3000))
            .looping()
    }
}

/// Animated text component
#[derive(Debug)]
pub struct AnimatedText {
    config: TextAnimationConfig,
    state: AnimationState,
    text: String,
    target_text: Option<String>, // For morphing animations
    visible_chars: usize,
    start_time: Option<Instant>,
    character_timings: Vec<Duration>,
    current_style_modifiers: Vec<Style>,
}

impl AnimatedText {
    pub fn new(config: TextAnimationConfig, text: String) -> Self {
        let char_count = text.chars().count();
        let character_timings = (0..char_count)
            .map(|i| config.delay_between_chars * i as u32)
            .collect();

        Self {
            config,
            state: AnimationState::Idle,
            text,
            target_text: None,
            visible_chars: 0,
            start_time: None,
            character_timings,
            current_style_modifiers: vec![Style::default(); char_count],
        }
    }

    /// Set the text to be animated
    pub fn set_text(&mut self, text: String) {
        self.text = text;
        let char_count = self.text.chars().count();
        self.character_timings = (0..char_count)
            .map(|i| self.config.delay_between_chars * i as u32)
            .collect();
        self.current_style_modifiers = vec![Style::default(); char_count];
        self.visible_chars = 0;
    }

    /// Set target text for morphing animations
    pub fn set_target_text(&mut self, target: String) {
        self.target_text = Some(target);
    }

    /// Calculate animation progress for a specific character
    fn char_progress(&self, char_index: usize, elapsed: Duration) -> f32 {
        if char_index >= self.character_timings.len() {
            return 1.0;
        }

        let char_start_time = self.character_timings[char_index];
        if elapsed < char_start_time {
            return 0.0;
        }

        let char_elapsed = elapsed - char_start_time;
        let char_duration = self.config.duration.saturating_sub(char_start_time);
        
        if char_duration.is_zero() {
            return 1.0;
        }

        (char_elapsed.as_secs_f32() / char_duration.as_secs_f32()).min(1.0)
    }

    /// Apply specific animation style to a character
    fn apply_char_animation(&self, char_index: usize, progress: f32, char: char) -> (char, Style) {
        let mut style = Style::default();
        let mut display_char = char;

        // Apply base color if specified
        if let Some(color) = &self.config.color {
            style = style.fg(color.to_color());
        }

        match self.config.style {
            TextAnimationStyle::Typewriter => {
                // Simple typewriter - character is visible when progress > 0
                if progress <= 0.0 {
                    display_char = ' ';
                }
            }
            TextAnimationStyle::FadeIn => {
                // Fade in by adjusting color intensity
                let intensity = self.config.easing.apply(progress);
                if let Some(color) = &self.config.color {
                    let faded_color = RgbColor::new(
                        (color.r as f32 * intensity) as u8,
                        (color.g as f32 * intensity) as u8,
                        (color.b as f32 * intensity) as u8,
                    );
                    style = style.fg(faded_color.to_color());
                }
            }
            TextAnimationStyle::CharacterFadeIn => {
                // Each character fades in individually
                let char_progress = self.char_progress(char_index, 
                    self.start_time.map(|t| t.elapsed()).unwrap_or_default());
                let intensity = self.config.easing.apply(char_progress);
                
                if let Some(color) = &self.config.color {
                    let faded_color = RgbColor::new(
                        (color.r as f32 * intensity) as u8,
                        (color.g as f32 * intensity) as u8,
                        (color.b as f32 * intensity) as u8,
                    );
                    style = style.fg(faded_color.to_color());
                }
            }
            TextAnimationStyle::Wave => {
                // Wave effect with vertical movement simulation
                let wave_offset = (char_index as f32 * 0.3 + progress * 6.0).sin();
                if wave_offset > 0.5 {
                    style = style.add_modifier(Modifier::BOLD);
                }
                if wave_offset > 0.8 {
                    style = style.add_modifier(Modifier::UNDERLINED);
                }
            }
            TextAnimationStyle::Glow => {
                // Pulsing glow effect
                let glow_intensity = (progress * 4.0 * std::f32::consts::PI).sin().abs();
                if glow_intensity > 0.6 {
                    style = style.add_modifier(Modifier::BOLD);
                }
                
                if let Some(highlight) = &self.config.highlight_color {
                    let default_color = RgbColor::new(255, 255, 255);
                    let base_color = self.config.color.as_ref().unwrap_or(&default_color);
                    let glowing_color = base_color.interpolate(highlight, glow_intensity);
                    style = style.fg(glowing_color.to_color());
                }
            }
            TextAnimationStyle::Shake => {
                // Shake effect with random character substitution occasionally
                let shake_intensity = (progress * 20.0).sin();
                if shake_intensity.abs() > 0.8 {
                    // Simulate shake by changing style
                    style = style.add_modifier(Modifier::RAPID_BLINK);
                }
            }
            TextAnimationStyle::Matrix => {
                // Matrix digital rain effect
                if progress < 1.0 {
                    // Random characters before final character appears
                    let matrix_chars = ['0', '1', 'ﾊ', 'ﾐ', 'ﾋ', 'ｰ', 'ｳ', 'ｼ', 'ﾅ', 'ﾓ'];
                    let random_index = (char_index + (progress * 10.0) as usize) % matrix_chars.len();
                    display_char = matrix_chars[random_index];
                    style = style.fg(Color::Green);
                } else {
                    style = style.fg(Color::Green).add_modifier(Modifier::BOLD);
                }
            }
            TextAnimationStyle::Morph => {
                // Morphing between characters
                if let Some(target) = &self.target_text {
                    let target_chars: Vec<char> = target.chars().collect();
                    if char_index < target_chars.len() && progress > 0.5 {
                        display_char = target_chars[char_index];
                    }
                }
            }
            TextAnimationStyle::Flicker => {
                // CRT-style flicker
                let flicker_chance = (progress * 50.0 + char_index as f32).sin();
                if flicker_chance > 0.9 {
                    display_char = ' ';
                } else if flicker_chance > 0.7 {
                    style = style.add_modifier(Modifier::DIM);
                }
            }
            TextAnimationStyle::FadeOut => {
                // Fade out effect
                let intensity = 1.0 - self.config.easing.apply(progress);
                if let Some(color) = &self.config.color {
                    let faded_color = RgbColor::new(
                        (color.r as f32 * intensity) as u8,
                        (color.g as f32 * intensity) as u8,
                        (color.b as f32 * intensity) as u8,
                    );
                    style = style.fg(faded_color.to_color());
                }
            }
        }

        (display_char, style)
    }

    /// Update visible character count for typewriter effect
    fn update_visible_chars(&mut self, elapsed: Duration) {
        if matches!(self.config.style, TextAnimationStyle::Typewriter) {
            let chars_to_show = (elapsed.as_millis() / self.config.delay_between_chars.as_millis()) as usize;
            self.visible_chars = chars_to_show.min(self.text.chars().count());
        }
    }
}

impl Animation for AnimatedText {
    fn start(&mut self) -> Result<()> {
        self.state = AnimationState::Running {
            start_time: Instant::now(),
            current_frame: 0,
        };
        self.start_time = Some(Instant::now());
        self.visible_chars = 0;
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        self.state = AnimationState::Complete;
        self.visible_chars = self.text.chars().count();
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
            if self.config.loop_animation {
                // Restart animation
                self.state = AnimationState::Running {
                    start_time: Instant::now(),
                    current_frame: 0,
                };
                self.start_time = Some(Instant::now());
                self.visible_chars = 0;
            } else {
                self.state = AnimationState::Complete;
                self.visible_chars = self.text.chars().count();
            }
        } else {
            self.update_visible_chars(elapsed);

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
        let elapsed = self.start_time
            .map(|t| t.elapsed())
            .unwrap_or_default();
        
        let progress = if self.config.duration.is_zero() {
            1.0
        } else {
            (elapsed.as_secs_f32() / self.config.duration.as_secs_f32()).min(1.0)
        };

        let chars: Vec<char> = self.text.chars().collect();
        let mut spans = Vec::new();

        for (i, &char) in chars.iter().enumerate() {
            // For typewriter effect, don't show characters beyond visible count
            if matches!(self.config.style, TextAnimationStyle::Typewriter) && i >= self.visible_chars {
                break;
            }

            let char_progress = self.char_progress(i, elapsed);
            let (display_char, style) = self.apply_char_animation(i, char_progress, char);
            
            spans.push(Span::styled(display_char.to_string(), style));
        }

        vec![Line::from(spans)]
    }
}

/// Text sequence animator for chaining multiple text animations
#[derive(Debug)]
pub struct TextSequence {
    animations: Vec<AnimatedText>,
    current_index: usize,
    is_active: bool,
    delay_between_animations: Duration,
    last_animation_end: Option<Instant>,
}

impl TextSequence {
    pub fn new() -> Self {
        Self {
            animations: Vec::new(),
            current_index: 0,
            is_active: false,
            delay_between_animations: Duration::from_millis(500),
            last_animation_end: None,
        }
    }

    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay_between_animations = delay;
        self
    }

    pub fn add_text(mut self, animation: AnimatedText) -> Self {
        self.animations.push(animation);
        self
    }

    pub fn start(&mut self) -> Result<()> {
        if !self.animations.is_empty() {
            self.current_index = 0;
            self.animations[0].start()?;
            self.is_active = true;
            self.last_animation_end = None;
        }
        Ok(())
    }

    pub fn update(&mut self) -> Result<()> {
        if !self.is_active || self.current_index >= self.animations.len() {
            return Ok(());
        }

        // Check if we need to wait between animations
        if let Some(end_time) = self.last_animation_end {
            if end_time.elapsed() < self.delay_between_animations {
                return Ok(());
            }
            self.last_animation_end = None;
        }

        // Update current animation
        self.animations[self.current_index].update()?;

        // Check if current animation is complete
        if self.animations[self.current_index].is_complete() {
            self.last_animation_end = Some(Instant::now());
            self.current_index += 1;

            // Start next animation if available
            if self.current_index < self.animations.len() {
                self.animations[self.current_index].start()?;
            } else {
                self.is_active = false;
            }
        }

        Ok(())
    }

    pub fn render(&self, area: Rect, theme: &Theme) -> Vec<Line> {
        let mut all_lines = Vec::new();

        // Render completed animations
        for i in 0..self.current_index {
            all_lines.extend(self.animations[i].render(area, theme));
        }

        // Render current animation if active
        if self.current_index < self.animations.len() && 
           (self.is_active || self.animations[self.current_index].is_complete()) {
            all_lines.extend(self.animations[self.current_index].render(area, theme));
        }

        all_lines
    }

    pub fn is_complete(&self) -> bool {
        !self.is_active
    }
}

impl Default for TextSequence {
    fn default() -> Self {
        Self::new()
    }
}

/// Presets for common animated text scenarios
pub struct AnimatedTextPresets;

impl AnimatedTextPresets {
    /// Welcome message with typewriter effect
    pub fn welcome_message(text: String) -> AnimatedText {
        AnimatedText::new(TextAnimationConfig::typewriter_slow(), text)
    }

    /// Error message with shake effect
    pub fn error_message(text: String) -> AnimatedText {
        AnimatedText::new(TextAnimationConfig::error_shake(), text)
    }

    /// Loading text with matrix effect
    pub fn loading_matrix(text: String) -> AnimatedText {
        AnimatedText::new(TextAnimationConfig::matrix_style(), text)
    }

    /// Notification with glow effect
    pub fn notification_glow(text: String) -> AnimatedText {
        AnimatedText::new(TextAnimationConfig::glow_pulse(), text)
    }

    /// Success message with wave effect
    pub fn success_wave(text: String) -> AnimatedText {
        AnimatedText::new(TextAnimationConfig::character_wave(), text)
    }

    /// Terminal boot sequence
    pub fn boot_sequence(messages: Vec<String>) -> TextSequence {
        let mut sequence = TextSequence::new().with_delay(Duration::from_millis(300));
        
        for message in messages {
            let config = TextAnimationConfig::typewriter_fast()
                .with_color(RgbColor::new(0, 255, 0));
            let animation = AnimatedText::new(config, message);
            sequence = sequence.add_text(animation);
        }
        
        sequence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animated_text_creation() {
        let config = TextAnimationConfig::typewriter_fast();
        let text = AnimatedText::new(config, "Hello World".to_string());
        
        assert_eq!(text.text, "Hello World");
        assert_eq!(text.visible_chars, 0);
        assert!(text.is_complete());
    }

    #[test]
    fn test_text_animation_lifecycle() {
        let config = TextAnimationConfig::default();
        let mut text = AnimatedText::new(config, "Test".to_string());
        
        assert!(text.is_complete());
        
        text.start().unwrap();
        assert!(!text.is_complete());
        
        text.stop().unwrap();
        assert!(text.is_complete());
        assert_eq!(text.visible_chars, 4); // All characters visible when stopped
    }

    #[test]
    fn test_text_sequence() {
        let mut sequence = TextSequence::new();
        
        let text1 = AnimatedText::new(TextAnimationConfig::default(), "First".to_string());
        let text2 = AnimatedText::new(TextAnimationConfig::default(), "Second".to_string());
        
        sequence = sequence.add_text(text1).add_text(text2);
        
        assert!(!sequence.is_active);
        assert!(sequence.is_complete());
        
        sequence.start().unwrap();
        assert!(sequence.is_active);
        assert!(!sequence.is_complete());
    }

    #[test]
    fn test_character_timing_calculation() {
        let config = TextAnimationConfig::default()
            .with_char_delay(Duration::from_millis(100));
        let text = AnimatedText::new(config, "ABC".to_string());
        
        assert_eq!(text.character_timings.len(), 3);
        assert_eq!(text.character_timings[0], Duration::from_millis(0));
        assert_eq!(text.character_timings[1], Duration::from_millis(100));
        assert_eq!(text.character_timings[2], Duration::from_millis(200));
    }

    #[test]
    fn test_animated_text_presets() {
        let welcome = AnimatedTextPresets::welcome_message("Welcome".to_string());
        let error = AnimatedTextPresets::error_message("Error".to_string());
        let loading = AnimatedTextPresets::loading_matrix("Loading".to_string());
        
        // Just verify they can be created without panicking
        assert!(welcome.is_complete());
        assert!(error.is_complete());
        assert!(loading.is_complete());
    }
}