//! Image format support and utilities
//! 
//! This module provides format-specific handling and utilities
//! for different image formats, including SVG support and
//! animated image handling.

use anyhow::Result;
use image::{DynamicImage, ImageFormat};
use std::collections::HashMap;

/// Information about an image format
#[derive(Debug, Clone)]
pub struct FormatInfo {
    /// Format name
    pub name: &'static str,
    
    /// File extensions
    pub extensions: Vec<&'static str>,
    
    /// MIME type
    pub mime_type: &'static str,
    
    /// Whether the format supports transparency
    pub supports_transparency: bool,
    
    /// Whether the format supports animation
    pub supports_animation: bool,
    
    /// Whether the format uses lossy compression
    pub is_lossy: bool,
    
    /// Maximum color depth (bits per pixel)
    pub max_color_depth: u8,
    
    /// Description
    pub description: &'static str,
}

/// Format registry for all supported image formats
pub struct FormatRegistry {
    formats: HashMap<ImageFormat, FormatInfo>,
}

impl FormatRegistry {
    /// Create a new format registry with all supported formats
    pub fn new() -> Self {
        let mut registry = Self {
            formats: HashMap::new(),
        };
        
        registry.register_standard_formats();
        registry
    }
    
    /// Register all standard image formats
    fn register_standard_formats(&mut self) {
        // PNG - Portable Network Graphics
        self.formats.insert(ImageFormat::Png, FormatInfo {
            name: "PNG",
            extensions: vec!["png"],
            mime_type: "image/png",
            supports_transparency: true,
            supports_animation: false,
            is_lossy: false,
            max_color_depth: 64, // RGBA 16-bit per channel
            description: "Portable Network Graphics - lossless compression with transparency",
        });
        
        // JPEG - Joint Photographic Experts Group
        self.formats.insert(ImageFormat::Jpeg, FormatInfo {
            name: "JPEG",
            extensions: vec!["jpg", "jpeg"],
            mime_type: "image/jpeg",
            supports_transparency: false,
            supports_animation: false,
            is_lossy: true,
            max_color_depth: 24, // RGB 8-bit per channel
            description: "JPEG - lossy compression optimized for photographs",
        });
        
        // GIF - Graphics Interchange Format
        self.formats.insert(ImageFormat::Gif, FormatInfo {
            name: "GIF",
            extensions: vec!["gif"],
            mime_type: "image/gif",
            supports_transparency: true,
            supports_animation: true,
            is_lossy: false,
            max_color_depth: 8, // 256 colors max
            description: "Graphics Interchange Format - supports animation and transparency",
        });
        
        // BMP - Bitmap
        self.formats.insert(ImageFormat::Bmp, FormatInfo {
            name: "BMP",
            extensions: vec!["bmp"],
            mime_type: "image/bmp",
            supports_transparency: true,
            supports_animation: false,
            is_lossy: false,
            max_color_depth: 32, // RGBA 8-bit per channel
            description: "Windows Bitmap - uncompressed raster format",
        });
        
        // ICO - Icon
        self.formats.insert(ImageFormat::Ico, FormatInfo {
            name: "ICO",
            extensions: vec!["ico"],
            mime_type: "image/x-icon",
            supports_transparency: true,
            supports_animation: false,
            is_lossy: false,
            max_color_depth: 32, // RGBA 8-bit per channel
            description: "Windows Icon format - multiple sizes in one file",
        });
        
        // TIFF - Tagged Image File Format
        self.formats.insert(ImageFormat::Tiff, FormatInfo {
            name: "TIFF",
            extensions: vec!["tiff", "tif"],
            mime_type: "image/tiff",
            supports_transparency: true,
            supports_animation: false,
            is_lossy: false, // Can be either, but typically lossless
            max_color_depth: 64, // RGBA 16-bit per channel
            description: "Tagged Image File Format - flexible professional format",
        });
        
        // WebP - Web Picture format
        self.formats.insert(ImageFormat::WebP, FormatInfo {
            name: "WebP",
            extensions: vec!["webp"],
            mime_type: "image/webp",
            supports_transparency: true,
            supports_animation: true,
            is_lossy: false, // Can be either lossy or lossless
            max_color_depth: 32, // RGBA 8-bit per channel
            description: "WebP - modern web format with excellent compression",
        });
        
        // PNM - Portable Anymap
        self.formats.insert(ImageFormat::Pnm, FormatInfo {
            name: "PNM",
            extensions: vec!["pnm", "pbm", "pgm", "ppm"],
            mime_type: "image/x-portable-anymap",
            supports_transparency: false,
            supports_animation: false,
            is_lossy: false,
            max_color_depth: 48, // RGB 16-bit per channel
            description: "Portable Anymap - simple uncompressed format family",
        });
        
        // DDS - DirectDraw Surface
        self.formats.insert(ImageFormat::Dds, FormatInfo {
            name: "DDS",
            extensions: vec!["dds"],
            mime_type: "image/vnd.ms-dds",
            supports_transparency: true,
            supports_animation: false,
            is_lossy: true, // Often uses compressed formats
            max_color_depth: 32, // RGBA 8-bit per channel
            description: "DirectDraw Surface - Microsoft texture format",
        });
        
        // TGA - Truevision TGA
        self.formats.insert(ImageFormat::Tga, FormatInfo {
            name: "TGA",
            extensions: vec!["tga"],
            mime_type: "image/x-targa",
            supports_transparency: true,
            supports_animation: false,
            is_lossy: false,
            max_color_depth: 32, // RGBA 8-bit per channel
            description: "Truevision TGA - simple raster format",
        });
        
        // EXR - OpenEXR
        self.formats.insert(ImageFormat::OpenExr, FormatInfo {
            name: "EXR",
            extensions: vec!["exr"],
            mime_type: "image/x-exr",
            supports_transparency: true,
            supports_animation: false,
            is_lossy: false,
            max_color_depth: 128, // RGBA 32-bit float per channel
            description: "OpenEXR - high dynamic range format for VFX",
        });
        
        // HDR - Radiance HDR
        self.formats.insert(ImageFormat::Hdr, FormatInfo {
            name: "HDR",
            extensions: vec!["hdr"],
            mime_type: "image/vnd.radiance",
            supports_transparency: false,
            supports_animation: false,
            is_lossy: false,
            max_color_depth: 96, // RGB 32-bit float per channel
            description: "Radiance HDR - high dynamic range format",
        });
        
        // Farbfeld
        self.formats.insert(ImageFormat::Farbfeld, FormatInfo {
            name: "Farbfeld",
            extensions: vec!["ff"],
            mime_type: "image/x-farbfeld",
            supports_transparency: true,
            supports_animation: false,
            is_lossy: false,
            max_color_depth: 64, // RGBA 16-bit per channel
            description: "Farbfeld - simple uncompressed format",
        });
        
        // AVIF - AV1 Image File Format
        self.formats.insert(ImageFormat::Avif, FormatInfo {
            name: "AVIF",
            extensions: vec!["avif"],
            mime_type: "image/avif",
            supports_transparency: true,
            supports_animation: true,
            is_lossy: true, // Can be lossless but typically lossy
            max_color_depth: 32, // RGBA 8-bit per channel (can support higher)
            description: "AVIF - modern format based on AV1 video codec",
        });
    }
    
