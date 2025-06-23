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

use crate::app::{Action, App, PopupMode};
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

/// The core application loop.
async fn run_app(
    app: &mut App,
    tui: &mut Tui,
    action_rx: &mut UnboundedReceiver<Action>,
    config: &mut Config,
) -> Result<()> {
    let dotfiles_path = config.dotfiles_path.clone().unwrap();

    tui.enter()?;

    // Initial Repository Check
    match Repository::open(&dotfiles_path) {
        Ok(_) => {
            app.send_action(Action::RefreshStatus)?;
        }
        Err(_) => {
            // If no repo, immediately enter the InitRepo popup mode.
            app.send_action(Action::EnterPopup(PopupMode::InitRepo))?;
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

/// Handles actions with side effects.
async fn handle_action(
    action: &Action,
    app: &mut App,
    config: &mut Config,
    dotfiles_path: &PathBuf,
) -> Result<()> {
    // First, update the app's internal state based on the action.
    app.update(action)?;

    // Then, perform any side effects (like Git operations).
    match action {
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
        Action::StageFile => {
            if let Some(file) = app.get_selected_unstaged_file() {
                if let Ok(repo) = Repository::open(dotfiles_path) {
                    git_utils::stage_file(&repo, &file.path)?;
                    app.send_action(Action::RefreshStatus)?;
                }
            }
        }
        Action::UnstageFile => {
            if let Some(file) = app.get_selected_staged_file() {
                if let Ok(repo) = Repository::open(dotfiles_path) {
                    git_utils::unstage_file(&repo, &file.path)?;
                    app.send_action(Action::RefreshStatus)?;
                }
            }
        }
        Action::StageAll => {
            if let Ok(repo) = Repository::open(dotfiles_path) {
                git_utils::stage_all(&repo)?;
                app.send_action(Action::RefreshStatus)?;
            }
        }
        Action::UnstageAll => {
            if let Ok(repo) = Repository::open(dotfiles_path) {
                git_utils::unstage_all(&repo)?;
                app.send_action(Action::RefreshStatus)?;
            }
        }
        Action::Commit => {
            if !app.input.is_empty() {
                if let Ok(repo) = Repository::open(dotfiles_path) {
                    git_utils::commit(&repo, &app.input)?;
                    app.send_action(Action::ExitPopup)?;
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
                        tx.send(Action::EnterPopup(PopupMode::AddRemote)).unwrap_or_default();
                        return Err(error::Error::Git(git2::Error::from_str(
                            "No remote 'origin' found. Please add one.",
                        )));
                    }
                    git_utils::push(&repo)?;
                    Ok(())
                })();
                if result.is_ok() {
                    tx.send(Action::PushCompleted(result)).unwrap_or_default();
                }
            });
        }
        Action::ExecuteCommand => {
            if let Some(index) = app.menu_state.selected() {
                match app.menu_items.get(index).map(|s| s.as_str()) {
                    Some("Commit") => app.send_action(Action::EnterPopup(PopupMode::Commit))?,
                    Some("Push") => app.send_action(Action::Push)?,
                    Some("Stage All") => app.send_action(Action::StageAll)?,
                    Some("Unstage All") => app.send_action(Action::UnstageAll)?,
                    Some("Refresh") => app.send_action(Action::RefreshStatus)?,
                    Some("Init Repo") => app.send_action(Action::EnterPopup(PopupMode::InitRepo))?,
                    _ => {}
                }
            }
        }
        Action::InitRepo => {
            git_utils::init_repo(dotfiles_path)?;
            app.send_action(Action::ExitPopup)?;
            app.send_action(Action::EnterPopup(PopupMode::AddRemote))?;
        }
        Action::AddRemote => {
            if !app.input.is_empty() {
                if let Ok(repo) = Repository::open(dotfiles_path) {
                    git_utils::add_remote(&repo, &app.input)?;
                    config.remote_url = Some(app.input.clone());
                    config.save()?;
                    app.send_action(Action::ExitPopup)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}
