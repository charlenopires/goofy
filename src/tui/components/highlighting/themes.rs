//! Syntax highlighting themes
//! 
//! This module provides predefined syntax highlighting themes that integrate
//! with the Goofy theme system, ensuring consistent visual appearance.

use ratatui::style::Color;
use std::collections::HashMap;

/// A syntax highlighting theme definition
#[derive(Debug, Clone)]
pub struct HighlightTheme {
    /// Theme name
    pub name: String,
    
    /// Whether this is a dark theme
    pub is_dark: bool,
    
    /// Color palette for syntax elements
    pub colors: HighlightColors,
}

/// Color definitions for syntax highlighting
#[derive(Debug, Clone)]
pub struct HighlightColors {
    /// Background color for code blocks
    pub background: Color,
    
    /// Default text color
    pub text: Color,
    
    /// Line numbers
    pub line_number: Color,
    pub line_number_active: Color,
    
    /// Comments
    pub comment: Color,
    pub comment_doc: Color,
    
    /// Keywords
    pub keyword: Color,
    pub keyword_control: Color,
    pub keyword_type: Color,
    
    /// Literals
    pub string: Color,
    pub string_escape: Color,
    pub number: Color,
    pub boolean: Color,
    pub null: Color,
    
    /// Identifiers
    pub function: Color,
    pub function_builtin: Color,
    pub variable: Color,
    pub variable_builtin: Color,
    pub constant: Color,
    pub parameter: Color,
    
    /// Types
    pub type_name: Color,
    pub type_builtin: Color,
    pub type_parameter: Color,
    
    /// Operators and punctuation
    pub operator: Color,
    pub punctuation: Color,
    pub delimiter: Color,
    
    /// Errors and warnings
    pub error: Color,
    pub warning: Color,
    
    /// Special tokens
    pub tag: Color,
    pub attribute: Color,
    pub property: Color,
    pub label: Color,
    
    /// Diff highlighting
    pub diff_added: Color,
    pub diff_removed: Color,
    pub diff_changed: Color,
    
    /// Markup (Markdown, HTML, etc.)
    pub markup_heading: Color,
    pub markup_bold: Color,
    pub markup_italic: Color,
    pub markup_link: Color,
    pub markup_code: Color,
}

/// Collection of predefined highlighting themes
pub struct ThemeCollection {
    themes: HashMap<String, HighlightTheme>,
}

impl ThemeCollection {
    /// Create a new theme collection with default themes
    pub fn new() -> Self {
        let mut collection = Self {
            themes: HashMap::new(),
        };
        
        collection.add_theme(goofy_dark_highlight_theme());
        collection.add_theme(goofy_light_highlight_theme());
        collection.add_theme(classic_dark_highlight_theme());
        collection.add_theme(classic_light_highlight_theme());
        collection.add_theme(high_contrast_highlight_theme());
        collection.add_theme(monochrome_highlight_theme());
        
        collection
    }
    
    /// Add a theme to the collection
    pub fn add_theme(&mut self, theme: HighlightTheme) {
        self.themes.insert(theme.name.clone(), theme);
    }
    
    /// Get a theme by name
    pub fn get_theme(&self, name: &str) -> Option<&HighlightTheme> {
        self.themes.get(name)
    }
    
    /// Get all theme names
    pub fn theme_names(&self) -> Vec<String> {
        self.themes.keys().cloned().collect()
    }
    
    /// Get themes filtered by dark/light preference
    pub fn themes_by_type(&self, is_dark: bool) -> Vec<&HighlightTheme> {
        self.themes
            .values()
            .filter(|theme| theme.is_dark == is_dark)
            .collect()
    }
}

impl Default for ThemeCollection {
    fn default() -> Self {
        Self::new()
    }
}

