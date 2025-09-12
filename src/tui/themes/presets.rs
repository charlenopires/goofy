//! Theme presets for Goofy TUI
//! 
//! This module provides pre-built themes including the default "Goofy" theme
//! based on the Charmbracelet color palette, as well as classic light and dark themes.

use super::Theme;
use ratatui::style::Color;

/// Create the default Goofy dark theme
/// 
/// This theme is based on the Charmbracelet Crush theme, using the charmtone
/// color palette to provide a sophisticated dark theme with excellent contrast
/// and visual hierarchy.
pub fn goofy_dark() -> Theme {
    Theme {
        name: "goofy_dark".to_string(),
        is_dark: true,
        
        // Primary brand colors - based on Charmbracelet's signature colors
        primary: Color::Rgb(0x8A, 0x67, 0xFF),   // Charple - signature purple
        secondary: Color::Rgb(0xFF, 0xE1, 0x9C), // Dolly - warm yellow
        tertiary: Color::Rgb(0x9A, 0xE4, 0x78),  // Bok - fresh green
        accent: Color::Rgb(0xFF, 0xA5, 0x00),    // Zest - vibrant orange
        
        // Background colors with subtle gradations
        bg_base: Color::Rgb(0x2D, 0x2D, 0x2D),        // Pepper - deep base
        bg_base_lighter: Color::Rgb(0x3A, 0x3A, 0x3A), // BBQ - slightly lighter
        bg_subtle: Color::Rgb(0x4A, 0x4A, 0x4A),       // Charcoal - subtle variation
        bg_overlay: Color::Rgb(0x5A, 0x5A, 0x5A),      // Iron - overlay backgrounds
        
        // Foreground colors for text hierarchy
        fg_base: Color::Rgb(0xD0, 0xD0, 0xD0),      // Ash - primary text
        fg_muted: Color::Rgb(0xA0, 0xA0, 0xA0),     // Squid - secondary text
        fg_half_muted: Color::Rgb(0xB0, 0xB0, 0xB0), // Smoke - intermediate
        fg_subtle: Color::Rgb(0x90, 0x90, 0x90),    // Oyster - subtle text
        fg_selected: Color::Rgb(0xF5, 0xF5, 0xF5),  // Salt - selected text
        
        // Border colors
        border: Color::Rgb(0x4A, 0x4A, 0x4A),        // Charcoal - default borders
        border_focus: Color::Rgb(0x8A, 0x67, 0xFF),  // Charple - focused borders
        
        // Status colors for semantic meaning
        success: Color::Rgb(0x4C, 0xAF, 0x50),  // Guac - success green
        error: Color::Rgb(0xF4, 0x43, 0x36),    // Sriracha - error red
        warning: Color::Rgb(0xFF, 0xA5, 0x00),  // Zest - warning orange
        info: Color::Rgb(0x29, 0xB6, 0xF6),     // Malibu - info blue
        
        // Extended color palette for advanced use cases
        white: Color::Rgb(0xFF, 0xF8, 0xE1),     // Butter - warm white
        blue_light: Color::Rgb(0x81, 0xC7, 0x84), // Sardine - light blue
        blue: Color::Rgb(0x29, 0xB6, 0xF6),      // Malibu - standard blue
        yellow: Color::Rgb(0xFF, 0xEB, 0x3B),    // Mustard - bright yellow
        green: Color::Rgb(0x66, 0xBB, 0x6A),     // Julep - standard green
        green_dark: Color::Rgb(0x4C, 0xAF, 0x50), // Guac - dark green
        green_light: Color::Rgb(0x9A, 0xE4, 0x78), // Bok - light green
        red: Color::Rgb(0xFF, 0x80, 0x74),       // Coral - standard red
        red_dark: Color::Rgb(0xF4, 0x43, 0x36),  // Sriracha - dark red
        red_light: Color::Rgb(0xFF, 0xAB, 0x91), // Salmon - light red
        cherry: Color::Rgb(0xE9, 0x1E, 0x63),    // Cherry - accent red
        
        styles: None, // Built lazily
    }
}

