//! Markdown styling system
//! 
//! This module provides comprehensive styling for markdown elements,
//! integrating with the Goofy theme system for consistent appearance.

use ratatui::style::{Color, Style, Modifier};
use crate::tui::themes::Theme;

/// Complete set of styles for markdown rendering
#[derive(Debug, Clone)]
pub struct MarkdownStyles {
    /// Base text style
    pub text: Style,
    
    /// Headings
    pub heading_1: Style,
    pub heading_2: Style,
    pub heading_3: Style,
    pub heading_4: Style,
    pub heading_5: Style,
    pub heading_6: Style,
    
    /// Text formatting
    pub emphasis: Style,
    pub strong: Style,
    pub strikethrough: Style,
    
    /// Code elements
    pub inline_code: Style,
    pub code_block: Style,
    pub code_language: Style,
    
    /// Lists
    pub list_marker: Style,
    pub task_marker: Style,
    
    /// Quotes
    pub quote_marker: Style,
    pub quote_text: Style,
    
    /// Links and images
    pub link: Style,
    pub link_text: Style,
    pub image: Style,
    
    /// Tables
    pub table_header: Style,
    pub table_cell: Style,
    pub table_separator: Style,
    
    /// Special elements
    pub rule: Style,
    pub footnote_reference: Style,
    pub footnote_definition: Style,
    
    /// Backgrounds and borders
    pub document_background: Color,
    pub code_background: Color,
    pub quote_background: Color,
}

impl MarkdownStyles {
    /// Create markdown styles from a theme
    pub fn from_theme(theme: &Theme) -> Self {
        Self {
            text: Style::default()
                .fg(theme.fg_primary),
            
            heading_1: Style::default()
                .fg(theme.accent_primary)
                .bg(theme.accent_secondary)
                .add_modifier(Modifier::BOLD),
            
            heading_2: Style::default()
                .fg(theme.accent_primary)
                .add_modifier(Modifier::BOLD),
            
            heading_3: Style::default()
                .fg(theme.accent_secondary)
                .add_modifier(Modifier::BOLD),
            
            heading_4: Style::default()
                .fg(theme.accent_tertiary)
                .add_modifier(Modifier::BOLD),
            
            heading_5: Style::default()
                .fg(theme.fg_secondary)
                .add_modifier(Modifier::BOLD),
            
            heading_6: Style::default()
                .fg(theme.fg_muted)
                .add_modifier(Modifier::BOLD),
            
            emphasis: Style::default()
                .fg(theme.fg_primary)
                .add_modifier(Modifier::ITALIC),
            
            strong: Style::default()
                .fg(theme.fg_primary)
                .add_modifier(Modifier::BOLD),
            
            strikethrough: Style::default()
                .fg(theme.fg_muted)
                .add_modifier(Modifier::CROSSED_OUT),
            
            inline_code: Style::default()
                .fg(theme.accent_tertiary)
                .bg(theme.bg_surface),
            
            code_block: Style::default()
                .fg(theme.fg_primary)
                .bg(theme.bg_surface),
            
            code_language: Style::default()
                .fg(theme.fg_muted)
                .add_modifier(Modifier::ITALIC),
            
            list_marker: Style::default()
                .fg(theme.accent_primary),
            
            task_marker: Style::default()
                .fg(theme.accent_secondary),
            
            quote_marker: Style::default()
                .fg(theme.border_primary),
            
            quote_text: Style::default()
                .fg(theme.fg_muted)
                .add_modifier(Modifier::ITALIC),
            
            link: Style::default()
                .fg(theme.info_primary)
                .add_modifier(Modifier::UNDERLINED),
            
            link_text: Style::default()
                .fg(theme.info_primary)
                .add_modifier(Modifier::BOLD),
            
            image: Style::default()
                .fg(theme.accent_tertiary)
                .add_modifier(Modifier::UNDERLINED),
            
            table_header: Style::default()
                .fg(theme.fg_primary)
                .bg(theme.bg_surface)
                .add_modifier(Modifier::BOLD),
            
            table_cell: Style::default()
                .fg(theme.fg_primary),
            
            table_separator: Style::default()
                .fg(theme.border_primary),
            
            rule: Style::default()
                .fg(theme.border_primary),
            
            footnote_reference: Style::default()
                .fg(theme.info_primary)
                .add_modifier(Modifier::ITALIC),
            
            footnote_definition: Style::default()
                .fg(theme.info_primary)
                .add_modifier(Modifier::BOLD),
            
            document_background: theme.bg_primary,
            code_background: theme.bg_surface,
            quote_background: theme.bg_surface,
        }
    }
    
