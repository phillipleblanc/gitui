mod app;
mod debug;
mod file_system;
mod git_ops;
mod ui;

use crossterm::{
    event::{self, DisableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use git2::Repository;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::time::Duration;

use crate::app::{App, AppResult};
use crate::ui::draw;

fn main() -> AppResult<()> {
    // Initialize debug channel
    let debug_receiver = debug::init_debug();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let repo = Repository::open(".").expect("Failed to open repository");
    let mut app = App::new(&repo);

    // Main loop
    loop {
        // Refresh file list
        app.refresh_file_list(&repo);
        terminal.draw(|f| draw(f, &mut app))?;

        // Check for debug messages
        if let Ok(debug_message) = debug_receiver.try_recv() {
            app.debug_log(&debug_message);
        }

        if event::poll(Duration::from_millis(16))? {
            if let Ok(event) = event::read() {
                match event {
                    Event::Key(KeyEvent {
                        code: KeyCode::Char('q'),
                        ..
                    }) => {
                        if !app.commit_modal.is_visible && !app.help_modal.is_visible {
                            break;
                        }
                    }
                    _ => app.handle_event(event, &repo)?,
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
