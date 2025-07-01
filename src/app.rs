// src/app.rs

use crate::error::Result;
use crate::git_utils; // CORRECTED: Added missing import
use crate::git_utils::StatusItem;
use git2::Repository; // CORRECTED: Added missing import
use ratatui::widgets::ListState;
use std::path::PathBuf;
use tokio::sync::mpsc;

#[derive(Debug, PartialEq, Eq)]
pub enum FocusedPanel {
    Unstaged,
    Staged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PopupMode {
    Commit,
    AddRemote,
    Help,
    InitRepo,
    ChangePath,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    Home,
    Status,
    Popup(PopupMode),
}

#[derive(Debug)]
pub enum Action {
    Quit,
    GoToHome,
    GoToStatus,
    ToggleHelp,
    EnterPopup(PopupMode),
    ExitPopup,
    RefreshStatus,
    StatusUpdated(Result<(Vec<StatusItem>, Vec<StatusItem>)>),
    DiffUpdated(String),
    StageFile,
    UnstageFile,
    StageAll,
    UnstageAll,
    Commit,
    Push,
    PushCompleted(Result<()>),
    InitRepo,
    AddRemote,
    ChangePath,
    FocusNextPanel,
    FocusPrevPanel,
    NavigateUp,
    NavigateDown,
    Input(char),
    InputDelete,
}

pub struct App {
    pub mode: AppMode,
    pub should_quit: bool,
    pub is_loading: bool,
    pub message: String,
    pub dotfiles_path: PathBuf,
    pub repo_status_summary: String,
    pub focused_panel: FocusedPanel,
    pub unstaged_changes: Vec<StatusItem>,
    pub staged_changes: Vec<StatusItem>,
    pub unstaged_state: ListState,
    pub staged_state: ListState,
    pub diff_text: String,
    pub input: String,
    pub action_tx: mpsc::UnboundedSender<Action>,
}

impl App {
    pub fn new(action_tx: mpsc::UnboundedSender<Action>, dotfiles_path: PathBuf) -> Self {
        let mut app = App {
            mode: AppMode::Home,
            should_quit: false,
            is_loading: true,
            message: "Checking repository status...".to_string(),
            dotfiles_path,
            repo_status_summary: "Checking...".to_string(),
            focused_panel: FocusedPanel::Unstaged,
            unstaged_changes: Vec::new(),
            staged_changes: Vec::new(),
            unstaged_state: ListState::default(),
            staged_state: ListState::default(),
            diff_text: "Select a file to see the diff.".to_string(),
            input: String::new(),
            action_tx,
        };
        app.unstaged_state.select(Some(0));
        app
    }

    pub fn update(&mut self, action: &Action) -> Result<()> {
        match action {
            Action::Quit => self.should_quit = true,
            Action::GoToHome => self.mode = AppMode::Home,
            Action::GoToStatus => self.mode = AppMode::Status,
            Action::ToggleHelp => {
                self.mode = if self.mode == AppMode::Popup(PopupMode::Help) {
                    AppMode::Home
                } else {
                    AppMode::Popup(PopupMode::Help)
                };
            }
            Action::EnterPopup(mode) => self.mode = AppMode::Popup(mode.clone()),
            Action::ExitPopup => {
                self.input.clear();
                self.mode = AppMode::Home;
            }
            Action::RefreshStatus => {
                self.is_loading = true;
                self.diff_text = "Refreshing...".to_string();
            }
            Action::StatusUpdated(Ok((unstaged, staged))) => {
                self.is_loading = false;
                self.unstaged_changes = unstaged.clone();
                self.staged_changes = staged.clone();
                self.repo_status_summary = format!("{} unstaged, {} staged", unstaged.len(), staged.len());
                self.check_selection_bounds();
                self.message = "Status refreshed.".to_string();
            }
            Action::StatusUpdated(Err(e)) => {
                self.is_loading = false;
                self.repo_status_summary = "Error".to_string();
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
            Action::FocusNextPanel | Action::FocusPrevPanel => self.toggle_panel_focus(),
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
        if self.unstaged_state.selected().is_some() && self.unstaged_state.selected().unwrap() >= self.unstaged_changes.len() {
            self.unstaged_state.select(self.unstaged_changes.len().checked_sub(1));
        }
        if self.staged_state.selected().is_some() && self.staged_state.selected().unwrap() >= self.staged_changes.len() {
            self.staged_state.select(self.staged_changes.len().checked_sub(1));
        }
    }

    fn toggle_panel_focus(&mut self) {
        self.focused_panel = match self.focused_panel {
            FocusedPanel::Unstaged => FocusedPanel::Staged,
            FocusedPanel::Staged => FocusedPanel::Unstaged,
        };
    }

    fn navigate_up(&mut self) {
        match self.focused_panel {
            FocusedPanel::Unstaged => previous_item(&mut self.unstaged_state, self.unstaged_changes.len()),
            FocusedPanel::Staged => previous_item(&mut self.staged_state, self.staged_changes.len()),
        }
    }

    fn navigate_down(&mut self) {
        match self.focused_panel {
            FocusedPanel::Unstaged => next_item(&mut self.unstaged_state, self.unstaged_changes.len()),
            FocusedPanel::Staged => next_item(&mut self.staged_state, self.staged_changes.len()),
        }
    }

    pub fn get_selected_unstaged_file(&self) -> Option<&StatusItem> {
        self.unstaged_state.selected().and_then(|i| self.unstaged_changes.get(i))
    }

    pub fn get_selected_staged_file(&self) -> Option<&StatusItem> {
        self.staged_state.selected().and_then(|i| self.staged_changes.get(i))
    }

    pub fn send_action(&self, action: Action) -> Result<()> {
        self.action_tx.send(action).map_err(|_| crate::error::Error::ChannelSend)
    }
}

fn next_item(state: &mut ListState, count: usize) {
    if count == 0 { state.select(None); return; }
    let i = match state.selected() {
        Some(i) => if i >= count - 1 { 0 } else { i + 1 },
        None => 0,
    };
    state.select(Some(i));
}

fn previous_item(state: &mut ListState, count: usize) {
    if count == 0 { state.select(None); return; }
    let i = match state.selected() {
        Some(i) => if i == 0 { count - 1 } else { i - 1 },
        None => 0,
    };
    state.select(Some(i));
}
