//! src/tui.rs

use crate::error::AppResult;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Stdout};

/// A wrapper around the `ratatui` Terminal.
pub struct Tui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Tui {
    /// Creates a new `Tui`.
    pub fn new() -> AppResult<Self> {
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    /// Enters the alternate screen and raw mode.
    pub fn enter(&mut self) -> AppResult<()> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
        Ok(())
    }

    /// Exits the alternate screen and raw mode.
    pub fn exit(&mut self) -> AppResult<()> {
        execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
        disable_raw_mode()?;
        Ok(())
    }

    /// Draws the given widget `f` to the terminal.
    pub fn draw<F>(&mut self, f: F) -> AppResult<()>
    where
        F: FnOnce(&mut ratatui::Frame), // Simplified signature works with `for<'a>` inference
    {
        self.terminal.draw(f)?;
        Ok(())
    }
}
