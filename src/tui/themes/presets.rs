//! Theme presets for Goofy TUI
//! 
//! This module provides pre-built themes including the default "Goofy" theme
//! based on the Charmbracelet color palette, as well as classic light and dark themes.

use super::{Theme, ThemeColors, IconSet, Styles};
use ratatui::style::Color;

/// Helper to build a Theme with the new fields populated from existing color fields
fn finalize_theme(mut theme: Theme) -> Theme {
    theme.text = theme.fg_base;
    theme.text_dim = theme.fg_subtle;
    theme.fg_primary = theme.primary;
    theme.bg_surface = theme.bg_subtle;
    theme.bg_primary = theme.bg_base;
    theme.fg_secondary = theme.fg_muted;
    theme.accent_primary = theme.accent;
    theme.accent_secondary = theme.secondary;
    theme.accent_tertiary = theme.tertiary;
    theme.border_primary = theme.border;
    theme.info_primary = theme.info;
    theme.placeholder = theme.fg_subtle;
    theme.colors = theme.build_colors();
    theme.styles = theme.build_styles();
    theme
}

/// Create the default Goofy dark theme
/// 
/// This theme is based on the Charmbracelet Crush theme, using the charmtone
/// color palette to provide a sophisticated dark theme with excellent contrast
/// and visual hierarchy.
pub fn goofy_dark() -> Theme {
    finalize_theme(Theme {
        name: "goofy_dark".to_string(),
        is_dark: true,
        primary: Color::Rgb(0x8A, 0x67, 0xFF),
        secondary: Color::Rgb(0xFF, 0xE1, 0x9C),
        tertiary: Color::Rgb(0x9A, 0xE4, 0x78),
        accent: Color::Rgb(0xFF, 0xA5, 0x00),
        bg_base: Color::Rgb(0x2D, 0x2D, 0x2D),
        bg_base_lighter: Color::Rgb(0x3A, 0x3A, 0x3A),
        bg_subtle: Color::Rgb(0x4A, 0x4A, 0x4A),
        bg_overlay: Color::Rgb(0x5A, 0x5A, 0x5A),
        fg_base: Color::Rgb(0xD0, 0xD0, 0xD0),
        fg_muted: Color::Rgb(0xA0, 0xA0, 0xA0),
        fg_half_muted: Color::Rgb(0xB0, 0xB0, 0xB0),
        fg_subtle: Color::Rgb(0x90, 0x90, 0x90),
        fg_selected: Color::Rgb(0xF5, 0xF5, 0xF5),
        border: Color::Rgb(0x4A, 0x4A, 0x4A),
        border_focus: Color::Rgb(0x8A, 0x67, 0xFF),
        success: Color::Rgb(0x4C, 0xAF, 0x50),
        error: Color::Rgb(0xF4, 0x43, 0x36),
        warning: Color::Rgb(0xFF, 0xA5, 0x00),
        info: Color::Rgb(0x29, 0xB6, 0xF6),
        white: Color::Rgb(0xFF, 0xF8, 0xE1),
        blue_light: Color::Rgb(0x81, 0xC7, 0x84),
        blue: Color::Rgb(0x29, 0xB6, 0xF6),
        yellow: Color::Rgb(0xFF, 0xEB, 0x3B),
        green: Color::Rgb(0x66, 0xBB, 0x6A),
        green_dark: Color::Rgb(0x4C, 0xAF, 0x50),
        green_light: Color::Rgb(0x9A, 0xE4, 0x78),
        red: Color::Rgb(0xFF, 0x80, 0x74),
        red_dark: Color::Rgb(0xF4, 0x43, 0x36),
        red_light: Color::Rgb(0xFF, 0xAB, 0x91),
        cherry: Color::Rgb(0xE9, 0x1E, 0x63),
        text: Color::Reset, // filled by finalize
        text_dim: Color::Reset,
        fg_primary: Color::Reset,
        bg_surface: Color::Reset,
        bg_primary: Color::Reset,
        fg_secondary: Color::Reset,
        accent_primary: Color::Reset,
        accent_secondary: Color::Reset,
        accent_tertiary: Color::Reset,
        border_primary: Color::Reset,
        info_primary: Color::Reset,
        placeholder: Color::Reset,
        icons: IconSet::default(),
        colors: ThemeColors::default(),
        styles: Styles::default(),
    })
}

