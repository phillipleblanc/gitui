use git2::{Repository, Status};
use std::collections::HashSet;
use std::path::PathBuf;

use crate::debug;

pub struct FileEntry {
    pub name: String,
    pub status: Status,
    pub is_dir: bool,
    pub depth: usize,
}

pub fn get_file_list(repo: &Repository) -> Vec<FileEntry> {
    let mut files = Vec::new();
    let mut file_set = HashSet::new();

    let mut opts = git2::StatusOptions::new();

    let statuses = repo
        .statuses(Some(&mut opts))
        .expect("Couldn't get repository status");

    let mut entries_debug = String::new();
    for entry in statuses.iter() {
        entries_debug.push_str(entry.path().unwrap_or_default());
        entries_debug.push('\n');
        let path = PathBuf::from(entry.path().unwrap_or_default());
        let name = path.to_string_lossy().into_owned();
        let is_dir = path.is_dir();
        let status = entry.status();
        let depth = path.components().count() - 1;

        if !file_set.contains(&name) {
            files.push(FileEntry {
                name: name.clone(),
                status,
                is_dir,
                depth,
            });
            file_set.insert(name.clone());
        }
    }

    debug::debug_log(&entries_debug);

    files.sort_by(|a, b| {
        if a.is_dir == b.is_dir {
            a.name.cmp(&b.name)
        } else {
            b.is_dir.cmp(&a.is_dir)
        }
    });

    files
}
