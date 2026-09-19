// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL #138: re-vendor `resources/native-v1` and `resources/complete-value`
//! from an explicit pinned commit, and check the vendored tree for drift.
//! QSL #131 PR 3 extends the same mechanism to `tests/fixtures/architecture`
//! and `tests/fixtures/modules`, vendored from `agent-ix/filament-core-data`
//! (`Source::Fcd`) at the single commit `Cargo.toml` pins its
//! `agent-ix-extraction-frontend`/`agent-ix-semantic-ir` git deps to.
//!
//! `resources/native-v1` is a **historical selection**, not a mirror of any
//! current source tree (see its README and
//! `src/linking/composed/definition_source.rs`). This command vendors
//! exactly the path list recorded in `VENDOR.json` at exactly the recorded
//! commit; it never adds a file by globbing a source tree, and it never
//! resolves "latest" or reads a working tree. A new pin replaces vendored
//! bytes wholesale — there is no migration or compatibility path between
//! pins.
#![forbid(unsafe_code)]

pub mod error;
mod fsutil;
pub mod git;
pub mod manifest;
pub mod tree;

pub use error::{Error, Result};
pub use manifest::{ExternalFile, Manifest, PinnedFile, Source};
pub use tree::Tree;

use quire_spec_language::ByteDigest;
use std::collections::BTreeSet;
use std::path::Path;

/// Where `revendor`/`revendor_check` read pinned bytes from.
pub struct Sources<'a> {
    /// This repository's own working copy, used for `Source::SelfRepo`.
    pub workspace_root: &'a Path,
    /// A local `agent-ix/quire-specification` clone, used for `Source::Qspec`.
    /// Required only when the manifest actually contains a `Qspec` source.
    pub qspec_clone: Option<&'a Path>,
    /// A local `agent-ix/filament-core-data` clone, used for `Source::Fcd`.
    /// Required only when the manifest actually contains an `Fcd` source.
    pub fcd_clone: Option<&'a Path>,
}

impl Sources<'_> {
    fn repo_for<'a>(&'a self, commit_owner: CommitOwner, commit: &str) -> Result<&'a Path> {
        match commit_owner {
            CommitOwner::SelfRepo => Ok(self.workspace_root),
            CommitOwner::Qspec => self.qspec_clone.ok_or_else(|| Error::MissingClone {
                commit: commit.to_owned(),
                flag: "--qspec-clone",
            }),
            CommitOwner::Fcd => self.fcd_clone.ok_or_else(|| Error::MissingClone {
                commit: commit.to_owned(),
                flag: "--fcd-clone",
            }),
        }
    }
}

#[derive(Clone, Copy)]
enum CommitOwner {
    SelfRepo,
    Qspec,
    Fcd,
}

/// Outcome of one `revendor` run over a single manifest.
#[derive(Debug, Default)]
pub struct RevendorReport {
    /// Destination paths whose bytes changed (created or overwritten).
    pub written: Vec<String>,
    /// Destination paths already matching the freshly read pinned bytes.
    pub unchanged: Vec<String>,
    /// External-source destination paths whose digest was confirmed.
    pub verified_external: Vec<String>,
    /// Paths removed because the manifest no longer lists them: a new pin
    /// replaces the tree wholesale, so a path dropped from `VENDOR.json`
    /// does not linger on disk as an unexplained stray file.
    pub removed: Vec<String>,
}

impl RevendorReport {
    /// Idempotency: a second run at the same pin writes and removes nothing.
    pub fn is_noop(&self) -> bool {
        self.written.is_empty() && self.removed.is_empty()
    }
}

/// One vendored file whose on-disk bytes no longer match its recorded pin.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DriftedFile {
    pub dest: String,
    pub expected_sha256: String,
    pub actual_sha256: String,
}

/// Outcome of a `revendor_check` run over a single manifest.
#[derive(Debug, Default)]
pub struct CheckReport {
    pub matched: Vec<String>,
    pub drifted: Vec<DriftedFile>,
    /// Files present in the tree that the manifest does not mention.
    pub stray: Vec<String>,
}

impl CheckReport {
    pub fn is_clean(&self) -> bool {
        self.drifted.is_empty() && self.stray.is_empty()
    }
}

fn digest_of(bytes: &[u8]) -> String {
    ByteDigest::of(bytes).to_string()
}

