//! Color utilities and definitions for the theme system

use ratatui::style::Color;
use std::str::FromStr;

/// Predefined color palettes
pub struct ColorPalette;

impl ColorPalette {
    // Goofy brand colors
    pub const GOOFY_ORANGE: Color = Color::Rgb(255, 165, 0);      // Primary brand color
    pub const GOOFY_PURPLE: Color = Color::Rgb(138, 43, 226);     // Secondary brand color
    pub const GOOFY_BLUE: Color = Color::Rgb(130, 130, 255);
    pub const GOOFY_RED: Color = Color::Rgb(255, 130, 130);
    pub const GOOFY_GREEN: Color = Color::Rgb(130, 255, 130);
    pub const GOOFY_YELLOW: Color = Color::Rgb(255, 255, 130);
    
    // Neutral grays
    pub const GRAY_100: Color = Color::Rgb(250, 250, 250);
    pub const GRAY_200: Color = Color::Rgb(229, 229, 229);
    pub const GRAY_300: Color = Color::Rgb(209, 209, 209);
    pub const GRAY_400: Color = Color::Rgb(156, 156, 156);
    pub const GRAY_500: Color = Color::Rgb(115, 115, 115);
    pub const GRAY_600: Color = Color::Rgb(82, 82, 82);
    pub const GRAY_700: Color = Color::Rgb(64, 64, 64);
    pub const GRAY_800: Color = Color::Rgb(38, 38, 38);
    pub const GRAY_900: Color = Color::Rgb(23, 23, 23);
    
    // Status colors
    pub const SUCCESS_GREEN: Color = Color::Rgb(34, 197, 94);
    pub const ERROR_RED: Color = Color::Rgb(239, 68, 68);
    pub const WARNING_YELLOW: Color = Color::Rgb(245, 158, 11);
    pub const INFO_BLUE: Color = Color::Rgb(59, 130, 246);
    
    // Terminal-safe colors (16-color palette)
    pub const TERM_BLACK: Color = Color::Black;
    pub const TERM_RED: Color = Color::Red;
    pub const TERM_GREEN: Color = Color::Green;
    pub const TERM_YELLOW: Color = Color::Yellow;
    pub const TERM_BLUE: Color = Color::Blue;
    pub const TERM_MAGENTA: Color = Color::Magenta;
    pub const TERM_CYAN: Color = Color::Cyan;
    pub const TERM_WHITE: Color = Color::White;
    pub const TERM_BRIGHT_BLACK: Color = Color::DarkGray;
    pub const TERM_BRIGHT_RED: Color = Color::LightRed;
    pub const TERM_BRIGHT_GREEN: Color = Color::LightGreen;
    pub const TERM_BRIGHT_YELLOW: Color = Color::LightYellow;
    pub const TERM_BRIGHT_BLUE: Color = Color::LightBlue;
    pub const TERM_BRIGHT_MAGENTA: Color = Color::LightMagenta;
    pub const TERM_BRIGHT_CYAN: Color = Color::LightCyan;
    pub const TERM_BRIGHT_WHITE: Color = Color::Gray;
}

/// Color conversion utilities
pub mod convert {
    use super::*;
    
    /// Convert hex string to Color
    pub fn hex_to_color(hex: &str) -> Result<Color, String> {
        let hex = hex.trim_start_matches('#');
        
        if hex.len() != 6 {
            return Err("Hex color must be 6 characters long".to_string());
        }
        
        let r = u8::from_str_radix(&hex[0..2], 16)
            .map_err(|_| "Invalid red component")?;
        let g = u8::from_str_radix(&hex[2..4], 16)
            .map_err(|_| "Invalid green component")?;
        let b = u8::from_str_radix(&hex[4..6], 16)
            .map_err(|_| "Invalid blue component")?;
        
        Ok(Color::Rgb(r, g, b))
    }
    
    /// Convert Color to hex string
    pub fn color_to_hex(color: Color) -> String {
        match color {
            Color::Rgb(r, g, b) => format!("#{:02x}{:02x}{:02x}", r, g, b),
            _ => "#000000".to_string(), // Fallback for non-RGB colors
        }
    }
    
