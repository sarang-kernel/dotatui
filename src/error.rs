//! src/error.rs

use std::io;
use thiserror::Error;

/// The primary error type for the application.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("IO Error: {0}")]
    Io(#[from] io::Error),

    #[error("Git Error: {0}")]
    Git(#[from] git2::Error),

    #[error("Event channel closed unexpectedly")]
    EventChannelClosed,
    
    #[error("No git repository found at or above the current directory")]
    RepoNotFound,

    #[error("Push failed: {0}")]
    PushFailed(String),
}

/// A specialized `Result` type for application functions.
pub type AppResult<T> = Result<T, AppError>;
