// src/app.rs

use crate::error::Result;
use crate::git_utils::FileState;
use ratatui::widgets::ListState;
use std::path::PathBuf;
use tokio::sync::mpsc;

/// Enum for the different kinds of popups that can be active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PopupMode {
    Commit,
    AddRemote,
    Help,
    InitRepo,
    ChangePath,
}

/// The AppMode is now extremely simple: either you are in the main view, or a popup is active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Popup(PopupMode),
}

/// Actions are refactored for the new, direct-manipulation UI.
#[derive(Debug)]
pub enum Action {
    Quit,
    ToggleHelp,
    EnterPopup(PopupMode),
    ExitPopup,
    RefreshStatus,
    StatusUpdated(Result<Vec<FileState>>),
    DiffUpdated(String),
    StageUnstage,
    StageAll,
    Commit,
    Push,
    PushCompleted(Result<()>),
    InitRepo,
    AddRemote,
    ChangePath,
    NavigateUp,
    NavigateDown,
    Input(char),
    InputDelete,
}

/// The main application state.
pub struct App {
    // Core State
    pub mode: AppMode,
    pub should_quit: bool,
    pub is_loading: bool,
    pub message: String,
    pub dotfiles_path: PathBuf,

    // Main View State
    pub files: Vec<FileState>,
    pub file_list_state: ListState,
    pub diff_text: String,
    
    // Popup State
    pub input: String,

    // System
    pub action_tx: mpsc::UnboundedSender<Action>,
}

impl App {
    pub fn new(action_tx: mpsc::UnboundedSender<Action>, dotfiles_path: PathBuf) -> Self {
        let mut app = App {
            mode: AppMode::Normal,
            should_quit: false,
            is_loading: true,
            message: "Refreshing status...".to_string(),
            dotfiles_path,
            files: Vec::new(),
            file_list_state: ListState::default(),
            diff_text: "Select a file to see the diff.".to_string(),
            input: String::new(),
            action_tx,
        };
        app.file_list_state.select(Some(0));
        app
    }

    pub fn update(&mut self, action: &Action) -> Result<()> {
        match action {
            Action::Quit => self.should_quit = true,
            Action::ToggleHelp => {
                self.mode = if self.mode == AppMode::Popup(PopupMode::Help) {
                    AppMode::Normal
                } else {
                    AppMode::Popup(PopupMode::Help)
                };
            }
            Action::EnterPopup(mode) => self.mode = AppMode::Popup(mode.clone()),
            Action::ExitPopup => {
                self.input.clear();
                self.mode = AppMode::Normal;
            }
            Action::RefreshStatus => {
                self.is_loading = true;
                self.diff_text = "Refreshing...".to_string();
            }
            Action::StatusUpdated(Ok(files)) => {
                self.is_loading = false;
                self.files = files.clone();
                self.check_selection_bounds();
                self.message = "Status refreshed.".to_string();
            }
            Action::StatusUpdated(Err(e)) => {
                self.is_loading = false;
                self.message = format!("Error: {}", e);
            }
            Action::DiffUpdated(diff) => {
                self.diff_text = diff.clone();
            }
            Action::PushCompleted(result) => {
                self.is_loading = false;
                match result {
                    Ok(_) => self.message = "Push successful.".to_string(),
                    Err(e) => self.message = format!("Push failed: {}", e),
                }
            }
            Action::NavigateUp => self.navigate_up(),
            Action::NavigateDown => self.navigate_down(),
            Action::Input(c) => self.input.push(*c),
            Action::InputDelete => {
                self.input.pop();
            }
            _ => {}
        }
        Ok(())
    }

    fn check_selection_bounds(&mut self) {
        if self.file_list_state.selected().is_some() && self.file_list_state.selected().unwrap() >= self.files.len() {
            self.file_list_state.select(self.files.len().checked_sub(1));
        }
        if self.files.is_empty() {
            self.file_list_state.select(None);
        }
    }

    fn navigate_up(&mut self) {
        if self.files.is_empty() { return; }
        let i = match self.file_list_state.selected() {
            Some(i) => if i == 0 { self.files.len() - 1 } else { i - 1 },
            None => 0,
        };
        self.file_list_state.select(Some(i));
    }

    fn navigate_down(&mut self) {
        if self.files.is_empty() { return; }
        let i = match self.file_list_state.selected() {
            Some(i) => if i >= self.files.len() - 1 { 0 } else { i + 1 },
            None => 0,
        };
        self.file_list_state.select(Some(i));
    }

    pub fn get_selected_file(&self) -> Option<&FileState> {
        self.file_list_state.selected().and_then(|i| self.files.get(i))
    }

    pub fn send_action(&self, action: Action) -> Result<()> {
        self.action_tx.send(action).map_err(|_| crate::error::Error::ChannelSend)
    }
}
