// src/ui.rs

use crate::app::{App, AppMode, PopupMode};
use crate::git_utils::{FileState, FileStatus, StagingStatus};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

/// The main drawing function that orchestrates the rendering of all UI components.
pub fn draw(f: &mut Frame, app: &mut App) {
    // Create a main layout with two chunks: one for the main content and one for the status bar.
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)].as_ref())
        .split(f.size());

    // Split the main content area into two panels: Files and Diff.
    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
        .split(main_chunks[0]);

    render_file_panel(f, app, top_chunks[0]);
    render_diff_panel(f, app, top_chunks[1]);
    render_status_bar(f, app, main_chunks[1]);

    // Render popups on top of the main UI if the app is in a popup mode.
    if let AppMode::Popup(popup_mode) = &app.mode {
        match popup_mode {
            PopupMode::Commit => render_input_popup(f, app, "Commit Message"),
            PopupMode::AddRemote => render_input_popup(f, app, "Input Remote URL"),
            PopupMode::InitRepo => render_init_repo_popup(f),
            PopupMode::ChangePath => render_input_popup(f, app, "Change Dotfiles Path"),
            PopupMode::Help => render_help_popup(f),
        }
    }
}

/// Renders the unified file list panel.
fn render_file_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let list_items: Vec<ListItem> = app
        .files
        .iter()
        .map(|file| {
            let (status_char, status_style) = match file.status {
                FileStatus::New => ("A", Style::default().fg(Color::Green)),
                FileStatus::Modified => ("M", Style::default().fg(Color::Yellow)),
                FileStatus::Deleted => ("D", Style::default().fg(Color::Red)),
                FileStatus::Renamed => ("R", Style::default().fg(Color::Cyan)),
                FileStatus::Typechange => ("T", Style::default().fg(Color::Magenta)),
                FileStatus::Conflicted => ("C", Style::default().fg(Color::LightRed)),
            };

            let (staging_char, staging_style) = match file.staging_status {
                StagingStatus::Staged => ("S", Style::default().fg(Color::Green)),
                StagingStatus::Unstaged => ("U", Style::default().fg(Color::Yellow)),
                StagingStatus::PartiallyStaged => ("P", Style::default().fg(Color::LightMagenta)),
            };

            let line = Line::from(vec![
                Span::styled(format!("[{}]", staging_char), staging_style.bold()),
                Span::raw(" "),
                Span::styled(format!("[{}]", status_char), status_style.bold()),
                Span::raw(" "),
                Span::raw(&file.path),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(list_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Files ({})", app.files.len())),
        )
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, area, &mut app.file_list_state);
}

/// Renders the diff panel with colored lines.
fn render_diff_panel(f: &mut Frame, app: &App, area: Rect) {
    let lines: Vec<Line> = app.diff_text.lines().map(|line| {
        let style = match line.chars().next() {
            Some('+') => Style::default().fg(Color::Green),
            Some('-') => Style::default().fg(Color::Red),
            _ => Style::default(),
        };
        Line::from(Span::styled(line, style))
    }).collect();

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Diff"))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

/// Renders the permanent status bar at the bottom.
fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let loading_indicator = if app.is_loading { " [Loading...]" } else { "" };
    let hints = "j/k: Nav | space: Stage/Unstage | a: Stage All | c: Commit | P: Push | ?: Help | q: Quit";

    let left = Span::raw(format!("{}{}", app.message, loading_indicator));
    let right = Span::styled(hints, Style::default().fg(Color::DarkGray));

    let status_bar = Paragraph::new(Line::from(vec![left, Span::raw(" | ").fg(Color::DarkGray), right]))
        .block(Block::default().style(Style::default().bg(Color::Black)));
    
    f.render_widget(status_bar, area);
}

/// Renders a generic popup for user input.
fn render_input_popup(f: &mut Frame, app: &App, title: &str) {
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .title_bottom(Line::from(" Enter: Submit | Esc: Cancel ").centered());
    let area = centered_rect(60, 3, f.size());
    
    let input = Paragraph::new(app.input.as_str()).block(block);
    
    f.render_widget(Clear, area);
    f.render_widget(input, area);
    f.set_cursor(area.x + app.input.len() as u16 + 1, area.y + 1);
}

/// Renders a confirmation popup for initializing a repository.
fn render_init_repo_popup(f: &mut Frame) {
    let text = vec![
        Line::from(""),
        Line::from("No Git repository found in this directory."),
        Line::from(""),
        Line::from("Do you want to initialize a new one here?"),
    ];
    let block = Block::default()
        .title("Initialize Repository")
        .borders(Borders::ALL)
        .title_bottom(Line::from(" Enter: Yes | Esc: No/Cancel ").centered());
    
    let area = centered_rect(50, 25, f.size());
    let paragraph = Paragraph::new(text).block(block).alignment(Alignment::Center);

    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}

/// Renders the descriptive help popup with new ASCII art.
fn render_help_popup(f: &mut Frame) {
    let logo = vec![
        Line::from(""),
        Line::from("    _       _   _   _ "),
        Line::from("   / \\   __| | | |_| |"),
        Line::from("  / _ \\ / _` | | __| |"),
        Line::from(" / ___ \\ (_| | | |_| |"),
        Line::from("/_/   \\_\\__,_|  \\__|_|"),
        Line::from(""),
    ];

    let text = vec![
        Line::from("").style(Style::default()),
        Line::from(" Global Commands").style(Style::default().bold().underlined()),
        Line::from(vec![Span::styled("  q", Style::default().bold()), Span::raw(": Quit the application.")]),
        Line::from(vec![Span::styled("  ?", Style::default().bold()), Span::raw(": Toggle this help popup.")]),
        Line::from(vec![Span::styled("  r", Style::default().bold()), Span::raw(": Manually refresh the Git status.")]),
        Line::from(""),
        Line::from(" File Actions").style(Style::default().bold().underlined()),
        Line::from(vec![Span::styled("  j/k, ↓/↑", Style::default().bold()), Span::raw(": Navigate up and down the file list.")]),
        Line::from(vec![Span::styled("  space", Style::default().bold()), Span::raw(":     Stage or unstage the selected file.")]),
        Line::from(vec![Span::styled("  a", Style::default().bold()), Span::raw(":         Stage all unstaged files.")]),
        Line::from(vec![Span::styled("  c", Style::default().bold()), Span::raw(":         Open the commit message input popup.")]),
        Line::from(vec![Span::styled("  P (Shift+P)", Style::default().bold()), Span::raw(": Push staged changes to the remote.")]),
    ];
    
    let block = Block::default()
        .title("Help")
        .borders(Borders::ALL)
        .title_bottom(Line::from(" Press ? or Esc to close ").centered());
        
    let area = centered_rect(80, 80, f.size());
    let content_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(0)])
        .margin(1)
        .split(area);

    let logo_p = Paragraph::new(logo).alignment(Alignment::Center);
    let paragraph = Paragraph::new(text).wrap(Wrap { trim: true });

    f.render_widget(Clear, area);
    f.render_widget(block, area);
    f.render_widget(logo_p, content_chunks[0]);
    f.render_widget(paragraph, content_chunks[1]);
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
