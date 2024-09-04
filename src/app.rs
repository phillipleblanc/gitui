use crate::file_system::{get_file_list, FileEntry};
use crate::git_ops::{create_commit, stage_all_modified, update_right_pane};
use crossterm::event::{Event, MouseButton, MouseEvent, MouseEventKind};
use crossterm::event::{KeyCode, KeyEvent};
use git2::Repository;
use std::collections::HashMap;

pub type AppResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct Modal {
    pub content: String,
    pub is_visible: bool,
}

pub struct App {
    pub files: Vec<FileEntry>,
    pub expanded_dirs: HashMap<String, bool>,
    pub selected_index: usize,
    pub right_pane_content: String,
    pub debug_content: String,
    pub commit_modal: Modal,
    pub help_modal: Modal,
    pub root_dir: String,
    pub debug_mode: bool,
    pub focused_pane: FocusedPane,
    pub details_scroll: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum FocusedPane {
    FileList,
    Details,
    Debug,
}

impl App {
    pub fn new(repo: &Repository) -> Self {
        let root_dir = repo.workdir().unwrap().to_str().unwrap().to_string();
        let files = get_file_list(repo);
        Self {
            files,
            expanded_dirs: HashMap::new(),
            selected_index: 0,
            right_pane_content: String::new(),
            debug_content: String::new(), // Add this line
            commit_modal: Modal {
                content: String::new(),
                is_visible: false,
            },
            help_modal: Modal {
                content: get_help_content(),
                is_visible: false,
            },
            root_dir,
            debug_mode: false,
            focused_pane: FocusedPane::FileList,
            details_scroll: 0,
        }
    }

    pub fn handle_event(&mut self, event: Event, repo: &Repository) -> AppResult<()> {
        match event {
            Event::Key(key) => self.handle_key_event(key, repo)?,
            Event::Mouse(mouse) => self.handle_mouse_event(mouse)?,
            _ => {}
        }
        Ok(())
    }

    fn handle_mouse_event(&mut self, mouse: MouseEvent) -> AppResult<()> {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.focused_pane = match mouse.column {
                    0..=30 => FocusedPane::FileList,
                    31..=65 => FocusedPane::Details,
                    _ => FocusedPane::Debug,
                };
            }
            _ => {}
        }
        Ok(())
    }

    pub fn handle_key_event(&mut self, key: KeyEvent, repo: &Repository) -> AppResult<()> {
        if self.commit_modal.is_visible {
            match key.code {
                KeyCode::Enter => self.perform_commit(repo)?,
                KeyCode::Esc => self.close_modals(),
                KeyCode::Char(c) => self.commit_modal.content.push(c),
                KeyCode::Backspace => {
                    self.commit_modal.content.pop();
                }
                _ => {}
            }
        } else {
            match (self.focused_pane, key.code) {
                (FocusedPane::FileList, KeyCode::Up) => self.move_selection_up(),
                (FocusedPane::FileList, KeyCode::Down) => self.move_selection_down(),
                (FocusedPane::Details, KeyCode::Up) => self.scroll_details_up(),
                (FocusedPane::Details, KeyCode::Down) => self.scroll_details_down(),
                (_, KeyCode::Left) => self.set_focused_pane(FocusedPane::FileList),
                (_, KeyCode::Right) => self.set_focused_pane(FocusedPane::Details),
                (_, KeyCode::Enter) => self.toggle_directory(repo)?,
                (_, KeyCode::Char('c')) => self.start_commit(repo)?,
                (_, KeyCode::Char('?')) => self.toggle_help(),
                (_, KeyCode::Char('d')) => self.toggle_debug_mode(), // Add this line
                (_, KeyCode::Esc) => self.close_modals(),
                _ => {}
            }
        }
        Ok(())
    }

    fn scroll_details_up(&mut self) {
        if self.details_scroll > 0 {
            self.details_scroll -= 1;
        }
    }

    fn scroll_details_down(&mut self) {
        self.details_scroll += 1;
    }

    fn move_selection_up(&mut self) {
        if !self.files.is_empty() && self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    fn move_selection_down(&mut self) {
        if !self.files.is_empty() && self.selected_index < self.files.len() - 1 {
            self.selected_index += 1;
        }
    }

    fn toggle_directory(&mut self, repo: &Repository) -> AppResult<()> {
        if !self.files.is_empty() {
            let selected_file = &self.files[self.selected_index];
            if selected_file.is_dir {
                let full_path = format!("{}/{}", self.root_dir, selected_file.name);
                let is_expanded = self.expanded_dirs.entry(full_path.clone()).or_insert(false);
                *is_expanded = !*is_expanded;

                if *is_expanded {
                    let new_files = get_file_list(repo);
                    let insert_index = self.selected_index + 1;
                    for (i, file) in new_files.into_iter().enumerate() {
                        self.files.insert(insert_index + i, file);
                    }
                } else {
                    self.collapse_directory(self.selected_index);
                }
            } else {
                update_right_pane(repo, self)?;
            }
        }
        Ok(())
    }

    fn collapse_directory(&mut self, start_index: usize) {
        let mut end_index = start_index + 1;
        let start_depth = self.files[start_index].depth;
        while end_index < self.files.len() && self.files[end_index].depth > start_depth {
            end_index += 1;
        }
        self.files.drain(start_index + 1..end_index);
    }

    fn start_commit(&mut self, repo: &Repository) -> AppResult<()> {
        stage_all_modified(repo)?;
        self.commit_modal.is_visible = true;
        Ok(())
    }

    fn toggle_help(&mut self) {
        self.help_modal.is_visible = !self.help_modal.is_visible;
    }

    fn close_modals(&mut self) {
        self.commit_modal.is_visible = false;
        self.help_modal.is_visible = false;
    }

    fn perform_commit(&mut self, repo: &Repository) -> AppResult<()> {
        create_commit(repo, &self.commit_modal.content)?;
        self.commit_modal.is_visible = false;
        self.commit_modal.content.clear();
        self.files = get_file_list(repo);
        self.expanded_dirs.clear();
        self.right_pane_content.clear();
        Ok(())
    }

    pub fn debug_log(&mut self, message: &str) {
        self.debug_content.push_str(message);
        self.debug_content.push('\n');
    }

    fn toggle_debug_mode(&mut self) {
        self.debug_mode = !self.debug_mode;
    }

    fn set_focused_pane(&mut self, pane: FocusedPane) {
        self.focused_pane = pane;
    }
}

fn get_help_content() -> String {
    "
    Key Bindings:
    ↑/↓: Navigate file list
    Enter: Expand/collapse directory or view file details/diff
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
