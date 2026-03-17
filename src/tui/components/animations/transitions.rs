//! Smooth transitions between UI states and component properties.
//! 
//! This module provides high-level transition management for common UI scenarios
//! like showing/hiding components, changing colors, moving elements, and resizing.

use super::animation_engine::{AnimationEngine, AnimationConfig, EasingType};
use super::timeline::{Timeline, TimelineBuilder, AnimationId};
use anyhow::Result;
use ratatui::layout::Rect;
use ratatui::style::Color;
use std::time::Duration;

/// Represents a property that can be animated
#[derive(Debug, Clone, PartialEq)]
pub enum AnimatedProperty {
    Opacity(f32),
    Position(i32, i32),
    Size(u16, u16),
    Color(u8, u8, u8),
    Scale(f32),
    Rotation(f32),
    Custom(String, f32),
}

impl AnimatedProperty {
    /// Interpolate between two properties
    pub fn interpolate(&self, target: &AnimatedProperty, progress: f32) -> Option<AnimatedProperty> {
        match (self, target) {
            (AnimatedProperty::Opacity(start), AnimatedProperty::Opacity(end)) => {
                Some(AnimatedProperty::Opacity(start + (end - start) * progress))
            }
            (AnimatedProperty::Position(x1, y1), AnimatedProperty::Position(x2, y2)) => {
                let new_x = *x1 + ((*x2 - *x1) as f32 * progress) as i32;
                let new_y = *y1 + ((*y2 - *y1) as f32 * progress) as i32;
                Some(AnimatedProperty::Position(new_x, new_y))
            }
            (AnimatedProperty::Size(w1, h1), AnimatedProperty::Size(w2, h2)) => {
                let new_w = *w1 + ((*w2 as f32 - *w1 as f32) * progress) as u16;
                let new_h = *h1 + ((*h2 as f32 - *h1 as f32) * progress) as u16;
                Some(AnimatedProperty::Size(new_w, new_h))
            }
            (AnimatedProperty::Color(r1, g1, b1), AnimatedProperty::Color(r2, g2, b2)) => {
                let new_r = (*r1 as f32 + (*r2 as f32 - *r1 as f32) * progress) as u8;
                let new_g = (*g1 as f32 + (*g2 as f32 - *g1 as f32) * progress) as u8;
                let new_b = (*b1 as f32 + (*b2 as f32 - *b1 as f32) * progress) as u8;
                Some(AnimatedProperty::Color(new_r, new_g, new_b))
            }
            (AnimatedProperty::Scale(start), AnimatedProperty::Scale(end)) => {
                Some(AnimatedProperty::Scale(start + (end - start) * progress))
            }
            (AnimatedProperty::Rotation(start), AnimatedProperty::Rotation(end)) => {
                Some(AnimatedProperty::Rotation(start + (end - start) * progress))
            }
            (AnimatedProperty::Custom(name1, val1), AnimatedProperty::Custom(name2, val2)) if name1 == name2 => {
                Some(AnimatedProperty::Custom(name1.clone(), val1 + (val2 - val1) * progress))
            }
            _ => None, // Incompatible property types
        }
    }
}

/// Transition state for a component
#[derive(Debug, Clone)]
pub struct TransitionState {
    pub properties: Vec<(String, AnimatedProperty)>,
    pub is_transitioning: bool,
    pub current_transition: Option<AnimationId>,
}

impl TransitionState {
    pub fn new() -> Self {
        Self {
            properties: Vec::new(),
            is_transitioning: false,
            current_transition: None,
        }
    }

    /// Get a property value by name
    pub fn get_property(&self, name: &str) -> Option<&AnimatedProperty> {
        self.properties
            .iter()
            .find(|(prop_name, _)| prop_name == name)
            .map(|(_, prop)| prop)
    }

    /// Set a property value
    pub fn set_property(&mut self, name: String, property: AnimatedProperty) {
        if let Some(pos) = self.properties.iter().position(|(prop_name, _)| prop_name == &name) {
            self.properties[pos] = (name, property);
        } else {
            self.properties.push((name, property));
        }
    }

    /// Get opacity value (default 1.0 if not set)
    pub fn opacity(&self) -> f32 {
        match self.get_property("opacity") {
            Some(AnimatedProperty::Opacity(val)) => *val,
            _ => 1.0,
        }
    }

    /// Get position (default 0,0 if not set)
    pub fn position(&self) -> (i32, i32) {
        match self.get_property("position") {
            Some(AnimatedProperty::Position(x, y)) => (*x, *y),
            _ => (0, 0),
        }
    }

    /// Get size (default 0,0 if not set)
    pub fn size(&self) -> (u16, u16) {
        match self.get_property("size") {
            Some(AnimatedProperty::Size(w, h)) => (*w, *h),
            _ => (0, 0),
        }
    }

    /// Get color (default white if not set)
    pub fn color(&self) -> (u8, u8, u8) {
        match self.get_property("color") {
            Some(AnimatedProperty::Color(r, g, b)) => (*r, *g, *b),
            _ => (255, 255, 255),
        }
    }
}