/// Create a light variant of the Goofy theme
pub fn goofy_light() -> Theme {
    Theme {
        name: "goofy_light".to_string(),
        is_dark: false,
        
        // Primary brand colors - adjusted for light theme
        primary: Color::Rgb(0x67, 0x3A, 0xB7),   // Darker purple for contrast
        secondary: Color::Rgb(0xF5, 0x7C, 0x00), // Darker orange
        tertiary: Color::Rgb(0x38, 0x8E, 0x3C),  // Darker green
        accent: Color::Rgb(0xD3, 0x2F, 0x2F),    // Darker red accent
        
        // Light background colors
        bg_base: Color::Rgb(0xFD, 0xFD, 0xFD),        // Very light gray
        bg_base_lighter: Color::Rgb(0xF8, 0xF9, 0xFA), // Slightly darker
        bg_subtle: Color::Rgb(0xF1, 0xF3, 0xF4),       // Subtle background
        bg_overlay: Color::Rgb(0xE8, 0xEA, 0xED),      // Overlay backgrounds
        
        // Dark foreground colors for text
        fg_base: Color::Rgb(0x20, 0x20, 0x20),      // Near black primary text
        fg_muted: Color::Rgb(0x5F, 0x63, 0x68),     // Gray secondary text
        fg_half_muted: Color::Rgb(0x40, 0x40, 0x40), // Intermediate gray
        fg_subtle: Color::Rgb(0x80, 0x86, 0x8B),    // Light gray text
        fg_selected: Color::Rgb(0xFF, 0xFF, 0xFF),  // White selected text
        
        // Light theme borders
        border: Color::Rgb(0xDA, 0xDD, 0xE1),        // Light gray borders
        border_focus: Color::Rgb(0x67, 0x3A, 0xB7),  // Purple focused borders
        
        // Status colors adjusted for light theme
        success: Color::Rgb(0x28, 0x72, 0x31),  // Darker green
        error: Color::Rgb(0xC6, 0x28, 0x28),    // Darker red
        warning: Color::Rgb(0xED, 0x6C, 0x02),  // Darker orange
        info: Color::Rgb(0x01, 0x65, 0xD4),     // Darker blue
        
        // Extended palette for light theme
        white: Color::Rgb(0xFF, 0xFF, 0xFF),     // Pure white
        blue_light: Color::Rgb(0x90, 0xCA, 0xF9), // Light blue
        blue: Color::Rgb(0x21, 0x96, 0xF3),      // Standard blue
        yellow: Color::Rgb(0xFF, 0xC1, 0x07),    // Golden yellow
        green: Color::Rgb(0x46, 0xA3, 0x5B),     // Standard green
        green_dark: Color::Rgb(0x2E, 0x7D, 0x32), // Dark green
        green_light: Color::Rgb(0x81, 0xC7, 0x84), // Light green
        red: Color::Rgb(0xF4, 0x43, 0x36),       // Standard red
        red_dark: Color::Rgb(0xC6, 0x28, 0x28),  // Dark red
        red_light: Color::Rgb(0xFF, 0xCF, 0xD1), // Light red
        cherry: Color::Rgb(0xC2, 0x18, 0x5B),    // Dark cherry
        
        styles: None, // Built lazily
    }
}