    /// Convert RGB values to HSL
    pub fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
        let r = r as f32 / 255.0;
        let g = g as f32 / 255.0;
        let b = b as f32 / 255.0;
        
        let max = r.max(g.max(b));
        let min = r.min(g.min(b));
        let delta = max - min;
        
        // Lightness
        let l = (max + min) / 2.0;
        
        if delta == 0.0 {
            return (0.0, 0.0, l); // Achromatic
        }
        
        // Saturation
        let s = if l < 0.5 {
            delta / (max + min)
        } else {
            delta / (2.0 - max - min)
        };
        
        // Hue
        let h = if max == r {
            (g - b) / delta + if g < b { 6.0 } else { 0.0 }
        } else if max == g {
            (b - r) / delta + 2.0
        } else {
            (r - g) / delta + 4.0
        };
        
        (h * 60.0, s, l)
    }
    
    /// Convert HSL to RGB
    pub fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
        let h = h / 360.0;
        
        if s == 0.0 {
            let gray = (l * 255.0) as u8;
            return (gray, gray, gray); // Achromatic
        }
        
        let hue_to_rgb = |p: f32, q: f32, t: f32| -> f32 {
            let t = if t < 0.0 { t + 1.0 } else if t > 1.0 { t - 1.0 } else { t };
            
            if t < 1.0 / 6.0 {
                p + (q - p) * 6.0 * t
            } else if t < 1.0 / 2.0 {
                q
            } else if t < 2.0 / 3.0 {
                p + (q - p) * (2.0 / 3.0 - t) * 6.0
            } else {
                p
            }
        };
        
        let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
        let p = 2.0 * l - q;
        
        let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
        let g = hue_to_rgb(p, q, h);
        let b = hue_to_rgb(p, q, h - 1.0 / 3.0);
        
        ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
    }
    
    /// Create a color from HSL values
    pub fn hsl_color(h: f32, s: f32, l: f32) -> Color {
        let (r, g, b) = hsl_to_rgb(h, s, l);
        Color::Rgb(r, g, b)
    }
}

/// Color manipulation utilities
pub mod manipulate {
    use super::*;
    
    /// Lighten a color by a percentage (0.0 - 1.0)
    pub fn lighten(color: Color, amount: f32) -> Color {
        match color {
            Color::Rgb(r, g, b) => {
                let (h, s, l) = convert::rgb_to_hsl(r, g, b);
                let new_l = (l + amount).min(1.0);
                let (nr, ng, nb) = convert::hsl_to_rgb(h, s, new_l);
                Color::Rgb(nr, ng, nb)
            }
            _ => color,
        }
    }
    
    /// Darken a color by a percentage (0.0 - 1.0)
    pub fn darken(color: Color, amount: f32) -> Color {
        match color {
            Color::Rgb(r, g, b) => {
                let (h, s, l) = convert::rgb_to_hsl(r, g, b);
                let new_l = (l - amount).max(0.0);
                let (nr, ng, nb) = convert::hsl_to_rgb(h, s, new_l);
                Color::Rgb(nr, ng, nb)
            }
            _ => color,
        }
    }
    
    /// Saturate a color by a percentage (0.0 - 1.0)
    pub fn saturate(color: Color, amount: f32) -> Color {
        match color {
            Color::Rgb(r, g, b) => {
                let (h, s, l) = convert::rgb_to_hsl(r, g, b);
                let new_s = (s + amount).min(1.0);
                let (nr, ng, nb) = convert::hsl_to_rgb(h, new_s, l);
                Color::Rgb(nr, ng, nb)
            }
            _ => color,
        }
    }
    
    /// Desaturate a color by a percentage (0.0 - 1.0)
    pub fn desaturate(color: Color, amount: f32) -> Color {
        match color {
            Color::Rgb(r, g, b) => {
                let (h, s, l) = convert::rgb_to_hsl(r, g, b);
                let new_s = (s - amount).max(0.0);
                let (nr, ng, nb) = convert::hsl_to_rgb(h, new_s, l);
                Color::Rgb(nr, ng, nb)
            }
            _ => color,
        }
    }
    
