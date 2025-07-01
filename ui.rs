use crate::app::{App, AppState};
use git2::Status;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

pub fn render(frame: &mut Frame, app: &mut App) {
    match app.state {
        AppState::Home => render_home(frame, app),
        AppState::Status => render_status(frame, app),
    }
}

fn render_home(frame: &mut Frame, app: &App) {
    let area = frame.size();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(area);

    let logo = Paragraph::new(Text::styled(
        r"
  ____ _  _ ____   __
 / ___/ \/ /|  _ \ /  \
| |  _ \  / | |_| | () |
 \ \_ \/ /  |  _ < \__/
  \____/_/   |___/
        ",
        Style::default().fg(Color::Magenta).bold(),
    ))
    .alignment(Alignment::Center);
    frame.render_widget(logo, chunks[0]);

    let help_text = "
    [s] status      [l] log      [p] push      [c] commit
    
    [q] quit
    ";
    let help = Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .title(" Commands ")
                .title_alignment(Alignment::Center)
                .borders(Borders::TOP),
        );
    frame.render_widget(help, chunks[2]);

    let repo_info = Paragraph::new(format!("Repo: {}", app.repo.path_str()))
        .alignment(Alignment::Center);
    frame.render_widget(repo_info, chunks[1]);
}

fn render_status(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
        .split(frame.size());
    
    // Left Panel: Files
    let file_list_items: Vec<ListItem> = app
        .status_items
        .iter()
        .map(|item| {
            let prefix = status_to_prefix(item.status);
            let color = status_to_color(item.status);
            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(color).bold()),
                Span::raw(item.path.clone()),
            ]))
        })
        .collect();

    let file_list = List::new(file_list_items)
        .block(Block::default().borders(Borders::ALL).title("Files"))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">> ");

    frame.render_stateful_widget(file_list, chunks[0], &mut app.status_list_state);

    // Right Panel: Diff
    let diff_lines = app.current_diff.lines().map(|line| {
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
    }).collect::<Vec<_>>();

    let diff_view = Paragraph::new(diff_lines)
        .block(Block::default().borders(Borders::ALL).title("Diff"))
        .wrap(Wrap { trim: false });
        
    frame.render_widget(diff_view, chunks[1]);
}


fn status_to_prefix(status: Status) -> &'static str {
    if status.is_wt_new() { "[A] " }
    else if status.is_wt_modified() { "[M] " }
    else if status.is_wt_deleted() { "[D] " }
    else if status.is_wt_renamed() { "[R] " }
    else if status.is_wt_typechange() { "[T] " }
    else if status.is_index_new() { "[A] " } // Staged
    else { "[?] " }
}

fn status_to_color(status: Status) -> Color {
    if status.is_wt_new() { Color::Green }
    else if status.is_wt_modified() { Color::Yellow }
    else if status.is_wt_deleted() { Color::Red }
    else { Color::White }
}
