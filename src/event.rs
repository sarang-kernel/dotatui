//! src/event.rs

use crate::error::{AppError, AppResult};
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::time::Duration;
use tokio::sync::mpsc;

/// Application-level events, used for communication between tasks.
#[derive(Debug)]
pub enum AppEvent {
    PushFinished(AppResult<()>),
    // Add other events like FetchFinished, etc.
}

/// Terminal events (user input).
#[derive(Debug)]
pub enum InputEvent {
    Key(KeyEvent),
    Tick,
}

/// The main event handler for the application.
pub struct EventHandler {
    input_rx: mpsc::UnboundedReceiver<InputEvent>,
    app_rx: mpsc::UnboundedReceiver<AppEvent>,
    app_tx: mpsc::UnboundedSender<AppEvent>,
    _input_handle: tokio::task::JoinHandle<()>,
}

impl EventHandler {
    pub fn new() -> Self {
        let (input_tx, input_rx) = mpsc::unbounded_channel();
        let (app_tx, app_rx) = mpsc::unbounded_channel();

        let input_handle = {
            tokio::spawn(async move {
                loop {
                    if event::poll(Duration::from_millis(100)).unwrap_or(false) {
                        if let Ok(CrosstermEvent::Key(key)) = event::read() {
                            if input_tx.send(InputEvent::Key(key)).is_err() {
                                break;
                            }
                        }
                    }
                    if input_tx.send(InputEvent::Tick).is_err() {
                        break;
                    }
                }
            })
        };

        Self {
            input_rx,
            app_rx,
            app_tx,
            _input_handle: input_handle,
        }
    }

    /// Receive the next event, from either user input or app-internal messages.
    pub async fn next(&mut self) -> AppResult<Either<InputEvent, AppEvent>> {
        tokio::select! {
            Some(event) = self.input_rx.recv() => Ok(Either::Left(event)),
            Some(event) = self.app_rx.recv() => Ok(Either::Right(event)),
            else => Err(AppError::EventChannelClosed),
        }
    }
    
    /// Get a sender to dispatch application-level events.
    pub fn get_app_event_sender(&self) -> mpsc::UnboundedSender<AppEvent> {
        self.app_tx.clone()
    }
}

/// A simple enum to represent one of two possible types.
pub enum Either<L, R> {
    Left(L),
    Right(R),
}
