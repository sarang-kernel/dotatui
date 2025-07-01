//! src/git.rs

use crate::error::{AppError, AppResult};
use chrono::{DateTime, Local};
// Removed Cred, PushOptions, RemoteCallbacks from this line
use git2::{Commit, DiffOptions, Repository, Status, StatusOptions};
use std::path::{Path, PathBuf};

pub struct GitRepo {
    repo: Repository,
    path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusItem {
    pub path: String,
    pub status: Status,
    pub is_staged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitInfo {
    pub id: String,
    pub message: String,
    pub author: String,
    pub time: String,
}

impl GitRepo {
    pub fn new<P: AsRef<Path>>(path: P) -> AppResult<Self> {
        let repo = Repository::discover(path.as_ref()).map_err(|_| AppError::RepoNotFound)?;
        let path = repo.path().parent().unwrap().to_path_buf();
        Ok(Self { repo, path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn path_str(&self) -> &str {
        self.path.to_str().unwrap_or("Invalid UTF-8 Path")
    }

    pub fn get_status(&self) -> AppResult<Vec<StatusItem>> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true);

        let statuses = self.repo.statuses(Some(&mut opts))?;
        let mut items = Vec::new();

        for entry in statuses.iter() {
            if let Some(path) = entry.path() {
                let status = entry.status();
                if status.is_wt_new()
                    || status.is_wt_modified()
                    || status.is_wt_deleted()
                    || status.is_wt_renamed()
                    || status.is_wt_typechange()
                {
                    items.push(StatusItem {
                        path: path.to_string(),
                        status,
                        is_staged: false,
                    });
                }
                if status.is_index_new()
                    || status.is_index_modified()
                    || status.is_index_deleted()
                    || status.is_index_renamed()
                    || status.is_index_typechange()
                {
                    items.push(StatusItem {
                        path: path.to_string(),
                        status,
                        is_staged: true,
                    });
                }
            }
        }
        Ok(items)
    }

    pub fn get_diff(&self, item: &StatusItem) -> AppResult<String> {
        let mut opts = DiffOptions::new();
        opts.pathspec(&item.path);

        let diff_target = if item.is_staged {
            let head_commit = self.find_last_commit()?;
            let tree = head_commit.tree()?;
            Some(tree)
        } else {
            None
        };

        let diff = if item.is_staged {
            self.repo
                .diff_tree_to_index(diff_target.as_ref(), None, Some(&mut opts))?
        } else {
            self.repo.diff_index_to_workdir(None, Some(&mut opts))?
        };

        let mut diff_text = String::new();
        diff.print(git2::DiffFormat::Patch, |_, _, line| {
            let prefix = match line.origin() {
                '+' | '>' => "+",
                '-' | '<' => "-",
                _ => " ",
            };
            if let Ok(content) = std::str::from_utf8(line.content()) {
                diff_text.push_str(&format!("{}{}", prefix, content));
            }
            true
        })?;
        Ok(diff_text)
    }

    pub fn stage_file(&self, path: &str) -> AppResult<()> {
        let mut index = self.repo.index()?;
        index.add_path(Path::new(path))?;
        index.write()?;
        Ok(())
    }

    pub fn unstage_file(&self, path: &str) -> AppResult<()> {
        let head = self.repo.head()?.peel(git2::ObjectType::Commit)?;
        let path_obj = Some(Path::new(path));
        self.repo.reset_default(Some(&head), path_obj)?;
        Ok(())
    }

    pub fn commit(&self, message: &str) -> AppResult<()> {
        let mut index = self.repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = self.repo.find_tree(tree_id)?;
        let signature = self.repo.signature()?;
        let parent_commit = self.find_last_commit()?;

        self.repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&parent_commit],
        )?;

        Ok(())
    }

    fn find_last_commit(&self) -> AppResult<Commit<'_>> {
        let obj = self.repo.head()?.resolve()?.peel(git2::ObjectType::Commit)?;
        Ok(obj
            .into_commit()
            .map_err(|_| git2::Error::from_str("Couldn't find commit"))?)
    }

    pub fn get_log(&self) -> AppResult<Vec<CommitInfo>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::TIME)?;

        let mut commits = Vec::new();
        for oid in revwalk {
            let commit = self.repo.find_commit(oid?)?;
            let author = commit.author();
            let name = author.name().unwrap_or("Unknown");

            let dt = DateTime::from_timestamp(commit.time().seconds(), 0).unwrap_or_default();
            let local_dt: DateTime<Local> = dt.into();

            commits.push(CommitInfo {
                id: commit.id().to_string().chars().take(7).collect(),
                message: commit.summary().unwrap_or("").to_string(),
                author: name.to_string(),
                time: local_dt.format("%Y-%m-%d %H:%M:%S").to_string(),
            });
        }
        Ok(commits)
    }
}
