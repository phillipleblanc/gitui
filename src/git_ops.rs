use crate::app::App;
use git2::{Diff, DiffOptions, Repository, Status};
use std::path::{Path, PathBuf};

pub fn update_right_pane(repo: &Repository, app: &mut App) -> Result<(), git2::Error> {
    let selected_file = &app.files[app.selected_index];
    let path = PathBuf::from(&selected_file.name);

    if selected_file.is_dir {
        app.right_pane_content = format!("Directory: {}", selected_file.name);
    } else {
        let mut diff_content = String::new();

        // Check for unstaged changes
        let mut opts = DiffOptions::new();
        opts.pathspec(selected_file.name.clone());
        opts.include_untracked(true);

        let diff = repo.diff_index_to_workdir(None, Some(&mut opts))?;
        diff_content.push_str("Unstaged changes:\n");
        append_diff(&mut diff_content, &diff, &path)?;

        // Check for staged changes
        let head = repo.head()?;
        let tree = head.peel_to_tree()?;
        let diff = repo.diff_tree_to_index(Some(&tree), None, Some(&mut opts))?;
        diff_content.push_str("\nStaged changes:\n");
        append_diff(&mut diff_content, &diff, &path)?;

        app.right_pane_content = if diff_content.trim() == "Unstaged changes:\nStaged changes:" {
            format!("No changes detected for file: {}", selected_file.name)
        } else {
            diff_content
        };
    }

    Ok(())
}

pub fn append_diff(content: &mut String, diff: &Diff, path: &Path) -> Result<(), git2::Error> {
    let mut has_changes = false;
    diff.print(git2::DiffFormat::Patch, |delta, _, line| {
        if delta.new_file().path() == Some(path) || delta.old_file().path() == Some(path) {
            has_changes = true;
            use git2::DiffLineType;
            match line.origin_value() {
                DiffLineType::Addition => content.push('+'),
                DiffLineType::Deletion => content.push('-'),
                DiffLineType::AddEOFNL => content.push_str("+\n"),
                DiffLineType::DeleteEOFNL => content.push_str("-\n"),
                DiffLineType::Context => content.push(' '),
                _ => {}
            }
            content.push_str(std::str::from_utf8(line.content()).unwrap_or(""));
        }
        true
    })?;
    if !has_changes {
        content.push_str("No changes\n");
    }
    Ok(())
}

pub fn stage_all_modified(repo: &Repository) -> Result<(), git2::Error> {
    let mut index = repo.index()?;
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true);
    let statuses = repo.statuses(Some(&mut opts))?;

    for entry in statuses.iter() {
        let path = entry.path().unwrap_or_default();
        if entry.status() != Status::CURRENT {
            index.add_path(Path::new(path))?;
        }
    }

    index.write()?;
    Ok(())
}

pub fn create_commit(repo: &Repository, message: &str) -> Result<(), git2::Error> {
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
