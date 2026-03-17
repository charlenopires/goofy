//! Slide animations for panels and sidebars.
//! 
//! This module provides smooth slide-in and slide-out animations for UI panels,
//! sidebars, dialogs, and other elements that need directional movement.

use super::animation_engine::{AnimationEngine, AnimationConfig, EasingType};
use super::interpolation::{Interpolatable, Point};
use super::{Animation, AnimationState};
use crate::tui::themes::Theme;
use anyhow::Result;
use ratatui::layout::Rect;
use ratatui::text::Line;
use std::time::{Duration, Instant};

/// Slide animation direction
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SlideDirection {
    /// Slide from left edge
    FromLeft,
    /// Slide from right edge
    FromRight,
    /// Slide from top edge
    FromTop,
    /// Slide from bottom edge
    FromBottom,
    /// Slide to left edge
    ToLeft,
    /// Slide to right edge
    ToRight,
    /// Slide to top edge
    ToTop,
    /// Slide to bottom edge
    ToBottom,
}

impl SlideDirection {
    /// Get the reverse direction
    pub fn reverse(self) -> Self {
        match self {
            SlideDirection::FromLeft => SlideDirection::ToLeft,
            SlideDirection::FromRight => SlideDirection::ToRight,
            SlideDirection::FromTop => SlideDirection::ToTop,
            SlideDirection::FromBottom => SlideDirection::ToBottom,
            SlideDirection::ToLeft => SlideDirection::FromLeft,
            SlideDirection::ToRight => SlideDirection::FromRight,
            SlideDirection::ToTop => SlideDirection::FromTop,
            SlideDirection::ToBottom => SlideDirection::FromBottom,
        }
    }

    /// Check if this is an entrance animation
    pub fn is_entrance(self) -> bool {
        matches!(self, 
            SlideDirection::FromLeft | 
            SlideDirection::FromRight | 
            SlideDirection::FromTop | 
            SlideDirection::FromBottom
        )
    }

    /// Check if this is an exit animation
    pub fn is_exit(self) -> bool {
        !self.is_entrance()
    }
}

/// Slide animation configuration
#[derive(Debug, Clone)]
pub struct SlideConfig {
    pub direction: SlideDirection,
    pub duration: Duration,
    pub easing: EasingType,
    pub distance: Option<f32>, // If None, uses full container dimension
    pub overshoot: f32, // Amount to overshoot for bounce effect
    pub delay: Duration,
}

impl Default for SlideConfig {
    fn default() -> Self {
        Self {
            direction: SlideDirection::FromLeft,
            duration: Duration::from_millis(300),
            easing: EasingType::EaseOutQuad,
            distance: None,
            overshoot: 0.0,
            delay: Duration::from_millis(0),
        }
    }
}

impl SlideConfig {
    pub fn new(direction: SlideDirection) -> Self {
        Self {
            direction,
            ..Default::default()
        }
    }

    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    pub fn with_easing(mut self, easing: EasingType) -> Self {
        self.easing = easing;
        self
    }

    pub fn with_distance(mut self, distance: f32) -> Self {
        self.distance = Some(distance);
        self
    }

    pub fn with_overshoot(mut self, overshoot: f32) -> Self {
        self.overshoot = overshoot;
        self
    }

    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// Quick configurations for common scenarios
    pub fn sidebar_in() -> Self {
        Self::new(SlideDirection::FromLeft)
            .with_duration(Duration::from_millis(250))
            .with_easing(EasingType::EaseOutQuad)
    }

    pub fn sidebar_out() -> Self {
        Self::new(SlideDirection::ToLeft)
            .with_duration(Duration::from_millis(200))
            .with_easing(EasingType::EaseInQuad)
    }

    pub fn panel_from_right() -> Self {
        Self::new(SlideDirection::FromRight)
            .with_duration(Duration::from_millis(300))
            .with_easing(EasingType::EaseOutCubic)
    }

    pub fn panel_to_right() -> Self {
        Self::new(SlideDirection::ToRight)
            .with_duration(Duration::from_millis(200))
            .with_easing(EasingType::EaseInCubic)
    }

