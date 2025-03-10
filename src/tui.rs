use std::io::{self, stdout};

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

/// A type alias for the terminal type used in this application
pub type Tui = Terminal<CrosstermBackend<io::Stdout>>;

/// Initialize the terminal
pub fn init() -> io::Result<Tui> {
    // Enter alternate screen and enable raw mode
    execute!(stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;

    // Set panic hook to restore terminal on panic
    set_panic_hook();

    // Create and return terminal instance
    Terminal::new(CrosstermBackend::new(stdout()))
}

/// Set panic hook to restore terminal on panic
fn set_panic_hook() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Try to restore terminal before showing panic message
        let _ = restore();
        hook(panic_info);
    }));
}

/// Restore the terminal to its original state
pub fn restore() -> io::Result<()> {
    // Leave alternate screen and disable raw mode
    execute!(stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}