/// Create a classic dark theme with traditional terminal colors
pub fn classic_dark() -> Theme {
    Theme {
        name: "classic_dark".to_string(),
        is_dark: true,
        
        // Traditional terminal colors
        primary: Color::Cyan,
        secondary: Color::Yellow,
        tertiary: Color::Green,
        accent: Color::Magenta,
        
        // Classic dark backgrounds
        bg_base: Color::Black,
        bg_base_lighter: Color::DarkGray,
        bg_subtle: Color::DarkGray,
        bg_overlay: Color::DarkGray,
        
        // Traditional text colors
        fg_base: Color::White,
        fg_muted: Color::Gray,
        fg_half_muted: Color::Gray,
        fg_subtle: Color::DarkGray,
        fg_selected: Color::Black,
        
        // Simple borders
        border: Color::DarkGray,
        border_focus: Color::Cyan,
        
        // Traditional status colors
        success: Color::Green,
        error: Color::Red,
        warning: Color::Yellow,
        info: Color::Blue,
        
        // Basic color palette
        white: Color::White,
        blue_light: Color::LightBlue,
        blue: Color::Blue,
        yellow: Color::Yellow,
        green: Color::Green,
        green_dark: Color::Green,
        green_light: Color::LightGreen,
        red: Color::Red,
        red_dark: Color::Red,
        red_light: Color::LightRed,
        cherry: Color::Magenta,
        
        styles: None, // Built lazily
    }
}

/// Create a classic light theme
pub fn classic_light() -> Theme {
    Theme {
        name: "classic_light".to_string(),
        is_dark: false,
        
        // Traditional colors adjusted for light background
        primary: Color::Blue,
        secondary: Color::Rgb(0xB8, 0x86, 0x00), // Dark yellow
        tertiary: Color::Rgb(0x00, 0x80, 0x00),  // Dark green
        accent: Color::Rgb(0x80, 0x00, 0x80),    // Dark magenta
        
        // Light backgrounds
        bg_base: Color::White,
        bg_base_lighter: Color::Gray,
        bg_subtle: Color::Gray,
        bg_overlay: Color::Gray,
        
        // Dark text for contrast
        fg_base: Color::Black,
        fg_muted: Color::DarkGray,
        fg_half_muted: Color::Gray,
        fg_subtle: Color::Gray,
        fg_selected: Color::White,
        
        // Light theme borders
        border: Color::Gray,
        border_focus: Color::Blue,
        
        // Darker status colors for visibility
        success: Color::Rgb(0x00, 0x80, 0x00),  // Dark green
        error: Color::Rgb(0x80, 0x00, 0x00),    // Dark red
        warning: Color::Rgb(0xB8, 0x86, 0x00),  // Dark yellow
        info: Color::Blue,
        
        // Traditional palette
        white: Color::White,
        blue_light: Color::LightBlue,
        blue: Color::Blue,
        yellow: Color::Rgb(0xB8, 0x86, 0x00),
        green: Color::Rgb(0x00, 0x80, 0x00),
        green_dark: Color::Rgb(0x00, 0x60, 0x00),
        green_light: Color::LightGreen,
        red: Color::Rgb(0x80, 0x00, 0x00),
        red_dark: Color::Rgb(0x60, 0x00, 0x00),
        red_light: Color::LightRed,
        cherry: Color::Rgb(0x80, 0x00, 0x80),
        
        styles: None, // Built lazily
    }
}

/// Create a high contrast theme for accessibility
pub fn high_contrast() -> Theme {
    Theme {
        name: "high_contrast".to_string(),
        is_dark: true,
        
        // Maximum contrast colors
        primary: Color::White,
        secondary: Color::Yellow,
        tertiary: Color::Cyan,
        accent: Color::Magenta,
        
        // Pure black/white backgrounds
        bg_base: Color::Black,
        bg_base_lighter: Color::DarkGray,
        bg_subtle: Color::DarkGray,
        bg_overlay: Color::Gray,
        
        // High contrast text
        fg_base: Color::White,
        fg_muted: Color::Gray,
        fg_half_muted: Color::Gray,
        fg_subtle: Color::DarkGray,
        fg_selected: Color::Black,
        
        // High contrast borders
        border: Color::White,
        border_focus: Color::Yellow,
        
        // Bright status colors
        success: Color::LightGreen,
        error: Color::LightRed,
        warning: Color::LightYellow,
        info: Color::LightCyan,
        
        // High visibility palette
        white: Color::White,
        blue_light: Color::LightBlue,
        blue: Color::Blue,
        yellow: Color::LightYellow,
        green: Color::LightGreen,
        green_dark: Color::Green,
        green_light: Color::LightGreen,
        red: Color::LightRed,
        red_dark: Color::Red,
        red_light: Color::LightRed,
        cherry: Color::LightMagenta,
        
        styles: None, // Built lazily
    }
}

