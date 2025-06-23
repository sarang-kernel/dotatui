// This file defines the App struct, which holds the entire state of the running application. It also defines the Action enum, which represents every possible event or user command that can change the state. This "action-based" architecture is a robust pattern that separates "what happened" (the Action) from "how to handle it" (the update method).

// Key features of this file:

//    Centralized State: All dynamic data (current mode, list of files, user input, etc.) is stored in one App struct, making it easy to reason about the application's condition at any moment.

//    Action-Driven Logic: The update method acts as a reducer, taking the current state and an Action, and producing the new state. This makes the logic predictable and easy to test.

//    Asynchronous Communication: The App holds a tokio::mpsc::UnboundedSender<Action>. This allows background tasks (like a Git push) to send actions back to the main loop to update the UI (e.g., changing a "Pushing..." message to "Push successful.").

//    Efficiency: The state is only modified in response to actions. The filtering logic for search is efficient and only re-computes when necessary.

// src/app.rs

use crate::error::Result;
use crate::git_utils::StatusItem;
use ratatui::widgets::ListState;
use tokio::sync::mpsc;

/// Enum to track which UI panel is currently focused.
#[derive(Debug, PartialEq, Eq)]
pub enum FocusedPanel {
    Unstaged,
    Staged,
    Menu,
}

/// Enum for the different kinds of popups that can be active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PopupMode {
    Commit,
    AddRemote,
    Help,
    InitRepo,
}

/// Simplified AppMode for a more direct interaction model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Popup(PopupMode),
}

/// Actions are updated to reflect the new panel-based UI.
#[derive(Debug)]
pub enum Action {
    Tick,
    Render,
    Quit,
    ToggleHelp,
    EnterPopup(PopupMode),
    ExitPopup,
    RefreshStatus,
    StatusUpdated(Result<(Vec<StatusItem>, Vec<StatusItem>)>),
    StageFile,
    UnstageFile,
    StageAll,
    UnstageAll,
    Commit,
    Push,
    PushCompleted(Result<()>),
    InitRepo,
    AddRemote,
    FocusNextPanel,
    FocusPrevPanel,
    NavigateUp,
    NavigateDown,
    ExecuteCommand,
    Input(char),
    InputDelete,
}

/// The main application state, restructured for a panel-based UI.
pub struct App {
    pub mode: AppMode,
    pub focused_panel: FocusedPanel,
    pub unstaged_changes: Vec<StatusItem>,
    pub staged_changes: Vec<StatusItem>,
    pub unstaged_state: ListState,
    pub staged_state: ListState,
    pub menu_items: Vec<String>,
    pub menu_state: ListState,
    pub should_quit: bool,
    pub input: String,
    pub message: String,
    pub is_loading: bool,
    pub action_tx: mpsc::UnboundedSender<Action>,
}

impl App {
    pub fn new(action_tx: mpsc::UnboundedSender<Action>) -> Self {
        let mut app = App {
            mode: AppMode::Normal,
            focused_panel: FocusedPanel::Unstaged,
            unstaged_changes: Vec::new(),
            staged_changes: Vec::new(),
            unstaged_state: ListState::default(),
            staged_state: ListState::default(),
            menu_items: vec![
                "Commit".to_string(),
                "Push".to_string(),
                "Stage All".to_string(),
                "Unstage All".to_string(),
                "Refresh".to_string(),
                "Init Repo".to_string(),
            ],
            menu_state: ListState::default(),
            should_quit: false,
            input: String::new(),
            message: "Welcome to DotaTUI! Press 'Tab' to switch panels, '?' for help.".to_string(),
            is_loading: false,
            action_tx,
        };
        app.unstaged_state.select(Some(0));
        app.menu_state.select(Some(0));
        app
    }