    pub fn dropdown_in() -> Self {
        Self::new(SlideDirection::FromTop)
            .with_duration(Duration::from_millis(200))
            .with_easing(EasingType::EaseOutBounce)
            .with_overshoot(10.0)
    }

    pub fn dropdown_out() -> Self {
        Self::new(SlideDirection::ToTop)
            .with_duration(Duration::from_millis(150))
            .with_easing(EasingType::EaseInQuad)
    }

    pub fn notification_from_bottom() -> Self {
        Self::new(SlideDirection::FromBottom)
            .with_duration(Duration::from_millis(400))
            .with_easing(EasingType::EaseOutElastic)
    }

    pub fn notification_to_bottom() -> Self {
        Self::new(SlideDirection::ToBottom)
            .with_duration(Duration::from_millis(300))
            .with_easing(EasingType::EaseInBack)
    }
}

/// Slide animation component
#[derive(Debug)]
pub struct SlideAnimation {
    config: SlideConfig,
    animation: AnimationEngine,
    target_area: Rect,
    current_area: Rect,
    start_position: Point,
    end_position: Point,
    is_active: bool,
    anim_state: AnimationState,
}

impl SlideAnimation {
    pub fn new(config: SlideConfig, target_area: Rect) -> Self {
        let animation_config = AnimationConfig::new(config.duration)
            .with_easing(config.easing)
            .with_delay(config.delay);

        let (start_pos, end_pos) = Self::calculate_positions(&config, target_area);

        Self {
            config,
            animation: AnimationEngine::new(animation_config),
            target_area,
            current_area: target_area,
            start_position: start_pos,
            end_position: end_pos,
            is_active: false,
            anim_state: AnimationState::Idle,
        }
    }

    /// Calculate start and end positions based on direction and target area
    fn calculate_positions(config: &SlideConfig, target_area: Rect) -> (Point, Point) {
        let target_center = Point::new(
            target_area.x as f32 + target_area.width as f32 / 2.0,
            target_area.y as f32 + target_area.height as f32 / 2.0,
        );

        let distance = config.distance.unwrap_or_else(|| {
            match config.direction {
                SlideDirection::FromLeft | SlideDirection::ToLeft => target_area.width as f32,
                SlideDirection::FromRight | SlideDirection::ToRight => target_area.width as f32,
                SlideDirection::FromTop | SlideDirection::ToTop => target_area.height as f32,
                SlideDirection::FromBottom | SlideDirection::ToBottom => target_area.height as f32,
            }
        });

        let (start_pos, end_pos) = match config.direction {
            SlideDirection::FromLeft => (
                Point::new(target_center.x - distance, target_center.y),
                target_center,
            ),
            SlideDirection::FromRight => (
                Point::new(target_center.x + distance, target_center.y),
                target_center,
            ),
            SlideDirection::FromTop => (
                Point::new(target_center.x, target_center.y - distance),
                target_center,
            ),
            SlideDirection::FromBottom => (
                Point::new(target_center.x, target_center.y + distance),
                target_center,
            ),
            SlideDirection::ToLeft => (
                target_center,
                Point::new(target_center.x - distance, target_center.y),
            ),
            SlideDirection::ToRight => (
                target_center,
                Point::new(target_center.x + distance, target_center.y),
            ),
            SlideDirection::ToTop => (
                target_center,
                Point::new(target_center.x, target_center.y - distance),
            ),
            SlideDirection::ToBottom => (
                target_center,
                Point::new(target_center.x, target_center.y + distance),
            ),
        };

        (start_pos, end_pos)
    }

    /// Set the target area for the slide animation
    pub fn set_target_area(&mut self, area: Rect) {
        self.target_area = area;
        let (start_pos, end_pos) = Self::calculate_positions(&self.config, area);
        self.start_position = start_pos;
        self.end_position = end_pos;
    }

    /// Start the slide animation
    pub fn start(&mut self) {
        self.animation.start();
        self.is_active = true;
        self.anim_state = AnimationState::Running {
            start_time: Instant::now(),
            current_frame: 0,
        };
        self.update_current_area(0.0);
    }

