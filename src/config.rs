//This module handles loading and saving the application's persistent state, specifically the path to the dotfiles directory and the remote Git URL. It's designed to be robust and platform-agnostic.

//Key features of this file:

//    Safe File Paths: Uses the directories crate to find the correct configuration directory on any OS (e.g., ~/.config/dotatui on Linux, ~/Library/Application Support/... on macOS).

//    Error Handling: All I/O and parsing operations return our custom Result<T> type, ensuring that any failures (like permission errors or malformed TOML) are handled gracefully.

//    Efficiency: The configuration is loaded only once at startup.


// src/config.rs

use crate::error::{Error, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Represents the application's configuration, which is persisted to a TOML file.
///
/// This struct is designed to be serialized and deserialized, allowing the application
/// to remember key information between runs.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Config {
    /// The absolute path to the user's dotfiles directory.
    /// This is the primary directory the application will manage.
    pub dotfiles_path: Option<PathBuf>,
    /// The URL of the 'origin' remote for the Git repository.
    /// This is stored to avoid repeatedly querying git for it.
    pub remote_url: Option<String>,
}

/// Locates the path to the configuration file, creating the directory if it doesn't exist.
///
/// This function uses the `directories` crate to find the appropriate system-specific
/// config location, ensuring cross-platform compatibility.
///
/// # Returns
/// A `Result` containing the `PathBuf` to the config file, or an `Error` if the
/// config directory cannot be determined or created.
fn get_config_path() -> Result<PathBuf> {
    // Use a reverse domain name notation for uniqueness.
    let proj_dirs =
        ProjectDirs::from("com", "SarangVehale", "DotaTUI").ok_or(Error::NoHomeDir)?;
    let config_dir = proj_dirs.config_dir();
    // Ensure the config directory exists before trying to write to it.
    fs::create_dir_all(config_dir)?;
    Ok(config_dir.join("config.toml"))
}

impl Config {
    /// Loads the configuration from the default file path.
    ///
    /// If the file doesn't exist, it returns a default `Config` instance.
    /// If the file exists but is malformed, it returns a `Config` error.
    ///
    /// # Returns
    /// A `Result` containing the loaded `Config` or an `Error`.
    pub fn load() -> Result<Self> {
        let path = get_config_path()?;
        if path.exists() {
            let content = fs::read_to_string(path)?;
            let config = toml::from_str(&content)?;
            Ok(config)
        } else {
            // If no config file is found, start with a fresh, default configuration.
            Ok(Config::default())
        }
    }

    /// Saves the current configuration to the default file path.
    ///
    /// This will overwrite the existing configuration file.
    ///
    /// # Returns
    /// An empty `Result` indicating success or an `Error` if saving fails.
    pub fn save(&self) -> Result<()> {
        let path = get_config_path()?;
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}
