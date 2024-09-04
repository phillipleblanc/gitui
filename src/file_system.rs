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
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false);

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

    // Add untracked files and directories
    add_untracked_files(
        repo,
        Path::new(current_dir),
        &mut files,
        &mut file_set,
        current_dir,
    );

    files.sort_by(|a, b| {
        if a.is_dir == b.is_dir {
            a.name.cmp(&b.name)
        } else {
            b.is_dir.cmp(&a.is_dir)
        }
    });

    files
}

fn add_untracked_files(
    repo: &Repository,
    dir: &Path,
    files: &mut Vec<FileEntry>,
    file_set: &mut HashSet<String>,
    current_dir: &str,
) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(file_name) = path.file_name() {
                let name = file_name.to_string_lossy().into_owned();
                if !file_set.contains(&name) {
                    let is_dir = path.is_dir();
                    let parent = path.parent().and_then(|p| p.to_str()).map(String::from);
                    let status = repo.status_file(&path).unwrap_or(Status::WT_NEW);

                    files.push(FileEntry {
                        name: name.clone(),
                        status,
                        is_dir,
                        parent,
                        depth: current_dir.split('/').count(),
                    });
                    file_set.insert(name.clone());

                    if is_dir {
                        let subdir = if current_dir.is_empty() {
                            name
                        } else {
                            format!("{}/{}", current_dir, name)
                        };
                        add_untracked_files(repo, &path, files, file_set, &subdir);
                    }
                }
            }
        }
    }
}