    /// Stop the slide animation
    pub fn stop(&mut self) {
        self.animation.stop();
        self.is_active = false;
        self.anim_state = AnimationState::Complete;
    }

    /// Update the animation
    pub fn update(&mut self) -> Result<bool> {
        if !self.is_active {
            return Ok(false);
        }

        if self.animation.should_update() {
            let progress = self.animation.eased_progress();
            self.update_current_area(progress);
            Ok(true)
        } else if self.animation.is_completed() {
            self.update_current_area(1.0);
            self.is_active = false;
            Ok(false)
        } else {
            Ok(false)
        }
    }

    /// Update the current area based on animation progress
    fn update_current_area(&mut self, progress: f32) {
        let mut adjusted_progress = progress;

        // Apply overshoot effect
        if self.config.overshoot > 0.0 && progress > 0.8 {
            let overshoot_phase = (progress - 0.8) / 0.2; // 0.8 to 1.0 mapped to 0.0 to 1.0
            let overshoot_amount = self.config.overshoot * (1.0 - overshoot_phase) * overshoot_phase * 4.0;
            adjusted_progress = progress + overshoot_amount * 0.01;
        }

        let current_pos = self.start_position.interpolate(&self.end_position, adjusted_progress);
        
        // Calculate the offset from target center
        let target_center = Point::new(
            self.target_area.x as f32 + self.target_area.width as f32 / 2.0,
            self.target_area.y as f32 + self.target_area.height as f32 / 2.0,
        );
        
        let offset_x = current_pos.x - target_center.x;
        let offset_y = current_pos.y - target_center.y;
        
        // Apply offset to target area
        self.current_area = Rect {
            x: (self.target_area.x as f32 + offset_x).max(0.0) as u16,
            y: (self.target_area.y as f32 + offset_y).max(0.0) as u16,
            width: self.target_area.width,
            height: self.target_area.height,
        };
    }

    /// Get the current animation area
    pub fn current_area(&self) -> Rect {
        self.current_area
    }

    /// Get the target area
    pub fn target_area(&self) -> Rect {
        self.target_area
    }

    /// Check if animation is running
    pub fn is_running(&self) -> bool {
        self.is_active && self.animation.is_running()
    }

    /// Check if animation is completed
    pub fn is_completed(&self) -> bool {
        self.animation.is_completed()
    }

    /// Get animation progress
    pub fn progress(&self) -> f32 {
        self.animation.progress()
    }

    /// Check if element is visible (not completely slid out)
    pub fn is_visible(&self) -> bool {
        if self.config.direction.is_exit() && self.is_completed() {
            false
        } else {
            self.current_area.width > 0 && self.current_area.height > 0
        }
    }

    /// Calculate clipping area for content that may extend outside bounds
    pub fn clip_area(&self, container: Rect) -> Rect {
        let current = self.current_area();
        
        Rect {
            x: current.x.max(container.x),
            y: current.y.max(container.y),
            width: current.width.min(container.width.saturating_sub(current.x.saturating_sub(container.x))),
            height: current.height.min(container.height.saturating_sub(current.y.saturating_sub(container.y))),
        }
    }
}

impl Animation for SlideAnimation {
    fn start(&mut self) -> Result<()> {
        self.animation.start();
        self.is_active = true;
        self.anim_state = AnimationState::Running {
            start_time: Instant::now(),
            current_frame: 0,
        };
        self.update_current_area(0.0);
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        self.animation.stop();
        self.is_active = false;
        self.anim_state = AnimationState::Complete;
        Ok(())
    }

    fn update(&mut self) -> Result<()> {
        if !self.is_active {
            return Ok(());
        }

        if self.animation.should_update() {
            let progress = self.animation.eased_progress();
            self.update_current_area(progress);
        } else if self.animation.is_completed() {
            self.update_current_area(1.0);
            self.is_active = false;
            self.anim_state = AnimationState::Complete;
        }

        Ok(())
    }

    fn is_complete(&self) -> bool {
        self.animation.is_completed()
    }

