// This file is a crucial abstraction layer. It encapsulates all the logic for interacting with the git2 library, providing a clean and simple API to the rest of the application. By isolating this logic, we make the main application code easier to read and test.

//Key features of this file:

//    Safe API: All functions return our custom Result<T>, ensuring that any Git-related failures are properly propagated and handled.

//    Efficiency: It uses native git2 bindings, which are significantly faster and more resource-efficient than shelling out to the git command-line executable.

//    Clarity: It defines simple enums like FileStatus to represent Git states, making the data easier to work with in the UI layer.

//    Authentication: The push function is configured to use the system's SSH agent for authentication, which is a common and secure setup for services like GitHub.

// src/git_utils.rs

use crate::error::Result;
use git2::{
    Cred, IndexAddOption, PushOptions, RemoteCallbacks, Repository, StatusOptions,
};
use std::path::Path;

/// Represents the status of a file in the Git working directory.
/// This simplified enum makes it easy for the UI to display status information.
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

/// Retrieves the status of all changed files in the repository.
///
/// This includes untracked, modified, deleted, and other changes.
///
/// # Arguments
/// * `repo` - A reference to the `git2::Repository` to query.
///
/// # Returns
/// A `Result` containing a `Vec<StatusItem>` or a `git2::Error`.
pub fn get_status(repo: &Repository) -> Result<Vec<StatusItem>> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);

    let statuses = repo.statuses(Some(&mut opts))?;
    let items = statuses
        .iter()
        .filter_map(|entry| {
            let path = entry.path()?.to_string();
            let status = entry.status();

            // Map the detailed git2 status to our simplified FileStatus enum.
            let file_status = if status.is_wt_new() {
                Some(FileStatus::New)
            } else if status.is_wt_modified() {
                Some(FileStatus::Modified)
            } else if status.is_wt_deleted() {
                Some(FileStatus::Deleted)
            } else if status.is_wt_renamed() {
                Some(FileStatus::Renamed)
            } else if status.is_wt_typechange() {
                Some(FileStatus::Typechange)
            } else if status.is_conflicted() {
                Some(FileStatus::Conflicted)
            } else {
                None // Ignore files with no changes.
            };

            file_status.map(|s| StatusItem { path, status: s })
        })
        .collect();
    Ok(items)
}

/// Stages all changes in the working directory (equivalent to `git add -A`).
pub fn add_all(repo: &Repository) -> Result<()> {
    let mut index = repo.index()?;
    // The `"*"` pattern adds all files, including untracked and deleted ones.
    index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
    index.write()?;
    Ok(())
}

/// Commits the currently staged changes.
///
/// It automatically uses the Git configuration's user name and email for the signature.
///
/// # Arguments
/// * `repo` - The repository to commit to.
/// * `message` - The commit message.
pub fn commit(repo: &Repository, message: &str) -> Result<()> {
    let signature = repo.signature()?; // Uses user.name and user.email from .gitconfig
    let mut index = repo.index()?;
    let oid = index.write_tree()?;
    let parent_commit = find_last_commit(repo)?;
    let tree = repo.find_tree(oid)?;

    repo.commit(
        Some("HEAD"), // Update the HEAD to point to this new commit
        &signature,   // Author
        &signature,   // Committer
        message,
        &tree,
        &[&parent_commit], // Parents of the new commit
    )?;
    Ok(())
}

/// Finds the most recent commit on the current HEAD.
/// This is a helper function for `commit`.
fn find_last_commit(repo: &Repository) -> Result<git2::Commit> {
    let obj = repo.head()?.resolve()?.peel(git2::ObjectType::Commit)?;
    // This unwrap is safe because we've already peeled the object to a commit.
    Ok(obj.into_commit().unwrap())
}

/// Pushes the current branch to the 'origin' remote.
///
/// This function is configured to use the system's SSH agent for authentication,
/// which is a secure and common practice.
pub fn push(repo: &Repository) -> Result<()> {
    let mut remote = repo.find_remote("origin")?;
    let mut callbacks = RemoteCallbacks::new();

    // Configure credentials to use the SSH agent.
    callbacks.credentials(|_url, username_from_url, _allowed_types| {
        Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
    });

    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(callbacks);

    // A more robust implementation could dynamically get the current branch name.
    // For dotfiles, assuming 'main' or 'master' is usually sufficient.
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
