//! src/main.rs

use dotatui::{
    app::{App, AppReturn},
    error::{AppError,AppResult},
    event::{AppEvent, Either, EventHandler, InputEvent},
    git::GitRepo,
    tui::Tui,
};
use std::{env, fs::File};

use log::LevelFilter;
use simplelog::{Config, WriteLogger};

#[tokio::main]
async fn main() -> AppResult<()> {
    // We can't initialize the logger yet, as the repo path might change the CWD.

    // --- NEW: Find and Set Current Directory ---
    // Discover the repository path from the current directory.
    let repo_path_raw = git2::Repository::discover(env::current_dir()?)?
        .path()
        .parent()
        .ok_or(AppError::RepoNotFound)?
        .to_path_buf();

    // Change the application's current directory to the repository root.
    // This ensures all subsequent file operations are relative to the correct path.
    env::set_current_dir(&repo_path_raw)?;
    // --- End New Section ---


    // Now that we are in the correct directory, we can initialize the logger.
    WriteLogger::init(
        LevelFilter::Debug,
        Config::default(),
        File::create("dotatui.log")?,
    )
    .expect("Failed to initialize logger");

    log::info!("Dotatui started in repository: {:?}", repo_path_raw);

    // Initialize git repo using the now-current directory.
    let repo = GitRepo::new(".")?;

    // Initialize terminal UI and event handler
    let mut tui = Tui::new()?;
    tui.enter()?;
    let mut event_handler = EventHandler::new();

    // Create and run the application
    let mut app = App::new(repo, &event_handler);

    while !app.is_exiting() {
        tui.draw(|frame| {
            dotatui::ui::render(frame, &mut app);
        })?;

        match event_handler.next().await? {
            Either::Left(InputEvent::Key(key_event)) => {
                if app.handle_key_event(key_event)? == AppReturn::Exit {
                    break;
                }
            }
            Either::Right(AppEvent::PushFinished(result)) => {
                app.handle_app_event(AppEvent::PushFinished(result))?;
            }
            _ => {}
        }
    }

    // Restore the terminal
    tui.exit()?;
    Ok(())
}
