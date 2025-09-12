//! Visual polish and UX improvements for Goofy TUI
//! 
//! This module provides enhanced visual elements, smooth animations,
//! improved responsiveness, and overall polish for the user experience.

use anyhow::Result;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    symbols::{border, scrollbar},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, Gauge, LineGauge, List, ListItem, Paragraph, 
        Scrollbar, ScrollbarOrientation, ScrollbarState, Widget, Wrap
    },
    Frame,
};
use std::time::{Duration, Instant};

use crate::tui::themes::Theme;
// Temporarily removed animation dependencies
// use crate::tui::components::animations::{AnimationState, Easing, Timeline};

// Simple animation state for compatibility
#[derive(Debug, Clone, PartialEq)]
pub enum AnimationState {
    Idle,
    Running { start_time: Instant, current_frame: u32 },
    Complete,
}

impl AnimationState {
    pub fn new() -> Self {
        Self::Idle
    }
    
    pub fn is_active(&self) -> bool {
        matches!(self, AnimationState::Running { .. })
    }
    
    pub fn update(&mut self, _delta_time: Duration) {
        // Simple implementation
    }
    
    pub fn progress(&self) -> f32 {
        match self {
            AnimationState::Idle => 0.0,
            AnimationState::Running { current_frame, .. } => {
                (*current_frame as f32 / 60.0).min(1.0)
            }
            AnimationState::Complete => 1.0,
        }
    }
}

// Simple easing type for compatibility
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Easing {
    Linear,
    EaseOut,
}

impl Easing {
    pub fn apply(self, t: f32) -> f32 {
        match self {
            Easing::Linear => t,
            Easing::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
        }
    }
}

// Simple timeline for compatibility
pub struct Timeline {
    _placeholder: (),
}

impl Timeline {
    pub fn new() -> Self {
        Self { _placeholder: () }
    }
    
    pub fn update(&mut self, _delta_time: Duration) {
        // Simple implementation
    }
}

/// Enhanced visual components with polish and animations
pub struct PolishEngine {
    /// Current theme
    theme: Theme,
    
    /// Animation timeline
    timeline: Timeline,
    
    /// Performance metrics
    metrics: PerformanceMetrics,
    
    /// Visual effects state
    effects: VisualEffects,
}

/// Performance monitoring for smooth UX
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Frame render times
    frame_times: Vec<Duration>,
    
    /// Current FPS
    fps: f64,
    
    /// Last frame time
    last_frame: Instant,
    
    /// Total frames rendered
    frame_count: u64,
    
    /// Memory usage estimate
    memory_usage: usize,
}

/// Visual effects and enhancements
#[derive(Debug, Clone)]
pub struct VisualEffects {
    /// Smooth scrolling state
    smooth_scroll: SmoothScrollState,
    
    /// Loading indicators
    loading_states: Vec<LoadingIndicator>,
    
    /// Notification system
    notifications: NotificationSystem,
    
    /// Focus indicators
    focus_effects: FocusEffects,
    
    /// Transition effects
    transitions: TransitionEffects,
}

/// Smooth scrolling implementation
#[derive(Debug, Clone)]
pub struct SmoothScrollState {
    /// Target scroll position
    target: f32,
    
    /// Current scroll position
    current: f32,
    
    /// Scroll velocity
    velocity: f32,
    
    /// Animation duration
    duration: Duration,
    
    /// Start time
    start_time: Instant,
    
    /// Whether scrolling is active
    active: bool,
}

/// Enhanced loading indicator
#[derive(Debug, Clone)]
pub struct LoadingIndicator {
    /// Loading text
    text: String,
    
    /// Progress (0.0 to 1.0)
    progress: f32,
    
    /// Animation phase
    phase: f32,
    
    /// Loading style
    style: LoadingStyle,
    
    /// Position on screen
    position: Rect,
    
    /// Whether visible
    visible: bool,
}

/// Loading indicator styles
#[derive(Debug, Clone, Copy)]
pub enum LoadingStyle {
    /// Spinner animation
    Spinner,
    /// Progress bar
    ProgressBar,
    /// Dots animation
    Dots,
    /// Pulse effect
    Pulse,
    /// Wave effect
    Wave,
}

