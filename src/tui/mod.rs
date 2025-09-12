//! Terminal User Interface module using ratatui
//! This is the equivalent of the Bubble Tea TUI in the Go version

mod app;
mod components;
mod events;
mod keys;
mod pages;
mod polish;
mod styles;
mod themes;
mod utils;

pub use app::App;
pub use events::{Event, EventHandler};
pub use keys::KeyMap;

use anyhow::Result;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;

pub type Backend = CrosstermBackend<io::Stdout>;
pub type Frame<'a> = ratatui::Frame<'a>;

/// Initialize the terminal for TUI mode
pub fn init_terminal() -> Result<Terminal<Backend>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore the terminal to normal mode
pub fn restore_terminal(terminal: &mut Terminal<Backend>) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

/// Main TUI entry point
pub async fn run() -> Result<()> {
    let mut terminal = init_terminal()?;
    let mut app = App::new().await?;
    let mut event_handler = EventHandler::new();
    
    let result = run_app(&mut terminal, &mut app, &mut event_handler).await;
    
    restore_terminal(&mut terminal)?;
    result
}

/// Run TUI with existing App instance
pub async fn run_with_app(app: &mut crate::app::App) -> Result<()> {
    let mut terminal = init_terminal()?;
    let mut tui_app = App::new_with_backend(app).await?;
    let mut event_handler = EventHandler::new();
    
    let result = run_app(&mut terminal, &mut tui_app, &mut event_handler).await;
    
    restore_terminal(&mut terminal)?;
    result
}

/// Main application loop
async fn run_app(
    terminal: &mut Terminal<Backend>,
    app: &mut App,
    event_handler: &mut EventHandler,
) -> Result<()> {
    loop {
        terminal.draw(|frame| app.render(frame))?;
        
        if let Some(event) = event_handler.next().await {
            if app.handle_event(event).await? {
                break; // Exit requested
            }
        }
    }
    Ok(())
}