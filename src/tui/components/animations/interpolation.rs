//! Advanced interpolation utilities for smooth value transitions.
//! 
//! This module provides specialized interpolation functions for different data types
//! commonly used in UI animations, including colors, rectangles, gradients, and
//! custom interpolation patterns.

use ratatui::layout::Rect;
use ratatui::style::Color;
use std::collections::HashMap;

/// Trait for types that can be interpolated
pub trait Interpolatable {
    fn interpolate(&self, other: &Self, t: f32) -> Self;
}

/// Interpolate between two f32 values
impl Interpolatable for f32 {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        self + (other - self) * t.clamp(0.0, 1.0)
    }
}

/// Interpolate between two f64 values
impl Interpolatable for f64 {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        self + (other - self) * t.clamp(0.0, 1.0) as f64
    }
}

/// Interpolate between two i32 values
impl Interpolatable for i32 {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        *self + ((other - self) as f32 * t.clamp(0.0, 1.0)) as i32
    }
}

/// Interpolate between two u16 values
impl Interpolatable for u16 {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        *self + ((*other as f32 - *self as f32) * t.clamp(0.0, 1.0)) as u16
    }
}

/// Color interpolation in RGB space
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn from_color(color: Color) -> Self {
        match color {
            Color::Rgb(r, g, b) => Self::new(r, g, b),
            Color::Black => Self::new(0, 0, 0),
            Color::Red => Self::new(255, 0, 0),
            Color::Green => Self::new(0, 255, 0),
            Color::Yellow => Self::new(255, 255, 0),
            Color::Blue => Self::new(0, 0, 255),
            Color::Magenta => Self::new(255, 0, 255),
            Color::Cyan => Self::new(0, 255, 255),
            Color::Gray => Self::new(128, 128, 128),
            Color::DarkGray => Self::new(64, 64, 64),
            Color::LightRed => Self::new(255, 128, 128),
            Color::LightGreen => Self::new(128, 255, 128),
            Color::LightYellow => Self::new(255, 255, 128),
            Color::LightBlue => Self::new(128, 128, 255),
            Color::LightMagenta => Self::new(255, 128, 255),
            Color::LightCyan => Self::new(128, 255, 255),
            Color::White => Self::new(255, 255, 255),
            _ => Self::new(255, 255, 255), // Default to white for indexed colors
        }
    }

    pub fn to_color(self) -> Color {
        Color::Rgb(self.r, self.g, self.b)
    }

    /// Linear interpolation in RGB space
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            r: (self.r as f32 + (other.r as f32 - self.r as f32) * t) as u8,
            g: (self.g as f32 + (other.g as f32 - self.g as f32) * t) as u8,
            b: (self.b as f32 + (other.b as f32 - self.b as f32) * t) as u8,
        }
    }

    /// Convert to HSL for better color interpolation
    pub fn to_hsl(&self) -> HslColor {
        let r = self.r as f32 / 255.0;
        let g = self.g as f32 / 255.0;
        let b = self.b as f32 / 255.0;

        let max = r.max(g.max(b));
        let min = r.min(g.min(b));
        let delta = max - min;

        let lightness = (max + min) / 2.0;

        let saturation = if delta == 0.0 {
            0.0
        } else if lightness < 0.5 {
            delta / (max + min)
        } else {
            delta / (2.0 - max - min)
        };

        let hue = if delta == 0.0 {
            0.0
        } else if max == r {
            60.0 * (((g - b) / delta) % 6.0)
        } else if max == g {
            60.0 * (((b - r) / delta) + 2.0)
        } else {
            60.0 * (((r - g) / delta) + 4.0)
        };

        HslColor::new(hue, saturation, lightness)
    }
}

impl Interpolatable for RgbColor {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        self.lerp(other, t)
    }
}

/// Color interpolation in HSL space for more natural color transitions
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HslColor {
    pub h: f32, // Hue: 0-360
    pub s: f32, // Saturation: 0-1
    pub l: f32, // Lightness: 0-1
}