    fn state(&self) -> &AnimationState {
        &self.anim_state
    }

    fn render(&self, _area: Rect, _theme: &Theme) -> Vec<Line> {
        // SlideAnimation affects positioning, not content rendering.
        // Return empty lines; the caller should use current_area() for positioning.
        Vec::new()
    }
}

/// Slide sequence for chaining multiple slide animations
#[derive(Debug)]
pub struct SlideSequence {
    slides: Vec<SlideAnimation>,
    current_index: usize,
    is_active: bool,
}

impl SlideSequence {
    pub fn new() -> Self {
        Self {
            slides: Vec::new(),
            current_index: 0,
            is_active: false,
        }
    }

    /// Add a slide animation to the sequence
    pub fn add_slide(mut self, slide: SlideAnimation) -> Self {
        self.slides.push(slide);
        self
    }

    /// Start the slide sequence
    pub fn start(&mut self) {
        if !self.slides.is_empty() {
            self.current_index = 0;
            self.slides[0].start();
            self.is_active = true;
        }
    }

    /// Update the slide sequence
    pub fn update(&mut self) -> Result<bool> {
        if !self.is_active || self.current_index >= self.slides.len() {
            return Ok(false);
        }

        let updated = self.slides[self.current_index].update()?;
        
        // Check if current slide is completed
        if self.slides[self.current_index].is_completed() {
            self.current_index += 1;
            
            // Start next slide if available
            if self.current_index < self.slides.len() {
                self.slides[self.current_index].start();
            } else {
                self.is_active = false;
            }
        }

        Ok(updated)
    }

    /// Get the current active slide
    pub fn current_slide(&self) -> Option<&SlideAnimation> {
        if self.current_index < self.slides.len() {
            Some(&self.slides[self.current_index])
        } else {
            None
        }
    }

    /// Get a mutable reference to the current active slide
    pub fn current_slide_mut(&mut self) -> Option<&mut SlideAnimation> {
        if self.current_index < self.slides.len() {
            Some(&mut self.slides[self.current_index])
        } else {
            None
        }
    }

    /// Check if sequence is running
    pub fn is_running(&self) -> bool {
        self.is_active
    }

    /// Get overall sequence progress
    pub fn progress(&self) -> f32 {
        if self.slides.is_empty() || !self.is_active {
            return 1.0;
        }

        let completed_slides = self.current_index;
        let current_progress = if self.current_index < self.slides.len() {
            self.slides[self.current_index].progress()
        } else {
            1.0
        };

        (completed_slides as f32 + current_progress) / self.slides.len() as f32
    }
}

impl Default for SlideSequence {
    fn default() -> Self {
        Self::new()
    }
}

/// Collection of slide presets for common UI scenarios
pub struct SlidePresets;

impl SlidePresets {
    /// Sidebar slide in from left
    pub fn sidebar_in(area: Rect) -> SlideAnimation {
        SlideAnimation::new(SlideConfig::sidebar_in(), area)
    }

    /// Sidebar slide out to left
    pub fn sidebar_out(area: Rect) -> SlideAnimation {
        SlideAnimation::new(SlideConfig::sidebar_out(), area)
    }

    /// Panel slide in from right
    pub fn panel_in(area: Rect) -> SlideAnimation {
        SlideAnimation::new(SlideConfig::panel_from_right(), area)
    }

    /// Panel slide out to right
    pub fn panel_out(area: Rect) -> SlideAnimation {
        SlideAnimation::new(SlideConfig::panel_to_right(), area)
    }

    /// Dropdown menu slide in
    pub fn dropdown_in(area: Rect) -> SlideAnimation {
        SlideAnimation::new(SlideConfig::dropdown_in(), area)
    }

    /// Dropdown menu slide out
    pub fn dropdown_out(area: Rect) -> SlideAnimation {
        SlideAnimation::new(SlideConfig::dropdown_out(), area)
    }

    /// Notification slide in from bottom
    pub fn notification_in(area: Rect) -> SlideAnimation {
        SlideAnimation::new(SlideConfig::notification_from_bottom(), area)
    }

