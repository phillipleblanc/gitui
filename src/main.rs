use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use git2::{IndexAddOption, Repository, Status};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::{io, path::PathBuf};

struct FileEntry {
    name: String,
    status: Status,
    is_dir: bool,
}

struct Modal {
    content: String,
    is_visible: bool,
}

struct App {
    files: Vec<FileEntry>,
    selected_index: usize,
    right_pane_content: String,
    commit_modal: Modal,
    help_modal: Modal,
}

fn main() -> Result<(), anyhow::Error> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let repo = Repository::open(".").expect("Failed to open repository");
    let files = get_file_list(&repo);
    let mut app = App {
        files,
        selected_index: 0,
        right_pane_content: String::new(),
        commit_modal: Modal {
            content: String::new(),
            is_visible: false,
        },
        help_modal: Modal {
            content: get_help_content(),
            is_visible: false,
        },
    };

    // Main loop
    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
                .split(f.size());

            let items: Vec<ListItem> = app
                .files
                .iter()
                .enumerate()
                .map(|(index, file)| {
                    let color = match file.status {
                        Status::WT_NEW => Color::Green,
                        Status::WT_MODIFIED => Color::Yellow,
                        Status::WT_DELETED => Color::Red,
                        _ => Color::White,
                    };
                    let prefix = if file.is_dir { "📁 " } else { "📄 " };
                    let content = prefix.to_string() + &file.name;
                    let style = if index == app.selected_index {
                        Style::default().fg(color).add_modifier(Modifier::REVERSED)
                    } else {
                        Style::default().fg(color)
                    };
                    ListItem::new(Spans::from(vec![Span::styled(content, style)]))
                })
                .collect();

            let file_list = List::new(items)
                .block(Block::default().title("Files").borders(Borders::ALL))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD));

            f.render_stateful_widget(
                file_list,
                chunks[0],
                &mut ListState::default().with_selected(Some(app.selected_index)),
            );

            let right_pane = Paragraph::new(app.right_pane_content.as_str())
                .block(Block::default().title("Details").borders(Borders::ALL))
                .wrap(ratatui::widgets::Wrap { trim: true });
            f.render_widget(right_pane, chunks[1]);

            if app.commit_modal.is_visible {
                render_modal(f, "Commit Message", &app.commit_modal.content, 60, 20);
            } else if app.help_modal.is_visible {
                render_modal(f, "Help", &app.help_modal.content, 60, 40);
            }
        })?;

        if app.commit_modal.is_visible {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => app.commit_modal.is_visible = false,
                    KeyCode::Enter => {
                        create_commit(&repo, &app.commit_modal.content)?;
                        app.commit_modal.is_visible = false;
                        app.commit_modal.content.clear();
                        app.files = get_file_list(&repo);
                    }
                    KeyCode::Char(c) => app.commit_modal.content.push(c),
                    KeyCode::Backspace => {
                        app.commit_modal.content.pop();
                    }
                    _ => {}
                }
            }
        } else if app.help_modal.is_visible {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Esc || key.code == KeyCode::Char('?') {
                    app.help_modal.is_visible = false;
                }
            }
        } else {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Up => {
                        if app.selected_index > 0 {
                            app.selected_index -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if app.selected_index < app.files.len() - 1 {
                            app.selected_index += 1;
                        }
                    }
                    KeyCode::Enter => {
                        update_right_pane(&repo, &mut app)?;
                    }
                    KeyCode::Char('c') => {
                        stage_all_modified(&repo)?;
                        app.commit_modal.is_visible = true;
                    }
                    KeyCode::Char('?') => {
                        app.help_modal.is_visible = true;
                    }
                    _ => {}
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn update_right_pane(repo: &Repository, app: &mut App) -> Result<(), io::Error> {
    let selected_file = &app.files[app.selected_index];
    let path = PathBuf::from(&selected_file.name);

    if selected_file.is_dir {
        app.right_pane_content = format!("Directory: {}", selected_file.name);
    } else if selected_file.status != Status::CURRENT {
        // Show diff for modified files
        let diff = repo
            .diff_tree_to_workdir_with_index(None, None)
            .expect("Failed to get diff");
        let mut diff_content = String::new();
        diff.print(git2::DiffFormat::Patch, |_, _, line| {
            if line.origin() == '+' || line.origin() == '-' {
                diff_content.push(line.origin());
                diff_content.push_str(std::str::from_utf8(line.content()).unwrap_or(""));
            }
            true
        })
        .expect("Failed to print diff");
        app.right_pane_content = diff_content;
    } else {
        // Show file contents for unmodified files
        app.right_pane_content = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| format!("Failed to read file: {}", selected_file.name));
    }

    Ok(())
}

fn get_file_list(repo: &Repository) -> Vec<FileEntry> {
    let mut files = Vec::new();
    let statuses = repo.statuses(None).expect("Couldn't get repository status");

    for entry in statuses.iter() {
        let path = PathBuf::from(entry.path().unwrap_or_default());
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let is_dir = path.is_dir();
        let status = entry.status();

        files.push(FileEntry {
            name,
            status,
            is_dir,
        });
    }

    // Add untracked files and directories
    for entry in std::fs::read_dir(".").expect("Failed to read directory") {
        if let Ok(entry) = entry {
            let path = entry.path();
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let is_dir = path.is_dir();

            if !files.iter().any(|f| f.name == name) {
                let status = if repo
                    .status_file(&path)
                    .map(|s| s.is_wt_new())
                    .unwrap_or(false)
                {
                    Status::WT_NEW
                } else {
                    Status::CURRENT
                };

                files.push(FileEntry {
                    name,
                    status,
                    is_dir,
                });
            }
        }
    }

    files.sort_by(|a, b| {
        if a.is_dir == b.is_dir {
            a.name.cmp(&b.name)
        } else {
            b.is_dir.cmp(&a.is_dir)
        }
    });

    files
}

fn stage_all_modified(repo: &Repository) -> Result<(), git2::Error> {
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
    index.write()?;
    Ok(())
}

fn create_commit(repo: &Repository, message: &str) -> Result<(), git2::Error> {
    let mut index = repo.index()?;
    let oid = index.write_tree()?;
    let signature = repo.signature()?;
    let parent_commit = repo.head()?.peel_to_commit()?;
    let tree = repo.find_tree(oid)?;
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &[&parent_commit],
    )?;
    Ok(())
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

fn render_modal(
    f: &mut ratatui::Frame<CrosstermBackend<std::io::Stdout>>,
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

fn get_help_content() -> String {
    "
    Key Bindings:
    ↑/↓: Navigate file list
    Enter: View file details/diff
    c: Stage all modified files and open commit dialog
    ?: Toggle this help menu
    q: Quit the application

    In commit dialog:
    Enter: Confirm commit
    Esc: Cancel commit
    "
    .trim()
    .to_string()
}
