// This file contains all the rendering logic. It takes the application state (App) and a ratatui Frame and draws the widgets. By keeping all drawing code here, we maintain a clean separation between application logic (state changes) and presentation (how the state is displayed).

// Key features of this file:

//    Declarative UI: The code describes what to draw, and ratatui handles how to draw it. This makes the UI code easy to read and modify.

//    Component-Based: The UI is broken down into smaller, manageable functions (render_main_list, render_status_bar, render_popup), making the code reusable and organized.

//    State-Driven: The UI is a pure function of the App state. For example, it shows a loading indicator if app.is_loading is true, and it displays different popups based on app.mode.

//    Efficiency: It only draws what's necessary for the current frame. The Clear widget is used for popups to avoid redrawing the entire screen, which is a minor but good optimization.

// src/ui.rs

use crate::app::{App, AppMode, FocusedPanel, PopupMode};
use crate::git_utils::FileStatus;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

/// The main drawing function that orchestrates the rendering of all UI components.
pub fn draw(f: &mut Frame, app: &mut App) {
    // Create a main layout with two chunks: one for the main content and one for the status bar.
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
        .split(f.size());

    // Create a layout for the three main panels within the top chunk.
    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(main_chunks[0]);
    
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(top_chunks[1]);
    
    // Main router for different views
    match app.mode {
        AppMode::Home => render_home(f, app),
        AppMode::Status => render_status_view(f, app),
        _ => {} // Popups are handled seperately
    }
    // Render the three main panels.
    render_file_panel(f, app, FocusedPanel::Unstaged, top_chunks[0]);
    render_file_panel(f, app, FocusedPanel::Staged, right_chunks[0]);
    render_menu_panel(f, app, right_chunks[1]);

    // Render the status bar at the bottom.
    render_status_bar(f, app, main_chunks[1]);

    // Render popups conditionally over the main UI.
    if let AppMode::Popup(popup_mode) = &app.mode {
        match popup_mode {
            PopupMode::Commit => render_input_popup(f, app, "Commit Message"),
            PopupMode::AddRemote => render_input_popup(f, app, "Input Remote URL"),
            PopupMode::InitRepo => render_init_repo_popup(f),
            PopupMode::Help => render_help_popup(f),
        }
    }
}

// Corrected code
fn render_home(f: &mut Frame, app: &App) {
    let logo = vec![
        Line::from(""),
        Line::from("  ____            _   _   _ "),
        Line::from(" |  _ \\   ___  __| | | |_| |"),
        Line::from(" | | | | / _ \\/ _` | | __| |"),
        Line::from(" | |_| ||  __/ (_| | | |_| |"),
        Line::from(" |____/  \\___|\\__,_|  \\__|_|"),
        Line::from(""),
    ];

    let status_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Managing: ", Style::default().bold()),
            Span::styled(
                app.dotfiles_path.to_string_lossy(),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::styled("Status:   ", Style::default().bold()),
            Span::raw(if app.is_loading {
                "Loading..."
            } else {
                &app.repo_status_summary
            }),
        ]),
    ];

    let menu_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("[s]", Style::default().fg(Color::Green).bold()), // Corrected: Style
            Span::raw(" Status"),
        ]),
        Line::from(vec![
            Span::styled("[h]", Style::default().fg(Color::Green).bold()), // Corrected: Style
            Span::raw(" Help"),
        ]),
        Line::from(vec![
            Span::styled("[q]", Style::default().fg(Color::Green).bold()), // Corrected: Style
            Span::raw(" Quit"),
        ]),
    ];

    let logo_p = Paragraph::new(logo).alignment(Alignment::Center);
    let status_p = Paragraph::new(status_lines).alignment(Alignment::Center);
    let menu_p = Paragraph::new(menu_lines).alignment(Alignment::Center); // Corrected: menu_lines

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(20),
            Constraint::Percentage(40),
        ])
        .split(f.size());

    f.render_widget(logo_p, chunks[0]); // Corrected: render_widget
    f.render_widget(status_p, chunks[1]);
    f.render_widget(menu_p, chunks[2]);
}

/// Renders the main three-panel status view.
fn render_status_view(f: &mut Frame, app: &mut App){
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
        .split(f.size());

    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(main_chunks[0]);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(top_chunks[1]);

    render_file_panel(f, app, FocusedPanel::Unstaged, top_chunks[0]);
    render_file_panel(f, app, FocusedPanel::Staged, right_chunks[0]);
    render_menu_panel(f, app, right_chunks[1]);

    render_status_bar(f, app, main_chunks[1]);
}

