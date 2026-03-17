//! Image display and rendering for Goofy TUI
//! 
//! This module provides comprehensive image support for the TUI,
//! including loading, resizing, and terminal-based rendering of
//! various image formats including PNG, JPEG, GIF, and SVG.

use anyhow::Result;
use image::{ImageFormat, DynamicImage, ImageReader};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
    Frame,
};
use std::io::Cursor;
use std::path::Path;
use tokio::fs;

pub mod renderer;
pub mod loader;
pub mod formats;

use renderer::ImageRenderer;
use loader::ImageLoader;

/// Image display component for TUI
#[derive(Debug)]
pub struct ImageWidget {
    /// Image content
    image: Option<DynamicImage>,
    
    /// Display configuration
    config: ImageConfig,
    
    /// Current state
    state: ImageState,
    
    /// Loading error if any
    error: Option<String>,
    
    /// Image metadata
    metadata: Option<ImageMetadata>,
}

/// Configuration for image display
#[derive(Debug, Clone)]
pub struct ImageConfig {
    /// Maximum width in terminal columns
    pub max_width: u16,
    
    /// Maximum height in terminal rows
    pub max_height: u16,
    
    /// Whether to maintain aspect ratio
    pub preserve_aspect_ratio: bool,
    
    /// Rendering quality
    pub quality: RenderQuality,
    
    /// Color mode
    pub color_mode: ColorMode,
    
    /// Whether to show image metadata
    pub show_metadata: bool,
    
    /// Border style
    pub border: Option<Borders>,
    
    /// Title display
    pub title: Option<String>,
}

/// Image rendering quality
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderQuality {
    /// Fast rendering, lower quality
    Fast,
    /// Balanced rendering
    Balanced,
    /// High quality rendering
    High,
}

/// Color rendering mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorMode {
    /// Full RGB color (24-bit)
    TrueColor,
    /// 256-color palette
    Palette256,
    /// 16-color palette
    Palette16,
    /// Monochrome (ASCII art style)
    Monochrome,
}

/// Current state of the image widget
#[derive(Debug, Clone, PartialEq)]
pub enum ImageState {
    /// No image loaded
    Empty,
    /// Loading image
    Loading,
    /// Image loaded and ready
    Ready,
    /// Error occurred
    Error,
}

/// Image metadata information
#[derive(Debug, Clone)]
pub struct ImageMetadata {
    /// Original width in pixels
    pub width: u32,
    
    /// Original height in pixels
    pub height: u32,
    
    /// Image format
    pub format: ImageFormat,
    
    /// File size in bytes
    pub file_size: Option<u64>,
    
    /// Color depth
    pub color_depth: u8,
    
    /// Has alpha channel
    pub has_alpha: bool,
    
    /// Additional format-specific metadata
    pub extra_info: std::collections::HashMap<String, String>,
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self {
            max_width: 80,
            max_height: 24,
            preserve_aspect_ratio: true,
            quality: RenderQuality::Balanced,
            color_mode: ColorMode::TrueColor,
            show_metadata: false,
            border: Some(Borders::ALL),
            title: None,
        }
    }
}

impl ImageWidget {
    /// Create a new image widget
    pub fn new() -> Self {
        Self {
            image: None,
            config: ImageConfig::default(),
            state: ImageState::Empty,
            error: None,
            metadata: None,
        }
    }
    
    /// Create with custom configuration
    pub fn with_config(config: ImageConfig) -> Self {
        Self {
            image: None,
            config,
            state: ImageState::Empty,
            error: None,
            metadata: None,
        }
    }
    
    /// Load image from file path
    pub async fn load_from_path<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.state = ImageState::Loading;
        self.error = None;
        