    /// Create default styles (fallback when no theme is available)
    pub fn default() -> Self {
        Self {
            text: Style::default()
                .fg(Color::White),
            
            heading_1: Style::default()
                .fg(Color::Yellow)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            
            heading_2: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            
            heading_3: Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            
            heading_4: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            
            heading_5: Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
            
            heading_6: Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::BOLD),
            
            emphasis: Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::ITALIC),
            
            strong: Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            
            strikethrough: Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::CROSSED_OUT),
            
            inline_code: Style::default()
                .fg(Color::Red)
                .bg(Color::DarkGray),
            
            code_block: Style::default()
                .fg(Color::White)
                .bg(Color::DarkGray),
            
            code_language: Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::ITALIC),
            
            list_marker: Style::default()
                .fg(Color::Yellow),
            
            task_marker: Style::default()
                .fg(Color::Green),
            
            quote_marker: Style::default()
                .fg(Color::Gray),
            
            quote_text: Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::ITALIC),
            
            link: Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::UNDERLINED),
            
            link_text: Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            
            image: Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::UNDERLINED),
            
            table_header: Style::default()
                .fg(Color::White)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
            
            table_cell: Style::default()
                .fg(Color::White),
            
            table_separator: Style::default()
                .fg(Color::Gray),
            
            rule: Style::default()
                .fg(Color::Gray),
            
            footnote_reference: Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::ITALIC),
            
            footnote_definition: Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            
            document_background: Color::Black,
            code_background: Color::DarkGray,
            quote_background: Color::DarkGray,
        }
    }
    
    /// Get style for specific text formatting
    pub fn get_text_style(&self, bold: bool, italic: bool, strikethrough: bool) -> Style {
        let mut style = self.text;
        
        if bold {
            style = style.add_modifier(Modifier::BOLD);
        }
        
        if italic {
            style = style.add_modifier(Modifier::ITALIC);
        }
        
        if strikethrough {
            style = style.add_modifier(Modifier::CROSSED_OUT);
        }
        
        style
    }
    
    /// Get style for heading level
    pub fn get_heading_style(&self, level: u8) -> Style {
        match level {
            1 => self.heading_1,
            2 => self.heading_2,
            3 => self.heading_3,
            4 => self.heading_4,
            5 => self.heading_5,
            6 => self.heading_6,
            _ => self.text,
        }
    }
    
    /// Create high contrast version of styles
    pub fn high_contrast(&self) -> Self {
        Self {
            text: Style::default()
                .fg(Color::White),
            
            heading_1: Style::default()
                .fg(Color::White)
                .bg(Color::Black)
                .add_modifier(Modifier::BOLD),
            
            heading_2: Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            
            heading_3: Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::BOLD),
            
            heading_4: Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::BOLD),
            
            heading_5: Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::BOLD),
            
            heading_6: Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::BOLD),
            
            emphasis: Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::ITALIC),
            
            strong: Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            
            strikethrough: Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::CROSSED_OUT),
            
            inline_code: Style::default()
                .fg(Color::Black)
                .bg(Color::White),
            
            code_block: Style::default()
                .fg(Color::White)
                .bg(Color::Black),
            
            code_language: Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::ITALIC),
            
            list_marker: Style::default()
                .fg(Color::White),
            
            task_marker: Style::default()
                .fg(Color::White),
            
            quote_marker: Style::default()
                .fg(Color::Gray),
            
            quote_text: Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::ITALIC),
            
            link: Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::UNDERLINED),
            
            link_text: Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            
            image: Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::UNDERLINED),
            
            table_header: Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD),
            
            table_cell: Style::default()
                .fg(Color::White),
            
            table_separator: Style::default()
                .fg(Color::Gray),
            
            rule: Style::default()
                .fg(Color::Gray),
            
            footnote_reference: Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::ITALIC),
            
            footnote_definition: Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            
            document_background: Color::Black,
            code_background: Color::Black,
            quote_background: Color::Black,
        }
    }
    
    /// Create monochrome version of styles
    pub fn monochrome(&self) -> Self {
        Self {
            text: Style::default()
                .fg(Color::White),
            
            heading_1: Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            
            heading_2: Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            
            heading_3: Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            
            heading_4: Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::BOLD),
            
            heading_5: Style::default()
                .fg(Color::Gray),
            
            heading_6: Style::default()
                .fg(Color::Gray),
            
            emphasis: Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::ITALIC),
            
            strong: Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            
            strikethrough: Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::CROSSED_OUT),
            
            inline_code: Style::default()
                .fg(Color::Gray),
            
            code_block: Style::default()
                .fg(Color::Gray),
            
            code_language: Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::ITALIC),
            
            list_marker: Style::default()
                .fg(Color::White),
            
            task_marker: Style::default()
                .fg(Color::Gray),
            
            quote_marker: Style::default()
                .fg(Color::Gray),
            
            quote_text: Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::ITALIC),
            
            link: Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::UNDERLINED),
            
            link_text: Style::default()
                .fg(Color::White),
            
            image: Style::default()
                .fg(Color::Gray),
            
            table_header: Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            
            table_cell: Style::default()
                .fg(Color::White),
            
            table_separator: Style::default()
                .fg(Color::Gray),
            
            rule: Style::default()
                .fg(Color::Gray),
            
            footnote_reference: Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::ITALIC),
            
            footnote_definition: Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::BOLD),
            
            document_background: Color::Black,
            code_background: Color::Black,
            quote_background: Color::Black,
        }
    }
}