/// Goofy Dark highlighting theme
pub fn goofy_dark_highlight_theme() -> HighlightTheme {
    HighlightTheme {
        name: "goofy_dark".to_string(),
        is_dark: true,
        colors: HighlightColors {
            background: Color::Rgb(0x2D, 0x2D, 0x2D),
            text: Color::Rgb(0xD0, 0xD0, 0xD0),
            
            line_number: Color::Rgb(0x90, 0x90, 0x90),
            line_number_active: Color::Rgb(0xB0, 0xB0, 0xB0),
            
            comment: Color::Rgb(0x80, 0x80, 0x80),
            comment_doc: Color::Rgb(0x9A, 0xE4, 0x78),
            
            keyword: Color::Rgb(0x8A, 0x67, 0xFF),        // Primary purple
            keyword_control: Color::Rgb(0xFF, 0xA5, 0x00), // Accent orange
            keyword_type: Color::Rgb(0x29, 0xB6, 0xF6),   // Info blue
            
            string: Color::Rgb(0x9A, 0xE4, 0x78),         // Tertiary green
            string_escape: Color::Rgb(0xFF, 0xE1, 0x9C),  // Secondary yellow
            number: Color::Rgb(0xFF, 0xE1, 0x9C),         // Secondary yellow
            boolean: Color::Rgb(0xFF, 0xA5, 0x00),        // Accent orange
            null: Color::Rgb(0x80, 0x80, 0x80),           // Comment gray
            
            function: Color::Rgb(0x29, 0xB6, 0xF6),       // Info blue
            function_builtin: Color::Rgb(0x66, 0xBB, 0x6A), // Green
            variable: Color::Rgb(0xD0, 0xD0, 0xD0),       // Base text
            variable_builtin: Color::Rgb(0xFF, 0x80, 0x74), // Red
            constant: Color::Rgb(0xFF, 0xE1, 0x9C),       // Secondary yellow
            parameter: Color::Rgb(0xB0, 0xB0, 0xB0),      // Half-muted
            
            type_name: Color::Rgb(0x29, 0xB6, 0xF6),      // Info blue
            type_builtin: Color::Rgb(0x8A, 0x67, 0xFF),   // Primary purple
            type_parameter: Color::Rgb(0xFF, 0xA5, 0x00), // Accent orange
            
            operator: Color::Rgb(0xFF, 0xA5, 0x00),       // Accent orange
            punctuation: Color::Rgb(0xB0, 0xB0, 0xB0),    // Half-muted
            delimiter: Color::Rgb(0xA0, 0xA0, 0xA0),      // Muted
            
            error: Color::Rgb(0xF4, 0x43, 0x36),          // Error red
            warning: Color::Rgb(0xFF, 0xA5, 0x00),        // Warning orange
            
            tag: Color::Rgb(0x8A, 0x67, 0xFF),            // Primary purple
            attribute: Color::Rgb(0x9A, 0xE4, 0x78),      // Tertiary green
            property: Color::Rgb(0x29, 0xB6, 0xF6),       // Info blue
            label: Color::Rgb(0xFF, 0xE1, 0x9C),          // Secondary yellow
            
            diff_added: Color::Rgb(0x4C, 0xAF, 0x50),     // Success green
            diff_removed: Color::Rgb(0xF4, 0x43, 0x36),   // Error red
            diff_changed: Color::Rgb(0xFF, 0xA5, 0x00),   // Warning orange
            
            markup_heading: Color::Rgb(0x8A, 0x67, 0xFF), // Primary purple
            markup_bold: Color::Rgb(0xD0, 0xD0, 0xD0),    // Base text, but bold
            markup_italic: Color::Rgb(0xB0, 0xB0, 0xB0),  // Half-muted, but italic
            markup_link: Color::Rgb(0x29, 0xB6, 0xF6),    // Info blue
            markup_code: Color::Rgb(0xFF, 0x80, 0x74),    // Red
        },
    }
}