    /// Get format information
    pub fn get_format_info(&self, format: ImageFormat) -> Option<&FormatInfo> {
        self.formats.get(&format)
    }
    
    /// Get all registered formats
    pub fn all_formats(&self) -> Vec<(ImageFormat, &FormatInfo)> {
        self.formats.iter().map(|(fmt, info)| (*fmt, info)).collect()
    }
    
    /// Find format by extension
    pub fn find_by_extension(&self, extension: &str) -> Option<(ImageFormat, &FormatInfo)> {
        let ext_lower = extension.to_lowercase();
        
        for (format, info) in &self.formats {
            if info.extensions.contains(&ext_lower.as_str()) {
                return Some((*format, info));
            }
        }

        None
    }

    /// Find format by MIME type
    pub fn find_by_mime_type(&self, mime_type: &str) -> Option<(ImageFormat, &FormatInfo)> {
        for (format, info) in &self.formats {
            if info.mime_type == mime_type {
                return Some((*format, info));
            }
        }
        
        None
    }
    
    /// Get formats that support transparency
    pub fn transparency_formats(&self) -> Vec<(ImageFormat, &FormatInfo)> {
        self.formats
            .iter()
            .filter(|(_, info)| info.supports_transparency)
            .map(|(fmt, info)| (*fmt, info))
            .collect()
    }
    