    /// Notification slide out to bottom
    pub fn notification_out(area: Rect) -> SlideAnimation {
        SlideAnimation::new(SlideConfig::notification_to_bottom(), area)
    }

    /// Card slide sequence (multiple cards sliding in from different directions)
    pub fn card_sequence(areas: Vec<Rect>) -> SlideSequence {
        let mut sequence = SlideSequence::new();
        
        for (i, area) in areas.into_iter().enumerate() {
            let direction = match i % 4 {
                0 => SlideDirection::FromLeft,
                1 => SlideDirection::FromRight,
                2 => SlideDirection::FromTop,
                _ => SlideDirection::FromBottom,
            };
            
            let config = SlideConfig::new(direction)
                .with_duration(Duration::from_millis(200))
                .with_delay(Duration::from_millis(i as u64 * 50));
            
            sequence = sequence.add_slide(SlideAnimation::new(config, area));
        }
        
        sequence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slide_direction() {
        assert_eq!(SlideDirection::FromLeft.reverse(), SlideDirection::ToLeft);
        assert_eq!(SlideDirection::ToRight.reverse(), SlideDirection::FromRight);
        
        assert!(SlideDirection::FromLeft.is_entrance());
        assert!(SlideDirection::ToLeft.is_exit());
    }

    #[test]
    fn test_slide_config_creation() {
        let config = SlideConfig::new(SlideDirection::FromLeft)
            .with_duration(Duration::from_millis(500))
            .with_easing(EasingType::EaseOutBounce)
            .with_overshoot(10.0);
        
        assert_eq!(config.direction, SlideDirection::FromLeft);
        assert_eq!(config.duration, Duration::from_millis(500));
        assert_eq!(config.easing, EasingType::EaseOutBounce);
        assert_eq!(config.overshoot, 10.0);
    }

    #[test]
    fn test_slide_animation_lifecycle() {
        let config = SlideConfig::sidebar_in();
        let area = Rect::new(10, 10, 20, 30);
        let mut slide = SlideAnimation::new(config, area);
        
        assert!(!slide.is_running());
        assert_eq!(slide.target_area(), area);
        
        slide.start();
        assert!(slide.is_running());
        
        slide.stop();
        assert!(!slide.is_running());
    }

    #[test]
    fn test_slide_sequence() {
        let area1 = Rect::new(0, 0, 10, 10);
        let area2 = Rect::new(10, 10, 10, 10);
        
        let slide1 = SlideAnimation::new(SlideConfig::sidebar_in(), area1);
        let slide2 = SlideAnimation::new(SlideConfig::panel_from_right(), area2);
        
        let mut sequence = SlideSequence::new()
            .add_slide(slide1)
            .add_slide(slide2);
        
        assert!(!sequence.is_running());
        assert_eq!(sequence.progress(), 1.0); // Empty or not started
        
        sequence.start();
        assert!(sequence.is_running());
    }

    #[test]
    fn test_position_calculation() {
        let area = Rect::new(10, 10, 20, 20);
        let config = SlideConfig::new(SlideDirection::FromLeft);
        
        let (start_pos, end_pos) = SlideAnimation::calculate_positions(&config, area);
        
        // End position should be center of area
        assert_eq!(end_pos.x, 20.0); // 10 + 20/2
        assert_eq!(end_pos.y, 20.0); // 10 + 20/2
        
        // Start position should be to the left
        assert!(start_pos.x < end_pos.x);
        assert_eq!(start_pos.y, end_pos.y);
    }

    #[test]
    fn test_slide_presets() {
        let area = Rect::new(0, 0, 100, 50);
        
        let sidebar = SlidePresets::sidebar_in(area);
        let panel = SlidePresets::panel_in(area);
        let dropdown = SlidePresets::dropdown_in(area);
        let notification = SlidePresets::notification_in(area);
        
        // Just verify they can be created without panicking
        assert!(!sidebar.is_running());
        assert!(!panel.is_running());
        assert!(!dropdown.is_running());
        assert!(!notification.is_running());
    }
}