/// Create a monochrome theme using only grayscale colors
pub fn monochrome() -> Theme {
    Theme {
        name: "monochrome".to_string(),
        is_dark: true,
        
        // Grayscale brand colors with different intensities
        primary: Color::White,
        secondary: Color::Gray,
        tertiary: Color::Gray,
        accent: Color::DarkGray,
        
        // Grayscale backgrounds
        bg_base: Color::Black,
        bg_base_lighter: Color::Rgb(0x1A, 0x1A, 0x1A),
        bg_subtle: Color::Rgb(0x2A, 0x2A, 0x2A),
        bg_overlay: Color::Rgb(0x3A, 0x3A, 0x3A),
        
        // Monochrome text
        fg_base: Color::White,
        fg_muted: Color::Gray,
        fg_half_muted: Color::Gray,
        fg_subtle: Color::DarkGray,
        fg_selected: Color::Black,
        
        // Grayscale borders
        border: Color::DarkGray,
        border_focus: Color::White,
        
        // Status colors using intensity
        success: Color::White,
        error: Color::Gray,
        warning: Color::Gray,
        info: Color::DarkGray,
        
        // Monochrome palette
        white: Color::White,
        blue_light: Color::Gray,
        blue: Color::Gray,
        yellow: Color::Gray,
        green: Color::Gray,
        green_dark: Color::DarkGray,
        green_light: Color::Gray,
        red: Color::Gray,
        red_dark: Color::DarkGray,
        red_light: Color::Gray,
        cherry: Color::Gray,
        
        styles: None, // Built lazily
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_theme_creation() {
        let theme = goofy_dark();
        assert_eq!(theme.name, "goofy_dark");
        assert!(theme.is_dark);
        assert!(theme.styles.is_none()); // Should be built lazily
    }
    
    #[test]
    fn test_all_presets() {
        let themes = vec![
            goofy_dark(),
            goofy_light(),
            classic_dark(),
            classic_light(),
            high_contrast(),
            monochrome(),
        ];
        
        for theme in themes {
            assert!(!theme.name.is_empty());
            // Verify all themes have valid color assignments
            match theme.primary {
                Color::Rgb(_, _, _) | Color::Black | Color::Red | Color::Green 
                | Color::Yellow | Color::Blue | Color::Magenta | Color::Cyan 
                | Color::White | Color::LightRed | Color::LightGreen 
                | Color::LightYellow | Color::LightBlue | Color::LightMagenta 
                | Color::LightCyan | Color::Gray | Color::DarkGray 
                | Color::Gray | Color::Indexed(_) => {
                    // Valid color
                }
            }
        }
    }
    
    #[test]
    fn test_theme_naming() {
        assert_eq!(goofy_dark().name, "goofy_dark");
        assert_eq!(goofy_light().name, "goofy_light");
        assert_eq!(classic_dark().name, "classic_dark");
        assert_eq!(classic_light().name, "classic_light");
        assert_eq!(high_contrast().name, "high_contrast");
        assert_eq!(monochrome().name, "monochrome");
    }
    
    #[test]
    fn test_theme_darkness() {
        assert!(goofy_dark().is_dark);
        assert!(!goofy_light().is_dark);
        assert!(classic_dark().is_dark);
        assert!(!classic_light().is_dark);
        assert!(high_contrast().is_dark);
        assert!(monochrome().is_dark);
    }
}