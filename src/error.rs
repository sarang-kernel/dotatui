//This is one of the most critical files for creating production-ready software. Instead of letting the program panic or using generic error types, we define a single, comprehensive Error enum for our entire application. Using the thiserror crate, we can cleanly represent every possible failure state, from file I/O issues to Git command failures.

//This approach makes our code:

//    Robust: We explicitly handle all expected errors.

//    Maintainable: All error types are in one place.

//    Ergonomic: The ? operator can be used throughout the codebase to propagate errors cleanly up to the main loop.

// src/error.rs

use std::io;
use thiserror::Error;

/// A centralized error type for the entire application. 
///
/// This enum consolidates all possible errors from different parts of the application, such as I/O, Git operations, configurations parsing, and asynchronous channel communications.
/// Using 'thiserror', we can automatically implement the 'std::error::Error' trait and provide conversions from underlying error types.

#[derive(Error, Debug)]
pub enum Error{
    /// Represents an error related to file or network I/O.
    /// This is a wrapper around 'std::io::Error'.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Represents an error originating from the 'git2' library.
    /// This captures all failures related to Git operations like status, commit, push, etc.
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    /// Represents an error that occurs when parsing the 'config.toml' file.
    /// This is a wrapper around 'toml::de::Error'.
    #[error("Configuration parsing error: {0}")]
    Config(#[from] toml::de::Error),

    /// Represents an error that occurs when serializing the application's config to TOML format.
    /// This is a wrapper around 'toml::ser::Error'.
    #[error("Configuration serialization error: {0}")]
    ConfigSerizalization(#[from] toml::ser::Error),

    /// A custom error for when the application cannot determine a valid home or config directory.
    #[error("Could not determine a valid home directory")]
    NoHomeDir,


    /// An error for when an asynchronous channel operation fails. This typically happens if the receiving end of a channel is dropped while a message is being sent, which can occur during application shutdown.
    #[error("Tokio channel send error")]
    ChannelSend,
}

/// A type alias for 'std::result::Result<T, crate::error::Error>'.
///
/// This simplifies function signatures throughout the application, making it clear that a function can return our custom 'Error' type.

pub type Result<T> = std::result::Result<T, Error>;
