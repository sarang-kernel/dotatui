// This module is responsible for the entire lifecycle of the terminal user interface. It encapsulates all the crossterm and ratatui setup and teardown logic.

// Key features of this file:

//    RAII (Resource Acquisition Is Initialization): The Tui struct handles entering the alternate screen and enabling raw mode in its enter method. Crucially, it implements the Drop trait to guarantee that the terminal is restored to its original state when the Tui object goes out of scope—even if the application panics. This is a cornerstone of writing robust TUI applications.

//    Decoupled Event Handling: The get_event function polls for keyboard events and translates them into Action enums. This decouples the raw input from the application logic, which only needs to care about the abstract Action.

//    Efficiency: Event polling has a short timeout (50ms), ensuring the application remains responsive to actions sent from background tasks while not consuming excessive CPU in a tight loop.

// src/tui.rs

use crate::app::{Action, App, AppMode};
use crate::error::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::{CrosstermBackend, Terminal};
use std::io::{self, Stderr};
use std::time::Duration;
use tokio::sync::mpsc;

/// A struct that handles the terminal user interface lifecycle.
///
/// It is responsible for initializing the terminal, drawing the UI, handling events,
/// and restoring the terminal to its original state upon exit.
pub struct Tui {
    /// The `ratatui` terminal instance.
    terminal: Terminal<CrosstermBackend<Stderr>>,
}

impl Tui {
    /// Constructs a new `Tui` instance.
    pub fn new() -> Result<Self> {
        let terminal = Terminal::new(CrosstermBackend::new(io::stderr()))?;
        Ok(Self { terminal })
    }

    /// Enters the alternate screen and enables raw mode, preparing the terminal for the TUI.
    pub fn enter(&mut self) -> Result<()> {
        enable_raw_mode()?;
        io::stderr().execute(EnterAlternateScreen)?;
        Ok(())
    }

    /// Restores the terminal to its original state by leaving the alternate screen
    /// and disabling raw mode.
    pub fn exit(&mut self) -> Result<()> {
        io::stderr().execute(LeaveAlternateScreen)?;
        disable_raw_mode()?;
        Ok(())
    }

    /// Draws the application's UI by calling the main `draw` function.
    pub fn draw(&mut self, app: &mut App) -> Result<()> {
        self.terminal.draw(|frame| crate::ui::draw(frame, app))?;
        Ok(())
    }

    /// Polls for a terminal event and handles it.
    ///
    /// This function waits for a short duration for an event. If a key press occurs,
    /// it's translated into an `Action` and sent over the channel.
    pub fn handle_events(&self, app: &App, action_tx: &mpsc::UnboundedSender<Action>) -> Result<()> {
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    self.handle_key_event(key, app, action_tx)?;
                }
            }
        }
        Ok(())
    }

    /// Translates a `KeyEvent` into an `Action` based on the current `AppMode`.
    fn handle_key_event(
        &self,
        key: KeyEvent,
        app: &App,
        action_tx: &mpsc::UnboundedSender<Action>,
    ) -> Result<()> {
        let action = match app.mode {
            // Keybindings for the initial setup prompt.
            AppMode::InitRepoPrompt => match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => Action::InitRepo,
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Char('q') => Action::Quit,
                _ => return Ok(()), // Ignore other keys in this specific mode.
            },
            // Keybindings for when in Normal or Help mode.
            AppMode::Normal | AppMode::Help => match key.code {
                KeyCode::Char('q') => Action::Quit,
                KeyCode::Char('?') => Action::ToggleHelp,
                KeyCode::Char('j') | KeyCode::Down => Action::NavigateDown,
                KeyCode::Char('k') | KeyCode::Up => Action::NavigateUp,
                KeyCode::Char('g') => Action::NavigateTop,
                KeyCode::Char('G') => Action::NavigateBottom,
                KeyCode::Char('/') => Action::EnterSearch,
                KeyCode::Char('r') => Action::RefreshStatus,
                KeyCode::Char('a') => Action::AddAll,
                KeyCode::Char('c') => Action::EnterCommit,
                KeyCode::Char('p') => Action::Push,
                _ => return Ok(()),
            },
            // Keybindings for when in an input mode (Search, Commit, etc.).
            AppMode::Search | AppMode::CommitInput | AppMode::AddRemote => match key.code {
                KeyCode::Enter => match app.mode {
                    AppMode::CommitInput => Action::Commit,
                    AppMode::AddRemote => Action::AddRemote,
                    _ => Action::EnterNormal,
                },
                KeyCode::Esc => Action::EnterNormal,
                KeyCode::Char(c) => Action::Input(c),
                KeyCode::Backspace => Action::InputDelete,
                _ => return Ok(()),
            },
        };
        // Send the determined action to the main loop for processing.
        action_tx.send(action).map_err(|_| crate::error::Error::ChannelSend)?;
        Ok(())
    }
}

/// The `Drop` implementation for `Tui`.
///
/// This ensures that `self.exit()` is called when the `Tui` instance goes out of scope,
/// restoring the terminal to a usable state even if the application panics.
impl Drop for Tui {
    fn drop(&mut self) {
        let _ = self.exit();
    }
}
