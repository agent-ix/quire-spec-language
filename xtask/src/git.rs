// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL #138: the only two git operations `revendor` performs, both read-only
//! against a repository the caller already has locally. Neither ever runs
//! `git fetch`, `git pull` or resolves a symbolic ref; the manifest always
//! supplies a full 40-character commit sha.
use crate::error::{Error, Result};
use std::path::Path;
use std::process::Command;

/// Variables through which an ambient git invocation (for example one
/// launched from inside a git hook) can redirect `-C <repo>` to a different
/// repository or working tree. Every command this module runs clears them,
/// so `-C` always wins.
const GIT_ENV_OVERRIDES: [&str; 5] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
];

fn git_command(repo: &Path) -> Command {
    let mut command = Command::new("git");
    command.arg("-C").arg(repo);
    for variable in GIT_ENV_OVERRIDES {
        command.env_remove(variable);
    }
    command
}

/// `git -C <repo> cat-file -e <commit>^{commit}`: confirm the pinned commit
/// is actually present, without downloading anything.
pub fn commit_exists(repo: &Path, commit: &str) -> Result<bool> {
    let output = git_command(repo)
        .arg("cat-file")
        .arg("-e")
        .arg(format!("{commit}^{{commit}}"))
        .output()
        .map_err(|source| Error::GitSpawn {
            repo: repo.to_owned(),
            source,
        })?;
    Ok(output.status.success())
}

/// `git -C <repo> show <commit>:<path>`: read one file's exact bytes at the
/// pinned commit. Never resolves "latest", never touches the working tree.
pub fn show(repo: &Path, commit: &str, path: &str) -> Result<Vec<u8>> {
    let output = git_command(repo)
        .arg("show")
        .arg(format!("{commit}:{path}"))
        .output()
        .map_err(|source| Error::GitSpawn {
            repo: repo.to_owned(),
            source,
        })?;
    if !output.status.success() {
        return Err(Error::Git {
            repo: repo.to_owned(),
            commit: commit.to_owned(),
            path: path.to_owned(),
            message: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(output.stdout)
}

/// Confirm the commit is present before reading anything from it.
pub fn require_commit(repo: &Path, commit: &str) -> Result<()> {
    if commit_exists(repo, commit)? {
        Ok(())
    } else {
        Err(Error::UnknownCommit {
            repo: repo.to_owned(),
            commit: commit.to_owned(),
        })
    }
}
