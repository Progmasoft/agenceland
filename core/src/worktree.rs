// SPDX-FileCopyrightText: 2026 Progmasoft <support@progmasoft.com>
// SPDX-License-Identifier: MIT

//! The Git worktree of a session: each session starts its own branch from the commit it begins
//! at, in a worktree of its own, so that agents never write in the user's checkout or in each
//! other's. Closing a session removes the worktree and keeps the branch.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The worktree of one session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionWorktree {
    /// Where it is.
    pub path: PathBuf,
    /// Its branch, `agenceland/<session id>`.
    pub branch: String,
    /// The commit the session started at.
    pub start_commit: String,
}

/// Runs `git` with `arguments` in `directory` and returns what it printed.
///
/// # Errors
///
/// What Git wrote to standard error when it failed, or why it could not start.
pub fn git(directory: &Path, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(arguments)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|error| format!("git could not start: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_owned())
    } else {
        Err(format!(
            "git {} failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// Starts the worktree of session `id` below `worktrees`, on a new branch from the commit
/// `repository` has checked out.
///
/// # Errors
///
/// When `repository` has no commit, the branch or the directory is there already, or Git fails.
pub fn start(repository: &Path, worktrees: &Path, id: &str) -> Result<SessionWorktree, String> {
    let start_commit = git(repository, &["rev-parse", "--verify", "HEAD^{commit}"])?;
    let branch = format!("agenceland/{id}");
    let path = worktrees.join(id);
    if path.exists() {
        return Err(format!("{} is there already", path.display()));
    }
    std::fs::create_dir_all(worktrees)
        .map_err(|error| format!("{}: {error}", worktrees.display()))?;
    let target = path
        .to_str()
        .ok_or_else(|| format!("{} is not UTF-8", path.display()))?;
    git(
        repository,
        &[
            "worktree",
            "add",
            "--quiet",
            "-b",
            &branch,
            target,
            &start_commit,
        ],
    )?;
    Ok(SessionWorktree {
        path,
        branch,
        start_commit,
    })
}

/// Removes the worktree of a session; its branch stays, with whatever the session committed.
/// A worktree with changes that are not committed is kept unless `discard` says to throw them
/// away, so that closing a session never loses work silently.
///
/// # Errors
///
/// When the worktree has changes and `discard` is false, or Git fails.
pub fn close(repository: &Path, worktree: &SessionWorktree, discard: bool) -> Result<(), String> {
    let target = worktree
        .path
        .to_str()
        .ok_or_else(|| format!("{} is not UTF-8", worktree.path.display()))?;
    let changes = git(
        &worktree.path,
        &["status", "--porcelain", "--untracked-files=all"],
    )?;
    if !changes.is_empty() && !discard {
        return Err(format!(
            "the worktree of {} has changes that are not committed; commit them, or close with discard",
            worktree.branch
        ));
    }
    let mut arguments = vec!["worktree", "remove"];
    if discard {
        arguments.push("--force");
    }
    arguments.push(target);
    git(repository, &arguments)?;
    Ok(())
}
