// src/ui.rs

use crate::app::{App, AppMode, FocusedPanel, PopupMode};
use crate::git_utils::FileStatus;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

pub fn draw(f: &mut Frame, app: &mut App) {
    f.render_widget(Block::default(), f.size());

    match app.mode {
        AppMode::Home => render_home(f, app),
        AppMode::Status => render_status_view(f, app),
        _ => {}
    }

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

// ... (render_home is unchanged from the last version with the new ASCII art) ...
fn render_home(f: &mut Frame, app: &App) {
    let logo = vec![
        Line::from(""),
        Line::from("DDDDDDDDDDDDD                                  tttt                                    tttt                              iiii  "),
        Line::from("D::::::::::::DDD                            ttt:::t                                 ttt:::t                             i::::i "),
        Line::from("D:::::::::::::::DD                          t:::::t                                 t:::::t                              iiii  "),
        Line::from("DDD:::::DDDDD:::::D                         t:::::t                                 t:::::t                                    "),
        Line::from("  D:::::D    D:::::D    ooooooooooo   ttttttt:::::ttttttt      aaaaaaaaaaaaa  ttttttt:::::ttttttt    uuuuuu    uuuuuu  iiiiiii "),
        Line::from("  D:::::D     D:::::D oo:::::::::::oo t:::::::::::::::::t      a::::::::::::a t:::::::::::::::::t    u::::u    u::::u  i:::::i "),
        Line::from("  D:::::D     D:::::Do:::::::::::::::ot:::::::::::::::::t      aaaaaaaaa:::::at:::::::::::::::::t    u::::u    u::::u   i::::i "),
        Line::from("  D:::::D     D:::::Do:::::ooooo:::::otttttt:::::::tttttt               a::::atttttt:::::::tttttt    u::::u    u::::u   i::::i "),
        Line::from("  D:::::D     D:::::Do::::o     o::::o      t:::::t              aaaaaaa:::::a      t:::::t          u::::u    u::::u   i::::i "),
        Line::from("  D:::::D     D:::::Do::::o     o::::o      t:::::t            aa::::::::::::a      t:::::t          u::::u    u::::u   i::::i "),
        Line::from("  D:::::D     D:::::Do::::o     o::::o      t:::::t           a::::aaaa::::::a      t:::::t          u::::u    u::::u   i::::i "),
        Line::from("  D:::::D    D:::::D o::::o     o::::o      t:::::t    tttttta::::a    a:::::a      t:::::t    ttttttu:::::uuuu:::::u   i::::i "),
        Line::from("DDD:::::DDDDD:::::D  o:::::ooooo:::::o      t::::::tttt:::::ta::::a    a:::::a      t::::::tttt:::::tu:::::::::::::::uui::::::i"),
        Line::from("D:::::::::::::::DD   o:::::::::::::::o      tt::::::::::::::ta:::::aaaa::::::a      tt::::::::::::::t u:::::::::::::::ui::::::i"),
        Line::from("D::::::::::::DDD      oo:::::::::::oo         tt:::::::::::tt a::::::::::aa:::a       tt:::::::::::tt  uu::::::::uu:::ui::::::i"),
        Line::from("DDDDDDDDDDDDD           ooooooooooo             ttttttttttt    aaaaaaaaaa  aaaa         ttttttttttt      uuuuuuuu  uuuuiiiiiiii"),
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
            Span::styled("[s]", Style::default().fg(Color::Green).bold()),
            Span::raw(" Status View"),
        ]),
        Line::from(vec![
            Span::styled("[c]", Style::default().fg(Color::Green).bold()),
            Span::raw(" Change Dotfiles Path"),
        ]),
        Line::from(vec![
            Span::styled("[h]", Style::default().fg(Color::Green).bold()),
            Span::raw(" Help"),
        ]),
        Line::from(vec![
            Span::styled("[q]", Style::default().fg(Color::Green).bold()),
            Span::raw(" Quit"),
        ]),
    ];

    let logo_p = Paragraph::new(logo).alignment(Alignment::Center);
    let status_p = Paragraph::new(status_lines).alignment(Alignment::Center);
    let menu_p = Paragraph::new(menu_lines).alignment(Alignment::Center);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(65),
            Constraint::Percentage(15),
            Constraint::Percentage(20),
        ])
        .split(f.size());

    f.render_widget(logo_p, chunks[0]);
    f.render_widget(status_p, chunks[1]);
    f.render_widget(menu_p, chunks[2]);
}


// MODIFIED: Renders a three-panel layout for Status, Staged, and Diff.
fn render_status_view(f: &mut Frame, app: &mut App) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
        .split(f.size());

    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(main_chunks[0]);
    
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(top_chunks[0]);

    render_file_panel(f, app, FocusedPanel::Unstaged, left_chunks[0]);
    render_file_panel(f, app, FocusedPanel::Staged, left_chunks[1]);
    render_diff_panel(f, app, top_chunks[1]);

    render_status_bar(f, app, main_chunks[1]);
}

// ... (render_file_panel is unchanged) ...
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


// NEW: Renders the diff panel with colored lines.
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

// MODIFIED: Updated hints for the new keybinding model.
fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let loading_indicator = if app.is_loading { " [Loading...]" } else { "" };
    let hints = "h: Home | ?: Help | q: Quit | Tab: Panels | space: Stage/Unstage | c: Commit | p: Push";

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

// ... (render_input_popup and render_init_repo_popup are unchanged) ...
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


// MODIFIED: Updated help text for the new keybinding model.
fn render_help_popup(f: &mut Frame) {
    let text = vec![
        Line::from("").style(Style::default()),
        Line::from(" Global Commands").style(Style::default().bold().underlined()),
        Line::from(vec![Span::styled("  q", Style::default().bold()), Span::raw(": Quit the application.")]),
        Line::from(vec![Span::styled("  ?", Style::default().bold()), Span::raw(": Toggle this help popup.")]),
        Line::from(vec![Span::styled("  h", Style::default().bold()), Span::raw(": Go to the Home screen (from Status view).")]),
        Line::from(vec![Span::styled("  s", Style::default().bold()), Span::raw(": Go to the Status screen (from Home view).")]),
        Line::from(""),
        Line::from(" Status Screen Navigation").style(Style::default().bold().underlined()),
        Line::from(vec![Span::styled("  j/k, ↓/↑", Style::default().bold()), Span::raw(": Navigate up and down in file panels.")]),
        Line::from(vec![Span::styled("  Tab, l", Style::default().bold()), Span::raw(":  Cycle focus between Unstaged and Staged panels.")]),
        Line::from(vec![Span::styled("  Shift+Tab, h", Style::default().bold()), Span::raw(": Cycle focus to the previous panel.")]),
        Line::from(""),
        Line::from(" Status Screen Actions").style(Style::default().bold().underlined()),
        Line::from(vec![Span::styled("  space", Style::default().bold()), Span::raw(": Stage (if in Unstaged) or unstage (if in Staged) the selected file.")]),
        Line::from(vec![Span::styled("  a", Style::default().bold()), Span::raw(":     Stage all unstaged files.")]),
        Line::from(vec![Span::styled("  u", Style::default().bold()), Span::raw(":     Unstage all staged files.")]),
        Line::from(vec![Span::styled("  c", Style::default().bold()), Span::raw(":     Open the commit message input popup.")]),
        Line::from(vec![Span::styled("  p", Style::default().bold()), Span::raw(":     Push staged changes to the remote.")]),
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

// ... (centered_rect is unchanged) ...
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
