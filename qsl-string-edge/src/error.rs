// SPDX-License-Identifier: AGPL-3.0-or-later
//! The typed errors a scan returns (FR-064). Every variant names the input
//! it concerns; none is a clean result, and the command maps each to exit 2.
use std::{io, path::PathBuf};

/// A request, rule, source read or Rust parse failure.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The request named no crate source root.
    #[error("string-edge: the request names no crate source root")]
    NoCrateRoots,
    /// The base directory is missing or cannot be resolved.
    #[error("string-edge: cannot resolve base directory {path}: {source}")]
    BaseDirectory {
        /// The base directory as the caller named it.
        path: PathBuf,
        /// Why it cannot be resolved.
        #[source]
        source: io::Error,
    },
    /// The base directory exists but is not a directory.
    #[error("string-edge: base directory {path} is not a directory")]
    BaseNotDirectory {
        /// The base directory as the caller named it.
        path: PathBuf,
    },
    /// A crate source root is missing or cannot be resolved.
    #[error("string-edge: cannot resolve crate source root {path}: {source}")]
    RootUnresolved {
        /// The root as the caller named it.
        path: PathBuf,
        /// Why it cannot be resolved.
        #[source]
        source: io::Error,
    },
    /// A crate source root is not a directory (a `Cargo.toml` or a file).
    #[error("string-edge: crate source root {path} is not a directory")]
    RootNotDirectory {
        /// The root as the caller named it.
        path: PathBuf,
    },
    /// A crate source root resolves outside the base directory.
    #[error("string-edge: crate source root {path} is outside the base directory")]
    RootOutsideBase {
        /// The root as the caller named it.
        path: PathBuf,
    },
    /// Two roots resolve to the same directory.
    #[error("string-edge: crate source root {path} is named twice")]
    RootDuplicate {
        /// The second spelling of the root.
        path: PathBuf,
    },
    /// One root lies inside another, so their crate scopes would overlap.
    #[error("string-edge: crate source root {inner} lies inside crate source root {outer}")]
    RootOverlap {
        /// The enclosing root.
        outer: PathBuf,
        /// The root inside it.
        inner: PathBuf,
    },
    /// A root holds no production Rust file, so a clean result would say
    /// nothing about it.
    #[error("string-edge: crate source root {path} holds no production Rust file")]
    NoProductionSources {
        /// The root, resolved against the base directory.
        path: PathBuf,
    },
    /// A directory or file could not be read.
    #[error("string-edge: cannot read {path}: {source}")]
    Read {
        /// The directory or file.
        path: PathBuf,
        /// Why it cannot be read.
        #[source]
        source: io::Error,
    },
    /// A source file could not be parsed as Rust source by `syn`.
    #[error("string-edge: cannot parse {path} as Rust source: {source}")]
    Parse {
        /// The file that failed to parse.
        path: PathBuf,
        /// The underlying parse failure.
        #[source]
        source: syn::Error,
    },
    /// A link under a root points outside the base directory.
    #[error("string-edge: link {path} escapes the base directory")]
    LinkEscapesBase {
        /// The link.
        path: PathBuf,
    },
    /// A directory link leads back into a directory the walk is inside.
    #[error("string-edge: directory link {path} forms a cycle")]
    LinkCycle {
        /// The link.
        path: PathBuf,
    },
    /// A link under a root leads into a selected root's tree (its own or
    /// another's), so the same source would be scanned, and reported, twice.
    #[error("string-edge: link {path} leads into a selected crate source root")]
    LinkIntoRoot {
        /// The link.
        path: PathBuf,
    },
    /// The rules file could not be read.
    #[error("string-edge: cannot read rules file {path}: {source}")]
    RulesRead {
        /// The rules file.
        path: PathBuf,
        /// Why it cannot be read.
        #[source]
        source: io::Error,
    },
    /// The rules are not the strict JSON object FR-064 defines: malformed
    /// JSON, an unknown or missing field, or a field of the wrong type.
    #[error("string-edge: malformed rules: {source}")]
    RulesMalformed {
        /// The underlying JSON failure.
        #[source]
        source: serde_json::Error,
    },
    /// An allow-list entry carries an empty reason.
    #[error("string-edge: allow-list entry {file}:{item} has an empty reason")]
    RuleReasonEmpty {
        /// The entry's file.
        file: String,
        /// The entry's item anchor.
        item: String,
    },
    /// An allow-list entry names a path that is not a file inside the base
    /// directory.
    #[error(
        "string-edge: allow-list entry path {file} is not a file path inside the base directory"
    )]
    RulePathOutsideBase {
        /// The entry's file as written.
        file: String,
    },
    /// An allow-list entry names an item with a branch-gating occurrence.
    /// FR-064-AC-5 refuses to admit such an entry, marked or not; the item
    /// must be an edge that converts the string once.
    #[error(
        "string-edge: allow-list entry {file}:{line} ({item}) gates a branch (feeds an if/while \
         condition or a match scrutinee/guard); FR-064-AC-5 refuses to admit it, remove it \
         from the allow-list and mark the call site #[string_edge] instead"
    )]
    AllowListGatesABranch {
        /// The entry's file.
        file: String,
        /// The line of the item's first branch-gating occurrence.
        line: u32,
        /// The entry's item anchor.
        item: String,
    },
}

/// This crate's own `Result` alias, fixed to [`Error`].
pub type Result<T> = std::result::Result<T, Error>;