/// Goofy Light highlighting theme
pub fn goofy_light_highlight_theme() -> HighlightTheme {
    HighlightTheme {
        name: "goofy_light".to_string(),
        is_dark: false,
        colors: HighlightColors {
            background: Color::Rgb(0xFD, 0xFD, 0xFD),
            text: Color::Rgb(0x20, 0x20, 0x20),
            
            line_number: Color::Rgb(0x80, 0x86, 0x8B),
            line_number_active: Color::Rgb(0x60, 0x66, 0x6B),
            
            comment: Color::Rgb(0x80, 0x86, 0x8B),
            comment_doc: Color::Rgb(0x38, 0x8E, 0x3C),
            
            keyword: Color::Rgb(0x67, 0x3A, 0xB7),        // Primary purple (darker)
            keyword_control: Color::Rgb(0xED, 0x6C, 0x02), // Orange (darker)
            keyword_type: Color::Rgb(0x01, 0x65, 0xD4),   // Blue (darker)
            
            string: Color::Rgb(0x38, 0x8E, 0x3C),         // Green (darker)
            string_escape: Color::Rgb(0xF5, 0x7C, 0x00),  // Orange (darker)
            number: Color::Rgb(0xF5, 0x7C, 0x00),         // Orange (darker)
            boolean: Color::Rgb(0xED, 0x6C, 0x02),        // Orange (darker)
            null: Color::Rgb(0x80, 0x86, 0x8B),           // Gray
            
            function: Color::Rgb(0x01, 0x65, 0xD4),       // Blue (darker)
            function_builtin: Color::Rgb(0x46, 0xA3, 0x5B), // Green
            variable: Color::Rgb(0x20, 0x20, 0x20),       // Base text
            variable_builtin: Color::Rgb(0xC6, 0x28, 0x28), // Red (darker)
            constant: Color::Rgb(0xF5, 0x7C, 0x00),       // Orange (darker)
            parameter: Color::Rgb(0x40, 0x40, 0x40),      // Half-muted
            
            type_name: Color::Rgb(0x01, 0x65, 0xD4),      // Blue (darker)
            type_builtin: Color::Rgb(0x67, 0x3A, 0xB7),   // Purple (darker)
            type_parameter: Color::Rgb(0xED, 0x6C, 0x02), // Orange (darker)
            
            operator: Color::Rgb(0xED, 0x6C, 0x02),       // Orange (darker)
            punctuation: Color::Rgb(0x40, 0x40, 0x40),    // Half-muted
            delimiter: Color::Rgb(0x5F, 0x63, 0x68),      // Muted
            
            error: Color::Rgb(0xC6, 0x28, 0x28),          // Red (darker)
            warning: Color::Rgb(0xED, 0x6C, 0x02),        // Orange (darker)
            
            tag: Color::Rgb(0x67, 0x3A, 0xB7),            // Purple (darker)
            attribute: Color::Rgb(0x38, 0x8E, 0x3C),      // Green (darker)
            property: Color::Rgb(0x01, 0x65, 0xD4),       // Blue (darker)
            label: Color::Rgb(0xF5, 0x7C, 0x00),          // Orange (darker)
            
            diff_added: Color::Rgb(0x28, 0x72, 0x31),     // Green (darker)
            diff_removed: Color::Rgb(0xC6, 0x28, 0x28),   // Red (darker)
            diff_changed: Color::Rgb(0xED, 0x6C, 0x02),   // Orange (darker)
            
            markup_heading: Color::Rgb(0x67, 0x3A, 0xB7), // Purple (darker)
            markup_bold: Color::Rgb(0x20, 0x20, 0x20),    // Base text, but bold
            markup_italic: Color::Rgb(0x40, 0x40, 0x40),  // Half-muted, but italic
            markup_link: Color::Rgb(0x01, 0x65, 0xD4),    // Blue (darker)
            markup_code: Color::Rgb(0xC6, 0x28, 0x28),    // Red (darker)
        },
    }
}

