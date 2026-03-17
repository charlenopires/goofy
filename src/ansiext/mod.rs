//! ANSI control character utilities
//!
//! This module provides utilities for handling ANSI control characters
//! in terminal output.

/// Escape replaces control characters with their Unicode Control Picture
/// representations to ensure they are displayed correctly in the UI.
pub fn escape(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    
    for ch in content.chars() {
        match ch {
            // Control characters 0x00-0x1F
            '\x00'..='\x1F' => {
                // Convert to Unicode Control Picture (U+2400 - U+241F)
                let control_picture = char::from_u32(0x2400 + ch as u32).unwrap_or(ch);
                result.push(control_picture);
            }
            // DEL character (0x7F)
            '\x7F' => {
                result.push('\u{2421}'); // Unicode symbol for DEL
            }
            // All other characters pass through unchanged
            _ => {
                result.push(ch);
            }
        }
    }
    
    result
}

/// Strip ANSI escape sequences from text
pub fn strip_ansi(text: &str) -> String {
    // Use a simple regex-like pattern to remove ANSI escape sequences
    let mut result = String::new();
    let mut chars = text.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // Start of ANSI escape sequence
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                // Skip until we find a letter (end of sequence)
                while let Some(next_ch) = chars.next() {
                    if next_ch.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(ch);
        }
    }
    
    result
}

/// Check if a string contains ANSI escape sequences
pub fn contains_ansi(text: &str) -> bool {
    text.contains('\x1b')
}

/// Calculate the display width of text, ignoring ANSI sequences
pub fn display_width(text: &str) -> usize {
    strip_ansi(text).chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_escape_control_chars() {
        assert_eq!(escape("\x00"), "\u{2400}");
        assert_eq!(escape("\x1F"), "\u{241F}");
        assert_eq!(escape("\x7F"), "\u{2421}");
        assert_eq!(escape("hello"), "hello");
        assert_eq!(escape("hello\x00world"), "hello\u{2400}world");
    }
    
    #[test]
    fn test_strip_ansi() {
        assert_eq!(strip_ansi("\x1b[31mred\x1b[0m"), "red");
        assert_eq!(strip_ansi("plain text"), "plain text");
        assert_eq!(strip_ansi("\x1b[1;32mgreen bold\x1b[0m"), "green bold");
    }
    
    #[test]
    fn test_contains_ansi() {
        assert!(contains_ansi("\x1b[31mred\x1b[0m"));
        assert!(!contains_ansi("plain text"));
    }
    
    #[test]
    fn test_display_width() {
        assert_eq!(display_width("\x1b[31mred\x1b[0m"), 3);
        assert_eq!(display_width("hello"), 5);
    }
}