    /// Shift hue by degrees (-360 to 360)
    pub fn shift_hue(color: Color, degrees: f32) -> Color {
        match color {
            Color::Rgb(r, g, b) => {
                let (h, s, l) = convert::rgb_to_hsl(r, g, b);
                let new_h = (h + degrees) % 360.0;
                let new_h = if new_h < 0.0 { new_h + 360.0 } else { new_h };
                let (nr, ng, nb) = convert::hsl_to_rgb(new_h, s, l);
                Color::Rgb(nr, ng, nb)
            }
            _ => color,
        }
    }
    
    /// Mix two colors with a ratio (0.0 = first color, 1.0 = second color)
    pub fn mix(color1: Color, color2: Color, ratio: f32) -> Color {
        let ratio = ratio.clamp(0.0, 1.0);
        
        match (color1, color2) {
            (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
                let r = (r1 as f32 * (1.0 - ratio) + r2 as f32 * ratio) as u8;
                let g = (g1 as f32 * (1.0 - ratio) + g2 as f32 * ratio) as u8;
                let b = (b1 as f32 * (1.0 - ratio) + b2 as f32 * ratio) as u8;
                Color::Rgb(r, g, b)
            }
            _ => color1, // Fallback
        }
    }
    
    /// Get the complement of a color
    pub fn complement(color: Color) -> Color {
        shift_hue(color, 180.0)
    }
    
    /// Generate a triadic color scheme
    pub fn triadic(color: Color) -> (Color, Color) {
        (shift_hue(color, 120.0), shift_hue(color, 240.0))
    }
    
    /// Generate an analogous color scheme
    pub fn analogous(color: Color) -> (Color, Color) {
        (shift_hue(color, 30.0), shift_hue(color, -30.0))
    }
    
    /// Create a linear gradient between two colors
    /// Returns a vector of colors for smooth transition
    pub fn linear_gradient(start: Color, end: Color, steps: usize) -> Vec<Color> {
        if steps == 0 {
            return vec![];
        }
        if steps == 1 {
            return vec![start];
        }
        
        let mut gradient = Vec::with_capacity(steps);
        
        for i in 0..steps {
            let ratio = i as f32 / (steps - 1) as f32;
            gradient.push(mix(start, end, ratio));
        }
        
        gradient
    }
    
    /// Apply gradient to text characters
    pub fn apply_gradient_to_text(text: &str, start: Color, end: Color) -> Vec<(char, Color)> {
        let chars: Vec<char> = text.chars().collect();
        let gradient = linear_gradient(start, end, chars.len());
        
        chars.into_iter().zip(gradient.into_iter()).collect()
    }
}

/// Color accessibility utilities
pub mod accessibility {
    use super::*;
    
    /// Calculate relative luminance of a color (0.0 - 1.0)
    pub fn luminance(color: Color) -> f32 {
        let (r, g, b) = match color {
            Color::Rgb(r, g, b) => (r, g, b),
            Color::White => (255, 255, 255),
            Color::Black => (0, 0, 0),
            Color::Red => (255, 0, 0),
            Color::Green => (0, 128, 0),
            Color::Yellow => (255, 255, 0),
            Color::Blue => (0, 0, 255),
            Color::Magenta => (255, 0, 255),
            Color::Cyan => (0, 255, 255),
            Color::Gray => (128, 128, 128),
            Color::DarkGray => (169, 169, 169),
            Color::LightRed => (255, 128, 128),
            Color::LightGreen => (144, 238, 144),
            Color::LightYellow => (255, 255, 224),
            Color::LightBlue => (173, 216, 230),
            Color::LightMagenta => (255, 128, 255),
            Color::LightCyan => (224, 255, 255),
            _ => return 0.5, // Fallback for indexed colors
        };
        let r = gamma_correct(r as f32 / 255.0);
        let g = gamma_correct(g as f32 / 255.0);
        let b = gamma_correct(b as f32 / 255.0);
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }
    