impl HslColor {
    pub fn new(h: f32, s: f32, l: f32) -> Self {
        Self {
            h: h % 360.0,
            s: s.clamp(0.0, 1.0),
            l: l.clamp(0.0, 1.0),
        }
    }

    /// Convert HSL to RGB
    pub fn to_rgb(&self) -> RgbColor {
        let c = (1.0 - (2.0 * self.l - 1.0).abs()) * self.s;
        let x = c * (1.0 - ((self.h / 60.0) % 2.0 - 1.0).abs());
        let m = self.l - c / 2.0;

        let (r_prime, g_prime, b_prime) = if self.h < 60.0 {
            (c, x, 0.0)
        } else if self.h < 120.0 {
            (x, c, 0.0)
        } else if self.h < 180.0 {
            (0.0, c, x)
        } else if self.h < 240.0 {
            (0.0, x, c)
        } else if self.h < 300.0 {
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

    /// Interpolate in HSL space with shortest hue path
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        
        // Calculate shortest hue interpolation
        let hue_diff = other.h - self.h;
        let hue = if hue_diff.abs() > 180.0 {
            if hue_diff > 0.0 {
                self.h + (hue_diff - 360.0) * t
            } else {
                self.h + (hue_diff + 360.0) * t
            }
        } else {
            self.h + hue_diff * t
        };

        Self::new(
            hue,
            self.s + (other.s - self.s) * t,
            self.l + (other.l - self.l) * t,
        )
    }
}

impl Interpolatable for HslColor {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        self.lerp(other, t)
    }
}

/// Rectangle interpolation for smooth position and size transitions
impl Interpolatable for Rect {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Rect {
            x: self.x.interpolate(&other.x, t),
            y: self.y.interpolate(&other.y, t),
            width: self.width.interpolate(&other.width, t),
            height: self.height.interpolate(&other.height, t),
        }
    }
}

/// Point interpolation for 2D positions
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl Interpolatable for Point {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        Self {
            x: self.x.interpolate(&other.x, t),
            y: self.y.interpolate(&other.y, t),
        }
    }
}

/// Bezier curve interpolation for smooth curves
#[derive(Debug, Clone)]
pub struct BezierCurve {
    pub points: Vec<Point>,
}

impl BezierCurve {
    pub fn new(points: Vec<Point>) -> Self {
        Self { points }
    }

    /// Create a quadratic Bezier curve
    pub fn quadratic(start: Point, control: Point, end: Point) -> Self {
        Self::new(vec![start, control, end])
    }

    /// Create a cubic Bezier curve
    pub fn cubic(start: Point, control1: Point, control2: Point, end: Point) -> Self {
        Self::new(vec![start, control1, control2, end])
    }

    /// Evaluate the curve at parameter t (0.0 to 1.0)
    pub fn evaluate(&self, t: f32) -> Point {
        let t = t.clamp(0.0, 1.0);
        
        if self.points.is_empty() {
            return Point::new(0.0, 0.0);
        }
        
        if self.points.len() == 1 {
            return self.points[0];
        }

        self.de_casteljau(t)
    }

    /// De Casteljau's algorithm for Bezier curve evaluation
    fn de_casteljau(&self, t: f32) -> Point {
        let mut points = self.points.clone();
        
        while points.len() > 1 {
            let mut new_points = Vec::new();
            for i in 0..points.len() - 1 {
                new_points.push(points[i].interpolate(&points[i + 1], t));
            }
            points = new_points;
        }
        
        points[0]
    }
}

/// Gradient interpolation for smooth color transitions
#[derive(Debug, Clone)]
pub struct ColorGradient {
    pub stops: Vec<(f32, RgbColor)>, // (position, color) pairs
}

impl ColorGradient {
    pub fn new() -> Self {
        Self { stops: Vec::new() }
    }

    /// Add a color stop
    pub fn add_stop(mut self, position: f32, color: RgbColor) -> Self {
        self.stops.push((position.clamp(0.0, 1.0), color));
        self.stops.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        self
    }

