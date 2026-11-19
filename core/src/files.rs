// SPDX-FileCopyrightText: 2026 Progmasoft <support@progmasoft.com>
// SPDX-License-Identifier: MIT

//! The file tools of a session: paths stay inside its worktree, and an agent never writes over a
//! change it has not read.
//!
//! The user may edit the worktree while an agent works in it (live review). So the tools remember
//! what each file held when the agent last read or wrote it, and a write to a file that has
//! changed since, or that the agent never read, is refused: the agent reads it again instead of
//! overwriting the user's edit.

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Component, Path, PathBuf};

/// The largest file the tools read or write.
pub const MAX_FILE: u64 = 8 * 1024 * 1024;

/// Why a file tool refused.
#[derive(Debug, Eq, PartialEq)]
pub enum FileError {
    /// The path leaves the worktree, through `..`, an absolute path or a symbolic link, or names
    /// its `.git`.
    Outside(String),
    /// The file changed since the agent last read it, or the agent never read it.
    Stale(String),
    /// The file is over [`MAX_FILE`], or is not UTF-8 text.
    NotText(String),
    /// The file system failed.
    Io(String),
}

/// What a file held: its length and a hash of its bytes, enough to see that it changed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Version {
    length: usize,
    hash: u64,
}

impl Version {
    fn of(bytes: &[u8]) -> Self {
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        Self {
            length: bytes.len(),
            hash: hasher.finish(),
        }
    }
}

/// The files of one worktree, and the version of each that the agent last saw.
#[derive(Debug)]
pub struct Files {
    root: PathBuf,
    seen: HashMap<PathBuf, Version>,
}

impl Files {
    /// The tools of the worktree at `root`.
    ///
    /// # Errors
    ///
    /// [`FileError::Io`] when `root` cannot be resolved.
    pub fn new(root: &Path) -> Result<Self, FileError> {
        let root = root
            .canonicalize()
            .map_err(|error| io_error(root, &error))?;
        Ok(Self {
            root,
            seen: HashMap::new(),
        })
    }

    /// The worktree, resolved.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Reads `path`, relative to the worktree, as text, and remembers what it held.
    ///
    /// # Errors
    ///
    /// [`FileError::Outside`] for a path outside the worktree, [`FileError::NotText`] for a large
    /// or binary file, [`FileError::Io`] when it cannot be read.
    pub fn read(&mut self, path: &str) -> Result<String, FileError> {
        let (relative, full) = self.resolve(path)?;
        let length = fs::metadata(&full)
            .map_err(|error| io_error(&full, &error))?
            .len();
        if length > MAX_FILE {
            return Err(FileError::NotText(format!(
                "{path} is over {MAX_FILE} bytes"
            )));
        }
        let bytes = fs::read(&full).map_err(|error| io_error(&full, &error))?;
        let version = Version::of(&bytes);
        let text = String::from_utf8(bytes)
            .map_err(|_| FileError::NotText(format!("{path} is not UTF-8 text")))?;
        self.seen.insert(relative, version);
        Ok(text)
    }

    /// Writes `text` to `path`, relative to the worktree, creating the file and its directories
    /// when there is none. The file is replaced in one step, through a temporary file beside it.
    ///
    /// # Errors
    ///
    /// [`FileError::Stale`] when the file is there and holds something other than what the agent
    /// last read or wrote, or was there when it read it and is gone now; [`FileError::Outside`],
    /// [`FileError::NotText`] and [`FileError::Io`] as for [`Files::read`].
    pub fn write(&mut self, path: &str, text: &str) -> Result<(), FileError> {
        if text.len() as u64 > MAX_FILE {
            return Err(FileError::NotText(format!(
                "the text for {path} is over {MAX_FILE} bytes"
            )));
        }
        let (relative, full) = self.resolve(path)?;
        self.check_fresh(path, &relative, &full)?;
        replace(&full, text)?;
        self.seen.insert(relative, Version::of(text.as_bytes()));
        Ok(())
    }

    /// Refuses a write to `full` when it no longer holds what the agent last read or wrote.
    fn check_fresh(&self, path: &str, relative: &Path, full: &Path) -> Result<(), FileError> {
        let current = match fs::read(full) {
            Ok(bytes) => Some(Version::of(&bytes)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(io_error(full, &error)),
        };
        let seen = self.seen.get(relative).copied();
        if current == seen {
            return Ok(());
        }
        Err(FileError::Stale(match (current, seen) {
            (Some(_), Some(_)) => format!("{path} changed since it was last read; read it again"),
            (Some(_), None) => format!("{path} was never read; read it first"),
            (None, _) => format!("{path} was deleted since it was last read"),
        }))
    }

    /// `path` relative to the worktree and joined to it, after checking that it stays inside.
    fn resolve(&self, path: &str) -> Result<(PathBuf, PathBuf), FileError> {
        let relative = relative_path(path)?;
        let full = self.root.join(&relative);
        // A symbolic link may lead out: what the nearest existing ancestor resolves to must be inside.
        let mut existing = full.as_path();
        while fs::symlink_metadata(existing).is_err() {
            existing = existing.parent().ok_or_else(|| outside(path))?;
        }
        let resolved = existing
            .canonicalize()
            .map_err(|error| io_error(existing, &error))?;
        if !resolved.starts_with(&self.root) {
            return Err(outside(path));
        }
        Ok((relative, full))
    }
}

/// `path` as a relative path of normal components: no `..`, root, prefix or NUL, not empty and
/// not in `.git`.
fn relative_path(path: &str) -> Result<PathBuf, FileError> {
    if path.contains('\0') {
        return Err(outside(path));
    }
    let mut relative = PathBuf::new();
    for component in Path::new(path).components() {
        match component {
            Component::Normal(part) => relative.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(outside(path));
            }
        }
    }
    if relative.as_os_str().is_empty() {
        return Err(outside(path));
    }
    // The worktree's .git leads to the repository: its hooks and config run outside the sandbox.
    if relative.components().next() == Some(Component::Normal(".git".as_ref())) {
        return Err(FileError::Outside(format!(
            "{path} is in .git, which the agent may not touch"
        )));
    }
    Ok(relative)
}

