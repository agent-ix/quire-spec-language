// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL #138: one error type for every re-vendor and drift-check failure.
use std::{fmt, io, path::PathBuf};

/// Stable failure category, independent of message prose.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Code {
    Usage,
    Io,
    Manifest,
    Git,
    MissingClone,
    Drift,
}

impl Code {
    fn as_str(self) -> &'static str {
        match self {
            Self::Usage => "usage",
            Self::Io => "io",
            Self::Manifest => "manifest",
            Self::Git => "git",
            Self::MissingClone => "missing-clone",
            Self::Drift => "drift",
        }
    }
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One concrete error type for the manifest, git and filesystem boundaries.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("usage: {0}")]
    Usage(&'static str),
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("{path}: malformed VENDOR.json: {source}")]
    Manifest {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("{path}: {message}")]
    InvalidManifest { path: PathBuf, message: String },
    #[error("git -C {repo} show {commit}:{path}: {message}")]
    Git {
        repo: PathBuf,
        commit: String,
        path: String,
        message: String,
    },
    #[error("git -C {repo} cannot spawn: {source}")]
    GitSpawn {
        repo: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error(
        "commit {commit} is not present in {repo}; this command never fetches over the network"
    )]
    UnknownCommit { repo: PathBuf, commit: String },
    #[error("no {flag} was given; it is required to revendor pinned commit {commit}")]
    MissingClone { commit: String, flag: &'static str },
    #[error("revendor-check does not take --qspec-clone; it checks recorded digests offline")]
    CheckRefusesQspecClone,
    #[error("revendor-check does not take --fcd-clone; it checks recorded digests offline")]
    CheckRefusesFcdClone,
    #[error(
        "{dest}: on-disk sha256 {actual} does not match the manifest's recorded {expected}; \
         external sources are never fetched, so re-download and update the manifest by hand"
    )]
    ExternalDrift {
        dest: String,
        expected: String,
        actual: String,
    },
    #[error("revendor-check found drift:\n{summary}")]
    CheckFailed { summary: String },
}

impl Error {
    pub fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    pub fn code(&self) -> Code {
        match self {
            Self::Usage(_) | Self::CheckRefusesQspecClone | Self::CheckRefusesFcdClone => {
                Code::Usage
            }
            Self::Io { .. } | Self::GitSpawn { .. } => Code::Io,
            Self::Manifest { .. } | Self::InvalidManifest { .. } => Code::Manifest,
            Self::Git { .. } | Self::UnknownCommit { .. } => Code::Git,
            Self::MissingClone { .. } => Code::MissingClone,
            Self::ExternalDrift { .. } | Self::CheckFailed { .. } => Code::Drift,
        }
    }

    /// Distinguish usage/environment failure (2) from a genuine content drift (1).
    pub fn exit_code(&self) -> u8 {
        match self.code() {
            Code::Drift => 1,
            Code::Usage | Code::Io | Code::Manifest | Code::Git | Code::MissingClone => 2,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
