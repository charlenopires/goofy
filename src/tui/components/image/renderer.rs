//! Image rendering engine for terminal display
//! 
//! This module provides the core image rendering functionality,
//! converting images to terminal-compatible format using various
//! techniques including Unicode block characters and color mapping.

use super::{ImageConfig, RenderQuality, ColorMode};
use anyhow::Result;
use image::{DynamicImage, GenericImageView, Rgb, Rgba};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
};

/// Image rendering engine
pub struct ImageRenderer {
    config: ImageConfig,
}

impl ImageRenderer {
    /// Create a new image renderer
    pub fn new(config: ImageConfig) -> Self {
        Self { config }
    }
    
    /// Render image to terminal lines
    pub fn render(&self, image: &DynamicImage, area: Rect) -> Result<Vec<Line<'static>>> {
        match self.config.color_mode {
            ColorMode::TrueColor => self.render_truecolor(image, area),
            ColorMode::Palette256 => self.render_palette256(image, area),
            ColorMode::Palette16 => self.render_palette16(image, area),
            ColorMode::Monochrome => self.render_monochrome(image, area),
        }
    }
    
    /// Render with full RGB color support
    fn render_truecolor(&self, image: &DynamicImage, area: Rect) -> Result<Vec<Line<'static>>> {
        let (width, height) = self.calculate_display_size(image, area);
        let resized = self.resize_image(image, width as u32, height as u32);
        
        let mut lines = Vec::new();
        
        // Use half-block characters to double vertical resolution
        for y in (0..height).step_by(2) {
            let mut spans = Vec::new();
            
            for x in 0..width {
                let top_pixel = resized.get_pixel(x as u32, y as u32);
                let bottom_pixel = if y + 1 < height {
                    resized.get_pixel(x as u32, (y + 1) as u32)
                } else {
                    top_pixel
                };
                
                let top_color = rgba_to_color(top_pixel);
                let bottom_color = rgba_to_color(bottom_pixel);
                
                // Use upper half block character (▀) with appropriate colors
                let span = Span::styled(
                    "▀",
                    Style::default()
                        .fg(top_color)
                        .bg(bottom_color),
                );
                
                spans.push(span);
            }
            
            lines.push(Line::from(spans));
        }
        
        Ok(lines)
    }
    
    /// Render with 256-color palette
    fn render_palette256(&self, image: &DynamicImage, area: Rect) -> Result<Vec<Line<'static>>> {
        let (width, height) = self.calculate_display_size(image, area);
        let resized = self.resize_image(image, width as u32, height as u32);
        
        let mut lines = Vec::new();
        
        for y in (0..height).step_by(2) {
            let mut spans = Vec::new();
            
            for x in 0..width {
                let top_pixel = resized.get_pixel(x as u32, y as u32);
                let bottom_pixel = if y + 1 < height {
                    resized.get_pixel(x as u32, (y + 1) as u32)
                } else {
                    top_pixel
                };
                
                let top_color = rgba_to_palette256(top_pixel);
                let bottom_color = rgba_to_palette256(bottom_pixel);
                
                let span = Span::styled(
                    "▀",
                    Style::default()
                        .fg(top_color)
                        .bg(bottom_color),
                );
                
                spans.push(span);
            }
            
            lines.push(Line::from(spans));
        }
        
        Ok(lines)
    }
    
    /// Render with 16-color palette
    fn render_palette16(&self, image: &DynamicImage, area: Rect) -> Result<Vec<Line<'static>>> {
        let (width, height) = self.calculate_display_size(image, area);
        let resized = self.resize_image(image, width as u32, height as u32);
        
        let mut lines = Vec::new();
        
        for y in (0..height).step_by(2) {
            let mut spans = Vec::new();
            
            for x in 0..width {
                let top_pixel = resized.get_pixel(x as u32, y as u32);
                let bottom_pixel = if y + 1 < height {
                    resized.get_pixel(x as u32, (y + 1) as u32)
                } else {
                    top_pixel
                };
                
                let top_color = rgba_to_palette16(top_pixel);
                let bottom_color = rgba_to_palette16(bottom_pixel);
                
                let span = Span::styled(
                    "▀",
                    Style::default()
                        .fg(top_color)
                        .bg(bottom_color),
                );
                
                spans.push(span);
            }
            
            lines.push(Line::from(spans));
        }
        
        Ok(lines)
    }
    
    /// Render as ASCII art (monochrome)
    fn render_monochrome(&self, image: &DynamicImage, area: Rect) -> Result<Vec<Line<'static>>> {
        let (width, height) = self.calculate_display_size(image, area);
        let resized = self.resize_image(image, width as u32, height as u32);
        
        // ASCII characters ordered by density (light to dark)
        const ASCII_CHARS: &[char] = &[
            ' ', '.', '\'', '`', '^', '"', ',', ':', ';', 'I', 'l', '!', 'i', '>', 
            '<', '~', '+', '_', '-', '?', ']', '[', '}', '{', '1', ')', '(', '|', 
            '\\', '/', 't', 'f', 'j', 'r', 'x', 'n', 'u', 'v', 'c', 'z', 'X', 
            'Y', 'U', 'J', 'C', 'L', 'Q', '0', 'O', 'Z', 'm', 'w', 'q', 'p', 
            'd', 'b', 'k', 'h', 'a', 'o', '*', '#', 'M', 'W', '&', '8', '%', 
            'B', '@', '$'
        ];
        
        let mut lines = Vec::new();
        
        for y in 0..height {
            let mut chars = Vec::new();
            
            for x in 0..width {
                let pixel = resized.get_pixel(x as u32, y as u32);
                let brightness = calculate_brightness(pixel);
                
                // Map brightness to ASCII character
                let char_index = ((1.0 - brightness) * (ASCII_CHARS.len() - 1) as f32) as usize;
                let ascii_char = ASCII_CHARS[char_index.min(ASCII_CHARS.len() - 1)];
                
                chars.push(ascii_char);
            }
            
            lines.push(Line::from(chars.into_iter().collect::<String>()));
        }
        
        Ok(lines)
    }
    
    /// Calculate optimal display size while preserving aspect ratio
    fn calculate_display_size(&self, image: &DynamicImage, area: Rect) -> (u16, u16) {
        let img_width = image.width() as f32;
        let img_height = image.height() as f32;
        let img_ratio = img_width / img_height;

        let max_width = self.config.max_width.min(area.width) as f32;
        let max_height = self.config.max_height.min(area.height) as f32;

        if !self.config.preserve_aspect_ratio {
            return (max_width as u16, max_height as u16);
        }

        // Calculate dimensions preserving aspect ratio
        let (display_width, display_height) = if img_ratio > max_width / max_height {
            // Width is the limiting factor
            let width = max_width;
            let height = width / img_ratio;
            (width, height)
        } else {
            // Height is the limiting factor
            let height = max_height;
            let width = height * img_ratio;
            (width, height)
        };

        (
            display_width.min(max_width) as u16,
            display_height.min(max_height) as u16,
        )
    }
    
    /// Resize image using configured quality settings
    fn resize_image(&self, image: &DynamicImage, width: u32, height: u32) -> DynamicImage {
        let filter = match self.config.quality {
            RenderQuality::Fast => image::imageops::FilterType::Nearest,
            RenderQuality::Balanced => image::imageops::FilterType::Triangle,
            RenderQuality::High => image::imageops::FilterType::Lanczos3,
        };
        
        image.resize(width, height, filter)
    }
}

