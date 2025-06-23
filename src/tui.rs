// This module is responsible for the entire lifecycle of the terminal user interface. It encapsulates all the crossterm and ratatui setup and teardown logic.

// Key features of this file:

//    RAII (Resource Acquisition Is Initialization): The Tui struct handles entering the alternate screen and enabling raw mode in its enter method. Crucially, it implements the Drop trait to guarantee that the terminal is restored to its original state when the Tui object goes out of scope—even if the application panics. This is a cornerstone of writing robust TUI applications.

//    Decoupled Event Handling: The get_event function polls for keyboard events and translates them into Action enums. This decouples the raw input from the application logic, which only needs to care about the abstract Action.

//    Efficiency: Event polling has a short timeout (50ms), ensuring the application remains responsive to actions sent from background tasks while not consuming excessive CPU in a tight loop.

// src/tui.rs

use crate::app::{Action, App, AppMode, FocusedPanel, PopupMode};
use crate::error::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    terminal::{disable_raw_mode, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::{CrosstermBackend, Terminal};
use std::io::{self, Stderr};
use std::time::Duration;
use tokio::sync::mpsc;

/// A struct that handles the terminal user interface lifecycle.
pub struct Tui {
    terminal: Terminal<CrosstermBackend<Stderr>>,
}

impl Tui {
    pub fn new() -> Result<Self> {
        let terminal = Terminal::new(CrosstermBackend::new(io::stderr()))?;
        Ok(Self { terminal })
    }

    pub fn enter(&mut self) -> Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        io::stderr().execute(crossterm::terminal::EnterAlternateScreen)?;
        Ok(())
    }

    /// Restores the terminal to its original state.
    pub fn exit(&mut self) -> Result<()> {
        disable_raw_mode()?;
        io::stderr().execute(LeaveAlternateScreen)?;
        Ok(())
    }

    pub fn draw(&mut self, app: &mut App) -> Result<()> {
        self.terminal.draw(|frame| crate::ui::draw(frame, app))?;
        Ok(())
    }

    /// Polls for a terminal event and handles it.
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
            AppMode::Normal => self.handle_normal_mode_key(key, app),
            AppMode::Popup(_) => self.handle_popup_mode_key(key, app),
        };

        if let Some(action) = action {
            action_tx.send(action).map_err(|_| crate::error::Error::ChannelSend)?;
        }

        Ok(())
    }

    /// Handles key events when the app is in `AppMode::Normal`.
    fn handle_normal_mode_key(&self, key: KeyEvent, app: &App) -> Option<Action> {
        match key.code {
            // Global keybindings
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('?') => Some(Action::ToggleHelp),
            KeyCode::Char('j') | KeyCode::Down => Some(Action::NavigateDown),
            KeyCode::Char('k') | KeyCode::Up => Some(Action::NavigateUp),
            KeyCode::Char('l') | KeyCode::Tab => Some(Action::FocusNextPanel),
            KeyCode::Char('h') | KeyCode::BackTab => Some(Action::FocusPrevPanel),
            KeyCode::Char('r') => Some(Action::RefreshStatus),
            KeyCode::Char('a') => Some(Action::StageAll),
            KeyCode::Char('u') => Some(Action::UnstageAll),
            KeyCode::Char('c') => Some(Action::EnterPopup(PopupMode::Commit)),

            // Context-sensitive keybindings
            KeyCode::Char(' ') => match app.focused_panel {
                FocusedPanel::Unstaged => Some(Action::StageFile),
                FocusedPanel::Staged => Some(Action::UnstageFile),
                _ => None,
            },
            KeyCode::Enter => match app.focused_panel {
                FocusedPanel::Menu => Some(Action::ExecuteCommand),
                _ => None,
            },
            _ => None,
        }
    }

    /// Handles key events when a popup is active.
    fn handle_popup_mode_key(&self, key: KeyEvent, app: &App) -> Option<Action> {
        match key.code {
            KeyCode::Esc => Some(Action::ExitPopup),
            KeyCode::Enter => match &app.mode {
                AppMode::Popup(PopupMode::Commit) => Some(Action::Commit),
                AppMode::Popup(PopupMode::AddRemote) => Some(Action::AddRemote),
                AppMode::Popup(PopupMode::InitRepo) => Some(Action::InitRepo),
                _ => None,
            },
            KeyCode::Char(c) => Some(Action::Input(c)),
            KeyCode::Backspace => Some(Action::InputDelete),
            _ => None,
        }
    }
}

/// The `Drop` implementation ensures the terminal is restored even on panic.
impl Drop for Tui {
    fn drop(&mut self) {
        let _ = self.exit();
    }
}