/// Renders a file panel (either Staged or Unstaged).
fn render_file_panel(f: &mut Frame, app: &mut App, panel_type: FocusedPanel, area: Rect) {
    let (title, items, state, is_focused) = match panel_type {
        FocusedPanel::Unstaged => (
            format!("Unstaged ({})", app.unstaged_changes.len()),
            &app.unstaged_changes,
            &mut app.unstaged_state,
            app.focused_panel == FocusedPanel::Unstaged,
        ),
        FocusedPanel::Staged => (
            format!("Staged ({})", app.staged_changes.len()),
            &app.staged_changes,
            &mut app.staged_state,
            app.focused_panel == FocusedPanel::Staged,
        ),
        _ => return,
    };

    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let list_items: Vec<ListItem> = items
        .iter()
        .map(|item| {
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
                Span::styled(prefix, style.bold()),
                Span::raw(&item.path),
            ]))
        })
        .collect();

    let list = List::new(list_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(border_style),
        )
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, area, state);
}

/// Renders the command menu panel.
fn render_menu_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.focused_panel == FocusedPanel::Menu;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let items: Vec<ListItem> = app
        .menu_items
        .iter()
        .map(|item| ListItem::new(item.as_str()))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Commands")
                .border_style(border_style),
        )
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, area, &mut app.menu_state);
}

/// Renders the status bar at the bottom of the screen.
fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let loading_indicator = if app.is_loading { " [Loading...]" } else { "" };
    let hints = " | Tab/h/l: Panels | j/k: Navigate | space: Stage/Unstage | c: Commit | ?: Help | q: Quit";

    let status_bar = Paragraph::new(Line::from(vec![
        Span::raw(&app.message),
        Span::raw(loading_indicator),
    ]))
    .alignment(Alignment::Left)
    .block(
        Block::default()
            .borders(Borders::TOP)
            .title(hints)
            .title_alignment(Alignment::Right),
    );
    
    f.render_widget(status_bar, area);
}

/// Renders a popup for user input.
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

/// Renders the help popup.
fn render_help_popup(f: &mut Frame) {
    let text = vec![
        Line::from("").style(Style::default()),
        Line::from(" Global Commands").style(Style::default().bold().underlined()),
        Line::from(vec![Span::styled("  q", Style::default().bold()), Span::raw(": Quit the application.")]),
        Line::from(vec![Span::styled("  ?", Style::default().bold()), Span::raw(": Toggle this help popup.")]),
        Line::from(vec![Span::styled("  h", Style::default().bold()), Span::raw(": Go to the Home screen.")]),
        Line::from(vec![Span::styled("  s", Style::default().bold()), Span::raw(": Go to the Status screen.")]),
        Line::from(""),
        Line::from(" Status Screen Navigation").style(Style::default().bold().underlined()),
        Line::from(vec![Span::styled("  j/k, ↓/↑", Style::default().bold()), Span::raw(": Navigate up and down in the focused panel.")]),
        Line::from(vec![Span::styled("  Tab, l", Style::default().bold()), Span::raw(":  Cycle focus to the next panel (Unstaged -> Staged -> Commands).")]),
        Line::from(vec![Span::styled("  Shift+Tab, h", Style::default().bold()), Span::raw(": Cycle focus to the previous panel.")]),
        Line::from(""),
        Line::from(" Status Screen Actions").style(Style::default().bold().underlined()),
        Line::from(vec![Span::styled("  space", Style::default().bold()), Span::raw(": Stage (if in Unstaged) or unstage (if in Staged) the selected file.")]),
        Line::from(vec![Span::styled("  a", Style::default().bold()), Span::raw(":     Stage all unstaged files.")]),
        Line::from(vec![Span::styled("  u", Style::default().bold()), Span::raw(":     Unstage all staged files.")]),
        Line::from(vec![Span::styled("  c", Style::default().bold()), Span::raw(":     Open the commit message input popup.")]),
        Line::from(vec![Span::styled("  Enter", Style::default().bold()), Span::raw(": (In Commands panel) Execute the selected command.")]),
        Line::from(vec![Span::styled("  r", Style::default().bold()), Span::raw(":     Manually refresh the Git status.")]),
    ];
    
    let block = Block::default()
        .title("Help")
        .borders(Borders::ALL)
        .title_bottom(Line::from(" Press ? or Esc to close ").centered());
        
    let area = centered_rect(80, 80, f.size());
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });

    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
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