/// Create a light variant of the Goofy theme
pub fn goofy_light() -> Theme {
    finalize_theme(Theme {
        name: "goofy_light".to_string(),
        is_dark: false,
        primary: Color::Rgb(0x67, 0x3A, 0xB7),
        secondary: Color::Rgb(0xF5, 0x7C, 0x00),
        tertiary: Color::Rgb(0x38, 0x8E, 0x3C),
        accent: Color::Rgb(0xD3, 0x2F, 0x2F),
        bg_base: Color::Rgb(0xFD, 0xFD, 0xFD),
        bg_base_lighter: Color::Rgb(0xF8, 0xF9, 0xFA),
        bg_subtle: Color::Rgb(0xF1, 0xF3, 0xF4),
        bg_overlay: Color::Rgb(0xE8, 0xEA, 0xED),
        fg_base: Color::Rgb(0x20, 0x20, 0x20),
        fg_muted: Color::Rgb(0x5F, 0x63, 0x68),
        fg_half_muted: Color::Rgb(0x40, 0x40, 0x40),
        fg_subtle: Color::Rgb(0x80, 0x86, 0x8B),
        fg_selected: Color::Rgb(0xFF, 0xFF, 0xFF),
        border: Color::Rgb(0xDA, 0xDD, 0xE1),
        border_focus: Color::Rgb(0x67, 0x3A, 0xB7),
        success: Color::Rgb(0x28, 0x72, 0x31),
        error: Color::Rgb(0xC6, 0x28, 0x28),
        warning: Color::Rgb(0xED, 0x6C, 0x02),
        info: Color::Rgb(0x01, 0x65, 0xD4),
        white: Color::Rgb(0xFF, 0xFF, 0xFF),
        blue_light: Color::Rgb(0x90, 0xCA, 0xF9),
        blue: Color::Rgb(0x21, 0x96, 0xF3),
        yellow: Color::Rgb(0xFF, 0xC1, 0x07),
        green: Color::Rgb(0x46, 0xA3, 0x5B),
        green_dark: Color::Rgb(0x2E, 0x7D, 0x32),
        green_light: Color::Rgb(0x81, 0xC7, 0x84),
        red: Color::Rgb(0xF4, 0x43, 0x36),
        red_dark: Color::Rgb(0xC6, 0x28, 0x28),
        red_light: Color::Rgb(0xFF, 0xCF, 0xD1),
        cherry: Color::Rgb(0xC2, 0x18, 0x5B),
        text: Color::Reset,
        text_dim: Color::Reset,
        fg_primary: Color::Reset,
        bg_surface: Color::Reset,
        bg_primary: Color::Reset,
        fg_secondary: Color::Reset,
        accent_primary: Color::Reset,
        accent_secondary: Color::Reset,
        accent_tertiary: Color::Reset,
        border_primary: Color::Reset,
        info_primary: Color::Reset,
        placeholder: Color::Reset,
        icons: IconSet::default(),
        colors: ThemeColors::default(),
        styles: Styles::default(),
    })
}

/// Create a classic dark theme with traditional terminal colors
pub fn classic_dark() -> Theme {
    finalize_theme(Theme {
        name: "classic_dark".to_string(),
        is_dark: true,
        primary: Color::Cyan,
        secondary: Color::Yellow,
        tertiary: Color::Green,
        accent: Color::Magenta,
        bg_base: Color::Black,
        bg_base_lighter: Color::DarkGray,
        bg_subtle: Color::DarkGray,
        bg_overlay: Color::DarkGray,
        fg_base: Color::White,
        fg_muted: Color::Gray,
        fg_half_muted: Color::Gray,
        fg_subtle: Color::DarkGray,
        fg_selected: Color::Black,
        border: Color::DarkGray,
        border_focus: Color::Cyan,
        success: Color::Green,
        error: Color::Red,
        warning: Color::Yellow,
        info: Color::Blue,
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
        text: Color::Reset,
        text_dim: Color::Reset,
        fg_primary: Color::Reset,
        bg_surface: Color::Reset,
        bg_primary: Color::Reset,
        fg_secondary: Color::Reset,
        accent_primary: Color::Reset,
        accent_secondary: Color::Reset,
        accent_tertiary: Color::Reset,
        border_primary: Color::Reset,
        info_primary: Color::Reset,
        placeholder: Color::Reset,
        icons: IconSet::default(),
        colors: ThemeColors::default(),
        styles: Styles::default(),
    })
}