/// Convert RGBA pixel to ratatui Color
fn rgba_to_color(pixel: Rgba<u8>) -> Color {
    Color::Rgb(pixel[0], pixel[1], pixel[2])
}

/// Convert RGBA pixel to nearest 256-color palette entry
fn rgba_to_palette256(pixel: Rgba<u8>) -> Color {
    // Simplified 256-color palette mapping
    // In practice, you'd use a more sophisticated color distance algorithm
    let r = pixel[0] as u16;
    let g = pixel[1] as u16;
    let b = pixel[2] as u16;
    
    // Use the 6x6x6 color cube for RGB colors (indices 16-231)
    let r_index = (r * 5 / 255) as u8;
    let g_index = (g * 5 / 255) as u8;
    let b_index = (b * 5 / 255) as u8;
    
    let color_index = 16 + 36 * r_index + 6 * g_index + b_index;
    Color::Indexed(color_index)
}

/// Convert RGBA pixel to nearest 16-color palette entry
fn rgba_to_palette16(pixel: Rgba<u8>) -> Color {
    let r = pixel[0];
    let g = pixel[1];
    let b = pixel[2];
    
    // Simple color mapping to 16-color palette
    match (r > 127, g > 127, b > 127) {
        (false, false, false) => Color::Black,
        (true, false, false) => Color::Red,
        (false, true, false) => Color::Green,
        (true, true, false) => Color::Yellow,
        (false, false, true) => Color::Blue,
        (true, false, true) => Color::Magenta,
        (false, true, true) => Color::Cyan,
        (true, true, true) => {
            // Distinguish between light and dark grays/white
            let brightness = (r as u16 + g as u16 + b as u16) / 3;
            if brightness > 200 {
                Color::White
            } else if brightness > 160 {
                Color::Gray
            } else {
                Color::Gray
            }
        }
    }
}