/// Notification system for user feedback
#[derive(Debug, Clone)]
pub struct NotificationSystem {
    /// Active notifications
    notifications: Vec<Notification>,
    
    /// Maximum notifications to show
    max_visible: usize,
    
    /// Default timeout
    default_timeout: Duration,
}

/// Individual notification
#[derive(Debug, Clone)]
pub struct Notification {
    /// Notification message
    message: String,
    
    /// Notification type
    notification_type: NotificationType,
    
    /// Creation time
    created_at: Instant,
    
    /// Timeout duration
    timeout: Duration,
    
    /// Animation state
    animation: AnimationState,
    
    /// Whether it's being dismissed
    dismissing: bool,
}

/// Notification types with different styling
#[derive(Debug, Clone, Copy)]
pub enum NotificationType {
    Info,
    Success,
    Warning,
    Error,
}

/// Focus effect enhancements
#[derive(Debug, Clone)]
pub struct FocusEffects {
    /// Current focused element
    focused_element: Option<String>,
    
    /// Focus animation state
    focus_animation: AnimationState,
    
    /// Glow intensity
    glow_intensity: f32,
    
    /// Focus border style
    border_style: FocusBorderStyle,
}

/// Focus border styles
#[derive(Debug, Clone, Copy)]
pub enum FocusBorderStyle {
    Solid,
    Dashed,
    Double,
    Rounded,
    Thick,
    Glow,
}

/// Transition effects between states
#[derive(Debug, Clone)]
pub struct TransitionEffects {
    /// Active transitions
    transitions: Vec<Transition>,
    
    /// Fade in/out effects
    fade_effects: Vec<FadeEffect>,
    
    /// Slide transitions
    slide_effects: Vec<SlideEffect>,
}

/// Generic transition
#[derive(Debug, Clone)]
pub struct Transition {
    /// Transition ID
    id: String,
    
    /// Start time
    start_time: Instant,
    
    /// Duration
    duration: Duration,
    
    /// Easing function
    easing: Easing,
    
    /// Progress (0.0 to 1.0)
    progress: f32,
    
    /// Whether completed
    completed: bool,
}

/// Fade effect
#[derive(Debug, Clone)]
pub struct FadeEffect {
    /// Target opacity
    target_opacity: f32,
    
    /// Current opacity
    current_opacity: f32,
    
    /// Animation state
    animation: AnimationState,
    
    /// Element area
    area: Rect,
}

/// Slide effect
#[derive(Debug, Clone)]
pub struct SlideEffect {
    /// Slide direction
    direction: SlideDirection,
    
    /// Distance to slide
    distance: i16,
    
    /// Current offset
    current_offset: i16,
    
    /// Animation state
    animation: AnimationState,
    
    /// Element area
    area: Rect,
}

/// Slide directions
#[derive(Debug, Clone, Copy)]
pub enum SlideDirection {
    Up,
    Down,
    Left,
    Right,
}

impl PolishEngine {
    /// Create a new polish engine
    pub fn new(theme: Theme) -> Self {
        Self {
            theme,
            timeline: Timeline::new(),
            metrics: PerformanceMetrics::new(),
            effects: VisualEffects::new(),
        }
    }
    
    /// Update animations and effects
    pub fn update(&mut self, delta_time: Duration) {
        self.timeline.update(delta_time);
        self.metrics.update();
        self.effects.update(delta_time);
    }
    
    /// Render enhanced UI elements
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        // Update performance metrics
        self.metrics.on_frame_start();
        
        // Render visual effects
        self.render_effects(frame, area);
        