    /// Gamma correction for luminance calculation
    fn gamma_correct(value: f32) -> f32 {
        if value <= 0.03928 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }
    
    /// Calculate contrast ratio between two colors (1.0 - 21.0)
    pub fn contrast_ratio(color1: Color, color2: Color) -> f32 {
        let lum1 = luminance(color1);
        let lum2 = luminance(color2);
        
        let lighter = lum1.max(lum2);
        let darker = lum1.min(lum2);
        
        (lighter + 0.05) / (darker + 0.05)
    }
    
    /// Check if color combination meets WCAG AA standard (4.5:1)
    pub fn meets_aa_contrast(foreground: Color, background: Color) -> bool {
        contrast_ratio(foreground, background) >= 4.5
    }
    
    /// Check if color combination meets WCAG AAA standard (7:1)
    pub fn meets_aaa_contrast(foreground: Color, background: Color) -> bool {
        contrast_ratio(foreground, background) >= 7.0
    }
    
    /// Find a contrasting text color for a background
    pub fn contrasting_text(background: Color) -> Color {
        let bg_luminance = luminance(background);
        
        // Use white text on dark backgrounds, black on light
        if bg_luminance < 0.5 {
            Color::White
        } else {
            Color::Black
        }
    }
    
    /// Adjust color to meet minimum contrast ratio
    pub fn adjust_for_contrast(
        foreground: Color,
        background: Color,
        min_ratio: f32,
    ) -> Color {
        let current_ratio = contrast_ratio(foreground, background);
        
        if current_ratio >= min_ratio {
            return foreground;
        }
        
        let bg_luminance = luminance(background);
        
        // If background is dark, lighten the foreground
        // If background is light, darken the foreground
        if bg_luminance < 0.5 {
            // Dark background - lighten foreground
            let mut adjusted = foreground;
            for _ in 0..100 {
                if contrast_ratio(adjusted, background) >= min_ratio {
                    break;
                }
                adjusted = manipulate::lighten(adjusted, 0.05);
            }
            adjusted
        } else {
            // Light background - darken foreground
            let mut adjusted = foreground;
            for _ in 0..100 {
                if contrast_ratio(adjusted, background) >= min_ratio {
                    break;
                }
                adjusted = manipulate::darken(adjusted, 0.05);
            }
            adjusted
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hex_conversion() {
        let color = convert::hex_to_color("#ff0000").unwrap();
        assert_eq!(color, Color::Rgb(255, 0, 0));
        
        let hex = convert::color_to_hex(Color::Rgb(255, 0, 0));
        assert_eq!(hex, "#ff0000");
    }
    
    #[test]
    fn test_color_mixing() {
        let red = Color::Rgb(255, 0, 0);
        let blue = Color::Rgb(0, 0, 255);
        let purple = manipulate::mix(red, blue, 0.5);
        
        if let Color::Rgb(r, g, b) = purple {
            assert_eq!(r, 127);
            assert_eq!(g, 0);
            assert_eq!(b, 127);
        }
    }
    
    #[test]
    fn test_contrast_calculation() {
        let white = Color::White;
        let black = Color::Black;
        
        let ratio = accessibility::contrast_ratio(white, black);
        assert!(ratio > 20.0); // Should be very high contrast
        
        assert!(accessibility::meets_aa_contrast(white, black));
        assert!(accessibility::meets_aaa_contrast(white, black));
    }
    
    #[test]
    fn test_hsl_conversion() {
        let (h, s, l) = convert::rgb_to_hsl(255, 0, 0); // Red
        assert!((h - 0.0).abs() < 1.0); // Hue should be ~0 for red
        assert!((s - 1.0).abs() < 0.1); // Should be fully saturated
        assert!((l - 0.5).abs() < 0.1); // Should be medium lightness
        
        let (r, g, b) = convert::hsl_to_rgb(0.0, 1.0, 0.5);
        assert_eq!(r, 255);
        assert_eq!(g, 0);
        assert_eq!(b, 0);
    }
}