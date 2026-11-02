// SPDX-License-Identifier: AGPL-3.0-or-later
//! The in-memory file tree and its move into place: the tree is written into
//! a sibling temporary directory and renamed, so a failure leaves nothing at
//! the requested path or beside it.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use super::{Environment, Error};

/// Relative path to exact bytes, in path order.
#[derive(Debug, Default)]
pub(super) struct Files(BTreeMap<String, Vec<u8>>);

impl Files {
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// Add `contents` at `path`, which must stay inside the directory.
    pub(super) fn insert(&mut self, path: &str, contents: Vec<u8>) -> Result<(), Error> {
        let inside = !path.is_empty()
            && Path::new(path)
                .components()
                .all(|component| matches!(component, Component::Normal(_)));
        if !inside {
            return Err(Error::Label {
                label: path.to_owned(),
            });
        }
        self.0.insert(path.to_owned(), contents);
        Ok(())
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = (&String, &Vec<u8>)> {
        self.0.iter()
    }
}

/// Whether anything exists at `path`, a dangling link included.
pub(super) fn exists(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

fn io(path: &Path) -> impl Fn(std::io::Error) -> Error + '_ {
    move |source| Error::Io {
        path: path.display().to_string(),
        source,
    }
}

/// The unused sibling of `directory` the tree is built in.
fn sibling(directory: &Path) -> PathBuf {
    let name = directory.file_name().map_or_else(
        || "handoff".to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    let parent = directory.parent().unwrap_or_else(|| Path::new("."));
    let mut attempt = 0_u32;
    loop {
        let candidate = parent.join(format!(".{name}.writing-{}-{attempt}", std::process::id()));
        if !exists(&candidate) {
            return candidate;
        }
        attempt += 1;
    }
}

/// Build `files` in a sibling temporary directory and move it to
/// `directory`. On any failure the temporary directory is removed.
pub(super) fn publish(
    directory: &Path,
    files: &Files,
    environment: &dyn Environment,
) -> Result<(), Error> {
    let temporary = sibling(directory);
    std::fs::create_dir(&temporary).map_err(io(&temporary))?;
    let built =
        build(&temporary, files, environment).and_then(|()| move_into_place(&temporary, directory));
    if built.is_err() {
        // Best effort: the temporary tree is ours, and a failed removal can
        // only leave that tree behind (a later write picks another name), never
        // anything at `directory`.
        let _ = std::fs::remove_dir_all(&temporary);
    }
    built
}

/// Move the finished tree to `directory`, which must not exist.
///
/// The writer relies on POSIX `rename` semantics (this repository is built and
/// tested on macOS and Linux only; see the Makefile). Renaming a directory onto
/// a path that does not exist is atomic, so a crash leaves either the finished
/// tree or nothing at `directory`, and never an empty claim that every later
/// write would refuse. The check and the rename are two calls, so a creator
/// that races between them with a non-empty directory makes the rename fail
/// (`ENOTEMPTY`, reported as an I/O error) and the temporary tree is removed;
/// one that races with an empty directory is replaced.
fn move_into_place(temporary: &Path, directory: &Path) -> Result<(), Error> {
    if exists(directory) {
        return Err(Error::Exists {
            path: directory.display().to_string(),
        });
    }
    std::fs::rename(temporary, directory).map_err(io(directory))
}

fn build(root: &Path, files: &Files, environment: &dyn Environment) -> Result<(), Error> {
    for (index, (path, contents)) in files.iter().enumerate() {
        let target = root.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(io(parent))?;
        }
        environment
            .write_file(index, &target, contents)
            .map_err(io(&target))?;
    }
    Ok(())
}