        // Update metrics after rendering
        self.metrics.on_frame_end();
    }
    
    /// Render visual effects
    fn render_effects(&self, frame: &mut Frame, area: Rect) {
        // Render transitions
        self.render_transitions(frame, area);
        
        // Render loading indicators
        self.render_loading_indicators(frame, area);
        
        // Render notifications
        self.render_notifications(frame, area);
        
        // Render focus effects
        self.render_focus_effects(frame, area);
    }
    
    /// Render transition effects
    fn render_transitions(&self, frame: &mut Frame, area: Rect) {
        for fade in &self.effects.transitions.fade_effects {
            if fade.animation.is_active() {
                self.render_fade_effect(frame, fade);
            }
        }
        
        for slide in &self.effects.transitions.slide_effects {
            if slide.animation.is_active() {
                self.render_slide_effect(frame, slide);
            }
        }
    }
    
    /// Render fade effect
    fn render_fade_effect(&self, frame: &mut Frame, fade: &FadeEffect) {
        let alpha = (fade.current_opacity * 255.0) as u8;
        let overlay_color = Color::Rgb(0, 0, 0); // Could be configurable
        
        // Create semi-transparent overlay
        let block = Block::default()
            .style(Style::default().bg(overlay_color));
        
        frame.render_widget(Clear, fade.area);
        frame.render_widget(block, fade.area);
    }
    
    /// Render slide effect
    fn render_slide_effect(&self, frame: &mut Frame, slide: &SlideEffect) {
        let offset = slide.current_offset;
        let adjusted_area = match slide.direction {
            SlideDirection::Up => Rect {
                y: slide.area.y.saturating_sub(offset as u16),
                ..slide.area
            },
            SlideDirection::Down => Rect {
                y: slide.area.y.saturating_add(offset as u16),
                ..slide.area
            },
            SlideDirection::Left => Rect {
                x: slide.area.x.saturating_sub(offset as u16),
                ..slide.area
            },
            SlideDirection::Right => Rect {
                x: slide.area.x.saturating_add(offset as u16),
                ..slide.area
            },
        };
        
        // Render element at offset position
        // This would typically be done by the calling code
    }
    
    /// Render loading indicators
    fn render_loading_indicators(&self, frame: &mut Frame, area: Rect) {
        for loading in &self.effects.loading_states {
            if loading.visible {
                self.render_loading_indicator(frame, loading);
            }
        }
    }
    
    /// Render individual loading indicator
    fn render_loading_indicator(&self, frame: &mut Frame, loading: &LoadingIndicator) {
        match loading.style {
            LoadingStyle::Spinner => self.render_spinner(frame, loading),
            LoadingStyle::ProgressBar => self.render_progress_bar(frame, loading),
            LoadingStyle::Dots => self.render_dots(frame, loading),
            LoadingStyle::Pulse => self.render_pulse(frame, loading),
            LoadingStyle::Wave => self.render_wave(frame, loading),
        }
    }
    
    /// Render spinner animation
    fn render_spinner(&self, frame: &mut Frame, loading: &LoadingIndicator) {
        let spinner_chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        let index = (loading.phase * spinner_chars.len() as f32) as usize % spinner_chars.len();
        
        let text_str = format!("{} {}", spinner_chars[index], loading.text);
        let paragraph = Paragraph::new(text_str)
            .style(Style::default().fg(self.theme.accent))
            .alignment(Alignment::Center);
        
        frame.render_widget(paragraph, loading.position);
    }
    
    /// Render progress bar
    fn render_progress_bar(&self, frame: &mut Frame, loading: &LoadingIndicator) {
        let progress = (loading.progress * 100.0) as u16;
        let gauge = Gauge::default()
            .block(Block::default().title(loading.text.as_str()).borders(Borders::ALL))
            .gauge_style(Style::default().fg(self.theme.accent))
            .percent(progress);
        
        frame.render_widget(gauge, loading.position);
    }
    
    /// Render dots animation
    fn render_dots(&self, frame: &mut Frame, loading: &LoadingIndicator) {
        let dots_count = ((loading.phase * 4.0) as usize % 4) + 1;
        let dots = ".".repeat(dots_count);
        let text = format!("{}{}", loading.text, dots);
        
        let paragraph = Paragraph::new(text.as_str())
            .style(Style::default().fg(self.theme.fg_base))
            .alignment(Alignment::Center);
        
        frame.render_widget(paragraph, loading.position);
    }
    
    /// Render pulse effect
    fn render_pulse(&self, frame: &mut Frame, loading: &LoadingIndicator) {
        let intensity = (loading.phase.sin() * 0.5 + 0.5) * 255.0;
        let color = Color::Rgb(intensity as u8, intensity as u8, intensity as u8);
        
        let block = Block::default()
            .title(loading.text.as_str())
            .borders(Borders::ALL)
            .style(Style::default().fg(color));
        
        frame.render_widget(block, loading.position);
    }
    
    /// Render wave effect
    fn render_wave(&self, frame: &mut Frame, loading: &LoadingIndicator) {
        let wave_chars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
        let mut wave_text = String::new();
        
        for i in 0..10 {
            let wave_value = ((loading.phase + i as f32 * 0.2).sin() * 0.5 + 0.5) * (wave_chars.len() - 1) as f32;
            let char_index = wave_value as usize;
            wave_text.push(wave_chars[char_index]);
        }
        
        let text = format!("{} {}", loading.text, wave_text);
        let paragraph = Paragraph::new(text.as_str())
            .style(Style::default().fg(self.theme.accent))
            .alignment(Alignment::Center);
        
        frame.render_widget(paragraph, loading.position);
    }
    
    /// Render notifications
    fn render_notifications(&self, frame: &mut Frame, area: Rect) {
        let notification_height = 3;
        let margin = 1;
        
        for (i, notification) in self.effects.notifications.notifications.iter().enumerate() {
            if i >= self.effects.notifications.max_visible {
                break;
            }
            
            let y_offset = i as u16 * (notification_height + margin);
            let notification_area = Rect {
                x: area.width.saturating_sub(50).max(area.x),
                y: area.y + y_offset,
                width: 48.min(area.width),
                height: notification_height,
            };
            
            self.render_notification(frame, notification, notification_area);
        }
    }
    
    /// Render individual notification
    fn render_notification(&self, frame: &mut Frame, notification: &Notification, area: Rect) {
        let (border_color, icon) = match notification.notification_type {
            NotificationType::Info => (self.theme.info, "ℹ"),
            NotificationType::Success => (self.theme.success, "✓"),
            NotificationType::Warning => (self.theme.warning, "⚠"),
            NotificationType::Error => (self.theme.error, "✗"),
        };
        
        let text = format!("{} {}", icon, notification.message);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(self.theme.bg_subtle));
        
        let paragraph = Paragraph::new(text.as_str())
            .block(block)
            .wrap(Wrap { trim: true });
        
        frame.render_widget(paragraph, area);
    }
    
    /// Render focus effects
    fn render_focus_effects(&self, frame: &mut Frame, area: Rect) {
        if let Some(_focused) = &self.effects.focus_effects.focused_element {
            // Focus effects would be rendered by individual components
            // This is a placeholder for focus highlighting
        }
    }
    
    /// Add a loading indicator
    pub fn add_loading(&mut self, text: String, style: LoadingStyle, position: Rect) {
        let loading = LoadingIndicator {
            text,
            progress: 0.0,
            phase: 0.0,
            style,
            position,
            visible: true,
        };
        
        self.effects.loading_states.push(loading);
    }
    
    /// Update loading progress
    pub fn update_loading_progress(&mut self, index: usize, progress: f32) {
        if let Some(loading) = self.effects.loading_states.get_mut(index) {
            loading.progress = progress.clamp(0.0, 1.0);
        }
    }
    
    /// Remove loading indicator
    pub fn remove_loading(&mut self, index: usize) {
        if index < self.effects.loading_states.len() {
            self.effects.loading_states.remove(index);
        }
    }
    
    /// Show notification
    pub fn show_notification(&mut self, message: String, notification_type: NotificationType) {
        let notification = Notification {
            message,
            notification_type,
            created_at: Instant::now(),
            timeout: self.effects.notifications.default_timeout,
            animation: AnimationState::new(),
            dismissing: false,
        };
        
        self.effects.notifications.notifications.push(notification);
        
        // Remove old notifications if we exceed the limit
        while self.effects.notifications.notifications.len() > self.effects.notifications.max_visible {
            self.effects.notifications.notifications.remove(0);
        }
    }
    
    /// Start smooth scroll
    pub fn start_smooth_scroll(&mut self, target: f32, duration: Duration) {
        self.effects.smooth_scroll.target = target;
        self.effects.smooth_scroll.duration = duration;
        self.effects.smooth_scroll.start_time = Instant::now();
        self.effects.smooth_scroll.active = true;
    }
    
    /// Get current scroll position
    pub fn get_scroll_position(&self) -> f32 {
        self.effects.smooth_scroll.current
    }
    
    /// Get performance metrics
    pub fn performance_metrics(&self) -> &PerformanceMetrics {
        &self.metrics
    }
    
    /// Set theme
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }
}