/// Classic Dark highlighting theme
pub fn classic_dark_highlight_theme() -> HighlightTheme {
    HighlightTheme {
        name: "classic_dark".to_string(),
        is_dark: true,
        colors: HighlightColors {
            background: Color::Black,
            text: Color::White,
            
            line_number: Color::DarkGray,
            line_number_active: Color::Gray,
            
            comment: Color::DarkGray,
            comment_doc: Color::Green,
            
            keyword: Color::Cyan,
            keyword_control: Color::Magenta,
            keyword_type: Color::Blue,
            
            string: Color::Green,
            string_escape: Color::Yellow,
            number: Color::Yellow,
            boolean: Color::Magenta,
            null: Color::DarkGray,
            
            function: Color::Blue,
            function_builtin: Color::Cyan,
            variable: Color::White,
            variable_builtin: Color::Red,
            constant: Color::Yellow,
            parameter: Color::Gray,
            
            type_name: Color::Blue,
            type_builtin: Color::Cyan,
            type_parameter: Color::Magenta,
            
            operator: Color::Magenta,
            punctuation: Color::Gray,
            delimiter: Color::Gray,
            
            error: Color::Red,
            warning: Color::Yellow,
            
            tag: Color::Cyan,
            attribute: Color::Green,
            property: Color::Blue,
            label: Color::Yellow,
            
            diff_added: Color::Green,
            diff_removed: Color::Red,
            diff_changed: Color::Yellow,
            
            markup_heading: Color::Cyan,
            markup_bold: Color::White,
            markup_italic: Color::Gray,
            markup_link: Color::Blue,
            markup_code: Color::Red,
        },
    }
}

/// Classic Light highlighting theme
pub fn classic_light_highlight_theme() -> HighlightTheme {
    HighlightTheme {
        name: "classic_light".to_string(),
        is_dark: false,
        colors: HighlightColors {
            background: Color::White,
            text: Color::Black,
            
            line_number: Color::Gray,
            line_number_active: Color::DarkGray,
            
            comment: Color::DarkGray,
            comment_doc: Color::Rgb(0x00, 0x80, 0x00),
            
            keyword: Color::Blue,
            keyword_control: Color::Rgb(0x80, 0x00, 0x80),
            keyword_type: Color::Blue,
            
            string: Color::Rgb(0x00, 0x80, 0x00),
            string_escape: Color::Rgb(0xB8, 0x86, 0x00),
            number: Color::Rgb(0xB8, 0x86, 0x00),
            boolean: Color::Rgb(0x80, 0x00, 0x80),
            null: Color::DarkGray,
            
            function: Color::Blue,
            function_builtin: Color::Blue,
            variable: Color::Black,
            variable_builtin: Color::Rgb(0x80, 0x00, 0x00),
            constant: Color::Rgb(0xB8, 0x86, 0x00),
            parameter: Color::DarkGray,
            
            type_name: Color::Blue,
            type_builtin: Color::Blue,
            type_parameter: Color::Rgb(0x80, 0x00, 0x80),
            
            operator: Color::Rgb(0x80, 0x00, 0x80),
            punctuation: Color::DarkGray,
            delimiter: Color::Gray,
            
            error: Color::Rgb(0x80, 0x00, 0x00),
            warning: Color::Rgb(0xB8, 0x86, 0x00),
            
            tag: Color::Blue,
            attribute: Color::Rgb(0x00, 0x80, 0x00),
            property: Color::Blue,
            label: Color::Rgb(0xB8, 0x86, 0x00),
            
            diff_added: Color::Rgb(0x00, 0x80, 0x00),
            diff_removed: Color::Rgb(0x80, 0x00, 0x00),
            diff_changed: Color::Rgb(0xB8, 0x86, 0x00),
            
            markup_heading: Color::Blue,
            markup_bold: Color::Black,
            markup_italic: Color::DarkGray,
            markup_link: Color::Blue,
            markup_code: Color::Rgb(0x80, 0x00, 0x00),
        },
    }
}

/// High Contrast highlighting theme
pub fn high_contrast_highlight_theme() -> HighlightTheme {
    HighlightTheme {
        name: "high_contrast".to_string(),
        is_dark: true,
        colors: HighlightColors {
            background: Color::Black,
            text: Color::White,
            
            line_number: Color::Gray,
            line_number_active: Color::White,
            
            comment: Color::Gray,
            comment_doc: Color::LightGreen,
            
            keyword: Color::LightCyan,
            keyword_control: Color::LightMagenta,
            keyword_type: Color::LightBlue,
            
            string: Color::LightGreen,
            string_escape: Color::LightYellow,
            number: Color::LightYellow,
            boolean: Color::LightMagenta,
            null: Color::Gray,
            
            function: Color::LightBlue,
            function_builtin: Color::LightCyan,
            variable: Color::White,
            variable_builtin: Color::LightRed,
            constant: Color::LightYellow,
            parameter: Color::White,
            
            type_name: Color::LightBlue,
            type_builtin: Color::LightCyan,
            type_parameter: Color::LightMagenta,
            
            operator: Color::LightMagenta,
            punctuation: Color::White,
            delimiter: Color::Gray,
            
            error: Color::LightRed,
            warning: Color::LightYellow,
            
            tag: Color::LightCyan,
            attribute: Color::LightGreen,
            property: Color::LightBlue,
            label: Color::LightYellow,
            
            diff_added: Color::LightGreen,
            diff_removed: Color::LightRed,
            diff_changed: Color::LightYellow,
            
            markup_heading: Color::LightCyan,
            markup_bold: Color::White,
            markup_italic: Color::Gray,
            markup_link: Color::LightBlue,
            markup_code: Color::LightRed,
        },
    }
}

