// src/tui.rs

use crate::app::{Action, App, AppMode, FocusedPanel, PopupMode};
use crate::error::{self, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::layout::Alignment;
use ratatui::prelude::{CrosstermBackend, Terminal};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
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

    pub fn exit(&mut self) -> Result<()> {
        disable_raw_mode()?;
        io::stderr().execute(LeaveAlternateScreen)?;
        Ok(())
    }

    /// Runs a special, minimal event loop to get the dotfiles path from the user.
    pub fn run_setup_prompt(&mut self) -> Result<String> {
        let mut path_input = String::new();
        loop {
            self.terminal.draw(|f| {
                let text = vec![
                    Line::from("").style(Style::default()),
                    Line::from(" Welcome to DotaTUI First-Time Setup").style(Style::default().bold()),
                    Line::from(""),
                    Line::from(" Please enter the absolute path to your dotfiles directory:"),
                    Line::from(""),
                    Line::from(path_input.as_str()).style(Style::default().fg(Color::Cyan)),
                ];
                let block = Block::default()
                    .borders(Borders::ALL)
                    .title("Configuration Required")
                    .title_bottom(Line::from(" Enter: Confirm | Ctrl-C: Quit ").centered());
                let paragraph = Paragraph::new(text).block(block).alignment(Alignment::Center);
                f.render_widget(paragraph, f.size());
            })?;

            if event::poll(Duration::from_millis(250))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
                                return Err(error::Error::Io(io::Error::new(
                                    io::ErrorKind::Interrupted,
                                    "Setup cancelled by user",
                                )));
                            }
                            KeyCode::Enter => {
                                if !path_input.is_empty() {
                                    break;
                                }
                            }
                            KeyCode::Char(c) => {
                                path_input.push(c);
                            }
                            KeyCode::Backspace => {
                                path_input.pop();
                            }
                        }
                        _ => {}
                        
                    }
                }
            }
        }
        Ok(path_input)
    }

    pub fn draw(&mut self, app: &mut App) -> Result<()> {
        self.terminal.draw(|frame| crate::ui::draw(frame, app))?;
        Ok(())
    }

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

    fn handle_key_event(
        &self,
        key: KeyEvent,
        app: &App,
        action_tx: &mpsc::UnboundedSender<Action>,
    ) -> Result<()> {
        let action = match app.mode {
            AppMode::Home => self.handle_home_mode_key(key),
            AppMode::Status => self.handle_status_mode_key(key, app),
            AppMode::Popup(_) => self.handle_popup_mode_key(key, app),
        };

        if let Some(action) = action {
            action_tx.send(action).map_err(|_| crate::error::Error::ChannelSend)?;
        }

        Ok(())
    }

    fn handle_home_mode_key(&self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('s') => Some(Action::GoToStatus),
            KeyCode::Char('h') | KeyCode::Char('?') => Some(Action::ToggleHelp),
            _ => None,
        }
    }

    fn handle_status_mode_key(&self, key: KeyEvent, app: &App) -> Option<Action> {
        match key.code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('?') => Some(Action::ToggleHelp),
            KeyCode::Char('h') => Some(Action::GoToHome),
            KeyCode::Char('j') | KeyCode::Down => Some(Action::NavigateDown),
            KeyCode::Char('k') | KeyCode::Up => Some(Action::NavigateUp),
            KeyCode::Char('l') | KeyCode::Tab => Some(Action::FocusNextPanel),
            KeyCode::BackTab => Some(Action::FocusPrevPanel),
            KeyCode::Char('r') => Some(Action::RefreshStatus),
            KeyCode::Char('a') => Some(Action::StageAll),
            KeyCode::Char('u') => Some(Action::UnstageAll),
            KeyCode::Char('c') => Some(Action::EnterPopup(PopupMode::Commit)),
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

impl Drop for Tui {
    fn drop(&mut self) {
        let _ = self.exit();
    }
}
