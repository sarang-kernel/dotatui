// src/main.rs

mod app;
mod config;
mod error;
mod git_utils;
mod tui;
mod ui;

use crate::app::{Action, App, FocusedPanel, PopupMode};
use crate::config::Config;
use crate::error::Result;
use crate::tui::Tui;
use git2::Repository;
use std::io;
use std::path::PathBuf;
use tokio::sync::mpsc::{self, UnboundedReceiver};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut config = Config::load()?;
    let mut tui = Tui::new()?;

    if config.dotfiles_path.is_none() {
        tui.enter()?;
        let path_str = tui.run_setup_prompt()?;
        config.dotfiles_path = Some(path_str.trim().into());
        config.save()?;
        tui.exit()?;
    }

    let mut dotfiles_path = config.dotfiles_path.clone().unwrap();
    let (action_tx, mut action_rx) = mpsc::unbounded_channel();
    let mut app = App::new(action_tx, dotfiles_path.clone());

    if let Err(e) = run_app(&mut app, &mut tui, &mut action_rx, &mut config, &mut dotfiles_path).await {
        tui.exit()?;
        eprintln!("FATAL ERROR: {}", e);
        std::process::exit(1);
    }

    Ok(())
}

async fn run_app(
    app: &mut App,
    tui: &mut Tui,
    action_rx: &mut UnboundedReceiver<Action>,
    config: &mut Config,
    dotfiles_path: &mut PathBuf,
) -> Result<()> {
    tui.enter()?;

    if Repository::open(&*dotfiles_path).is_ok() {
        app.send_action(Action::RefreshStatus)?;
    } else {
        app.is_loading = false;
        app.repo_status_summary = "Not a Git repository.".to_string();
        app.message = "Use 'Init Repo' command or '?' for help.".to_string();
    }

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

        handle_action(&action, app, config, dotfiles_path).await?;
    }

    tui.exit()?;
    Ok(())
}

fn refresh_diff(app: &App, dotfiles_path: &PathBuf) {
    let selected_file = match app.focused_panel {
        FocusedPanel::Unstaged => app.get_selected_unstaged_file(),
        FocusedPanel::Staged => app.get_selected_staged_file(),
    };

    let diff_text = if let Some(file) = selected_file {
        let tx = app.action_tx.clone();
        let path = dotfiles_path.clone();
        let file_path = file.path.clone();
        let is_staged = app.focused_panel == FocusedPanel::Staged;

        tokio::spawn(async move {
            if let Ok(repo) = Repository::open(path) {
                let diff = git_utils::get_file_diff(&repo, &file_path, is_staged)
                    .unwrap_or_else(|e| e.to_string());
                tx.send(Action::DiffUpdated(diff)).unwrap_or_default();
            }
        });
        "Loading diff...".to_string()
    } else {
        "No file selected.".to_string()
    };

    app.action_tx.send(Action::DiffUpdated(diff_text)).unwrap_or_default();
}

async fn handle_action(
    action: &Action,
    app: &mut App,
    config: &mut Config,
    dotfiles_path: &mut PathBuf,
) -> Result<()> {
    app.update(action)?;

    match action {
        Action::NavigateUp | Action::NavigateDown | Action::FocusNextPanel | Action::FocusPrevPanel | Action::StatusUpdated(_) => {
            refresh_diff(app, dotfiles_path);
        }
        _ => {}
    }

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
                    app.send_action(Action::GoToHome)?;
                    app.send_action(Action::RefreshStatus)?;
                }
            }
        }
        Action::ChangePath => {
            if !app.input.is_empty() {
                let new_path = PathBuf::from(app.input.clone());
                config.dotfiles_path = Some(new_path.clone());
                config.save()?;
                *dotfiles_path = new_path;
                app.dotfiles_path = dotfiles_path.clone();
                app.send_action(Action::ExitPopup)?;
                app.send_action(Action::GoToHome)?;
                app.send_action(Action::RefreshStatus)?;
            }
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
                tx.send(Action::PushCompleted(result)).unwrap_or_default();
            });
        }
        _ => {}
    }
    Ok(())
}