    /// Get formats that support animation
    pub fn animation_formats(&self) -> Vec<(ImageFormat, &FormatInfo)> {
        self.formats
            .iter()
            .filter(|(_, info)| info.supports_animation)
            .map(|(fmt, info)| (*fmt, info))
            .collect()
    }
    
    /// Get lossless formats
    pub fn lossless_formats(&self) -> Vec<(ImageFormat, &FormatInfo)> {
        self.formats
            .iter()
            .filter(|(_, info)| !info.is_lossy)
            .map(|(fmt, info)| (*fmt, info))
            .collect()
    }
    
    /// Get lossy formats
    pub fn lossy_formats(&self) -> Vec<(ImageFormat, &FormatInfo)> {
        self.formats
            .iter()
            .filter(|(_, info)| info.is_lossy)
            .map(|(fmt, info)| (*fmt, info))
            .collect()
    }
    
    /// Check if format is suitable for photographs
    pub fn is_photo_format(&self, format: ImageFormat) -> bool {
        matches!(format, 
            ImageFormat::Jpeg | 
            ImageFormat::WebP | 
            ImageFormat::Avif |
            ImageFormat::Hdr |
            ImageFormat::OpenExr
        )
    }
    
    /// Check if format is suitable for graphics/illustrations
    pub fn is_graphics_format(&self, format: ImageFormat) -> bool {
        matches!(format,
            ImageFormat::Png |
            ImageFormat::Gif |
            ImageFormat::Bmp |
            ImageFormat::Tiff
        )
    }
    
    /// Check if format is suitable for web use
    pub fn is_web_format(&self, format: ImageFormat) -> bool {
        matches!(format,
            ImageFormat::Png |
            ImageFormat::Jpeg |
            ImageFormat::Gif |
            ImageFormat::WebP |
            ImageFormat::Avif
        )
    }
    
    /// Get recommended format for a specific use case
    pub fn recommend_format(&self, use_case: UseCase) -> Option<ImageFormat> {
        match use_case {
            UseCase::WebPhoto => Some(ImageFormat::WebP),
            UseCase::WebGraphics => Some(ImageFormat::Png),
            UseCase::WebAnimation => Some(ImageFormat::WebP),
            UseCase::Photography => Some(ImageFormat::Jpeg),
            UseCase::Screenshots => Some(ImageFormat::Png),
            UseCase::Icons => Some(ImageFormat::Ico),
            UseCase::HighDynamicRange => Some(ImageFormat::OpenExr),
            UseCase::Archival => Some(ImageFormat::Tiff),
            UseCase::Simple => Some(ImageFormat::Bmp),
        }
    }
}

/// Use cases for format recommendation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UseCase {
    /// Photos for web use
    WebPhoto,
    /// Graphics/illustrations for web
    WebGraphics,
    /// Animated images for web
    WebAnimation,
    /// General photography
    Photography,
    /// Screenshots and UI captures
    Screenshots,
    /// Application icons
    Icons,
    /// HDR images for professional use
    HighDynamicRange,
    /// Long-term archival storage
    Archival,
    /// Simple, uncompressed storage
    Simple,
}

impl Default for FormatRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// SVG support utilities (requires special handling)
pub struct SvgHandler;

impl SvgHandler {
    /// Check if a file appears to be SVG based on content
    pub fn is_svg_content(data: &[u8]) -> bool {
        // Check for SVG signature
        let content = String::from_utf8_lossy(data);
        let content_lower = content.to_lowercase();
        
        content_lower.contains("<svg") || 
        content_lower.contains("<?xml") && content_lower.contains("svg")
    }
    
