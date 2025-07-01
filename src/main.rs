//! src/main.rs

use dotatui::{
    app::{App, AppReturn},
    error::AppResult,
    event::{AppEvent, Either, EventHandler, InputEvent},
    git::GitRepo,
    tui::Tui,
};
use std::env;

#[tokio::main]
async fn main() -> AppResult<()> {
    // Initialize git repo
    let repo_path = env::current_dir()?;
    let repo = GitRepo::new(repo_path)?;

    // Initialize terminal UI and event handler
    let mut tui = Tui::new()?;
    tui.enter()?;
    let mut event_handler = EventHandler::new();

    // Create and run the application
    let mut app = App::new(repo, &event_handler);

    while !app.is_exiting() {
        // Render the UI
        tui.draw(|frame| {
            dotatui::ui::render(frame, &mut app);
        })?;

        // Handle events
        match event_handler.next().await? {
            Either::Left(InputEvent::Key(key_event)) => {
                if app.handle_key_event(key_event)? == AppReturn::Exit {
                    break;
                }
            }
            Either::Right(AppEvent::PushFinished(result)) => {
                app.handle_app_event(AppEvent::PushFinished(result))?;
            }
            _ => {} // Ticks are ignored for now
        }
    }

    // Restore the terminal
    tui.exit()?;
    Ok(())
}
