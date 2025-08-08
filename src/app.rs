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

    pub fn handle_mouse_event(&mut self, event: MouseEvent) -> AppResult<()> {
        if let Mode::Status(_) = self.mode {
            let layout = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Directoin::Horizontal)
                .constraints([ratatui::Layout::Constraint::Percentage(40), ratatui::layout::Constraint::Percentage(60)].as_ref())
                .split(Rect { x: 0, y: 1, width: 999, height: 999 });

            let files_panel_rect = layout[0];
            let diff_panel_rect = layout[1];

            match event.kind {
                MouseEventKind::ScrollUp => {
                    if self.active_panel == ActivePanel::Files {
                        self.select_previous_status_item();
                    }

                    // Future scroll diff panel
                }
                MouseEventKind::ScrollDown => {
                    if self.active_panel == ActivePanel::Files {
                        self.select_next_status_item();
                    }
                }
                MouseEventKind::Down(_) => {
                    if is_inside(event.column, event.row, files_panel_rect) {
                        self.active_panel = ActivePanel::Files;
                        // y - top_border - top_of_panel
                        let index = (event.row - 1 - files_panel_rect.y) as usize;
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
                    ActivePanel::Diff=> {
                        // Future: diff scrollling keys 
                    }
                }

                if key == self.keys.log_mode { self.mode = Mode::Log; }
                else if key == self.keys.commit { self.popup = Some(Popup::Commit); }
                else if key == self.keys.push { self.push_to_remote(); }
            }
            StatusMode::HunkSelection => {
                
            }
        }
    }
}
