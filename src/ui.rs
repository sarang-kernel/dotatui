//! src/ui.rs

use crate::app::{App, Mode, Popup, StatusMode};
use crate::git::StatusItem;
use git2::Status;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table, Tabs, Wrap},
};

pub fn render(frame: &mut Frame, app: &mut App) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
        .split(frame.size());

    render_tabs(frame, app, main_layout[0]);
    render_footer(frame, app, main_layout[2]);

    match app.mode {
        Mode::Status(sub_mode) => render_status_view(frame, app, main_layout[1], sub_mode),
        Mode::Log => render_log_view(frame, app, main_layout[1]),
    }

    if let Some(popup) = &app.popup {
        render_popup(frame, popup, &app.commit_msg, app.cursor_pos);
    }
}

fn render_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let titles = vec!["[S]tatus", "[L]og"];
    let selected_index = match app.mode {
        Mode::Status(_) => 0,
        Mode::Log => 1,
    };
    let tabs = Tabs::new(titles)
        .block(Block::default())
        .select(selected_index)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::DarkGray),
        );
    frame.render_widget(tabs, area);
}

fn render_status_view(frame: &mut Frame, app: &mut App, area: Rect, sub_mode: StatusMode) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
        .split(area);

    let (staged_items, unstaged_items): (Vec<_>, Vec<_>) =
        app.status_items.iter().partition(|item| item.is_staged);
    let mut all_list_items = Vec::new();
    if !staged_items.is_empty() {
        all_list_items
            .push(ListItem::new("Staged changes:").style(Style::default().add_modifier(Modifier::BOLD)));
        all_list_items.extend(staged_items.iter().map(|item| status_to_list_item(item)));
    }
    if !unstaged_items.is_empty() {
        all_list_items.push(
            ListItem::new("Unstaged changes:").style(Style::default().add_modifier(Modifier::BOLD)),
        );
        all_list_items.extend(unstaged_items.iter().map(|item| status_to_list_item(item)));
    }
    let file_list = List::new(all_list_items)
        .block(Block::default().borders(Borders::ALL).title("Files"))
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol(">> ");
    frame.render_stateful_widget(file_list, chunks[0], &mut app.status_list_state);

    let diff_title = match sub_mode {
        StatusMode::FileSelection => "Diff (Press 'enter' to select hunks)",
        StatusMode::HunkSelection => "Diff (Press 'q' to exit hunk-mode)",
    };

    // Use the correct function name: get_diff_text
    let diff_text = if let Some(item) = app.get_selected_status_item() {
        app.repo
            .get_diff_text(item)
            .unwrap_or_else(|_| "Error loading diff".to_string())
    } else {
        "Select a file to see the diff.".to_string()
    };

    let diff_lines: Vec<Line> = diff_text
        .lines()
        .map(|line| {
            let (style, line_content) = if line.starts_with('+') {
                (Style::default().fg(Color::Green), line)
            } else if line.starts_with('-') {
                (Style::default().fg(Color::Red), line)
            } else if line.starts_with("@@") {
                (Style::default().fg(Color::Cyan), line)
            } else {
                (Style::default(), line)
            };
            Line::styled(line_content.to_string(), style)
        })
        .collect();
    let diff_view = Paragraph::new(diff_lines).block(Block::default().borders(Borders::ALL).title(diff_title));
    frame.render_widget(diff_view, chunks[1]);
}

fn render_log_view(frame: &mut Frame, app: &mut App, area: Rect) {
    let header_cells = ["Commit", "Author", "Date"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1).bottom_margin(1);
    let rows = app.log_entries.iter().map(|commit| {
        Row::new(vec![
            Cell::from(commit.id.clone()),
            Cell::from(commit.author.clone()),
            Cell::from(commit.time.clone()),
        ])
    });
    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Length(15),
            Constraint::Min(20),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title("Log"))
    .highlight_style(Style::default().bg(Color::DarkGray))
    .highlight_symbol(">> ");
    frame.render_stateful_widget(table, area, &mut app.log_table_state);
}

fn status_to_list_item(item: &StatusItem) -> ListItem {
    let (prefix, color) = status_to_prefix_and_color(item.status);
    let style = Style::default().fg(color);
    ListItem::new(Line::from(vec![
        Span::styled(prefix, style.clone().add_modifier(Modifier::BOLD)),
        Span::styled(item.path.clone(), style),
    ]))
}

fn status_to_prefix_and_color(status: Status) -> (&'static str, Color) {
    if status.is_wt_new() || status.is_index_new() {
        ("A ", Color::Green)
    } else if status.is_wt_modified() || status.is_index_modified() {
        ("M ", Color::Yellow)
    } else if status.is_wt_deleted() || status.is_index_deleted() {
        ("D ", Color::Red)
    } else if status.is_wt_renamed() || status.is_index_renamed() {
        ("R ", Color::Cyan)
    } else if status.is_wt_typechange() || status.is_index_typechange() {
        ("T ", Color::Magenta)
    } else {
        ("? ", Color::White)
    }
}

fn render_popup(frame: &mut Frame, popup: &Popup, commit_msg: &str, cursor_pos: usize) {
    let popup_area = centered_rect(60, 25, frame.size());
    let block = Block::default().borders(Borders::ALL);
    frame.render_widget(Clear, popup_area);
    let content = match popup {
        Popup::Help => {
            let text = vec![
                Line::from(vec![
                    Span::styled("q", Style::default().bold()),
                    Span::raw(": quit"),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("s", Style::default().bold()),
                    Span::raw(": Status View"),
                ]),
                Line::from(vec![
                    Span::styled("l", Style::default().bold()),
                    Span::raw(": Log View"),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("j/k", Style::default().bold()),
                    Span::raw(" or "),
                    Span::styled("↓/↑", Style::default().bold()),
                    Span::raw(": navigate lists"),
                ]),
                Line::from(vec![
                    Span::styled("enter", Style::default().bold()),
                    Span::raw(": enter hunk selection mode"),
                ]),
                Line::from(vec![
                    Span::styled("space", Style::default().bold()),
                    Span::raw(": stage item/hunk"),
                ]),
                Line::from(vec![
                    Span::styled("u", Style::default().bold()),
                    Span::raw(": unstage item"),
                ]),
                Line::from(vec![
                    Span::styled("c", Style::default().bold()),
                    Span::raw(": commit"),
                ]),
                Line::from(vec![
                    Span::styled("Shift+P", Style::default().bold()),
                    Span::raw(": push to origin"),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("esc", Style::default().bold()),
                    Span::raw(": close popups"),
                ]),
            ];
            Paragraph::new(text)
                .block(block.title(" Help (?) "))
                .alignment(Alignment::Left)
        }
        Popup::Commit => {
            let p = Paragraph::new(commit_msg)
                .block(block.title(" Commit Message (Enter to confirm, Esc to cancel) "));
            frame.set_cursor(popup_area.x + cursor_pos as u16 + 1, popup_area.y + 1);
            p
        }
        Popup::Pushing(msg) => Paragraph::new(msg.clone())
            .block(block.title(" Pushing to remote... (Esc to close) "))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
    };
    frame.render_widget(content, popup_area);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let text = format!("Repo: {} | Press '?' for help", app.repo.path_str());
    let footer = Paragraph::new(text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Left);
    frame.render_widget(footer, area);
}

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
