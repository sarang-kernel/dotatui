// This file is the application's entry point and orchestrator. It sets up the initial configuration, initializes the TUI and application state, and runs the main event loop.

// Key features of this file:

//    Asynchronous Main Loop: The #[tokio::main] macro sets up an async runtime. The main loop uses tokio::select! to concurrently handle user input events and actions coming from background tasks. This is the key to a responsive, non-blocking application.

//    Clear Orchestration: The main function handles the high-level setup. The run_app function contains the core event loop. The handle_action function is responsible for executing side effects (like Git operations) based on actions.

//    Side-Effect Management: State mutations are handled synchronously within app.update(). I/O-bound or slow operations (like git push or git status) are spawned as asynchronous tokio tasks. This keeps the UI thread free to handle user input and redraws.

//    First-Time Setup: It includes a user-friendly prompt to configure the dotfiles path on the very first run, making the tool easy to get started with.

// src/main.rs

mod app;
mod config;
mod error;
mod git_utils;
mod tui;
mod ui;

use crate::app::{Action, App, AppMode};
use crate::config::Config;
use crate::error::Result;
use crate::tui::Tui;
use git2::Repository;
use std::io;
use std::path::PathBuf;
use tokio::sync::mpsc::{self, UnboundedReceiver};

/// The main entry point of the application.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // --- One-time setup before TUI ---
    let mut config = Config::load()?;
    if config.dotfiles_path.is_none() {
        println!("Welcome to DotaTUI!");
        println!("Please provide the absolute path to your dotfiles directory:");
        let mut path_str = String::new();
        io::stdin().read_line(&mut path_str)?;
        config.dotfiles_path = Some(path_str.trim().into());
        config.save()?;
        println!("Configuration saved. Starting TUI...");
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    // --- App and TUI Initialization ---
    let (action_tx, mut action_rx) = mpsc::unbounded_channel();
    let mut app = App::new(action_tx);
    let mut tui = Tui::new()?;

    if let Err(e) = run_app(&mut app, &mut tui, &mut action_rx, &mut config).await {
        tui.exit()?;
        eprintln!("FATAL ERROR: {}", e);
        std::process::exit(1);
    }

    Ok(())
}

/// The core application loop, now with initial setup detection.
async fn run_app(
    app: &mut App,
    tui: &mut Tui,
    action_rx: &mut UnboundedReceiver<Action>,
    config: &mut Config,
) -> Result<()> {
    let dotfiles_path = config.dotfiles_path.clone().unwrap();

    tui.enter()?;

    // --- Initial Repository Check ---
    match Repository::open(&dotfiles_path) {
        Ok(_) => {
            // Repository exists, start normal operation.
            app.send_action(Action::RefreshStatus)?;
        }
        Err(_) => {
            // No repository found, enter setup mode.
            app.mode = AppMode::InitRepoPrompt;
            app.message = format!(
                "No git repository found in {:?}. Initialize one now?",
                dotfiles_path
            );
        }
    }

    // --- Main Loop ---
    while !app.should_quit {
        tui.draw(app)?;

        let action = tokio::select! {
            event_res = async { tui.handle_events(app, &app.action_tx) } => {
                event_res?;
                continue;
            },
            action = action_rx.recv() => {
                action.ok_or(error::Error::ChannelSend)?
            }
        };

        handle_action(&action, app, config, &dotfiles_path).await?;
    }

    tui.exit()?;
    Ok(())
}

/// Handles actions with side effects, now including `InitRepo`.
async fn handle_action(
    action: &Action,
    app: &mut App,
    _config: &mut Config,
    dotfiles_path: &PathBuf,
) -> Result<()> {
    app.update(action)?;

    match action {
        Action::InitRepo => {
            git_utils::init_repo(dotfiles_path)?;
            // After initializing, immediately prompt for the remote URL for a smooth setup flow.
            app.send_action(Action::EnterAddRemote)?;
        }
        Action::RefreshStatus => {
            let tx = app.action_tx.clone();
            let path = dotfiles_path.clone();
            tokio::spawn(async move {
                let status_result = (|| -> Result<_> {
                    let repo = Repository::open(path)?;
                    Ok(git_utils::get_status(&repo)?)
                })();
                tx.send(Action::StatusUpdated(status_result)).unwrap_or_default();
            });
        }
        Action::AddAll => {
            if let Ok(repo) = Repository::open(dotfiles_path) {
                git_utils::add_all(&repo)?;
                app.send_action(Action::RefreshStatus)?;
            }
        }
        Action::Commit => {
            if let (Ok(repo), AppMode::CommitInput) = (Repository::open(dotfiles_path), &app.mode) {
                if !app.input.is_empty() {
                    git_utils::commit(&repo, &app.input)?;
                    app.send_action(Action::EnterNormal)?;
                    app.send_action(Action::RefreshStatus)?;
                }
            }
        }
        Action::Push => {
            let tx = app.action_tx.clone();
            let path = dotfiles_path.clone();
            tokio::spawn(async move {
                let result = (|| -> Result<()> {
                    let repo = Repository::open(&path)?;
                    if !git_utils::has_remote(&repo) {
                        // If user tries to push without a remote, guide them to add one.
                        tx.send(Action::EnterAddRemote).unwrap_or_default();
                        // Return a user-friendly error to display.
                        return Err(error::Error::Git(git2::Error::from_str(
                            "No remote 'origin' found. Please add one.",
                        )));
                    }
                    git_utils::push(&repo)?;
                    Ok(())
                })();
                // Only send PushCompleted if we actually tried to push.
                if result.is_ok() {
                    tx.send(Action::PushCompleted(result)).unwrap_or_default();
                }
            });
        }
        Action::AddRemote => {
            if let (Ok(repo), AppMode::AddRemote) = (Repository::open(dotfiles_path), &app.mode) {
                if !app.input.is_empty() {
                    git_utils::add_remote(&repo, &app.input)?;
                    config.remote_url = Some(app.input.clone());
                    config.save()?;
                    app.message = "Remote 'origin' added successfully.".to_string();
                    app.send_action(Action::EnterNormal)?;
                    app.send_action(Action::RefreshStatus)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}