impl PerformanceMetrics {
    fn new() -> Self {
        Self {
            frame_times: Vec::with_capacity(60),
            fps: 0.0,
            last_frame: Instant::now(),
            frame_count: 0,
            memory_usage: 0,
        }
    }
    
    fn update(&mut self) {
        // Update performance calculations
        let now = Instant::now();
        let frame_time = now.duration_since(self.last_frame);
        
        self.frame_times.push(frame_time);
        if self.frame_times.len() > 60 {
            self.frame_times.remove(0);
        }
        
        // Calculate FPS
        if !self.frame_times.is_empty() {
            let avg_frame_time: Duration = self.frame_times.iter().sum::<Duration>() / self.frame_times.len() as u32;
            self.fps = 1.0 / avg_frame_time.as_secs_f64();
        }
        
        self.last_frame = now;
    }
    
    fn on_frame_start(&mut self) {
        self.frame_count += 1;
    }
    
    fn on_frame_end(&mut self) {
        // Update memory usage estimate
        self.memory_usage = self.frame_times.len() * std::mem::size_of::<Duration>();
    }
    
    pub fn fps(&self) -> f64 {
        self.fps
    }
    
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }
    
    pub fn average_frame_time(&self) -> Duration {
        if self.frame_times.is_empty() {
            Duration::ZERO
        } else {
            self.frame_times.iter().sum::<Duration>() / self.frame_times.len() as u32
        }
    }
}