    /// Evaluate the gradient at position t (0.0 to 1.0)
    pub fn evaluate(&self, t: f32) -> RgbColor {
        let t = t.clamp(0.0, 1.0);
        
        if self.stops.is_empty() {
            return RgbColor::new(0, 0, 0);
        }
        
        if self.stops.len() == 1 {
            return self.stops[0].1;
        }

        // Find the two stops to interpolate between
        for i in 0..self.stops.len() - 1 {
            let (pos1, color1) = self.stops[i];
            let (pos2, color2) = self.stops[i + 1];
            
            if t >= pos1 && t <= pos2 {
                let local_t = if pos2 == pos1 {
                    0.0
                } else {
                    (t - pos1) / (pos2 - pos1)
                };
                return color1.interpolate(&color2, local_t);
            }
        }

        // If we're before the first stop or after the last stop
        if t <= self.stops[0].0 {
            self.stops[0].1
        } else {
            self.stops[self.stops.len() - 1].1
        }
    }
}

impl Default for ColorGradient {
    fn default() -> Self {
        Self::new()
    }
}

/// Custom interpolator for complex animations
pub trait CustomInterpolator {
    type Value;
    
    fn interpolate(&self, from: &Self::Value, to: &Self::Value, t: f32) -> Self::Value;
}

/// Keyframe animation support
#[derive(Debug, Clone)]
pub struct Keyframe<T> {
    pub time: f32,
    pub value: T,
    pub ease_in: Option<fn(f32) -> f32>,
    pub ease_out: Option<fn(f32) -> f32>,
}

impl<T> Keyframe<T> {
    pub fn new(time: f32, value: T) -> Self {
        Self {
            time: time.clamp(0.0, 1.0),
            value,
            ease_in: None,
            ease_out: None,
        }
    }

    pub fn with_ease_in(mut self, ease_fn: fn(f32) -> f32) -> Self {
        self.ease_in = Some(ease_fn);
        self
    }

    pub fn with_ease_out(mut self, ease_fn: fn(f32) -> f32) -> Self {
        self.ease_out = Some(ease_fn);
        self
    }
}

/// Keyframe animation sequence
#[derive(Debug, Clone)]
pub struct KeyframeSequence<T> {
    pub keyframes: Vec<Keyframe<T>>,
}

impl<T> KeyframeSequence<T> 
where
    T: Interpolatable + Clone,
{
    pub fn new() -> Self {
        Self {
            keyframes: Vec::new(),
        }
    }

    pub fn add_keyframe(mut self, keyframe: Keyframe<T>) -> Self {
        self.keyframes.push(keyframe);
        self.keyframes.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
        self
    }

    /// Evaluate the sequence at time t (0.0 to 1.0)
    pub fn evaluate(&self, t: f32) -> Option<T> {
        let t = t.clamp(0.0, 1.0);
        
        if self.keyframes.is_empty() {
            return None;
        }
        
        if self.keyframes.len() == 1 {
            return Some(self.keyframes[0].value.clone());
        }

        // Find the keyframes to interpolate between
        for i in 0..self.keyframes.len() - 1 {
            let kf1 = &self.keyframes[i];
            let kf2 = &self.keyframes[i + 1];
            
            if t >= kf1.time && t <= kf2.time {
                let local_t = if kf2.time == kf1.time {
                    0.0
                } else {
                    (t - kf1.time) / (kf2.time - kf1.time)
                };
                
                // Apply easing if specified
                let eased_t = if let Some(ease_out) = kf1.ease_out {
                    ease_out(local_t)
                } else if let Some(ease_in) = kf2.ease_in {
                    ease_in(local_t)
                } else {
                    local_t
                };
                
                return Some(kf1.value.interpolate(&kf2.value, eased_t));
            }
        }

        // Before first or after last keyframe
        if t <= self.keyframes[0].time {
            Some(self.keyframes[0].value.clone())
        } else {
            Some(self.keyframes[self.keyframes.len() - 1].value.clone())
        }
    }
}

impl<T> Default for KeyframeSequence<T>
where
    T: Interpolatable + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Multi-value interpolator for complex objects