impl Default for TransitionState {
    fn default() -> Self {
        Self::new()
    }
}

/// Manager for component transitions
#[derive(Debug)]
pub struct TransitionManager {
    timeline: Timeline,
    states: std::collections::HashMap<String, TransitionState>,
    next_id: u32,
}

impl TransitionManager {
    pub fn new() -> Self {
        Self {
            timeline: Timeline::new(),
            states: std::collections::HashMap::new(),
            next_id: 0,
        }
    }

    /// Generate unique animation ID
    fn next_animation_id(&mut self) -> AnimationId {
        self.next_id += 1;
        format!("transition_{}", self.next_id)
    }

    /// Start a property transition for a component
    pub fn transition_property(
        &mut self,
        component_id: String,
        property_name: String,
        from: AnimatedProperty,
        to: AnimatedProperty,
        config: AnimationConfig,
    ) -> Result<AnimationId> {
        let animation_id = self.next_animation_id();
        
        // Ensure component state exists
        if !self.states.contains_key(&component_id) {
            self.states.insert(component_id.clone(), TransitionState::new());
        }
        
        // Set initial property value
        if let Some(state) = self.states.get_mut(&component_id) {
            state.set_property(property_name.clone(), from.clone());
            state.is_transitioning = true;
            state.current_transition = Some(animation_id.clone());
        }
        
        // Create timeline animation
        let timeline_animation = super::timeline::TimelineAnimation::new(animation_id.clone(), config);
        self.timeline.add_animation(timeline_animation)?;
        
        // Store transition info for updates
        self.timeline.start();
        
        Ok(animation_id)
    }