/// Monochrome highlighting theme
pub fn monochrome_highlight_theme() -> HighlightTheme {
    HighlightTheme {
        name: "monochrome".to_string(),
        is_dark: true,
        colors: HighlightColors {
            background: Color::Black,
            text: Color::White,
            
            line_number: Color::DarkGray,
            line_number_active: Color::Gray,
            
            comment: Color::DarkGray,
            comment_doc: Color::Gray,
            
            keyword: Color::White,
            keyword_control: Color::Gray,
            keyword_type: Color::Gray,
            
            string: Color::Gray,
            string_escape: Color::Gray,
            number: Color::Gray,
            boolean: Color::Gray,
            null: Color::DarkGray,
            
            function: Color::White,
            function_builtin: Color::Gray,
            variable: Color::White,
            variable_builtin: Color::Gray,
            constant: Color::Gray,
            parameter: Color::Gray,
            
            type_name: Color::White,
            type_builtin: Color::Gray,
            type_parameter: Color::Gray,
            
            operator: Color::Gray,
            punctuation: Color::Gray,
            delimiter: Color::DarkGray,
            
            error: Color::Gray,
            warning: Color::Gray,
            
            tag: Color::White,
            attribute: Color::Gray,
            property: Color::Gray,
            label: Color::Gray,
            
            diff_added: Color::White,
            diff_removed: Color::Gray,
            diff_changed: Color::Gray,
            
            markup_heading: Color::White,
            markup_bold: Color::White,
            markup_italic: Color::Gray,
            markup_link: Color::Gray,
            markup_code: Color::Gray,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_theme_collection() {
        let collection = ThemeCollection::new();
        let theme_names = collection.theme_names();
        
        assert!(theme_names.contains(&"goofy_dark".to_string()));
        assert!(theme_names.contains(&"goofy_light".to_string()));
        assert!(theme_names.contains(&"classic_dark".to_string()));
        assert!(theme_names.contains(&"classic_light".to_string()));
        assert!(theme_names.contains(&"high_contrast".to_string()));
        assert!(theme_names.contains(&"monochrome".to_string()));
    }
    
    #[test]
    fn test_theme_properties() {
        let dark_theme = goofy_dark_highlight_theme();
        assert!(dark_theme.is_dark);
        assert_eq!(dark_theme.name, "goofy_dark");
        
        let light_theme = goofy_light_highlight_theme();
        assert!(!light_theme.is_dark);
        assert_eq!(light_theme.name, "goofy_light");
    }
    
    #[test]
    fn test_dark_light_filtering() {
        let collection = ThemeCollection::new();
        
        let dark_themes = collection.themes_by_type(true);
        let light_themes = collection.themes_by_type(false);
        
        assert!(!dark_themes.is_empty());
        assert!(!light_themes.is_empty());
        
        for theme in dark_themes {
            assert!(theme.is_dark);
        }
        
        for theme in light_themes {
            assert!(!theme.is_dark);
        }
    }
    
    #[test]
    fn test_theme_retrieval() {
        let collection = ThemeCollection::new();
        
        let theme = collection.get_theme("goofy_dark");
        assert!(theme.is_some());
        assert_eq!(theme.unwrap().name, "goofy_dark");
        
        let nonexistent = collection.get_theme("nonexistent");
        assert!(nonexistent.is_none());
    }
}