/// Fetch (or, for external sources, verify) every path the manifest records,
/// write any changed bytes under `tree_root`, and update the manifest's
/// recorded digests to match what was just read. The caller persists the
/// mutated manifest with [`Manifest::save`].
///
/// Running this twice against the same pin is a no-op the second time:
/// `RevendorReport::is_noop` is `true` and no byte on disk changes.
pub fn revendor(
    manifest: &mut Manifest,
    tree_root: &Path,
    sources: &Sources<'_>,
) -> Result<RevendorReport> {
    let mut report = RevendorReport::default();
    for source in &mut manifest.sources {
        match source {
            Source::Qspec {
                commit,
                dest_prefix,
                files,
                ..
            } => {
                revendor_pinned(
                    sources,
                    PinnedSource {
                        owner: CommitOwner::Qspec,
                        commit,
                        source_prefix: "",
                        dest_prefix,
                    },
                    files,
                    tree_root,
                    &mut report,
                )?;
            }
            Source::SelfRepo {
                commit,
                dest_prefix,
                files,
            } => {
                revendor_pinned(
                    sources,
                    PinnedSource {
                        owner: CommitOwner::SelfRepo,
                        commit,
                        source_prefix: "",
                        dest_prefix,
                    },
                    files,
                    tree_root,
                    &mut report,
                )?;
            }
            Source::Fcd {
                commit,
                source_prefix,
                dest_prefix,
                files,
                ..
            } => {
                revendor_pinned(
                    sources,
                    PinnedSource {
                        owner: CommitOwner::Fcd,
                        commit,
                        source_prefix,
                        dest_prefix,
                    },
                    files,
                    tree_root,
                    &mut report,
                )?;
            }
            Source::ExternalUrl { files } => {
                for file in files {
                    let bytes = fsutil::read(tree_root, &file.dest)?;
                    let actual = digest_of(&bytes);
                    if actual != file.sha256 {
                        return Err(Error::ExternalDrift {
                            dest: file.dest.clone(),
                            expected: file.sha256.clone(),
                            actual,
                        });
                    }
                    report.verified_external.push(file.dest.clone());
                }
            }
        }
    }
    remove_files_the_manifest_no_longer_lists(manifest, tree_root, &mut report)?;
    Ok(report)
}

/// A new pin replaces the tree wholesale: any entry under `tree_root` that
/// `manifest` no longer lists (other than `VENDOR.json` and `README.md`) is
/// removed rather than left behind as a stray file.
fn remove_files_the_manifest_no_longer_lists(
    manifest: &Manifest,
    tree_root: &Path,
    report: &mut RevendorReport,
) -> Result<()> {
    let known: BTreeSet<String> = manifest.dest_paths().into_iter().collect();
    for rel in fsutil::walk_relative(tree_root)? {
        if rel == "VENDOR.json" || rel == "README.md" || known.contains(&rel) {
            continue;
        }
        fsutil::remove(tree_root, &rel)?;
        report.removed.push(rel);
    }
    Ok(())
}

/// One pinned source's own coordinates -- which repo owns `commit`, and the
/// (possibly differing) read/write path prefixes -- grouped so
/// [`revendor_pinned`] takes one struct rather than four flat parameters.
struct PinnedSource<'a> {
    owner: CommitOwner,
    commit: &'a str,
    /// Repo-relative read path (empty for `Qspec`/`SelfRepo`, whose own
    /// paths already mirror their destinations).
    source_prefix: &'a str,
    dest_prefix: &'a str,
}

fn revendor_pinned(
    sources: &Sources<'_>,
    source: PinnedSource<'_>,
    files: &mut [PinnedFile],
    tree_root: &Path,
    report: &mut RevendorReport,
) -> Result<()> {
    let repo = sources.repo_for(source.owner, source.commit)?;
    git::require_commit(repo, source.commit)?;
    for file in files.iter_mut() {
        let source_path = manifest::join_dest(source.source_prefix, &file.path);
        let bytes = git::show(repo, source.commit, &source_path)?;
        let digest = digest_of(&bytes);
        let dest = manifest::join_dest(source.dest_prefix, &file.path);
        let changed = fsutil::write_if_changed(tree_root, &dest, &bytes)?;
        file.sha256 = digest;
        if changed {
            report.written.push(dest);
        } else {
            report.unchanged.push(dest);
        }
    }
    Ok(())
}

/// Offline drift check: every manifest-recorded destination's on-disk bytes
/// must match its recorded digest, and every file actually present in
/// `tree_root` (other than `VENDOR.json` and `README.md`) must be recorded
/// in the manifest. Needs no git repository and no network access, so it is
/// safe to run from `cargo test`.
pub fn revendor_check(manifest: &Manifest, tree_root: &Path) -> Result<CheckReport> {
    let mut report = CheckReport::default();
    let mut known: BTreeSet<String> = BTreeSet::new();
    for source in &manifest.sources {
        for (dest, expected) in source.dest_digests() {
            known.insert(dest.clone());
            check_one(tree_root, &dest, expected, &mut report)?;
        }
    }
    known.insert("VENDOR.json".to_owned());
    known.insert("README.md".to_owned());
    for rel in fsutil::walk_relative(tree_root)? {
        if !known.contains(&rel) {
            report.stray.push(rel);
        }
    }
    Ok(report)
}

fn check_one(
    tree_root: &Path,
    dest: &str,
    expected_sha256: &str,
    report: &mut CheckReport,
) -> Result<()> {
    let path = tree_root.join(dest);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            report.drifted.push(DriftedFile {
                dest: dest.to_owned(),
                expected_sha256: expected_sha256.to_owned(),
                actual_sha256: "missing".to_owned(),
            });
            return Ok(());
        }
        Err(source) => return Err(Error::io(&path, source)),
    };
    let actual = digest_of(&bytes);
    if actual == expected_sha256 {
        report.matched.push(dest.to_owned());
    } else {
        report.drifted.push(DriftedFile {
            dest: dest.to_owned(),
            expected_sha256: expected_sha256.to_owned(),
            actual_sha256: actual,
        });
    }
    Ok(())
}
