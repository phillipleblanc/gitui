use crate::file_system::{get_file_list, FileEntry};
use crate::git_ops::{create_commit, stage_all_modified, update_right_pane};
use crossterm::event::KeyCode;
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
    pub commit_modal: Modal,
    pub help_modal: Modal,
    pub root_dir: String,
    pub debug_mode: bool,
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
            commit_modal: Modal {
                content: String::new(),
                is_visible: false,
            },
            help_modal: Modal {
                content: get_help_content(),
                is_visible: false,
            },
            root_dir,
            debug_mode: true, // Add this line
        }
    }

    pub fn handle_key_event(
        &mut self,
        key: crossterm::event::KeyEvent,
        repo: &Repository,
    ) -> AppResult<()> {
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
            match key.code {
                KeyCode::Up => self.move_selection_up(),
                KeyCode::Down => self.move_selection_down(),
                KeyCode::Enter => self.toggle_directory(repo)?,
                KeyCode::Char('c') => self.start_commit(repo)?,
                KeyCode::Char('?') => self.toggle_help(),
                KeyCode::Char('d') => self.toggle_debug_mode(), // Add this line
                KeyCode::Esc => self.close_modals(),
                _ => {}
            }
        }
        Ok(())
    }

    pub fn is_modal_visible(&self) -> bool {
        self.commit_modal.is_visible || self.help_modal.is_visible
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

    // Add this new method
    pub fn debug_log(&mut self, message: &str) {
        if self.debug_mode {
            self.right_pane_content.push_str(message);
            self.right_pane_content.push_str("\n\n");
        }
    }

    // Add this new method
    fn toggle_debug_mode(&mut self) {
        self.debug_mode = !self.debug_mode;
        if self.debug_mode {
            self.debug_log("Debug mode enabled");
        } else {
            self.right_pane_content.clear();
        }
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
