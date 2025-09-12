//! Formatting utilities and spinner for non-interactive mode
//!
//! This module provides formatting utilities and progress indicators
//! for command-line output.

use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use crossterm::{cursor, execute, terminal};

/// Spinner animation characters
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Spinner for non-interactive mode
pub struct Spinner {
    message: String,
    running: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Spinner {
    /// Create a new spinner with the given message
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            running: Arc::new(AtomicBool::new(false)),
            handle: None,
        }
    }
    
    /// Start the spinner animation
    pub fn start(&mut self) {
        if self.running.load(Ordering::SeqCst) {
            return;
        }
        
        self.running.store(true, Ordering::SeqCst);
        let running = Arc::clone(&self.running);
        let message = self.message.clone();
        
        let handle = thread::spawn(move || {
            let mut frame = 0;
            let mut stderr = io::stderr();
            
            // Hide cursor
            let _ = execute!(stderr, cursor::Hide);
            
            while running.load(Ordering::SeqCst) {
                // Clear line and print spinner
                let _ = execute!(stderr, terminal::Clear(terminal::ClearType::CurrentLine));
                let _ = execute!(stderr, cursor::MoveToColumn(0));
                let _ = write!(stderr, "{} {}", SPINNER_FRAMES[frame], message);
                let _ = stderr.flush();
                
                frame = (frame + 1) % SPINNER_FRAMES.len();
                thread::sleep(Duration::from_millis(80));
            }
            
            // Clear the line and show cursor
            let _ = execute!(stderr, terminal::Clear(terminal::ClearType::CurrentLine));
            let _ = execute!(stderr, cursor::MoveToColumn(0));
            let _ = execute!(stderr, cursor::Show);
            let _ = stderr.flush();
        });
        
        self.handle = Some(handle);
    }
    
    /// Stop the spinner animation
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
    
    /// Update the spinner message
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Format helpers for terminal output
pub struct Format;

impl Format {
    /// Create a styled success message
    pub fn success(message: &str) -> String {
        format!("\x1b[32m✓\x1b[0m {}", message)
    }
    
    /// Create a styled error message
    pub fn error(message: &str) -> String {
        format!("\x1b[31m✗\x1b[0m {}", message)
    }
    
    /// Create a styled warning message
    pub fn warning(message: &str) -> String {
        format!("\x1b[33m⚠\x1b[0m {}", message)
    }
    
    /// Create a styled info message
    pub fn info(message: &str) -> String {
        format!("\x1b[34mℹ\x1b[0m {}", message)
    }
    
    /// Create a bold text
    pub fn bold(text: &str) -> String {
        format!("\x1b[1m{}\x1b[0m", text)
    }
    
    /// Create dimmed text
    pub fn dim(text: &str) -> String {
        format!("\x1b[2m{}\x1b[0m", text)
    }
    
    /// Create underlined text
    pub fn underline(text: &str) -> String {
        format!("\x1b[4m{}\x1b[0m", text)
    }
    
    /// Create colored text
    pub fn color(text: &str, color: Color) -> String {
        format!("{}{}\x1b[0m", color.ansi_code(), text)
    }
}

/// Terminal colors
#[derive(Debug, Clone, Copy)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

impl Color {
    /// Get ANSI escape code for the color
    pub fn ansi_code(&self) -> &'static str {
        match self {
            Color::Black => "\x1b[30m",
            Color::Red => "\x1b[31m",
            Color::Green => "\x1b[32m",
            Color::Yellow => "\x1b[33m",
            Color::Blue => "\x1b[34m",
            Color::Magenta => "\x1b[35m",
            Color::Cyan => "\x1b[36m",
            Color::White => "\x1b[37m",
            Color::BrightBlack => "\x1b[90m",
            Color::BrightRed => "\x1b[91m",
            Color::BrightGreen => "\x1b[92m",
            Color::BrightYellow => "\x1b[93m",
            Color::BrightBlue => "\x1b[94m",
            Color::BrightMagenta => "\x1b[95m",
            Color::BrightCyan => "\x1b[96m",
            Color::BrightWhite => "\x1b[97m",
        }
    }
}

/// Progress bar for showing task progress
pub struct ProgressBar {
    total: usize,
    current: usize,
    width: usize,
    message: String,
}

impl ProgressBar {
    /// Create a new progress bar
    pub fn new(total: usize, message: impl Into<String>) -> Self {
        Self {
            total,
            current: 0,
            width: 40,
            message: message.into(),
        }
    }
    
    /// Update the current progress
    pub fn set_current(&mut self, current: usize) {
        self.current = current.min(self.total);
        self.render();
    }
    
    /// Increment the progress by one
    pub fn increment(&mut self) {
        self.set_current(self.current + 1);
    }
    
    /// Render the progress bar
    fn render(&self) {
        let percentage = if self.total > 0 {
            (self.current as f32 / self.total as f32 * 100.0) as usize
        } else {
            0
        };
        
        let filled = (self.width * self.current) / self.total.max(1);
        let empty = self.width - filled;
        
        let bar = format!(
            "[{}{}] {}/{} ({}%) {}",
            "=".repeat(filled),
            " ".repeat(empty),
            self.current,
            self.total,
            percentage,
            self.message
        );
        
        let mut stderr = io::stderr();
        let _ = execute!(stderr, terminal::Clear(terminal::ClearType::CurrentLine));
        let _ = execute!(stderr, cursor::MoveToColumn(0));
        let _ = write!(stderr, "{}", bar);
        let _ = stderr.flush();
    }
    
    /// Finish the progress bar
    pub fn finish(&self) {
        let mut stderr = io::stderr();
        let _ = writeln!(stderr);
        let _ = stderr.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_messages() {
        assert!(Format::success("Test").contains("✓"));
        assert!(Format::error("Test").contains("✗"));
        assert!(Format::warning("Test").contains("⚠"));
        assert!(Format::info("Test").contains("ℹ"));
    }
    
    #[test]
    fn test_color_codes() {
        assert_eq!(Color::Red.ansi_code(), "\x1b[31m");
        assert_eq!(Color::Green.ansi_code(), "\x1b[32m");
        assert_eq!(Color::Blue.ansi_code(), "\x1b[34m");
    }
}