    /// Get SVG format information
    pub fn svg_format_info() -> FormatInfo {
        FormatInfo {
            name: "SVG",
            extensions: vec!["svg"],
            mime_type: "image/svg+xml",
            supports_transparency: true,
            supports_animation: true,
            is_lossy: false,
            max_color_depth: 32, // Conceptually unlimited, but 32-bit for practical purposes
            description: "Scalable Vector Graphics - XML-based vector format",
        }
    }
    
    /// Check if SVG is supported for rasterization
    pub fn can_rasterize() -> bool {
        // In a real implementation, you'd check if resvg or similar is available
        cfg!(feature = "svg-support")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_registry_creation() {
        let registry = FormatRegistry::new();
        assert!(!registry.formats.is_empty());
    }
    
    #[test]
    fn test_format_info_retrieval() {
        let registry = FormatRegistry::new();
        
        let png_info = registry.get_format_info(ImageFormat::Png);
        assert!(png_info.is_some());
        
        let info = png_info.unwrap();
        assert_eq!(info.name, "PNG");
        assert!(info.supports_transparency);
        assert!(!info.is_lossy);
    }
    
    #[test]
    fn test_extension_lookup() {
        let registry = FormatRegistry::new();
        
        let (format, _info) = registry.find_by_extension("png").unwrap();
        assert_eq!(format, ImageFormat::Png);
        
        let (format, _info) = registry.find_by_extension("jpg").unwrap();
        assert_eq!(format, ImageFormat::Jpeg);
        
        assert!(registry.find_by_extension("unknown").is_none());
    }
    
    #[test]
    fn test_mime_type_lookup() {
        let registry = FormatRegistry::new();
        
        let (format, _info) = registry.find_by_mime_type("image/png").unwrap();
        assert_eq!(format, ImageFormat::Png);
        
        let (format, _info) = registry.find_by_mime_type("image/jpeg").unwrap();
        assert_eq!(format, ImageFormat::Jpeg);
        
        assert!(registry.find_by_mime_type("unknown/type").is_none());
    }
    
    #[test]
    fn test_format_filtering() {
        let registry = FormatRegistry::new();
        
        let transparency_formats = registry.transparency_formats();
        assert!(!transparency_formats.is_empty());
        
        let animation_formats = registry.animation_formats();
        assert!(!animation_formats.is_empty());
        
        let lossless_formats = registry.lossless_formats();
        assert!(!lossless_formats.is_empty());
        
        let lossy_formats = registry.lossy_formats();
        assert!(!lossy_formats.is_empty());
    }
    
    #[test]
    fn test_format_categories() {
        let registry = FormatRegistry::new();
        
        assert!(registry.is_photo_format(ImageFormat::Jpeg));
        assert!(registry.is_graphics_format(ImageFormat::Png));
        assert!(registry.is_web_format(ImageFormat::WebP));
    }
    
    #[test]
    fn test_format_recommendations() {
        let registry = FormatRegistry::new();
        
        assert_eq!(registry.recommend_format(UseCase::WebPhoto), Some(ImageFormat::WebP));
        assert_eq!(registry.recommend_format(UseCase::Photography), Some(ImageFormat::Jpeg));
        assert_eq!(registry.recommend_format(UseCase::Screenshots), Some(ImageFormat::Png));
        assert_eq!(registry.recommend_format(UseCase::Icons), Some(ImageFormat::Ico));
    }
    
    #[test]
    fn test_svg_detection() {
        let svg_content = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><rect/></svg>";
        assert!(SvgHandler::is_svg_content(svg_content));
        
        let xml_svg = b"<?xml version=\"1.0\"?><svg><circle/></svg>";
        assert!(SvgHandler::is_svg_content(xml_svg));
        
        let not_svg = b"This is not SVG content";
        assert!(!SvgHandler::is_svg_content(not_svg));
    }
    
    #[test]
    fn test_svg_format_info() {
        let info = SvgHandler::svg_format_info();
        assert_eq!(info.name, "SVG");
        assert_eq!(info.mime_type, "image/svg+xml");
        assert!(info.supports_transparency);
        assert!(info.supports_animation);
        assert!(!info.is_lossy);
    }
}