    /// Main state update function.
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
            Action::RefreshStatus => self.is_loading = true,
            Action::StatusUpdated(Ok((unstaged, staged))) => {
                self.is_loading = false;
                self.unstaged_changes = unstaged.clone();
                self.staged_changes = staged.clone();
                self.check_selection_bounds();
                self.message = "Status refreshed.".to_string();
            }
            Action::StatusUpdated(Err(e)) => {
                self.is_loading = false;
                self.message = format!("Error: {}", e);
            }
            Action::FocusNextPanel => self.focus_next(),
            Action::FocusPrevPanel => self.focus_prev(),
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

    /// Ensures list selections are not out of bounds after data changes.
    fn check_selection_bounds(&mut self) {
        if self.unstaged_state.selected().is_some() && self.unstaged_state.selected().unwrap() >= self.unstaged_changes.len() {
            self.unstaged_state.select(self.unstaged_changes.len().checked_sub(1));
        }
        if self.staged_state.selected().is_some() && self.staged_state.selected().unwrap() >= self.staged_changes.len() {
            self.staged_state.select(self.staged_changes.len().checked_sub(1));
        }
    }

    /// Cycles focus to the next panel.
    fn focus_next(&mut self) {
        self.focused_panel = match self.focused_panel {
            FocusedPanel::Unstaged => FocusedPanel::Staged,
            FocusedPanel::Staged => FocusedPanel::Menu,
            FocusedPanel::Menu => FocusedPanel::Unstaged,
        }
    }

    /// Cycles focus to the previous panel.
    fn focus_prev(&mut self) {
        self.focused_panel = match self.focused_panel {
            FocusedPanel::Unstaged => FocusedPanel::Menu,
            FocusedPanel::Staged => FocusedPanel::Unstaged,
            FocusedPanel::Menu => FocusedPanel::Staged,
        }
    }

    /// Navigates up in the currently focused list.
    fn navigate_up(&mut self) {
        match self.focused_panel {
            FocusedPanel::Unstaged => previous_item(&mut self.unstaged_state, self.unstaged_changes.len()),
            FocusedPanel::Staged => previous_item(&mut self.staged_state, self.staged_changes.len()),
            FocusedPanel::Menu => previous_item(&mut self.menu_state, self.menu_items.len()),
        }
    }

    /// Navigates down in the currently focused list.
    fn navigate_down(&mut self) {
        match self.focused_panel {
            FocusedPanel::Unstaged => next_item(&mut self.unstaged_state, self.unstaged_changes.len()),
            FocusedPanel::Staged => next_item(&mut self.staged_state, self.staged_changes.len()),
            FocusedPanel::Menu => next_item(&mut self.menu_state, self.menu_items.len()),
        }
    }

    /// Gets the currently selected item in the unstaged changes panel.
    pub fn get_selected_unstaged_file(&self) -> Option<&StatusItem> {
        self.unstaged_state.selected().and_then(|i| self.unstaged_changes.get(i))
    }

    /// Gets the currently selected item in the staged changes panel.
    pub fn get_selected_staged_file(&self) -> Option<&StatusItem> {
        self.staged_state.selected().and_then(|i| self.staged_changes.get(i))
    }

    /// Helper to send an action to the main loop.
    pub fn send_action(&self, action: Action) -> Result<()> {
        self.action_tx.send(action).map_err(|_| crate::error::Error::ChannelSend)
    }
}

/// Helper function for moving to the next item in a list.
fn next_item(state: &mut ListState, count: usize) {
    if count == 0 {
        state.select(None);
        return;
    }
    let i = match state.selected() {
        Some(i) => if i >= count - 1 { 0 } else { i + 1 },
        None => 0,
    };
    state.select(Some(i));
}

/// Helper function for moving to the previous item in a list.
fn previous_item(state: &mut ListState, count: usize) {
    if count == 0 {
        state.select(None);
        return;
    }
    let i = match state.selected() {
        Some(i) => if i == 0 { count - 1 } else { i - 1 },
        None => 0,
    };
    state.select(Some(i));
}
