//! src/event.rs

use crate::error::{AppError, AppResult};
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum AppEvent {
    PushFinished(AppResult<()>),
}

/// Terminal events (user input)
#[derive(Debug)]
pub enum InputEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Tick,
}

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
                        match event::read() {
                            Ok(CrosstermEvent::Key(key)) => {
                                if input_tx.send(InputEvent::Key(key)).is_err() {
                                    break;
                                }
                            }

                            // Capture mouse events
                            Ok(CrosstermEvent::Mouse(mouse)) => {
                                if input_tx.send(InputEvent::Mouse(mouse)).is_err() {
                                    break;
                                }
                            }
                            _ => {} //Other events like Resize are ignored for now
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

    pub async fn next(&mut self) -> AppResult<Either<InputEvent,AppEvent>> {
        tokio::select! {
            Some(event) = self.input_rx.recv() => Ok(Either::Left(event)),
            Some(event) = self.app_rx.recv() => Ok(Either::Right(event)),
            else => Err(AppError::EventChannelClosed),
        }
    }

    pub fn get_app_event_sender(&self) -> mpsc::UnboundedSender<AppEvent> {
        self.app_tx.clone()
    }
}

pub enum Either<L, R> {
    Left(L),
    Right(R),
}
