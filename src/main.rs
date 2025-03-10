mod app;
mod tui;
mod ui;

use std::path::PathBuf;
use app::App;
use clap::Parser;
use tasd_lib::{Serializable, TASD};
use color_eyre::Result;
use crossterm::event::{self, Event, KeyEventKind};

/// A CLI interface to read and write TASD files, and to send them to a TAStm32.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the TASD file
    #[arg(short, long)]
    file: PathBuf,
}

fn main() -> Result<()> {
    // Initialize color_eyre for better error reporting
    color_eyre::install()?;

    // Parse command line arguments
    let args = Args::parse();

    // Read and parse TASD file - fix lifetime issue by cloning the content
    let content = std::fs::read(&args.file)?;
    let (_, tasd) = TASD::deserialize(&content).map_err(|e| color_eyre::eyre::eyre!("Failed to parse TASD file: {:?}", e))?;

    // Initialize application state
    let app = App::new(tasd, args.file);

    // Run the application using TUI
    run(app)
}

fn run(mut app: App) -> Result<()> {
    // Setup terminal
    let mut terminal = tui::init()?;

    // Main event loop
    while !app.exit {
        // Draw UI - pass mutable reference to app
        terminal.draw(|frame| ui::components::render(&mut app, frame))?;

        // Handle events
        match event::read()? {
            // It's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                app.handle_key_event(key_event)?;
            }
            _ => {}
        }
    }

    // Restore terminal
    if let Err(err) = tui::restore() {
        eprintln!(
            "Failed to restore terminal: Run `reset` or restart your terminal to recover: {}",
            err
        );
    }

    Ok(())
}