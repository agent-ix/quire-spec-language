// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `qsl-string-edge` scanner's integration tests: the public library, the
//! command and the worked consumer runs of FR-064, on fixture trees outside
//! any QSL layout.
#![forbid(unsafe_code)]

mod command;
mod consumer;
mod library;
mod links;
mod marker;
mod rules;

use std::fs;
use std::path::{Path, PathBuf};

/// A fixture directory whose path is already canonical (macOS reports
/// `$TMPDIR` under the `/var` symlink).
pub(crate) struct Tree {
    dir: tempfile::TempDir,
}

impl Tree {
    pub(crate) fn new() -> Self {
        let base = std::env::temp_dir()
            .canonicalize()
            .expect("canonical temp dir");
        Self {
            dir: tempfile::tempdir_in(base).expect("tempdir"),
        }
    }

    pub(crate) fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Writes `content` at `relative`, creating its directories.
    pub(crate) fn write(&self, relative: &str, content: &str) -> PathBuf {
        let path = self.dir.path().join(relative);
        fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
        fs::write(&path, content).expect("write");
        path
    }
}

/// A production file holding one unmarked, branch-gating literal dispatch on
/// its first line.
pub(crate) const VIOLATION: &str =
    "pub fn accepted(kind: &str) -> bool { match kind { \"accepted\" => true, _ => false } }\n";

/// A production file with no string dispatch.
pub(crate) const CLEAN: &str = "pub enum Kind { Accepted, Other }\n\
     pub fn accepted(kind: Kind) -> bool { matches!(kind, Kind::Accepted) }\n";

/// The command-line request for `roots` under `base`.
pub(crate) fn request(base: &Path, roots: &[&str]) -> qsl_string_edge::ScanRequest {
    qsl_string_edge::ScanRequest {
        base_dir: base.to_path_buf(),
        crate_roots: roots.iter().map(PathBuf::from).collect(),
        rules: qsl_string_edge::EdgeRules::default(),
    }
}