/// Calculate brightness of a pixel (0.0 = black, 1.0 = white)
fn calculate_brightness(pixel: Rgba<u8>) -> f32 {
    // Use perceived brightness formula
    let r = pixel[0] as f32 / 255.0;
    let g = pixel[1] as f32 / 255.0;
    let b = pixel[2] as f32 / 255.0;
    
    // Standard luminance formula
    0.299 * r + 0.587 * g + 0.114 * b
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{RgbImage, Rgb};
    
    #[test]
    fn test_brightness_calculation() {
        let black = Rgba([0, 0, 0, 255]);
        let white = Rgba([255, 255, 255, 255]);
        let gray = Rgba([128, 128, 128, 255]);
        
        assert_eq!(calculate_brightness(black), 0.0);
        assert_eq!(calculate_brightness(white), 1.0);
        assert!((calculate_brightness(gray) - 0.5).abs() < 0.01);
    }
    
    #[test]
    fn test_color_conversion() {
        let red_pixel = Rgba([255, 0, 0, 255]);
        let color = rgba_to_color(red_pixel);
        
        if let Color::Rgb(r, g, b) = color {
            assert_eq!(r, 255);
            assert_eq!(g, 0);
            assert_eq!(b, 0);
        } else {
            panic!("Expected RGB color");
        }
    }
    
    #[test]
    fn test_palette16_conversion() {
        let red_pixel = Rgba([255, 0, 0, 255]);
        let green_pixel = Rgba([0, 255, 0, 255]);
        let blue_pixel = Rgba([0, 0, 255, 255]);
        let white_pixel = Rgba([255, 255, 255, 255]);
        let black_pixel = Rgba([0, 0, 0, 255]);
        
        assert_eq!(rgba_to_palette16(red_pixel), Color::Red);
        assert_eq!(rgba_to_palette16(green_pixel), Color::Green);
        assert_eq!(rgba_to_palette16(blue_pixel), Color::Blue);
        assert_eq!(rgba_to_palette16(white_pixel), Color::White);
        assert_eq!(rgba_to_palette16(black_pixel), Color::Black);
    }
    
    #[test]
    fn test_display_size_calculation() {
        let config = ImageConfig::default();
        let renderer = ImageRenderer::new(config);
        
        // Create a test image
        let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(100, 100, Rgb([255, 255, 255])));
        
        let area = Rect::new(0, 0, 80, 24);
        let (width, height) = renderer.calculate_display_size(&img, area);
        
        assert!(width <= 80);
        assert!(height <= 24);
    }
    
    #[test]
    fn test_aspect_ratio_preservation() {
        let mut config = ImageConfig::default();
        config.preserve_aspect_ratio = true;
        config.max_width = 40;
        config.max_height = 20;
        
        let renderer = ImageRenderer::new(config);
        
        // Test with a wide image (3:1 ratio) - wider than the 2:1 area
        let wide_img = DynamicImage::ImageRgb8(RgbImage::from_pixel(300, 100, Rgb([255, 255, 255])));
        let area = Rect::new(0, 0, 40, 20);
        let (width, height) = renderer.calculate_display_size(&wide_img, area);

        // Should be limited by width
        assert_eq!(width, 40);
        assert!(height < 20); // 40/3 ~= 13

        // Test with a tall image (1:3 ratio) - taller than the 2:1 area
        let tall_img = DynamicImage::ImageRgb8(RgbImage::from_pixel(100, 300, Rgb([255, 255, 255])));
        let (width, height) = renderer.calculate_display_size(&tall_img, area);

        // Should be limited by height
        assert!(width < 40); // 20/3 ~= 6
        assert_eq!(height, 20);
    }
}