impl Default for MarkdownStyles {
    fn default() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_styles() {
        let styles = MarkdownStyles::default();
        assert_eq!(styles.text.fg, Some(Color::White));
        assert_eq!(styles.document_background, Color::Black);
    }
    
    #[test]
    fn test_heading_styles() {
        let styles = MarkdownStyles::default();
        
        for level in 1..=6 {
            let style = styles.get_heading_style(level);
            assert!(style.add_modifier.contains(Modifier::BOLD));
        }
    }
    
    #[test]
    fn test_text_formatting() {
        let styles = MarkdownStyles::default();
        
        let bold_style = styles.get_text_style(true, false, false);
        assert!(bold_style.add_modifier.contains(Modifier::BOLD));
        
        let italic_style = styles.get_text_style(false, true, false);
        assert!(italic_style.add_modifier.contains(Modifier::ITALIC));
        
        let strikethrough_style = styles.get_text_style(false, false, true);
        assert!(strikethrough_style.add_modifier.contains(Modifier::CROSSED_OUT));
        
        let combined_style = styles.get_text_style(true, true, false);
        assert!(combined_style.add_modifier.contains(Modifier::BOLD));
        assert!(combined_style.add_modifier.contains(Modifier::ITALIC));
    }
    
    #[test]
    fn test_high_contrast_styles() {
        let styles = MarkdownStyles::default();
        let high_contrast = styles.high_contrast();
        
        assert_eq!(high_contrast.text.fg, Some(Color::White));
        assert_eq!(high_contrast.document_background, Color::Black);
    }
    
    #[test]
    fn test_monochrome_styles() {
        let styles = MarkdownStyles::default();
        let monochrome = styles.monochrome();
        
        // All colors should be grayscale
        assert!(matches!(monochrome.text.fg, Some(Color::White) | Some(Color::Gray) | Some(Color::Gray) | Some(Color::DarkGray) | Some(Color::Black)));
    }
}