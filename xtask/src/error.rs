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
    /// This repository's own working copy.
    SelfRepo,
    /// A local `agent-ix/quire-specification` clone.
    Qspec,
    /// A local `agent-ix/filament-core-data` clone.
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
    /// A command-line usage refusal.
    Usage,
    /// A filesystem read/write failure.
    Io,
    /// A malformed or invalid `VENDOR.json` manifest.
    Manifest,
    /// A git read (`show`/spawn/unknown-commit) failure.
    Git,
    /// A required `--*-clone` flag was not supplied.
    MissingClone,
    /// The vendored tree drifted from its manifest, or a required pin disagrees.
    Drift,
    /// A `Cargo.toml`/`Cargo.lock` pin for a tracked crate is missing or disagrees.
    CargoPin,
    /// The `cargo xtask seam-probe` build/comparison failed.
    SeamProbe,
    /// The `cargo xtask string-edge` scan found an allow-list defect or an
    /// unmarked, un-allow-listed occurrence.
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
    /// A command-line invocation was malformed.
    #[error("usage: {0}")]
    Usage(&'static str),
    /// A filesystem read or write failed.
    #[error("{path}: {source}")]
    Io {
        /// The path the operation was attempted against.
        path: PathBuf,
        /// The underlying I/O failure.
        #[source]
        source: io::Error,
    },
    /// `VENDOR.json` could not be parsed as JSON.
    #[error("{path}: malformed VENDOR.json: {source}")]
    Manifest {
        /// The manifest file that failed to parse.
        path: PathBuf,
        /// The underlying JSON parse failure.
        #[source]
        source: serde_json::Error,
    },
    /// `VENDOR.json` parsed as JSON but its contents violate a manifest invariant.
    #[error("{path}: {message}")]
    InvalidManifest {
        /// The manifest file that failed validation.
        path: PathBuf,
        /// Why the manifest's contents are invalid.
        message: String,
    },
    /// `git show` failed to read a pinned path at a pinned commit.
    #[error("git -C {repo} show {commit}:{path}: {message}")]
    Git {
        /// The repository `git` was run against.
        repo: PathBuf,
        /// The pinned commit the read was attempted at.
        commit: String,
        /// The repo-relative path that could not be read.
        path: String,
        /// `git`'s own failure message.
        message: String,
    },
    /// The `git` process itself could not be spawned.
    #[error("git -C {repo} cannot spawn: {source}")]
    GitSpawn {
        /// The repository `git` was to be run against.
        repo: PathBuf,
        /// The underlying spawn failure.
        #[source]
        source: io::Error,
    },
    /// A pinned commit is not present in the local clone.
    #[error(
        "commit {commit} is not present in {repo}; this command never fetches over the network"
    )]
    UnknownCommit {
        /// The repository the commit was expected to be present in.
        repo: PathBuf,
        /// The missing commit.
        commit: String,
    },
    /// A required `--qspec-clone`/`--fcd-clone` flag was not supplied.
    #[error("no {flag} was given; it is required to revendor {owner}'s pinned commit {commit}")]
    MissingClone {
        /// Which repository's clone is missing.
        owner: CommitOwner,
        /// The pinned commit that clone is needed to read.
        commit: String,
        /// The command-line flag that would have supplied the clone.
        flag: &'static str,
    },
    /// `revendor-check` was invoked with `--qspec-clone`, which it never needs.
    #[error("revendor-check does not take --qspec-clone; it checks recorded digests offline")]
    CheckRefusesQspecClone,
    /// `revendor-check` was invoked with `--fcd-clone`, which it never needs.
    #[error("revendor-check does not take --fcd-clone; it checks recorded digests offline")]
    CheckRefusesFcdClone,
    /// An externally sourced file's on-disk digest disagrees with the manifest's recorded one.
    #[error(
        "{dest}: on-disk sha256 {actual} does not match the manifest's recorded {expected}; \
         external sources are never fetched, so re-download and update the manifest by hand"
    )]
    ExternalDrift {
        /// The destination path whose digest disagreed.
        dest: String,
        /// The digest recorded in the manifest.
        expected: String,
        /// The digest actually found on disk.
        actual: String,
    },
    /// `revendor-check` found drift and is reporting it as a hard failure.
    #[error("revendor-check found drift:\n{summary}")]
    CheckFailed {
        /// A human-readable summary of every drifted/stray file found.
        summary: String,
    },
    /// `Cargo.toml` does not pin the named crate to a git revision.
    #[error("{path}: does not pin {crate_name} to a git rev")]
    CargoPinMissing {
        /// The `Cargo.toml` that was expected to pin the crate.
        path: PathBuf,
        /// The crate expected to be pinned.
        crate_name: &'static str,
    },
    /// `Cargo.toml`'s declared pin and `Cargo.lock`'s resolved revision disagree.
    #[error(
        "{crate_name}'s pinned git rev lives in two places that disagree: Cargo.toml pins \
         {cargo_toml_rev}, Cargo.lock resolved {cargo_lock_rev}"
    )]
    CargoPinDisagreement {
        /// The crate whose pin disagrees.
        crate_name: &'static str,
        /// The revision `Cargo.toml` declares.
        cargo_toml_rev: String,
        /// The revision `Cargo.lock` actually resolved.
        cargo_lock_rev: String,
    },
    /// `VENDOR.json`'s hand-edited `Fcd` commit disagrees with `Cargo.toml`/`Cargo.lock`'s pin.
    #[error(
        "{tree_root}: VENDOR.json pins its fcd source to commit {manifest_commit}, which is \
         not Cargo.toml/Cargo.lock's own pinned {crate_name} rev {cargo_rev}; re-vendor at the \
         Cargo-pinned rev instead of editing VENDOR.json's commit by hand"
    )]
    FcdRevMismatch {
        /// The vendored tree whose manifest disagrees with the Cargo pin.
        tree_root: PathBuf,
        /// The crate whose Cargo-pinned revision is authoritative.
        crate_name: &'static str,
        /// The commit `VENDOR.json` records for its `Fcd` source.
        manifest_commit: String,
        /// The commit `Cargo.toml`/`Cargo.lock` actually pin the crate to.
        cargo_rev: String,
    },
    /// `cargo` itself could not be spawned to run the seam-probe build.
    #[error("cannot spawn cargo for the seam probe: {source}")]
    SeamProbeSpawn {
        /// The underlying spawn failure.
        #[source]
        source: io::Error,
    },
    /// An E0004 diagnostic named a source line, but that file could not be
    /// read back to resolve the line to its enclosing item.
    #[error(
        "seam-probe: cannot read {path} to resolve an E0004 line to its enclosing item: {source}"
    )]
    SeamProbeReadSource {
        /// The source file an E0004 location pointed at.
        path: PathBuf,
        /// The underlying read failure.
        #[source]
        source: io::Error,
    },
    /// The normal (non-probe) build failed to compile, so nothing else the
    /// probe checks is meaningful until that build succeeds on its own.
    #[error("seam-probe: the normal (non-probe) build failed to compile; nothing else about the probe is meaningful until it succeeds. stderr:\n{stderr}")]
    SeamProbeNormalBuildFailed {
        /// The failed build's captured stderr.
        stderr: String,
    },
    /// The normal (non-probe) build failed under `--offline` because its
    /// registry cache was cold -- not a genuine compile error. A warm
    /// (non-`--offline`) build must run first.
    #[error("seam-probe: the normal (non-probe) build did not compile -- it failed dependency resolution under --offline (a cold registry cache, not a genuine compile error). Run a warm (non---offline) build first, then retry. stderr:\n{stderr}")]
    SeamProbeOfflineRegistryUnavailable {
        /// The failed build's captured stderr.
        stderr: String,
    },
    /// The normal (non-probe) build reported E0004 at one or more
    /// locations. FR-063-AC-3 requires every probe-marked seam to be
    /// unreachable outside the probe build, so any E0004 in the normal
    /// build is itself a defect.
    #[error("seam-probe: the normal (non-probe) build reported E0004 at: {locations}; the probe variant must be unreachable outside the probe build (FR-063-AC-3)")]
    SeamProbeNormalBuildHasE0004 {
        /// The E0004 locations the normal build reported.
        locations: String,
    },
    /// The build under `RUSTFLAGS=--cfg seam_probe` compiled cleanly. It
    /// must instead fail with E0004 at every checked-in seam location, or
    /// the probe is not exhaustive.
    #[error("seam-probe: the build under RUSTFLAGS=--cfg seam_probe succeeded; it must fail with E0004 at every checked-in seam location")]
    SeamProbeBuildUnexpectedlySucceeded,
    /// The checked-in seam list and the probe build's actual E0004
    /// locations disagree.
    #[error(
        "seam-probe: checked-in list and the probe build's E0004 locations differ -- \
         unexpected-but-present: {unexpected_but_present}; expected-but-missing: {expected_but_missing}"
    )]
    SeamProbeMismatch {
        /// Locations the checked-in list did not expect, but the probe
        /// build reported anyway.
        unexpected_but_present: String,
        /// Locations the checked-in list expected, but the probe build did
        /// not report.
        expected_but_missing: String,
    },
    /// A source file could not be parsed as Rust source by `syn`.
    #[error("string-edge: cannot parse {path} as Rust source: {source}")]
    StringEdgeParse {
        /// The file that failed to parse.
        path: PathBuf,
        /// The underlying parse failure.
        #[source]
        source: syn::Error,
    },
    /// An allow-list entry names a string comparison or match that gates a
    /// branch (feeds an `if`/`while` condition or a `match` scrutinee/
    /// guard). FR-064-AC-5 refuses to admit such an entry to the
    /// allow-list; the call site must be marked `#[string_edge]` instead.
    #[error(
        "string-edge: allow-list entry {file}:{line} gates a branch (feeds an if/while \
         condition or a match scrutinee/guard); FR-064-AC-5 refuses to admit it, remove it \
         from the allow-list and mark the call site #[string_edge] instead"
    )]
    StringEdgeAllowListGatesABranch {
        /// The file containing the offending allow-list entry.
        file: String,
        /// The line the offending allow-list entry names.
        line: u32,
    },
    /// The `string-edge` scan found one or more un-marked, un-allow-listed
    /// string comparisons or matches; `summary` lists them.
    #[error("{summary}")]
    StringEdgeFound {
        /// The findings, formatted for display.
        summary: String,
    },
}

impl Error {
    /// Build an [`Error::Io`] for a failed read/write at `path`.
    pub fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    /// This error's stable failure category, independent of its message text.
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
            | Self::SeamProbeReadSource { .. }
            | Self::SeamProbeNormalBuildFailed { .. }
            | Self::SeamProbeOfflineRegistryUnavailable { .. }
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

/// This crate's own `Result` alias, fixed to [`Error`].
pub type Result<T> = std::result::Result<T, Error>;
