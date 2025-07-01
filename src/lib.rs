//! src/lib.rs

/// Core application state and logic.
pub mod app;
/// Keybinding configuration.
pub mod config;
/// Custom error types.
pub mod error;
/// Event handling (input and custom app events).
pub mod event;
/// Git repository interactions.
pub mod git;
/// Terminal User Interface setup and teardown.
pub mod tui;
/// UI rendering logic.
pub mod ui;