    /// Update all transitions
    pub fn update(&mut self) -> Result<()> {
        let events = self.timeline.update()?;
        
        // Update component states based on animation progress
        for (component_id, state) in &mut self.states {
            if let Some(transition_id) = &state.current_transition {
                if let Some(progress) = self.timeline.animation_eased_progress(transition_id) {
                    // Update properties based on progress
                    // This would need property-specific interpolation logic
                    
                    if progress >= 1.0 {
                        state.is_transitioning = false;
                        state.current_transition = None;
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Get component transition state
    pub fn get_state(&self, component_id: &str) -> Option<&TransitionState> {
        self.states.get(component_id)
    }

    /// Check if component is transitioning
    pub fn is_transitioning(&self, component_id: &str) -> bool {
        self.states
            .get(component_id)
            .map(|state| state.is_transitioning)
            .unwrap_or(false)
    }
}

impl Default for TransitionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Pre-built transition types
pub struct Transitions;

impl Transitions {
    /// Fade in transition
    pub fn fade_in(duration: Duration) -> AnimationConfig {
        AnimationConfig::new(duration)
            .with_easing(EasingType::EaseOut)
    }

    /// Fade out transition
    pub fn fade_out(duration: Duration) -> AnimationConfig {
        AnimationConfig::new(duration)
            .with_easing(EasingType::EaseIn)
    }

    /// Slide in from left
    pub fn slide_in_left(duration: Duration) -> AnimationConfig {
        AnimationConfig::new(duration)
            .with_easing(EasingType::EaseOutQuad)
    }

    /// Slide in from right
    pub fn slide_in_right(duration: Duration) -> AnimationConfig {
        AnimationConfig::new(duration)
            .with_easing(EasingType::EaseOutQuad)
    }

    /// Slide in from top
    pub fn slide_in_top(duration: Duration) -> AnimationConfig {
        AnimationConfig::new(duration)
            .with_easing(EasingType::EaseOutQuad)
    }

    /// Slide in from bottom
    pub fn slide_in_bottom(duration: Duration) -> AnimationConfig {
        AnimationConfig::new(duration)
            .with_easing(EasingType::EaseOutQuad)
    }

    /// Scale up animation
    pub fn scale_up(duration: Duration) -> AnimationConfig {
        AnimationConfig::new(duration)
            .with_easing(EasingType::EaseOutBounce)
    }

    /// Scale down animation
    pub fn scale_down(duration: Duration) -> AnimationConfig {
        AnimationConfig::new(duration)
            .with_easing(EasingType::EaseInQuad)
    }

    /// Smooth color transition
    pub fn color_change(duration: Duration) -> AnimationConfig {
        AnimationConfig::new(duration)
            .with_easing(EasingType::EaseInOut)
    }

    /// Elastic bounce animation
    pub fn elastic_bounce(duration: Duration) -> AnimationConfig {
        AnimationConfig::new(duration)
            .with_easing(EasingType::EaseOutElastic)
    }

    /// Quick snap animation
    pub fn quick_snap(duration: Duration) -> AnimationConfig {
        AnimationConfig::new(duration)
            .with_easing(EasingType::EaseInOutQuart)
    }
}

/// Helper for creating complex transition sequences
pub struct TransitionSequence {
    builder: TimelineBuilder,
}

impl TransitionSequence {
    pub fn new() -> Self {
        Self {
            builder: TimelineBuilder::new(),
        }
    }

    /// Add a fade-in transition
    pub fn fade_in(mut self, id: AnimationId, duration: Duration) -> Self {
        self.builder = self.builder.add(id, Transitions::fade_in(duration));
        self
    }

    /// Add a fade-out transition
    pub fn fade_out(mut self, id: AnimationId, duration: Duration) -> Self {
        self.builder = self.builder.add(id, Transitions::fade_out(duration));
        self
    }

    /// Add a slide-in transition
    pub fn slide_in(mut self, id: AnimationId, duration: Duration, direction: SlideDirection) -> Self {
        let config = match direction {
            SlideDirection::Left => Transitions::slide_in_left(duration),
            SlideDirection::Right => Transitions::slide_in_right(duration),
            SlideDirection::Up => Transitions::slide_in_top(duration),
            SlideDirection::Down => Transitions::slide_in_bottom(duration),
        };
        self.builder = self.builder.add(id, config);
        self
    }

    /// Add a delay
    pub fn delay(mut self, duration: Duration) -> Self {
        let delay_id = format!("delay_{}", rand::random::<u32>());
        let config = AnimationConfig::new(duration).with_easing(EasingType::Linear);
        self.builder = self.builder.add(delay_id, config);
        self
    }

    /// Add transitions in parallel
    pub fn parallel(mut self, transitions: Vec<(AnimationId, AnimationConfig)>) -> Self {
        let group_name = format!("parallel_{}", rand::random::<u32>());
        self.builder = self.builder.add_parallel_group(group_name, transitions);
        self
    }

    /// Build the timeline
    pub fn build(self) -> Timeline {
        self.builder.build()
    }
}

impl Default for TransitionSequence {
    fn default() -> Self {
        Self::new()
    }
}

/// Slide direction for transitions
#[derive(Debug, Clone, Copy)]
pub enum SlideDirection {
    Left,
    Right,
    Up,
    Down,
}

/// Utility functions for common transition patterns
pub mod utils {
    use super::*;

    /// Create a dialog entrance animation (fade + scale)
    pub fn dialog_entrance() -> Timeline {
        TransitionSequence::new()
            .parallel(vec![
                ("fade".to_string(), Transitions::fade_in(Duration::from_millis(300))),
                ("scale".to_string(), Transitions::scale_up(Duration::from_millis(300))),
            ])
            .build()
    }

    /// Create a dialog exit animation (fade + scale down)
    pub fn dialog_exit() -> Timeline {
        TransitionSequence::new()
            .parallel(vec![
                ("fade".to_string(), Transitions::fade_out(Duration::from_millis(200))),
                ("scale".to_string(), Transitions::scale_down(Duration::from_millis(200))),
            ])
            .build()
    }

    /// Create a sidebar slide animation
    pub fn sidebar_slide(direction: SlideDirection) -> Timeline {
        TransitionSequence::new()
            .slide_in("slide".to_string(), Duration::from_millis(400), direction)
            .build()
    }

    /// Create a notification popup animation
    pub fn notification_popup() -> Timeline {
        TransitionSequence::new()
            .slide_in("slide".to_string(), Duration::from_millis(300), SlideDirection::Down)
            .delay(Duration::from_millis(100))
            .fade_in("fade".to_string(), Duration::from_millis(200))
            .build()
    }

    /// Create a loading pulse animation
    pub fn loading_pulse() -> Timeline {
        let config = AnimationConfig::new(Duration::from_millis(1000))
            .with_easing(EasingType::EaseInOut)
            .infinite()
            .with_reverse();
        
        TimelineBuilder::new()
            .add("pulse".to_string(), config)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animated_property_interpolation() {
        let start = AnimatedProperty::Opacity(0.0);
        let end = AnimatedProperty::Opacity(1.0);
        
        let mid = start.interpolate(&end, 0.5).unwrap();
        match mid {
            AnimatedProperty::Opacity(val) => assert_eq!(val, 0.5),
            _ => panic!("Expected opacity property"),
        }
    }

    #[test]
    fn test_transition_state() {
        let mut state = TransitionState::new();
        state.set_property("opacity".to_string(), AnimatedProperty::Opacity(0.8));
        
        assert_eq!(state.opacity(), 0.8);
        assert_eq!(state.position(), (0, 0)); // Default value
    }

    #[test]
    fn test_transition_sequence() {
        let timeline = TransitionSequence::new()
            .fade_in("test".to_string(), Duration::from_millis(300))
            .build();
        
        assert_eq!(timeline.state(), super::super::animation_engine::AnimationState::Idle);
    }

    #[test]
    fn test_color_interpolation() {
        let start = AnimatedProperty::Color(255, 0, 0); // Red
        let end = AnimatedProperty::Color(0, 255, 0);   // Green
        
        let mid = start.interpolate(&end, 0.5).unwrap();
        match mid {
            AnimatedProperty::Color(r, g, b) => {
                assert_eq!(r, 127);
                assert_eq!(g, 127);
                assert_eq!(b, 0);
            }
            _ => panic!("Expected color property"),
        }
    }
}