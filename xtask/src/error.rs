// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL #138: one error type for every re-vendor and drift-check failure.
use std::{fmt, io, path::PathBuf};

/// Which repository owns a pinned commit: this repository's own working
/// copy (`Source::SelfRepo`), a local `agent-ix/quire-specification` clone
/// (`Source::Qspec`), or a local `agent-ix/filament-core-data` clone
/// (`Source::Fcd`). Carried on [`Error::MissingClone`] so the refusal names
/// which of the two `--*-clone` flags is missing, not just that one is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommitOwner {
    SelfRepo,
    Qspec,
    Fcd,
}

impl fmt::Display for CommitOwner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::SelfRepo => "self",
            Self::Qspec => "qspec",
            Self::Fcd => "fcd",
        })
    }
}

/// Stable failure category, independent of message prose.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Code {
    Usage,
    Io,
    Manifest,
    Git,
    MissingClone,
    Drift,
    CargoPin,
    SeamProbe,
    StringEdge,
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
            Self::CargoPin => "cargo-pin",
            Self::SeamProbe => "seam-probe",
            Self::StringEdge => "string-edge",
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
    #[error("no {flag} was given; it is required to revendor {owner}'s pinned commit {commit}")]
    MissingClone {
        owner: CommitOwner,
        commit: String,
        flag: &'static str,
    },
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
    #[error("{path}: does not pin {crate_name} to a git rev")]
    CargoPinMissing {
        path: PathBuf,
        crate_name: &'static str,
    },
    #[error(
        "{crate_name}'s pinned git rev lives in two places that disagree: Cargo.toml pins \
         {cargo_toml_rev}, Cargo.lock resolved {cargo_lock_rev}"
    )]
    CargoPinDisagreement {
        crate_name: &'static str,
        cargo_toml_rev: String,
        cargo_lock_rev: String,
    },
    #[error(
        "{tree_root}: VENDOR.json pins its fcd source to commit {manifest_commit}, which is \
         not Cargo.toml/Cargo.lock's own pinned {crate_name} rev {cargo_rev}; re-vendor at the \
         Cargo-pinned rev instead of editing VENDOR.json's commit by hand"
    )]
    FcdRevMismatch {
        tree_root: PathBuf,
        crate_name: &'static str,
        manifest_commit: String,
        cargo_rev: String,
    },
    #[error("cannot spawn cargo for the seam probe: {source}")]
    SeamProbeSpawn {
        #[source]
        source: io::Error,
    },
    #[error("seam-probe: the normal (non-probe) build failed to compile; nothing else about the probe is meaningful until it succeeds")]
    SeamProbeNormalBuildFailed,
    #[error("seam-probe: the normal (non-probe) build reported E0004 at: {locations}; the probe variant must be unreachable outside the probe build (FR-063-AC-3)")]
    SeamProbeNormalBuildHasE0004 { locations: String },
    #[error("seam-probe: the build under RUSTFLAGS=--cfg seam_probe succeeded; it must fail with E0004 at every checked-in seam location")]
    SeamProbeBuildUnexpectedlySucceeded,
    #[error(
        "seam-probe: checked-in list and the probe build's E0004 locations differ -- \
         unexpected-but-present: {unexpected_but_present}; expected-but-missing: {expected_but_missing}"
    )]
    SeamProbeMismatch {
        unexpected_but_present: String,
        expected_but_missing: String,
    },
    #[error("string-edge: cannot parse {path} as Rust source: {source}")]
    StringEdgeParse {
        path: PathBuf,
        #[source]
        source: syn::Error,
    },
    #[error(
        "string-edge: allow-list entry {file}:{line} gates a branch (feeds an if/while \
         condition or a match scrutinee/guard); FR-064-AC-5 refuses to admit it, remove it \
         from the allow-list and mark the call site #[string_edge] instead"
    )]
    StringEdgeAllowListGatesABranch { file: String, line: u32 },
    #[error("{summary}")]
    StringEdgeFound { summary: String },
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
            Self::ExternalDrift { .. } | Self::CheckFailed { .. } | Self::FcdRevMismatch { .. } => {
                Code::Drift
            }
            Self::CargoPinMissing { .. } | Self::CargoPinDisagreement { .. } => Code::CargoPin,
            Self::SeamProbeSpawn { .. }
            | Self::SeamProbeNormalBuildFailed
            | Self::SeamProbeNormalBuildHasE0004 { .. }
            | Self::SeamProbeBuildUnexpectedlySucceeded
            | Self::SeamProbeMismatch { .. } => Code::SeamProbe,
            Self::StringEdgeParse { .. }
            | Self::StringEdgeAllowListGatesABranch { .. }
            | Self::StringEdgeFound { .. } => Code::StringEdge,
        }
    }

    /// Distinguish usage/environment failure (2) from a genuine content drift (1).
    pub fn exit_code(&self) -> u8 {
        match self.code() {
            Code::Drift | Code::SeamProbe | Code::StringEdge => 1,
            Code::Usage
            | Code::Io
            | Code::Manifest
            | Code::Git
            | Code::MissingClone
            | Code::CargoPin => 2,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
