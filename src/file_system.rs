use git2::{Repository, Status};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub struct FileEntry {
    pub name: String,
    pub status: Status,
    pub is_dir: bool,
    pub parent: Option<String>,
    pub depth: usize,
}

pub fn get_file_list(repo: &Repository, current_dir: &str) -> Vec<FileEntry> {
    let mut files = Vec::new();
    let mut file_set = HashSet::new();

    let mut opts = git2::StatusOptions::new();

    let statuses = repo
        .statuses(Some(&mut opts))
        .expect("Couldn't get repository status");

    for entry in statuses.iter() {
        let path = PathBuf::from(entry.path().unwrap_or_default());
        if path.starts_with(current_dir) {
            let relative_path = path.strip_prefix(current_dir).unwrap_or(&path);
            let name = relative_path.to_string_lossy().into_owned();
            let is_dir = path.is_dir();
            let status = entry.status();
            let depth = relative_path.components().count() - 1;

            if !file_set.contains(&name) {
                files.push(FileEntry {
                    name: name.clone(),
                    status,
                    is_dir,
                    parent: None,
                    depth,
                });
                file_set.insert(name.clone());
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
