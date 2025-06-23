// This file contains all the rendering logic. It takes the application state (App) and a ratatui Frame and draws the widgets. By keeping all drawing code here, we maintain a clean separation between application logic (state changes) and presentation (how the state is displayed).

// Key features of this file:

//    Declarative UI: The code describes what to draw, and ratatui handles how to draw it. This makes the UI code easy to read and modify.

//    Component-Based: The UI is broken down into smaller, manageable functions (render_main_list, render_status_bar, render_popup), making the code reusable and organized.

//    State-Driven: The UI is a pure function of the App state. For example, it shows a loading indicator if app.is_loading is true, and it displays different popups based on app.mode.

//    Efficiency: It only draws what's necessary for the current frame. The Clear widget is used for popups to avoid redrawing the entire screen, which is a minor but good optimization.

// src/ui.rs

use crate::app::{App, AppMode};
use crate::git_utils::FileStatus;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

/// The main drawing function. It now checks for setup mode first.
pub fn draw(f: &mut Frame, app: &mut App) {
    // If in setup mode, draw a dedicated screen and stop further rendering.
    if app.mode == AppMode::InitRepoPrompt {
        render_setup_prompt(f, app);
        return;
    }

    // Otherwise, draw the normal UI.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
        .split(f.size());

    render_main_list(f, app, chunks[0]);
    render_status_bar(f, app, chunks[1]);

    // Render popups on top of the normal UI.
    match &app.mode {
        AppMode::Help => render_popup(f, "Help", render_help_text()),
        AppMode::Search => render_input_popup(f, app, "Search"),
        AppMode::CommitInput => render_input_popup(f, app, "Commit Message"),
        AppMode::AddRemote => render_input_popup(f, app, "Input Remote URL"),
        _ => {}
    }
}

/// A new function to render the initial setup prompt.
fn render_setup_prompt(f: &mut Frame, app: &App) {
    let text = vec![
        Line::from(""),
        Line::from("Welcome to DotaTUI Setup").style(Style::default().bold()),
        Line::from(""),
        Line::from(Span::raw(&app.message)),
        Line::from(""),
        Line::from(vec![
            Span::styled("y", Style::default().fg(Color::Green).bold()),
            Span::raw("es / "),
            Span::styled("n", Style::default().fg(Color::Red).bold()),
            Span::raw("o (or q to quit)"),
        ]),
    ];
    let prompt = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Setup Required"))
        .alignment(Alignment::Center);
    f.render_widget(prompt, f.size());
}

