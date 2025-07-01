// src/git_utils.rs

use crate::error::Result;
use git2::{Cred, DiffOptions, IndexAddOption, PushOptions, RemoteCallbacks, Repository, Status, StatusOptions};
use std::collections::BTreeMap;
use std::path::Path;

/// Represents the Git status of a file (e.g., Modified, New).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    Modified,
    New,
    Deleted,
    Renamed,
    Typechange,
    Conflicted,
}

/// Represents the state of a file in relation to the index and working directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StagingStatus {
    Unstaged,
    Staged,
    PartiallyStaged,
}

/// A unified struct representing a single file with changes.
#[derive(Debug, Clone)]
pub struct FileState {
    pub path: String,
    pub status: FileStatus,
    pub staging_status: StagingStatus,
}

/// Retrieves a single, unified list of all changed files.
pub fn get_status(repo: &Repository) -> Result<Vec<FileState>> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_unmodified(false);

    let statuses = repo.statuses(Some(&mut opts))?;
    let mut file_map = BTreeMap::new();

    for entry in statuses.iter() {
        let path = match entry.path() {
            Some(p) => p.to_string(),
            None => continue,
        };
        let status = entry.status();

        let is_staged = status.is_index_new() || status.is_index_modified() || status.is_index_deleted();
        let is_unstaged = status.is_wt_new() || status.is_wt_modified() || status.is_wt_deleted() || status.is_conflicted();

        let staging_status = match (is_staged, is_unstaged) {
            (true, true) => StagingStatus::PartiallyStaged,
            (true, false) => StagingStatus::Staged,
            (false, true) => StagingStatus::Unstaged,
            _ => continue,
        };

        let file_status = determine_file_status(status);

        file_map.insert(path.clone(), FileState {
            path,
            status: file_status,
            staging_status,
        });
    }

    Ok(file_map.into_values().collect())
}

/// Determines the primary status of a file (e.g., Modified, New).
fn determine_file_status(status: Status) -> FileStatus {
    if status.is_conflicted() { FileStatus::Conflicted }
    else if status.is_wt_new() || status.is_index_new() { FileStatus::New }
    else if status.is_wt_deleted() || status.is_index_deleted() { FileStatus::Deleted }
    else if status.is_wt_renamed() || status.is_index_renamed() { FileStatus::Renamed }
    else if status.is_wt_typechange() || status.is_index_typechange() { FileStatus::Typechange }
    else { FileStatus::Modified }
}

/// Generates a diff for a specific file.
pub fn get_file_diff(repo: &Repository, file: &FileState) -> Result<String> {
    let mut opts = DiffOptions::new();
    opts.pathspec(&file.path);
    opts.context_lines(3);

    let diff = match file.staging_status {
        StagingStatus::Unstaged | StagingStatus::PartiallyStaged => {
            repo.diff_index_to_workdir(None, Some(&mut opts))?
        }
        StagingStatus::Staged => {
            let head_tree = repo.head()?.peel_to_tree()?;
            repo.diff_tree_to_index(Some(&head_tree), None, Some(&mut opts))?
        }
    };

    let mut diff_text = String::new();
    diff.foreach(
        &mut |_, _| true,
        None,
        None,
        Some(&mut |_delta, _hunk, line| {
            let prefix = match line.origin() {
                '+' | '-' | ' ' => line.origin(),
                _ => ' ',
            };
            diff_text.push(prefix);
            diff_text.push_str(&String::from_utf8_lossy(line.content()));
            true
        }),
    )?;

    if diff_text.is_empty() {
        Ok("No changes to display for this file.".to_string())
    } else {
        Ok(diff_text)
    }
}

/// Stages a single file or part of a file.
pub fn stage_file(repo: &Repository, path: &str) -> Result<()> {
    let mut index = repo.index()?;
    index.add_path(Path::new(path))?;
    index.write()?;
    Ok(())
}

/// Unstages a single file or part of a file.
pub fn unstage_file(repo: &Repository, path: &str) -> Result<()> {
    let head = repo.head()?.peel_to_commit()?;
    repo.reset_default(Some(head.as_object()), &[path])?;
    Ok(())
}

/// Stages all changes.
pub fn stage_all(repo: &Repository) -> Result<()> {
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
    index.write()?;
    Ok(())
}

/// Commits the currently staged changes.
pub fn commit(repo: &Repository, message: &str) -> Result<()> {
    let signature = repo.signature()?;
    let mut index = repo.index()?;
    let oid = index.write_tree()?;
    let parent_commit = find_last_commit(repo)?;
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

fn find_last_commit(repo: &Repository) -> Result<git2::Commit> {
    let obj = repo.head()?.resolve()?.peel(git2::ObjectType::Commit)?;
    Ok(obj.into_commit().unwrap())
}

/// Pushes the current branch to the 'origin' remote.
pub fn push(repo: &Repository) -> Result<()> {
    let mut remote = repo.find_remote("origin")?;
    let mut callbacks = RemoteCallbacks::new();

    callbacks.credentials(|_url, username_from_url, _allowed_types| {
        Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
    });

    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(callbacks);

    let refspec = "refs/heads/main:refs/heads/main";
    remote.push(&[refspec], Some(&mut push_options))?;
    Ok(())
}

/// Adds a remote named 'origin' to the repository.
pub fn add_remote(repo: &Repository, url: &str) -> Result<()> {
    repo.remote("origin", url)?;
    Ok(())
}

/// Checks if a remote named 'origin' already exists.
pub fn has_remote(repo: &Repository) -> bool {
    repo.find_remote("origin").is_ok()
}

/// Initializes a new Git repository in the specified path.
pub fn init_repo(path: &Path) -> Result<Repository> {
    Ok(Repository::init(path)?)
}