impl VisualEffects {
    fn new() -> Self {
        Self {
            smooth_scroll: SmoothScrollState::new(),
            loading_states: Vec::new(),
            notifications: NotificationSystem::new(),
            focus_effects: FocusEffects::new(),
            transitions: TransitionEffects::new(),
        }
    }
    
    fn update(&mut self, delta_time: Duration) {
        self.smooth_scroll.update(delta_time);
        self.update_loading_states(delta_time);
        self.notifications.update(delta_time);
        self.focus_effects.update(delta_time);
        self.transitions.update(delta_time);
    }
    
    fn update_loading_states(&mut self, delta_time: Duration) {
        for loading in &mut self.loading_states {
            loading.phase += delta_time.as_secs_f32() * 2.0; // 2 Hz animation
            if loading.phase > std::f32::consts::TAU {
                loading.phase -= std::f32::consts::TAU;
            }
        }
    }
}

impl SmoothScrollState {
    fn new() -> Self {
        Self {
            target: 0.0,
            current: 0.0,
            velocity: 0.0,
            duration: Duration::from_millis(300),
            start_time: Instant::now(),
            active: false,
        }
    }
    
    fn update(&mut self, delta_time: Duration) {
        if !self.active {
            return;
        }
        
        let elapsed = self.start_time.elapsed();
        if elapsed >= self.duration {
            self.current = self.target;
            self.active = false;
            return;
        }
        
        let t = elapsed.as_secs_f32() / self.duration.as_secs_f32();
        let eased_t = Easing::EaseOut.apply(t);
        
        let start_value = self.current;
        self.current = start_value + (self.target - start_value) * eased_t;
    }
}

impl NotificationSystem {
    fn new() -> Self {
        Self {
            notifications: Vec::new(),
            max_visible: 5,
            default_timeout: Duration::from_secs(5),
        }
    }
    
    fn update(&mut self, delta_time: Duration) {
        let now = Instant::now();
        
        // Remove expired notifications
        self.notifications.retain(|notification| {
            now.duration_since(notification.created_at) < notification.timeout
        });
        
        // Update animations
        for notification in &mut self.notifications {
            notification.animation.update(delta_time);
        }
    }
}

