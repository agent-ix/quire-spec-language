// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL #138: small filesystem helpers shared by `revendor` and `revendor_check`.
use crate::error::{Error, Result};
use std::path::{Path, PathBuf};

/// Every entry under `root` that is not itself a directory, as slash-separated
/// paths relative to it, in sorted order. This includes symlinks and any other
/// non-regular entry (a vendored tree should hold nothing but plain files, so
/// such an entry is always surfaced to the caller rather than silently
/// skipped); [`walk_relative`] does not follow a symlink into a subtree.
pub fn walk_relative(root: &Path) -> Result<Vec<String>> {
    let mut out = Vec::new();
    walk_into(root, root, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk_into(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<()> {
    let entries = std::fs::read_dir(dir).map_err(|source| Error::io(dir, source))?;
    for entry in entries {
        let entry = entry.map_err(|source| Error::io(dir, source))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|source| Error::io(&path, source))?;
        if file_type.is_dir() {
            walk_into(root, &path, out)?;
        } else {
            // A regular file, or anything else (symlink, socket, fifo, ...).
            // Neither the manifest nor a vendored tree ever contains the
            // latter, so recording it here is what lets the caller's
            // known-destination comparison catch it as stray.
            let rel = path
                .strip_prefix(root)
                .expect("walked path is under root")
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/");
            out.push(rel);
        }
    }
    Ok(())
}

/// Write `bytes` to `root/rel`, creating parent directories as needed.
/// Returns whether the file's content actually changed.
pub fn write_if_changed(root: &Path, rel: &str, bytes: &[u8]) -> Result<bool> {
    let path: PathBuf = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| Error::io(parent, source))?;
    }
    let changed = match std::fs::read(&path) {
        Ok(existing) => existing != bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
        Err(source) => return Err(Error::io(&path, source)),
    };
    if changed {
        std::fs::write(&path, bytes).map_err(|source| Error::io(&path, source))?;
    }
    Ok(changed)
}

pub fn read(root: &Path, rel: &str) -> Result<Vec<u8>> {
    let path = root.join(rel);
    std::fs::read(&path).map_err(|source| Error::io(&path, source))
}

/// Remove `root/rel`. `std::fs::remove_file` un-links the entry itself
/// without following it, so a stray symlink is removed rather than its
/// target.
pub fn remove(root: &Path, rel: &str) -> Result<()> {
    let path = root.join(rel);
    std::fs::remove_file(&path).map_err(|source| Error::io(&path, source))
}