/// Replaces `full` with `text` in one step, through a temporary file beside it.
fn replace(full: &Path, text: &str) -> Result<(), FileError> {
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).map_err(|error| io_error(parent, &error))?;
    }
    let temporary = full.with_file_name(format!(
        ".{}.agenceland-{}",
        full.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id()
    ));
    fs::write(&temporary, text).map_err(|error| io_error(&temporary, &error))?;
    fs::rename(&temporary, full).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        io_error(full, &error)
    })
}

fn outside(path: &str) -> FileError {
    FileError::Outside(format!("{path} is not a path inside the worktree"))
}

fn io_error(path: &Path, error: &io::Error) -> FileError {
    FileError::Io(format!("{}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::{FileError, Files};

    fn worktree(name: &str) -> PathBuf {
        // Below the crate's own target directory, not the temporary directory other users share.
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-scratch")
            .join(format!("files-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn a_change_the_agent_did_not_read_is_never_written_over() {
        let root = worktree("stale");
        fs::write(root.join("a.txt"), "first").unwrap();
        let mut files = Files::new(&root).unwrap();
        assert!(matches!(
            files.write("a.txt", "agent"),
            Err(FileError::Stale(_))
        ));
        assert_eq!(files.read("a.txt").unwrap(), "first");
        fs::write(root.join("a.txt"), "the user's edit").unwrap();
        assert!(matches!(
            files.write("a.txt", "agent"),
            Err(FileError::Stale(_))
        ));
        assert_eq!(
            fs::read_to_string(root.join("a.txt")).unwrap(),
            "the user's edit"
        );
        assert_eq!(files.read("a.txt").unwrap(), "the user's edit");
        files.write("a.txt", "agent").unwrap();
        files.write("a.txt", "agent again").unwrap();
        assert_eq!(
            fs::read_to_string(root.join("a.txt")).unwrap(),
            "agent again"
        );
        fs::remove_file(root.join("a.txt")).unwrap();
        assert!(matches!(
            files.write("a.txt", "x"),
            Err(FileError::Stale(_))
        ));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_new_file_and_its_directories_are_created() {
        let root = worktree("new");
        let mut files = Files::new(&root).unwrap();
        files.write("src/new/b.rs", "fn b() {}").unwrap();
        assert_eq!(
            fs::read_to_string(root.join("src/new/b.rs")).unwrap(),
            "fn b() {}"
        );
        let leftovers = fs::read_dir(root.join("src/new")).unwrap().count();
        assert_eq!(leftovers, 1);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn paths_that_leave_the_worktree_are_refused() {
        let root = worktree("outside");
        fs::create_dir_all(root.join(".git")).unwrap();
        let mut files = Files::new(&root).unwrap();
        for bad in [
            "../x",
            "/etc/passwd",
            "a/../../x",
            "",
            ".",
            ".git/config",
            "./.git/hooks/pre-commit",
        ] {
            assert!(
                matches!(files.read(bad), Err(FileError::Outside(_))),
                "{bad}"
            );
            assert!(
                matches!(files.write(bad, "x"), Err(FileError::Outside(_))),
                "{bad}"
            );
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn a_symbolic_link_out_of_the_worktree_is_refused() {
        let root = worktree("link");
        let elsewhere = worktree("link-target");
        fs::write(elsewhere.join("secret"), "key").unwrap();
        std::os::unix::fs::symlink(&elsewhere, root.join("out")).unwrap();
        let mut files = Files::new(&root).unwrap();
        assert!(matches!(
            files.read("out/secret"),
            Err(FileError::Outside(_))
        ));
        assert!(matches!(
            files.write("out/new", "x"),
            Err(FileError::Outside(_))
        ));
        assert!(!elsewhere.join("new").exists());
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&elsewhere);
    }

    #[test]
    fn binary_files_are_not_text() {
        let root = worktree("binary");
        fs::write(root.join("b.bin"), [0xFF, 0xFE, 0x00]).unwrap();
        let mut files = Files::new(&root).unwrap();
        assert!(matches!(files.read("b.bin"), Err(FileError::NotText(_))));
        let _ = fs::remove_dir_all(&root);
    }
}
