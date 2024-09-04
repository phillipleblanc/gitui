use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};
use std::io::Stdout;

use crate::app::App;

pub fn draw(f: &mut Frame<CrosstermBackend<Stdout>>, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
        .split(f.size());

    draw_file_list(f, app, chunks[0]);
    draw_right_pane(f, app, chunks[1]);

    if app.commit_modal.is_visible {
        draw_modal(f, "Commit Message", &app.commit_modal.content, 60, 20);
    } else if app.help_modal.is_visible {
        draw_modal(f, "Help", &app.help_modal.content, 60, 40);
    }
}

fn draw_file_list(f: &mut Frame<CrosstermBackend<Stdout>>, app: &App, area: Rect) {
    let items: Vec<ListItem> = if app.files.is_empty() {
        vec![ListItem::new("(empty directory)")]
    } else {
        app.files
            .iter()
            .enumerate()
            .map(|(index, file)| {
                let color = match file.status {
                    git2::Status::WT_NEW => Color::Green,
                    git2::Status::WT_MODIFIED => Color::Yellow,
                    git2::Status::WT_DELETED => Color::Red,
                    _ => Color::White,
                };
                let prefix = if file.is_dir { "📁 " } else { "📄 " };
                let content = format!("{}{}", prefix, file.name);
                let style = if index == app.selected_index {
                    Style::default().fg(color).add_modifier(Modifier::REVERSED)
                } else {
                    Style::default().fg(color)
                };
                ListItem::new(Spans::from(vec![Span::styled(content, style)]))
            })
            .collect()
    };

    let file_list = List::new(items)
        .block(Block::default().title("Files").borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    f.render_stateful_widget(
        file_list,
        area,
        &mut ListState::default().with_selected(Some(app.selected_index)),
    );
}

fn draw_right_pane(f: &mut Frame<CrosstermBackend<Stdout>>, app: &App, area: Rect) {
    let right_pane = Paragraph::new(app.right_pane_content.as_str())
        .block(Block::default().title("Details").borders(Borders::ALL))
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(right_pane, area);
}

fn draw_modal(
    f: &mut Frame<CrosstermBackend<Stdout>>,
    title: &str,
    content: &str,
    percent_x: u16,
    percent_y: u16,
) {
    let modal_area = centered_rect(percent_x, percent_y, f.size());
    let modal = Paragraph::new(content)
        .block(Block::default().title(title).borders(Borders::ALL))
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(Clear, modal_area);
    f.render_widget(modal, modal_area);
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