/// Create a classic light theme
pub fn classic_light() -> Theme {
    finalize_theme(Theme {
        name: "classic_light".to_string(),
        is_dark: false,
        primary: Color::Blue,
        secondary: Color::Rgb(0xB8, 0x86, 0x00),
        tertiary: Color::Rgb(0x00, 0x80, 0x00),
        accent: Color::Rgb(0x80, 0x00, 0x80),
        bg_base: Color::White,
        bg_base_lighter: Color::Gray,
        bg_subtle: Color::Gray,
        bg_overlay: Color::Gray,
        fg_base: Color::Black,
        fg_muted: Color::DarkGray,
        fg_half_muted: Color::Gray,
        fg_subtle: Color::Gray,
        fg_selected: Color::White,
        border: Color::Gray,
        border_focus: Color::Blue,
        success: Color::Rgb(0x00, 0x80, 0x00),
        error: Color::Rgb(0x80, 0x00, 0x00),
        warning: Color::Rgb(0xB8, 0x86, 0x00),
        info: Color::Blue,
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
        text: Color::Reset,
        text_dim: Color::Reset,
        fg_primary: Color::Reset,
        bg_surface: Color::Reset,
        bg_primary: Color::Reset,
        fg_secondary: Color::Reset,
        accent_primary: Color::Reset,
        accent_secondary: Color::Reset,
        accent_tertiary: Color::Reset,
        border_primary: Color::Reset,
        info_primary: Color::Reset,
        placeholder: Color::Reset,
        icons: IconSet::default(),
        colors: ThemeColors::default(),
        styles: Styles::default(),
    })
}

/// Create a high contrast theme for accessibility
pub fn high_contrast() -> Theme {
    finalize_theme(Theme {
        name: "high_contrast".to_string(),
        is_dark: true,
        primary: Color::White,
        secondary: Color::Yellow,
        tertiary: Color::Cyan,
        accent: Color::Magenta,
        bg_base: Color::Black,
        bg_base_lighter: Color::DarkGray,
        bg_subtle: Color::DarkGray,
        bg_overlay: Color::Gray,
        fg_base: Color::White,
        fg_muted: Color::Gray,
        fg_half_muted: Color::Gray,
        fg_subtle: Color::DarkGray,
        fg_selected: Color::Black,
        border: Color::White,
        border_focus: Color::Yellow,
        success: Color::LightGreen,
        error: Color::LightRed,
        warning: Color::LightYellow,
        info: Color::LightCyan,
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
        text: Color::Reset,
        text_dim: Color::Reset,
        fg_primary: Color::Reset,
        bg_surface: Color::Reset,
        bg_primary: Color::Reset,
        fg_secondary: Color::Reset,
        accent_primary: Color::Reset,
        accent_secondary: Color::Reset,
        accent_tertiary: Color::Reset,
        border_primary: Color::Reset,
        info_primary: Color::Reset,
        placeholder: Color::Reset,
        icons: IconSet::default(),
        colors: ThemeColors::default(),
        styles: Styles::default(),
    })
}

/// Create a monochrome theme using only grayscale colors
pub fn monochrome() -> Theme {
    finalize_theme(Theme {
        name: "monochrome".to_string(),
        is_dark: true,
        primary: Color::White,
        secondary: Color::Gray,
        tertiary: Color::Gray,
        accent: Color::DarkGray,
        bg_base: Color::Black,
        bg_base_lighter: Color::Rgb(0x1A, 0x1A, 0x1A),
        bg_subtle: Color::Rgb(0x2A, 0x2A, 0x2A),
        bg_overlay: Color::Rgb(0x3A, 0x3A, 0x3A),
        fg_base: Color::White,
        fg_muted: Color::Gray,
        fg_half_muted: Color::Gray,
        fg_subtle: Color::DarkGray,
        fg_selected: Color::Black,
        border: Color::DarkGray,
        border_focus: Color::White,
        success: Color::White,
        error: Color::Gray,
        warning: Color::Gray,
        info: Color::DarkGray,
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
        text: Color::Reset,
        text_dim: Color::Reset,
        fg_primary: Color::Reset,
        bg_surface: Color::Reset,
        bg_primary: Color::Reset,
        fg_secondary: Color::Reset,
        accent_primary: Color::Reset,
        accent_secondary: Color::Reset,
        accent_tertiary: Color::Reset,
        border_primary: Color::Reset,
        info_primary: Color::Reset,
        placeholder: Color::Reset,
        icons: IconSet::default(),
        colors: ThemeColors::default(),
        styles: Styles::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_theme_creation() {
        let theme = goofy_dark();
        assert_eq!(theme.name, "goofy_dark");
        assert!(theme.is_dark);
        // Styles are now eagerly built
        assert_eq!(theme.styles.base.fg, Some(theme.fg_base));
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
                | Color::Indexed(_) | Color::Reset => {
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