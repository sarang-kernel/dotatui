//! src/app.rs

use crate::{
    config::KeyBindings,
    error::{AppError, AppResult},
    event::{AppEvent, EventHandler},
    git::{CommitInfo, GitRepo, Hunk, StatusItem},
};
use crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use log::{debug, error, info};
use ratatui::{layout::Rect, widgets::ListState, widgets::TableState};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum StatusItemType {
    Header(String),
    Item(StatusItem),
}

#[derive(Debug, PartialEq, Eq)]
pub enum AppReturn {
    Continue,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusMode {
    FileSelection,
    HunkSelection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Status(StatusMode),
    Log,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Popup {
    Help,
    Commit,
    Pushing(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePanel {
    Files,
    Diff,
}

pub struct App {
    pub repo: GitRepo,
    pub keys: KeyBindings,
    pub mode: Mode,
    pub popup: Option<Popup>,
    pub status_display_list: Vec<StatusItemType>,
    pub status_list_state: ListState,
    pub log_entries: Vec<CommitInfo>,
    pub log_table_state: TableState,
    pub commit_msg: String,
    pub cursor_pos: usize,
    exiting: bool,
    app_event_sender: mpsc::UnboundedSender<AppEvent>,
    pub current_hunks: Vec<Hunk>,
    pub hunk_list_state: ListState,
    pub active_panel: ActivePanel,
}

impl App {
    pub fn new(repo: GitRepo, event_handler: &EventHandler) -> Self {
        let mut app = Self {
            repo,
            keys: KeyBindings::default(),
            mode: Mode::Status(StatusMode::FileSelection),
            popup: None,
            status_display_list: Vec::new(),
            status_list_state: ListState::default(),
            log_entries: Vec::new(),
            log_table_state: TableState::default(),
            commit_msg: String::new(),
            cursor_pos: 0,
            exiting: false,
            app_event_sender: event_handler.get_app_event_sender(),
            current_hunks: Vec::new(),
            hunk_list_state: ListState::default(),
            active_panel: ActivePanel::Files,
        };
        app.refresh().unwrap();
        app
    }

    pub fn is_exiting(&self) -> bool {
        self.exiting
    }

    pub fn refresh(&mut self) -> AppResult<()> {
        info!("Refreshing app state...");
        let raw_status_items = self.repo.get_status()?;
        self.log_entries = self.repo.get_log()?;
        self.status_display_list.clear();
        let (staged, unstaged): (Vec<_>, Vec<_>) =
            raw_status_items.into_iter().partition(|i| i.is_staged);

        if !staged.is_empty() {
            self.status_display_list
                .push(StatusItemType::Header("Staged changes:".to_string()));
            self.status_display_list
                .extend(staged.into_iter().map(StatusItemType::Item));
        }
        if !unstaged.is_empty() {
            self.status_display_list
                .push(StatusItemType::Header("Unstaged changes:".to_string()));
            self.status_display_list
                .extend(unstaged.into_iter().map(StatusItemType::Item));
        }

        info!(
            "Refresh complete. Display list has {} items.",
            self.status_display_list.len()
        );

        if self.status_display_list.is_empty() {
            self.status_list_state.select(None);
        } else {
            if let Some(selected) = self.status_list_state.selected() {
                if selected >= self.status_display_list.len() {
                    self.status_list_state
                        .select(Some(self.status_display_list.len() - 1));
                }
            } else {
                self.status_list_state.select(Some(0));
            }
            self.skip_headers_forward();
        }

        if self.log_entries.is_empty() {
            self.log_table_state.select(None);
        } else if self.log_table_state.selected().is_none() {
            self.log_table_state.select(Some(0));
        }
        Ok(())
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> AppResult<AppReturn> {
        debug!("Received key event: {:?}", key.code);
        if self.popup.is_some() {
            let popup = self.popup.clone().unwrap();
            return self.handle_popup_keys(key, popup);
        }
        if key == self.keys.quit {
            if let Mode::Status(StatusMode::HunkSelection) = self.mode {
                info!("Quitting HunkSelection mode, returning to FileSelection");
                self.mode = Mode::Status(StatusMode::FileSelection);
                self.current_hunks.clear();
                self.hunk_list_state.select(None);
                return Ok(AppReturn::Continue);
            }
            self.exiting = true;
            return Ok(AppReturn::Exit);
        }
        if key == self.keys.show_help {
            self.popup = Some(Popup::Help);
            return Ok(AppReturn::Continue);
        }
        match self.mode {
            Mode::Status(sub_mode) => self.handle_status_keys(key, sub_mode)?,
            Mode::Log => self.handle_log_keys(key)?,
        }
        Ok(AppReturn::Continue)
    }

    pub fn handle_mouse_event(&mut self, event: MouseEvent) -> AppResult<()> {
        debug!("Received mouse event: {:?}", event);
        if let Mode::Status(_) = self.mode {
            // This is a simplified calculation and may need adjustment based on final layout.
            // It assumes the status view starts at y=1 and the files panel is 40% of the width.
            let terminal_width = 200; // A reasonable assumption, adjust if needed.
            let files_panel_width = (terminal_width as f32 * 0.4) as u16;

            let files_panel_rect = Rect::new(0, 1, files_panel_width, 999);
            let diff_panel_rect = Rect::new(files_panel_width, 1, terminal_width - files_panel_width, 999);

            match event.kind {
                MouseEventKind::ScrollUp => {
                    if self.active_panel == ActivePanel::Files {
                        self.select_previous_status_item();
                    }
                }
                MouseEventKind::ScrollDown => {
                    if self.active_panel == ActivePanel::Files {
                        self.select_next_status_item();
                    }
                }
                MouseEventKind::Down(_) => {
                    if is_inside(event.column, event.row, files_panel_rect) {
                        self.active_panel = ActivePanel::Files;
                        let index = (event.row.saturating_sub(1)) as usize;
                        if index < self.status_display_list.len() {
                            self.status_list_state.select(Some(index));
                            self.skip_headers_forward();
                        }
                    } else if is_inside(event.column, event.row, diff_panel_rect) {
                        self.active_panel = ActivePanel::Diff;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn handle_app_event(&mut self, event: AppEvent) -> AppResult<()> {
        match event {
            AppEvent::PushFinished(result) => {
                let msg = match result {
                    Ok(_) => {
                        info!("Async push operation completed successfully.");
                        "Push successful!".to_string()
                    }
                    Err(e) => {
                        error!("Async push operation failed: {}", e);
                        format!("Push failed: {}", e)
                    }
                };
                self.popup = Some(Popup::Pushing(msg));
            }
        }
        Ok(())
    }

    fn handle_popup_keys(&mut self, key: KeyEvent, popup: Popup) -> AppResult<AppReturn> {
        match popup {
            Popup::Commit => {
                if key == self.keys.close_popup {
                    self.popup = None;
                } else if key == self.keys.confirm {
                    self.submit_commit()?;
                } else {
                    self.handle_commit_input(key);
                }
            }
            _ => {
                if key == self.keys.close_popup || key == self.keys.confirm {
                    self.popup = None;
                    if let Popup::Pushing(_) = popup {
                        self.refresh()?;
                    }
                }
            }
        }
        Ok(AppReturn::Continue)
    }

    fn handle_status_keys(&mut self, key: KeyEvent, sub_mode: StatusMode) -> AppResult<()> {
        if key == self.keys.panel_left {
            self.active_panel = ActivePanel::Files;
            return Ok(());
        }
        if key == self.keys.panel_right {
            self.active_panel = ActivePanel::Diff;
            return Ok(());
        }

        match sub_mode {
            StatusMode::FileSelection => {
                match self.active_panel {
                    ActivePanel::Files => {
                        if key == self.keys.select_next { self.select_next_status_item(); }
                        else if key == self.keys.select_prev { self.select_previous_status_item(); }
                        else if key == self.keys.stage_item { self.stage_selected()?; }
                        else if key == self.keys.unstage_item { self.unstage_selected()?; }
                        else if key == self.keys.confirm {
                            if let Some(item) = self.get_selected_status_item() {
                                self.current_hunks = self.repo.get_diff_hunks(&item)?;
                                if !self.current_hunks.is_empty() {
                                    info!("Entering HunkSelection mode for file: {}", item.path);
                                    self.mode = Mode::Status(StatusMode::HunkSelection);
                                    self.hunk_list_state.select(Some(0));
                                } else {
                                    info!("No hunks to select for file: {}", item.path);
                                }
                            }
                        }
                    }
                    ActivePanel::Diff => {}
                }

                if key == self.keys.log_mode { self.mode = Mode::Log; }
                else if key == self.keys.commit { self.popup = Some(Popup::Commit); }
                else if key == self.keys.push { self.push_to_remote(); }
            }
            StatusMode::HunkSelection => {
                if key == self.keys.select_next { self.select_next_hunk(); }
                else if key == self.keys.select_prev { self.select_previous_hunk(); }
            }
        }
        Ok(())
    }

    fn handle_log_keys(&mut self, key: KeyEvent) -> AppResult<()> {
        if let Mode::Status(_) = self.mode {
            self.mode = Mode::Status(StatusMode::FileSelection);
        } else if key == self.keys.select_next {
            self.select_next_log_item();
        } else if key == self.keys.select_prev {
            self.select_previous_log_item();
        }
        Ok(())
    }

    fn handle_commit_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(c) => {
                self.commit_msg.insert(self.cursor_pos, c);
                self.cursor_pos += 1;
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.commit_msg.remove(self.cursor_pos);
                }
            }
            KeyCode::Left => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_pos < self.commit_msg.len() {
                    self.cursor_pos += 1;
                }
            }
            _ => {}
        }
    }

    fn stage_selected(&mut self) -> AppResult<()> {
        if let Some(item) = self.get_selected_status_item() {
            if !item.is_staged {
                info!("Staging item: {}", item.path);
                self.repo.stage_item(&item)?;
                self.refresh()?;
            }
        }
        Ok(())
    }

    fn unstage_selected(&mut self) -> AppResult<()> {
        if let Some(item) = self.get_selected_status_item() {
            if item.is_staged {
                info!("Unstaging file: {}", item.path);
                self.repo.unstage_file(&item.path)?;
                self.refresh()?;
            }
        }
        Ok(())
    }

    fn submit_commit(&mut self) -> AppResult<()> {
        if !self.commit_msg.is_empty() {
            info!("Attempting to commit with message: '{}'", self.commit_msg);
            self.repo.commit(&self.commit_msg)?;
            info!("Commit successful.");
            self.commit_msg.clear();
            self.cursor_pos = 0;
            self.popup = None;
            self.refresh()?;
        }
        Ok(())
    }

    fn push_to_remote(&mut self) {
        info!("Spawning background task for git push.");
        self.popup = Some(Popup::Pushing("Pushing...".to_string()));
        let repo_path = self.repo.path().to_path_buf();
        let sender = self.app_event_sender.clone();
        tokio::spawn(async move {
            let push_result = async {
                let repo = git2::Repository::open(repo_path)?;
                let mut remote = repo.find_remote("origin")?;
                let mut callbacks = git2::RemoteCallbacks::new();
                callbacks.credentials(|_url, username, _| {
                    git2::Cred::ssh_key_from_agent(username.unwrap_or("git"))
                });
                let mut push_options = git2::PushOptions::new();
                push_options.remote_callbacks(callbacks);
                let head = repo.head()?;
                let head_name = head.shorthand().unwrap_or("main");
                let refspec = format!("refs/heads/{}:refs/heads/{}", head_name, head_name);
                remote
                    .push(&[refspec], Some(&mut push_options))
                    .map_err(|e| AppError::PushFailed(e.to_string()))
            }
            .await;
            let _ = sender.send(AppEvent::PushFinished(push_result));
        });
    }

    fn select_next_status_item(&mut self) {
        if self.status_display_list.is_empty() { return; }
        let selected = self.status_list_state.selected().unwrap_or(0);
        let new_selected = if selected >= self.status_display_list.len() - 1 { 0 } else { selected + 1 };
        self.status_list_state.select(Some(new_selected));
        self.skip_headers_forward();
    }

    fn select_previous_status_item(&mut self) {
        if self.status_display_list.is_empty() { return; }
        let selected = self.status_list_state.selected().unwrap_or(0);
        let new_selected = if selected == 0 { self.status_display_list.len() - 1 } else { selected - 1 };
        self.status_list_state.select(Some(new_selected));
        self.skip_headers_backward();
    }

    fn skip_headers_forward(&mut self) {
        if let Some(selected) = self.status_list_state.selected() {
            if let Some(item_type) = self.status_display_list.get(selected) {
                if matches!(item_type, StatusItemType::Header(_)) {
                    if selected >= self.status_display_list.len() - 1 {
                        self.status_list_state.select(Some(0));
                        self.skip_headers_forward();
                    } else {
                        self.select_next_status_item();
                    }
                }
            }
        }
    }

    fn skip_headers_backward(&mut self) {
        if let Some(selected) = self.status_list_state.selected() {
            if let Some(item_type) = self.status_display_list.get(selected) {
                if matches!(item_type, StatusItemType::Header(_)) {
                    if selected == 0 {
                        self.status_list_state.select(Some(self.status_display_list.len() - 1));
                        self.skip_headers_backward();
                    } else {
                        self.select_previous_status_item();
                    }
                }
            }
        }
    }

    pub fn get_selected_status_item(&self) -> Option<StatusItem> {
        self.status_list_state
            .selected()
            .and_then(|i| self.status_display_list.get(i))
            .and_then(|item_type| match item_type {
                StatusItemType::Item(item) => Some(item.clone()),
                StatusItemType::Header(_) => None,
            })
    }

    fn select_next_hunk(&mut self) {
        if self.current_hunks.is_empty() { return; }
        let i = self.hunk_list_state.selected().map_or(0, |i| (i + 1) % self.current_hunks.len());
        self.hunk_list_state.select(Some(i));
        debug!("Selected hunk index: {}", i);
    }

    fn select_previous_hunk(&mut self) {
        if self.current_hunks.is_empty() { return; }
        let i = self.hunk_list_state.selected().map_or(0, |i| {
            if i == 0 { self.current_hunks.len() - 1 } else { i - 1 }
        });
        self.hunk_list_state.select(Some(i));
        debug!("Selected hunk index: {}", i);
    }

    fn select_next_log_item(&mut self) {
        if self.log_entries.is_empty() { return; }
        let i = self.log_table_state.selected().map_or(0, |i| (i + 1) % self.log_entries.len());
        self.log_table_state.select(Some(i));
    }

    fn select_previous_log_item(&mut self) {
        if self.log_entries.is_empty() { return; }
        let i = self.log_table_state.selected().map_or(0, |i| {
            if i == 0 { self.log_entries.len() - 1 } else { i - 1 }
        });
        self.log_table_state.select(Some(i));
    }
}

fn is_inside(cx: u16, cy: u16, rect: Rect) -> bool {
    cx >= rect.x && cx < rect.x + rect.width && cy >= rect.y && cy < rect.y + rect.height
}