/// Renders the main list of file statuses, or a help message if the list is empty.
fn render_main_list(f: &mut Frame, app: &mut App, area: Rect) {
    if app.filtered_items.is_empty() && !app.is_loading {
        let help_text = Paragraph::new(vec![
            Line::from("").style(Style::default()),
            Line::from(" ✔ Repository is clean ").style(Style::default().fg(Color::Green)),
            Line::from(""),
            Line::from("   Press 'r' to refresh status"),
            Line::from("   Press 'q' to quit"),
            Line::from("   Press '?' for all commands"),
        ])
        .block(Block::default().borders(Borders::ALL).title("Dotfiles Status"))
        .alignment(Alignment::Center);
        f.render_widget(help_text, area);
        return;
    }

    let items: Vec<ListItem> = app
        .filtered_items
        .iter()
        .map(|&i| {
            let item = &app.status_items[i];
            let style = match item.status {
                FileStatus::New => Style::default().fg(Color::Green),
                FileStatus::Modified => Style::default().fg(Color::Yellow),
                FileStatus::Deleted => Style::default().fg(Color::Red),
                _ => Style::default(),
            };
            let prefix = match item.status {
                FileStatus::New => "A ",
                FileStatus::Modified => "M ",
                FileStatus::Deleted => "D ",
                _ => "  ",
            };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, style.add_modifier(Modifier::BOLD)),
                Span::raw(item.path.clone()),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Dotfiles Status"))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, area, &mut app.list_state);
}

/// Renders the status bar with contextual hints.
fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let mode_text = format!(" Mode: {} ", match app.mode {
        AppMode::Normal => "NORMAL",
        AppMode::Search => "SEARCH",
        AppMode::CommitInput => "COMMIT",
        AppMode::Help => "HELP",
        AppMode::AddRemote => "ADD REMOTE",
        AppMode::InitRepoPrompt => "SETUP",
    });

    let loading_indicator = if app.is_loading { " [Loading...]" } else { "" };

    let hints = match app.mode {
        AppMode::Normal => " | ?: Help | q: Quit",
        AppMode::Search => " | Enter: Apply | Esc: Cancel",
        AppMode::CommitInput => " | Enter: Commit | Esc: Cancel",
        AppMode::AddRemote => " | Enter: Save | Esc: Cancel",
        AppMode::Help => " | ?: Close | q: Quit",
        AppMode::InitRepoPrompt => " | y: Yes | n: No/Quit",
    };

    let left = Span::styled(mode_text, Style::default().bg(Color::Blue).fg(Color::White));
    let right = Span::styled(
        format!("{} files{} ", app.status_items.len(), hints),
        Style::default().fg(Color::Gray),
    );

    let status_bar = Paragraph::new(Line::from(vec![
        left,
        Span::raw(" | "),
        Span::raw(&app.message),
        Span::raw(loading_indicator),
    ]))
    .alignment(Alignment::Left)
    .block(
        Block::default()
            .borders(Borders::TOP)
            .title(right)
            .title_alignment(Alignment::Right),
    );
    
    f.render_widget(status_bar, area);
}

/// Renders a generic popup with a title and content.
fn render_popup(f: &mut Frame, title: &str, content: Paragraph) {
    let block = Block::default().title(title).borders(Borders::ALL);
    let area = centered_rect(60, 50, f.size());
    let content_with_block = content.block(block).wrap(Wrap { trim: true });
    f.render_widget(Clear, area);
    f.render_widget(content_with_block, area);
}

/// Renders a specific popup for user input.
fn render_input_popup(f: &mut Frame, app: &App, title: &str) {
    let text = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title(title));
    let area = centered_rect(60, 3, f.size());
    f.render_widget(Clear, area);
    f.render_widget(text, area);
    f.set_cursor(area.x + app.input.len() as u16 + 1, area.y + 1);
}

/// Creates the content for the help popup.
fn render_help_text<'a>() -> Paragraph<'a> {
    Paragraph::new(vec![
        Line::from(vec![Span::styled("q", Style::default().bold()), Span::raw(": Quit")]),
        Line::from(vec![Span::styled("j/k/↓/↑", Style::default().bold()), Span::raw(": Navigate list")]),
        Line::from(vec![Span::styled("g/G", Style::default().bold()), Span::raw(": Go to top/bottom")]),
        Line::from(vec![Span::styled("/", Style::default().bold()), Span::raw(": Search files")]),
        Line::from(vec![Span::styled("a", Style::default().bold()), Span::raw(": Add all changes")]),
        Line::from(vec![Span::styled("c", Style::default().bold()), Span::raw(": Enter commit mode")]),
        Line::from(vec![Span::styled("Enter", Style::default().bold()), Span::raw(": (In commit mode) Submit commit")]),
        Line::from(vec![Span::styled("p", Style::default().bold()), Span::raw(": Push to remote")]),
        Line::from(vec![Span::styled("r", Style::default().bold()), Span::raw(": Refresh status")]),
        Line::from(vec![Span::styled("?", Style::default().bold()), Span::raw(": Toggle this help screen")]),
        Line::from(vec![Span::styled("Esc", Style::default().bold()), Span::raw(": Exit current mode")]),
    ])
}

/// Helper function to create a centered rectangle for popups.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
