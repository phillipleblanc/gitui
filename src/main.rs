use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use git2::{Repository, Status};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem},
    Terminal,
};
use std::{
    io,
    path::{Path, PathBuf},
};

struct FileEntry {
    name: String,
    status: Status,
    is_dir: bool,
}

fn main() -> Result<(), io::Error> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let repo = Repository::open(".").expect("Failed to open repository");
    let files = get_file_list(&repo);

    // Main loop
    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
                .split(f.size());

            let items: Vec<ListItem> = files
                .iter()
                .map(|file| {
                    let color = match file.status {
                        Status::WT_NEW => Color::Green,
                        Status::WT_MODIFIED => Color::Yellow,
                        _ => Color::Reset,
                    };
                    let prefix = if file.is_dir { "📁 " } else { "📄 " };
                    ListItem::new(prefix.to_string() + &file.name).style(Style::default().fg(color))
                })
                .collect();

            let file_list = List::new(items)
                .block(Block::default().title("Files").borders(Borders::ALL))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD));

            f.render_widget(file_list, chunks[0]);

            let right_block = Block::default().title("Details").borders(Borders::ALL);
            f.render_widget(right_block, chunks[1]);
        })?;

        if let Event::Key(key) = event::read()? {
            if let KeyCode::Char('q') = key.code {
                break;
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

fn get_file_list(repo: &Repository) -> Vec<FileEntry> {
    let mut files = Vec::new();
    let statuses = repo.statuses(None).expect("Couldn't get repository status");

    for entry in walkdir::WalkDir::new(".")
        .min_depth(1)
        .into_iter()
        .filter_entry(|e| !should_ignore(repo, e.path()))
    {
        if let Ok(entry) = entry {
            let path = entry.path().to_path_buf();
            if path.starts_with("./.git") {
                continue;
            }
            let name = path
                .strip_prefix("./")
                .unwrap_or(&path)
                .to_string_lossy()
                .into_owned();
            let is_dir = entry.file_type().is_dir();
            let status = statuses
                .iter()
                .find_map(|s| {
                    if Path::new(s.path().unwrap()) == path {
                        Some(s.status())
                    } else {
                        None
                    }
                })
                .unwrap_or(Status::CURRENT);

            files.push(FileEntry {
                name,
                status,
                is_dir,
            });
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

fn should_ignore(repo: &Repository, path: &Path) -> bool {
    repo.status_should_ignore(path).unwrap_or(false)
}
