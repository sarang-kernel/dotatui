//! src/main.rs

use dotatui::{
    app::{App, AppReturn},
    error::{AppError, AppResult},
    event::{AppEvent, Either, EventHandler, InputEvent},
    git::GitRepo,
    tui::Tui,
};
use std::{env, fs::File};

use log::LevelFilter;
use simplelog::{Config, WriteLogger};

#[tokio::main]
async fn main() -> AppResult<()> {
    let repo_path_raw = git2::Repository::discover(env::current_dir()?)?
        .path()
        .parent()
        .ok_or(AppError::RepoNotFound)?
        .to_path_buf();

    env::set_current_dir(&repo_path_raw)?;

    WriteLogger::init(
        LevelFilter::Debug,
        Config::default(),
        File::create("dotatui.log")?,
    )
    .expect("Failed to initialize logger");

    log::info!("Dotatui started in repository: {:?}", repo_path_raw);

    let repo = GitRepo::new(".")?;

    let mut tui = Tui::new()?;
    tui.enter()?;
    let mut event_handler = EventHandler::new();

    let mut app = App::new(repo, &event_handler);

    while !app.is_exiting() {
        tui.draw(|frame| {
            dotatui::ui::render(frame, &mut app);
        })?;

        // Update the main event loop match
        match event_handler.next().await? {
            Either::Left(InputEvent::Key(key_event)) => {
                if app.handle_key_event(key_event)? == AppReturn::Exit {
                    break;
                }
            }
            // Add a new arm for Mouse events
            Either::Left(InputEvent::Mouse(mouse_event)) => {
                app.handle_mouse_event(mouse_event)?;
            }
            Either::Right(AppEvent::PushFinished(result)) => {
                app.handle_app_event(AppEvent::PushFinished(result))?;
            }
            _ => {}
        }
    }

    tui.exit()?;
    Ok(())
}
