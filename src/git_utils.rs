// src/git_utils.rs

use crate::error::Result;
use git2::{Cred, DiffOptions, IndexAddOption, PushOptions, RemoteCallbacks, Repository, Status, StatusOptions};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    New,
    Modified,
    Deleted,
    Renamed,
    Typechange,
    Conflicted,
}

#[derive(Debug, Clone)]
pub struct StatusItem {
    pub path: String,
    pub status: FileStatus,
}

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

        if let Some(file_status) = status_to_file_status(status, false) {
            unstaged.push(StatusItem { path: path.clone(), status: file_status });
        }

        if let Some(file_status) = status_to_file_status(status, true) {
            staged.push(StatusItem { path, status: file_status });
        }
    }
    Ok((unstaged, staged))
}

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

pub fn get_file_diff(repo: &Repository, file_path: &str, is_staged: bool) -> Result<String> {
    let mut opts = DiffOptions::new();
    opts.pathspec(file_path);
    opts.context_lines(3);

    let diff = if is_staged {
        let head_tree = repo.head()?.peel_to_tree()?;
        repo.diff_tree_to_index(Some(&head_tree), None, Some(&mut opts))?
    } else {
        repo.diff_index_to_workdir(None, Some(&mut opts))?
    };

    let mut diff_text = String::new();
    diff.foreach(
        &mut |_, _| true,
        None,
        None,
        // CORRECTED: Prefixed unused variables with an underscore.
        Some(&mut |_delta, _hunk, line| {
            let (prefix, content) = match line.origin() {
                '+' | '-' | ' ' => (line.origin().to_string(), String::from_utf8_lossy(line.content()).to_string()),
                _ => ("".to_string(), "".to_string()),
            };
            diff_text.push_str(&format!("{} {}", prefix, content));
            true
        }),
    )?;

    if diff_text.is_empty() {
        Ok("No changes to display.".to_string())
    } else {
        Ok(diff_text)
    }
}

pub fn stage_file(repo: &Repository, path: &str) -> Result<()> {
    let mut index = repo.index()?;
    index.add_path(Path::new(path))?;
    index.write()?;
    Ok(())
}

pub fn unstage_file(repo: &Repository, path: &str) -> Result<()> {
    let head = repo.head()?.peel_to_commit()?;
    repo.reset_default(Some(head.as_object()), &[path])?;
    Ok(())
}

pub fn stage_all(repo: &Repository) -> Result<()> {
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
    index.write()?;
    Ok(())
}

pub fn unstage_all(repo: &Repository) -> Result<()> {
    let head = repo.head()?.peel_to_commit()?;
    repo.reset_default(Some(head.as_object()), ["*"])?;
    Ok(())
}

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

pub fn add_remote(repo: &Repository, url: &str) -> Result<()> {
    repo.remote("origin", url)?;
    Ok(())
}

pub fn has_remote(repo: &Repository) -> bool {
    repo.find_remote("origin").is_ok()
}

pub fn init_repo(path: &Path) -> Result<Repository> {
    Ok(Repository::init(path)?)
}
