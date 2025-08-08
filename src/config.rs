//! src/config.rs

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Representsthe keybindings for the application
pub struct KeyBindings {
    pub quit: KeyEvent, 
    pub show_help: KeyEvent,
    pub status_mode: KeyEvent,
    pub log_mode: KeyEvent,
    pub select_next: KeyEvent,
    pub select_prev: KeyEvent,
    pub stage_item: KeyEvent,
    pub unstage_item: KeyEvent,
    pub commit: KeyEvent,
    pub push: KeyEvent,
    pub confirm: KeyEvent,
    pub close_popup: KeyEvent,
    pub panel_right: KeyEvent,
    pub panel_left: KeyEvent,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            quit: KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
            show_help: KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE),
            status_mode: KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE),
            log_mode: KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
            select_next: KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE),
            stage_item: KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE),
            unstage_item: KeyEvent::new(KeyCode::Char('u'), KeyModifiers::NONE),
            commit: KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE),
            push: KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE),
            confirm: KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            close_popup: KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            panel_right: KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE),
            panel_left: KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE),
        }
    }
}