#[derive(Debug, Clone)]
pub struct MultiValueInterpolator {
    pub values: HashMap<String, f32>,
}

impl MultiValueInterpolator {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn set(mut self, key: String, value: f32) -> Self {
        self.values.insert(key, value);
        self
    }

    pub fn interpolate_with(&self, other: &Self, t: f32) -> Self {
        let mut result = Self::new();
        
        // Interpolate common keys
        for (key, &start_val) in &self.values {
            if let Some(&end_val) = other.values.get(key) {
                result.values.insert(key.clone(), start_val.interpolate(&end_val, t));
            } else {
                result.values.insert(key.clone(), start_val);
            }
        }
        
        // Add keys that only exist in other
        for (key, &end_val) in &other.values {
            if !self.values.contains_key(key) {
                result.values.insert(key.clone(), end_val * t);
            }
        }
        
        result
    }

    pub fn get(&self, key: &str) -> Option<f32> {
        self.values.get(key).copied()
    }
}

impl Default for MultiValueInterpolator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_interpolation() {
        let red = RgbColor::new(255, 0, 0);
        let blue = RgbColor::new(0, 0, 255);
        let purple = red.interpolate(&blue, 0.5);
        
        assert_eq!(purple.r, 127);
        assert_eq!(purple.g, 0);
        assert_eq!(purple.b, 127);
    }

    #[test]
    fn test_hsl_conversion() {
        let red = RgbColor::new(255, 0, 0);
        let hsl = red.to_hsl();
        let back_to_rgb = hsl.to_rgb();
        
        // Allow for small rounding errors
        assert!((red.r as i32 - back_to_rgb.r as i32).abs() <= 1);
        assert!((red.g as i32 - back_to_rgb.g as i32).abs() <= 1);
        assert!((red.b as i32 - back_to_rgb.b as i32).abs() <= 1);
    }

    #[test]
    fn test_point_interpolation() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(10.0, 10.0);
        let mid = start.interpolate(&end, 0.5);
        
        assert_eq!(mid.x, 5.0);
        assert_eq!(mid.y, 5.0);
    }

    #[test]
    fn test_bezier_curve() {
        let curve = BezierCurve::quadratic(
            Point::new(0.0, 0.0),
            Point::new(5.0, 10.0),
            Point::new(10.0, 0.0),
        );
        
        let start = curve.evaluate(0.0);
        let end = curve.evaluate(1.0);
        let mid = curve.evaluate(0.5);
        
        assert_eq!(start.x, 0.0);
        assert_eq!(start.y, 0.0);
        assert_eq!(end.x, 10.0);
        assert_eq!(end.y, 0.0);
        assert_eq!(mid.x, 5.0);
        assert_eq!(mid.y, 5.0); // Peak of the curve
    }

    #[test]
    fn test_color_gradient() {
        let gradient = ColorGradient::new()
            .add_stop(0.0, RgbColor::new(255, 0, 0))
            .add_stop(1.0, RgbColor::new(0, 0, 255));
        
        let start = gradient.evaluate(0.0);
        let end = gradient.evaluate(1.0);
        let mid = gradient.evaluate(0.5);
        
        assert_eq!(start, RgbColor::new(255, 0, 0));
        assert_eq!(end, RgbColor::new(0, 0, 255));
        assert_eq!(mid.r, 127);
        assert_eq!(mid.b, 127);
    }

    #[test]
    fn test_keyframe_sequence() {
        let mut sequence = KeyframeSequence::new()
            .add_keyframe(Keyframe::new(0.0, 0.0))
            .add_keyframe(Keyframe::new(0.5, 10.0))
            .add_keyframe(Keyframe::new(1.0, 0.0));
        
        assert_eq!(sequence.evaluate(0.0), Some(0.0));
        assert_eq!(sequence.evaluate(0.5), Some(10.0));
        assert_eq!(sequence.evaluate(1.0), Some(0.0));
        assert_eq!(sequence.evaluate(0.25), Some(5.0));
    }
}