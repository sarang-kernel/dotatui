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

/// Defines the different modes the application can be in, including a dedicated setup prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Search,
    CommitInput,
    Help,
    AddRemote,
    /// A new mode for when the app starts in a directory with no .git folder.
    InitRepoPrompt,
}

/// Represents all possible actions, now including one for initializing the repository.
#[derive(Debug)]
pub enum Action {
    Tick,
    Render,
    Quit,
    ToggleHelp,
    EnterSearch,
    EnterCommit,
    EnterAddRemote,
    EnterNormal,
    RefreshStatus,
    StatusUpdated(Result<Vec<StatusItem>>),
    AddAll,
    Commit,
    Push,
    PushCompleted(Result<()>),
    /// A new action to trigger repository initialization.
    InitRepo,
    AddRemote,
    NavigateUp,
    NavigateDown,
    NavigateTop,
    NavigateBottom,
    Input(char),
    InputDelete,
}

/// The main application state.
pub struct App {
    pub mode: AppMode,
    pub should_quit: bool,
    pub status_items: Vec<StatusItem>,
    pub filtered_items: Vec<usize>,
    pub list_state: ListState,
    pub input: String,
    pub message: String,
    pub is_loading: bool,
    pub action_tx: mpsc::UnboundedSender<Action>,
}

impl App {
    /// Creates a new `App` instance.
    pub fn new(action_tx: mpsc::UnboundedSender<Action>) -> Self {
        let mut app = App {
            // The initial mode is set to Normal here, but will be immediately
            // overridden in `main.rs` if a setup is required.
            mode: AppMode::Normal,
            should_quit: false,
            status_items: Vec::new(),
            filtered_items: Vec::new(),
            list_state: ListState::default(),
            input: String::new(),
            message: "Welcome to DotaTUI! Press 'r' to refresh status or '?' for help.".to_string(),
            is_loading: false,
            action_tx,
        };
        app.list_state.select(Some(0));
        app
    }

    /// The main state update function. It takes an action and modifies the app state accordingly.
    pub fn update(&mut self, action: &Action) -> Result<()> {
        match action {
            Action::Quit => self.should_quit = true,
            Action::ToggleHelp => {
                self.mode = if self.mode != AppMode::Help { AppMode::Help } else { AppMode::Normal };
            }
            Action::EnterSearch => self.mode = AppMode::Search,
            Action::EnterCommit => self.mode = AppMode::CommitInput,
            Action::EnterAddRemote => {
                self.message = "Enter the full SSH or HTTPS URL for the 'origin' remote.".to_string();
                self.mode = AppMode::AddRemote;
            }
            Action::EnterNormal => {
                self.input.clear();
                self.apply_filter();
                self.mode = AppMode::Normal;
            }
            Action::RefreshStatus => self.is_loading = true,
            Action::StatusUpdated(Ok(items)) => {
                self.is_loading = false;
                let old_item_count = self.status_items.len();
                self.status_items = items.clone();
                self.apply_filter();

                if self.status_items.is_empty() {
                    self.message = "Repository is clean. No changes found.".to_string();
                } else if self.status_items.len() == old_item_count {
                    self.message = format!("{} uncommitted changes found.", self.status_items.len());
                } else {
                    self.message = format!("Status updated. {} changes found.", self.status_items.len());
                }
            }
            Action::StatusUpdated(Err(e)) => {
                self.is_loading = false;
                self.message = format!("Error fetching status: {}", e);
            }
            Action::Push => {
                self.is_loading = true;
                self.message = "Pushing to remote...".to_string();
            }
            Action::PushCompleted(Ok(_)) => {
                self.is_loading = false;
                self.message = "Push successful.".to_string();
                self.send_action(Action::RefreshStatus)?;
            }
            Action::PushCompleted(Err(e)) => {
                self.is_loading = false;
                self.message = format!("Push failed: {}", e);
            }
            Action::NavigateUp => self.previous(),
            Action::NavigateDown => self.next(),
            Action::NavigateTop => self.go_to_top(),
            Action::NavigateBottom => self.go_to_bottom(),
            Action::Input(c) => self.input.push(*c),
            Action::InputDelete => {
                self.input.pop();
            }
            // Actions with side-effects like InitRepo are handled in main.rs
            _ => {}
        };

        if self.mode == AppMode::Search {
            self.apply_filter();
        }

        Ok(())
    }

    /// A helper function to send an action to the main event loop.
    pub fn send_action(&self, action: Action) -> Result<()> {
        self.action_tx.send(action).map_err(|_| crate::error::Error::ChannelSend)
    }

    /// Updates the `filtered_items` list based on the current search query in `self.input`.
    fn apply_filter(&mut self) {
        let query = self.input.to_lowercase();
        self.filtered_items = self.status_items.iter().enumerate()
            .filter(|(_, item)| item.path.to_lowercase().contains(&query))
            .map(|(i, _)| i)
            .collect();
        
        if self.filtered_items.is_empty() {
            self.list_state.select(None);
        } else {
            let new_selection = self.list_state.selected().map_or(0, |i| i.min(self.filtered_items.len() - 1));
            self.list_state.select(Some(new_selection));
        }
    }

    // --- Navigation Methods ---

    pub fn next(&mut self) {
        if self.filtered_items.is_empty() { return; }
        let i = match self.list_state.selected() {
            Some(i) => if i >= self.filtered_items.len() - 1 { 0 } else { i + 1 },
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.filtered_items.is_empty() { return; }
        let i = match self.list_state.selected() {
            Some(i) => if i == 0 { self.filtered_items.len() - 1 } else { i - 1 },
            None => 0,
        };
        self.list_state.select(Some(i));
    }
    
    pub fn go_to_top(&mut self) {
        if !self.filtered_items.is_empty() { self.list_state.select(Some(0)); }
    }

    pub fn go_to_bottom(&mut self) {
        if !self.filtered_items.is_empty() { self.list_state.select(Some(self.filtered_items.len() - 1)); }
    }
}