        match ImageLoader::load_from_path(path.as_ref()).await {
            Ok((image, metadata)) => {
                self.image = Some(image);
                self.metadata = Some(metadata);
                self.state = ImageState::Ready;
                Ok(())
            }
            Err(e) => {
                self.error = Some(e.to_string());
                self.state = ImageState::Error;
                Err(e)
            }
        }
    }
    
    /// Load image from URL
    pub async fn load_from_url(&mut self, url: &str) -> Result<()> {
        self.state = ImageState::Loading;
        self.error = None;
        
        match ImageLoader::load_from_url(url).await {
            Ok((image, metadata)) => {
                self.image = Some(image);
                self.metadata = Some(metadata);
                self.state = ImageState::Ready;
                Ok(())
            }
            Err(e) => {
                self.error = Some(e.to_string());
                self.state = ImageState::Error;
                Err(e)
            }
        }
    }
    
    /// Load image from bytes
    pub fn load_from_bytes(&mut self, data: &[u8]) -> Result<()> {
        self.state = ImageState::Loading;
        self.error = None;
        
        match ImageLoader::load_from_bytes(data) {
            Ok((image, metadata)) => {
                self.image = Some(image);
                self.metadata = Some(metadata);
                self.state = ImageState::Ready;
                Ok(())
            }
            Err(e) => {
                self.error = Some(e.to_string());
                self.state = ImageState::Error;
                Err(e)
            }
        }
    }
    
    /// Set image configuration
    pub fn set_config(&mut self, config: ImageConfig) {
        self.config = config;
    }
    
    /// Get current configuration
    pub fn config(&self) -> &ImageConfig {
        &self.config
    }
    
    /// Get current state
    pub fn state(&self) -> ImageState {
        self.state.clone()
    }
    
    /// Get error message if any
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    
    /// Get image metadata if available
    pub fn metadata(&self) -> Option<&ImageMetadata> {
        self.metadata.as_ref()
    }
    
    /// Check if image is loaded
    pub fn is_loaded(&self) -> bool {
        self.state == ImageState::Ready && self.image.is_some()
    }
    
    /// Clear loaded image
    pub fn clear(&mut self) {
        self.image = None;
        self.metadata = None;
        self.error = None;
        self.state = ImageState::Empty;
    }
    
    /// Render the image widget
    pub fn render(&self, area: Rect) -> Result<Vec<Line<'static>>> {
        match self.state {
            ImageState::Empty => Ok(vec![Line::from("No image loaded")]),
            ImageState::Loading => Ok(vec![Line::from("Loading image...")]),
            ImageState::Error => {
                let error_msg = self.error.as_deref().unwrap_or("Unknown error");
                Ok(vec![Line::from(format!("Error: {}", error_msg))])
            }
            ImageState::Ready => {
                if let Some(image) = &self.image {
                    self.render_image(image, area)
                } else {
                    Ok(vec![Line::from("Image data not available")])
                }
            }
        }
    }
    
    /// Render the actual image content
    fn render_image(&self, image: &DynamicImage, area: Rect) -> Result<Vec<Line<'static>>> {
        let renderer = ImageRenderer::new(self.config.clone());
        
        // Calculate available area for image (excluding border if present)
        let image_area = if self.config.border.is_some() {
            Rect {
                x: area.x + 1,
                y: area.y + 1,
                width: area.width.saturating_sub(2),
                height: area.height.saturating_sub(2),
            }
        } else {
            area
        };
        
        // Reserve space for metadata if enabled
        let render_area = if self.config.show_metadata && self.metadata.is_some() {
            Rect {
                x: image_area.x,
                y: image_area.y,
                width: image_area.width,
                height: image_area.height.saturating_sub(3), // 3 lines for metadata
            }
        } else {
            image_area
        };
        
        let mut lines = renderer.render(image, render_area)?;
        
        // Add metadata if enabled
        if self.config.show_metadata {
            if let Some(metadata) = &self.metadata {
                lines.extend(self.render_metadata(metadata, image_area.width));
            }
        }
        
        Ok(lines)
    }
    
    /// Render image metadata
    fn render_metadata(&self, metadata: &ImageMetadata, width: u16) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        
        // Add separator line
        lines.push(Line::from("─".repeat(width as usize)));
        
        // Format metadata
        let size_info = format!("{}×{}", metadata.width, metadata.height);
        let format_info = format!("{:?}", metadata.format);
        let depth_info = format!("{}bit", metadata.color_depth);
        
        let mut info_spans = vec![
            Span::styled("Size: ", Style::default().fg(Color::Gray)),
            Span::raw(size_info),
            Span::raw(" | "),
            Span::styled("Format: ", Style::default().fg(Color::Gray)),
            Span::raw(format_info),
            Span::raw(" | "),
            Span::styled("Depth: ", Style::default().fg(Color::Gray)),
            Span::raw(depth_info),
        ];
        
        if let Some(size) = metadata.file_size {
            info_spans.extend([
                Span::raw(" | "),
                Span::styled("Size: ", Style::default().fg(Color::Gray)),
                Span::raw(format_file_size(size)),
            ]);
        }
        
        lines.push(Line::from(info_spans));
        
        // Add extra info if available
        if !metadata.extra_info.is_empty() {
            let extra: Vec<String> = metadata.extra_info
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect();
            lines.push(Line::from(extra.join(" | ")));
        }
        
        lines
    }
}