impl FocusEffects {
    fn new() -> Self {
        Self {
            focused_element: None,
            focus_animation: AnimationState::new(),
            glow_intensity: 0.0,
            border_style: FocusBorderStyle::Rounded,
        }
    }
    
    fn update(&mut self, delta_time: Duration) {
        self.focus_animation.update(delta_time);
        
        // Update glow intensity
        if self.focused_element.is_some() {
            self.glow_intensity = (self.glow_intensity + delta_time.as_secs_f32() * 3.0).min(1.0);
        } else {
            self.glow_intensity = (self.glow_intensity - delta_time.as_secs_f32() * 3.0).max(0.0);
        }
    }
}

impl TransitionEffects {
    fn new() -> Self {
        Self {
            transitions: Vec::new(),
            fade_effects: Vec::new(),
            slide_effects: Vec::new(),
        }
    }
    
    fn update(&mut self, delta_time: Duration) {
        // Update transitions
        for transition in &mut self.transitions {
            let elapsed = transition.start_time.elapsed();
            transition.progress = (elapsed.as_secs_f32() / transition.duration.as_secs_f32()).min(1.0);
            transition.completed = transition.progress >= 1.0;
        }
        
        // Remove completed transitions
        self.transitions.retain(|t| !t.completed);
        
        // Update fade effects
        for fade in &mut self.fade_effects {
            fade.animation.update(delta_time);
            if fade.animation.progress() >= 1.0 {
                fade.current_opacity = fade.target_opacity;
            }
        }
        
        // Update slide effects
        for slide in &mut self.slide_effects {
            slide.animation.update(delta_time);
        }
    }
}

/// Enhanced widget trait with polish effects
pub trait PolishedWidget {
    /// Render with polish effects
    fn render_polished(&self, frame: &mut Frame, area: Rect, polish: &PolishEngine);
    
    /// Handle focus changes
    fn on_focus_changed(&mut self, focused: bool);
    
    /// Handle hover events
    fn on_hover(&mut self, hovering: bool);
    
    /// Get animation state
    fn animation_state(&self) -> Option<&AnimationState>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::themes::presets;
    
    #[test]
    fn test_polish_engine_creation() {
        let theme = presets::goofy_dark();
        let engine = PolishEngine::new(theme);
        assert_eq!(engine.effects.loading_states.len(), 0);
        assert_eq!(engine.effects.notifications.notifications.len(), 0);
    }
    
    #[test]
    fn test_loading_indicator() {
        let theme = presets::goofy_dark();
        let mut engine = PolishEngine::new(theme);
        
        let area = Rect::new(0, 0, 20, 3);
        engine.add_loading("Loading...".to_string(), LoadingStyle::Spinner, area);
        
        assert_eq!(engine.effects.loading_states.len(), 1);
        assert!(engine.effects.loading_states[0].visible);
    }
    
    #[test]
    fn test_notifications() {
        let theme = presets::goofy_dark();
        let mut engine = PolishEngine::new(theme);
        
        engine.show_notification("Test message".to_string(), NotificationType::Info);
        assert_eq!(engine.effects.notifications.notifications.len(), 1);
        
        engine.show_notification("Another message".to_string(), NotificationType::Success);
        assert_eq!(engine.effects.notifications.notifications.len(), 2);
    }
    
    #[test]
    fn test_smooth_scroll() {
        let theme = presets::goofy_dark();
        let mut engine = PolishEngine::new(theme);
        
        engine.start_smooth_scroll(100.0, Duration::from_millis(300));
        assert!(engine.effects.smooth_scroll.active);
        assert_eq!(engine.effects.smooth_scroll.target, 100.0);
    }
    
    #[test]
    fn test_performance_metrics() {
        let mut metrics = PerformanceMetrics::new();
        
        metrics.on_frame_start();
        std::thread::sleep(Duration::from_millis(1));
        metrics.update();
        metrics.on_frame_end();
        
        assert!(metrics.frame_count() > 0);
        assert!(metrics.fps() > 0.0);
    }
}