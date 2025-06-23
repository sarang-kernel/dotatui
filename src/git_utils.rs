// This file is a crucial abstraction layer. It encapsulates all the logic for interacting with the git2 library, providing a clean and simple API to the rest of the application. By isolating this logic, we make the main application code easier to read and test.

//Key features of this file:

//    Safe API: All functions return our custom Result<T>, ensuring that any Git-related failures are properly propagated and handled.

//    Efficiency: It uses native git2 bindings, which are significantly faster and more resource-efficient than shelling out to the git command-line executable.

//    Clarity: It defines simple enums like FileStatus to represent Git states, making the data easier to work with in the UI layer.

//    Authentication: The push function is configured to use the system's SSH agent for authentication, which is a common and secure setup for services like GitHub.

// src/git_utils.rs

use crate::error::Result;
use git2::{Cred, IndexAddOption, PushOptions, RemoteCallbacks, Repository, Status, StatusOptions};
use std::path::Path;

/// Represents the status of a file in the Git working directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    New,
    Modified,
    Deleted,
    Renamed,
    Typechange,
    Conflicted,
}

/// A struct to hold information about a single file's status.
#[derive(Debug, Clone)]
pub struct StatusItem {
    pub path: String,
    pub status: FileStatus,
}

/// Retrieves the status of all changed files, separated into unstaged and staged lists.
///
/// This is a core function for the panel-based UI.
///
/// # Returns
/// A `Result` containing a tuple of `(unstaged_changes, staged_changes)`.
pub fn get_status(repo: &Repository) -> Result<(Vec<StatusItem>, Vec<StatusItem>)> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);

    let statuses = repo.statuses(Some(&mut opts))?;
    let mut unstaged = Vec::new();
    let mut staged = Vec::new();

    for entry in statuses.iter() {
        let path = match entry.path() {
            Some(p) => p.to_string(),
            None => continue,
        };
        let status = entry.status();

        // Unstaged changes are those in the Working Directory.
        if let Some(file_status) = status_to_file_status(status, false) {
            unstaged.push(StatusItem { path: path.clone(), status: file_status });
        }

        // Staged changes are those in the Index.
        if let Some(file_status) = status_to_file_status(status, true) {
            staged.push(StatusItem { path, status: file_status });
        }
    }
    Ok((unstaged, staged))
}

/// Helper to convert a detailed `git2::Status` into our simplified `FileStatus`.
fn status_to_file_status(status: Status, is_staged: bool) -> Option<FileStatus> {
    if is_staged {
        match status {
            s if s.is_index_new() => Some(FileStatus::New),
            s if s.is_index_modified() => Some(FileStatus::Modified),
            s if s.is_index_deleted() => Some(FileStatus::Deleted),
            s if s.is_index_renamed() => Some(FileStatus::Renamed),
            s if s.is_index_typechange() => Some(FileStatus::Typechange),
            _ => None,
        }
    } else {
        match status {
            s if s.is_wt_new() => Some(FileStatus::New),
            s if s.is_wt_modified() => Some(FileStatus::Modified),
            s if s.is_wt_deleted() => Some(FileStatus::Deleted),
            s if s.is_wt_renamed() => Some(FileStatus::Renamed),
            s if s.is_wt_typechange() => Some(FileStatus::Typechange),
            s if s.is_conflicted() => Some(FileStatus::Conflicted),
            _ => None,
        }
    }
}

/// Stages a single file by its path.
pub fn stage_file(repo: &Repository, path: &str) -> Result<()> {
    let mut index = repo.index()?;
    index.add_path(Path::new(path))?;
    index.write()?;
    Ok(())
}

/// Unstages a single file by its path.
pub fn unstage_file(repo: &Repository, path: &str) -> Result<()> {
    let head = repo.head()?.peel_to_commit()?;
    repo.reset_default(Some(head.as_object()), &[path])?;
    Ok(())
}

/// Stages all changes in the working directory.
pub fn stage_all(repo: &Repository) -> Result<()> {
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
    index.write()?;
    Ok(())
}

/// Unstages all changes in the index.
pub fn unstage_all(repo: &Repository) -> Result<()> {
    let head = repo.head()?.peel_to_commit()?;
    repo.reset_default(Some(head.as_object()), ["*"])?;
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

/// Finds the most recent commit on the current HEAD.
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