impl Default for ImageWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for ImageWidget {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        // Create a block if border is configured
        let block = if let Some(borders) = self.config.border {
            let mut block = Block::default().borders(borders);
            if let Some(title) = &self.config.title {
                block = block.title(title.as_str());
            }
            Some(block)
        } else {
            None
        };
        
        // Calculate the inner area
        let inner_area = if let Some(ref block) = block {
            let outer_area = area;
            block.clone().render(outer_area, buf);
            block.inner(outer_area)
        } else {
            area
        };
        
        // Render the image content
        if let Ok(lines) = ImageWidget::render(&self, inner_area) {
            for (i, line) in lines.iter().enumerate() {
                if i as u16 >= inner_area.height {
                    break;
                }
                
                let y = inner_area.y + i as u16;
                let x = inner_area.x;
                
                // Render the line
                let mut current_x = x;
                for span in &line.spans {
                    if current_x >= inner_area.x + inner_area.width {
                        break;
                    }
                    
                    let content = &span.content;
                    let style = span.style;
                    
                    for (j, ch) in content.chars().enumerate() {
                        let char_x = current_x + j as u16;
                        if char_x >= inner_area.x + inner_area.width {
                            break;
                        }
                        
                        buf.get_mut(char_x, y)
                            .set_char(ch)
                            .set_style(style);
                    }
                    
                    current_x += content.chars().count() as u16;
                }
            }
        }
    }
}

/// Format file size in human-readable format
fn format_file_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    const THRESHOLD: u64 = 1024;
    
    if size < THRESHOLD {
        return format!("{} B", size);
    }
    
    let mut size = size as f64;
    let mut unit_index = 0;
    
    while size >= THRESHOLD as f64 && unit_index < UNITS.len() - 1 {
        size /= THRESHOLD as f64;
        unit_index += 1;
    }
    
    format!("{:.1} {}", size, UNITS[unit_index])
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_image_widget_creation() {
        let widget = ImageWidget::new();
        assert_eq!(widget.state(), ImageState::Empty);
        assert!(!widget.is_loaded());
    }
    
    #[test]
    fn test_image_config() {
        let config = ImageConfig {
            max_width: 100,
            max_height: 50,
            quality: RenderQuality::High,
            ..Default::default()
        };
        
        let widget = ImageWidget::with_config(config.clone());
        assert_eq!(widget.config().max_width, 100);
        assert_eq!(widget.config().max_height, 50);
        assert_eq!(widget.config().quality, RenderQuality::High);
    }
    
    #[test]
    fn test_image_state_transitions() {
        let mut widget = ImageWidget::new();
        
        assert_eq!(widget.state(), ImageState::Empty);
        
        widget.state = ImageState::Loading;
        assert_eq!(widget.state(), ImageState::Loading);
        
        widget.state = ImageState::Ready;
        assert_eq!(widget.state(), ImageState::Ready);
        
        widget.clear();
        assert_eq!(widget.state(), ImageState::Empty);
    }
    
    #[test]
    fn test_file_size_formatting() {
        assert_eq!(format_file_size(500), "500 B");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(1048576), "1.0 MB");
        assert_eq!(format_file_size(1073741824), "1.0 GB");
    }
    
    #[test]
    fn test_color_modes() {
        assert_eq!(ColorMode::TrueColor, ColorMode::TrueColor);
        assert_ne!(ColorMode::TrueColor, ColorMode::Monochrome);
    }
    
    #[test]
    fn test_render_quality() {
        assert_eq!(RenderQuality::Fast, RenderQuality::Fast);
        assert_ne!(RenderQuality::Fast, RenderQuality::High);
    }
}