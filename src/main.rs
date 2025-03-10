mod ui;
mod tui;
mod components;

use std::path::PathBuf;
use clap::Parser;
use tasd_lib::{Serializable, TASD};
use color_eyre::Result;
use crate::ui::App;

/// A CLI interface to read and write TASD files, and to send them to a TAStm32.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the TASD file
    #[arg(short, long)]
    file: PathBuf,
}


fn main() -> Result<()> {
    let args = Args::parse();
    let content = std::fs::read(&args.file).expect("could not read file");
    let tasd = TASD::deserialize(&content).expect("could not deserialize file");

    color_eyre::install()?;
    let mut terminal = tui::init()?;

    let result = App::new(tasd.1).run(&mut terminal);
    if let Err(err) = tui::restore() {
        eprintln!(
            "failed to restore terminal. Run `reset` or restart your terminal to recover: {}",
            err
        );
    }
    result
}