// SPDX-License-Identifier: AGPL-3.0-or-later
//! One error type for this crate's two gates: `cargo xtask seam-probe` and
//! `cargo xtask string-edge`.
use std::{fmt, io, path::PathBuf};

/// Stable failure category, independent of message prose.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Code {
    /// A command-line usage refusal.
    Usage,
    /// A filesystem read/write failure.
    Io,
    /// The `cargo xtask seam-probe` build/comparison failed.
    SeamProbe,
    /// The `cargo xtask string-edge` scan found an allow-list defect or an
    /// unmarked, un-allow-listed occurrence.
    StringEdge,
    /// QSL-139 (FR-068) TC-172/TC-176's resolved import-graph scan
    /// (`xtask::import_graph`) could not parse a source file.
    ImportGraph,
    /// QSL-46 (FR-080-AC-3) `cargo xtask route-lint` found a `static`,
    /// `OnceLock` or `thread_local!` item in the `#185` registry module.
    RouteLint,
}

impl Code {
    fn as_str(self) -> &'static str {
        match self {
            Self::Usage => "usage",
            Self::Io => "io",
            Self::SeamProbe => "seam-probe",
            Self::StringEdge => "string-edge",
            Self::ImportGraph => "import-graph",
            Self::RouteLint => "route-lint",
        }
    }
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One concrete error type for this crate's filesystem and subprocess boundaries.
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
    /// A probe build compiled cleanly. It must instead fail with E0004 at
    /// every checked-in seam location in that package, or the probe is not
    /// exhaustive. Names the package or packages and the checked-in
    /// locations no probe build reported (FR-063-AC-2, TC-161 step 4).
    #[error(
        "seam-probe: the probe build of {packages} succeeded; it must fail with E0004 at every \
         checked-in seam location -- expected-but-missing: {expected_but_missing}"
    )]
    SeamProbeBuildUnexpectedlySucceeded {
        /// The packages whose probe build compiled.
        packages: String,
        /// Checked-in locations no probe build reported.
        expected_but_missing: String,
    },
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
        "string-edge: allow-list entry {file}:{item} gates a branch (feeds an if/while \
         condition or a match scrutinee/guard); FR-064-AC-5 refuses to admit it, remove it \
         from the allow-list and mark the call site #[string_edge] instead"
    )]
    StringEdgeAllowListGatesABranch {
        /// The file containing the offending allow-list entry.
        file: String,
        /// The enclosing item the offending allow-list entry names
        /// (PR #262 review, finding F8: item-keyed, not line-keyed).
        item: String,
    },
    /// The `string-edge` scan found one or more un-marked, un-allow-listed
    /// string comparisons or matches; `summary` lists them.
    #[error("{summary}")]
    StringEdgeFound {
        /// The findings, formatted for display.
        summary: String,
    },
    /// A configured `string-edge` crate root does not exist. Every listed
    /// root is required (QSL-178 review F4): a crate rename or move this
    /// scan's own root list has not caught up with must fail loudly, not
    /// silently drop that tree's coverage.
    #[error("string-edge: configured crate root does not exist: {path}")]
    StringEdgeMissingRoot {
        /// The missing root.
        path: PathBuf,
    },
    /// A source file could not be parsed as Rust source by
    /// `xtask::import_graph`'s resolver (TC-172/TC-176).
    #[error("import-graph: cannot parse {path} as Rust source: {source}")]
    ImportGraphParse {
        /// The file that failed to parse.
        path: PathBuf,
        /// The underlying parse failure.
        #[source]
        source: syn::Error,
    },
    /// The registry module could not be parsed as Rust source by
    /// `xtask::route_lint`.
    #[error("route-lint: cannot parse {path} as Rust source: {source}")]
    RouteLintParse {
        /// The file that failed to parse.
        path: PathBuf,
        /// The underlying parse failure.
        #[source]
        source: syn::Error,
    },
    /// `xtask::route_lint` found one or more `static`, `OnceLock` or
    /// `thread_local!` items in the registry module; `summary` lists them.
    #[error("{summary}")]
    RouteLintFound {
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
            Self::Usage(_) => Code::Usage,
            Self::Io { .. } => Code::Io,
            Self::SeamProbeSpawn { .. }
            | Self::SeamProbeReadSource { .. }
            | Self::SeamProbeNormalBuildFailed { .. }
            | Self::SeamProbeOfflineRegistryUnavailable { .. }
            | Self::SeamProbeNormalBuildHasE0004 { .. }
            | Self::SeamProbeBuildUnexpectedlySucceeded { .. }
            | Self::SeamProbeMismatch { .. } => Code::SeamProbe,
            Self::StringEdgeParse { .. }
            | Self::StringEdgeAllowListGatesABranch { .. }
            | Self::StringEdgeFound { .. } => Code::StringEdge,
            Self::StringEdgeMissingRoot { .. } => Code::Usage,
            Self::ImportGraphParse { .. } => Code::ImportGraph,
            Self::RouteLintParse { .. } | Self::RouteLintFound { .. } => Code::RouteLint,
        }
    }

    /// Distinguish usage/environment failure (2) from a genuine content
    /// finding (1).
    pub fn exit_code(&self) -> u8 {
        match self.code() {
            Code::SeamProbe | Code::StringEdge | Code::ImportGraph | Code::RouteLint => 1,
            Code::Usage | Code::Io => 2,
        }
    }
}

/// This crate's own `Result` alias, fixed to [`Error`].
pub type Result<T> = std::result::Result<T, Error>;
