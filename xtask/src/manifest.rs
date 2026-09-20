// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL #138: the checked-in `VENDOR.json` pin recording, per vendored resource
//! tree, exactly which paths are vendored, from which explicit commit (or
//! external URL), and the SHA-256 each one is expected to carry.
//!
//! The manifest is the only source that may add a vendored path. `revendor`
//! never globs or scans a source tree for files to add.
use crate::error::{Error, Result};
use qsl_attrs::string_edge;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The `VENDOR.json` schema version this crate reads and writes.
pub const SCHEMA_VERSION: u32 = 1;

/// The parsed contents of a `VENDOR.json` pin file for one vendored resource tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// The `VENDOR.json` schema version this manifest was written against.
    pub schema_version: u32,
    /// The vendored resource tree this manifest pins, e.g. `native-v1`.
    pub tree: String,
    /// The pinned sources making up this tree.
    pub sources: Vec<Source>,
}

/// One pin: either a commit to read paths from with `git show`, or a closed
/// set of externally hosted files pinned by digest alone.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum Source {
    /// Bytes read from `agent-ix/quire-specification` at an explicit commit.
    Qspec {
        /// The `agent-ix/quire-specification` repository identity, recorded for provenance.
        repo: String,
        /// The pinned commit bytes are read from.
        commit: String,
        /// Prefix prepended to each file's `dest` when writing into the vendored tree.
        #[serde(default)]
        dest_prefix: String,
        /// The pinned files read from this source.
        files: Vec<PinnedFile>,
    },
    /// Bytes read from this repository's own history at an explicit commit
    /// (the `native-v1/external/quire-spec-language` historical selection).
    #[serde(rename = "self")]
    SelfRepo {
        /// The pinned commit, within this repository's own history, bytes are read from.
        commit: String,
        /// Prefix prepended to each file's `dest` when writing into the vendored tree.
        #[serde(default)]
        dest_prefix: String,
        /// The pinned files read from this source.
        files: Vec<PinnedFile>,
    },
    /// Bytes read from `agent-ix/filament-core-data` at an explicit commit.
    /// Unlike [`Self::Qspec`]/[`Self::SelfRepo`], the source tree's own shape
    /// (`crates/extraction-frontend/fixtures/...`) does not match this
    /// repository's vendored destination (`tests/fixtures/...`), so `path` is
    /// a shared relative tail joined onto `source_prefix` to read and onto
    /// `dest_prefix` to write.
    Fcd {
        /// The `agent-ix/filament-core-data` repository identity, recorded for provenance.
        repo: String,
        /// The pinned commit bytes are read from.
        commit: String,
        /// Prefix joined onto each file's shared relative tail when reading from the source repo.
        source_prefix: String,
        /// Prefix joined onto each file's shared relative tail when writing into the vendored tree.
        #[serde(default)]
        dest_prefix: String,
        /// The pinned files read from this source.
        files: Vec<PinnedFile>,
    },
    /// Bytes downloaded once from an external host and pinned by digest;
    /// `revendor`/`revendor_check` verify the digest and never fetch it.
    ExternalUrl {
        /// The pinned externally sourced files.
        files: Vec<ExternalFile>,
    },
}

/// A single vendored file read with `git show <commit>:<path>`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinnedFile {
    /// Path relative to the source repository root.
    pub path: String,
    /// `sha256:<64 lowercase hex>`, `quire_spec_language::ByteDigest`'s display form.
    pub sha256: String,
}

/// A single vendored file pinned by digest with no fetchable source of truth.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalFile {
    /// Path relative to the resource tree root.
    pub dest: String,
    /// Authoritative origin URL, recorded for provenance only; never fetched.
    pub url: String,
    /// `sha256:<64 lowercase hex>`, `quire_spec_language::ByteDigest`'s display form.
    pub sha256: String,
}

fn is_full_commit_sha(commit: &str) -> bool {
    commit.len() == 40 && commit.bytes().all(|b| b.is_ascii_hexdigit())
}

/// A non-empty, tree-relative path: no leading `/`, no `\`, and no `.` or
/// `..` component. Every `PinnedFile::path`, `ExternalFile::dest` and
/// non-empty `dest_prefix` must satisfy this before it is ever joined onto a
/// resource tree root, so a manifest can only ever name a destination inside
/// that tree.
/// A typed-wire-reader edge (FR-064's own list): converts a `VENDOR.json`
/// path string into a safe/unsafe judgment.
#[string_edge]
fn is_safe_relative_path(text: &str) -> bool {
    if text.is_empty() || text.starts_with('/') || text.contains('\\') {
        return false;
    }
    text.split('/')
        .all(|component| !component.is_empty() && component != "." && component != "..")
}

pub(crate) fn join_dest(dest_prefix: &str, path: &str) -> String {
    if dest_prefix.is_empty() {
        path.to_owned()
    } else {
        format!("{dest_prefix}/{path}")
    }
}

/// A pinned file's repo-relative read path: `source_prefix` joined onto its
/// own `path`, mirroring [`join_dest`] for the write side. Kept as its own
/// named function (rather than reusing `join_dest` for both directions) so
/// a read site and a write site never silently trade places.
pub(crate) fn join_source(source_prefix: &str, path: &str) -> String {
    if source_prefix.is_empty() {
        path.to_owned()
    } else {
        format!("{source_prefix}/{path}")
    }
}

impl Source {
    /// Every tree-relative destination path this source vendors.
    pub fn dest_paths(&self) -> Vec<String> {
        self.dest_digests()
            .into_iter()
            .map(|(dest, _digest)| dest)
            .collect()
    }

    /// Every (tree-relative destination path, recorded `sha256:...` digest)
    /// this source vendors.
    pub fn dest_digests(&self) -> Vec<(String, &str)> {
        match self {
            Self::Qspec {
                dest_prefix, files, ..
            }
            | Self::SelfRepo {
                dest_prefix, files, ..
            }
            | Self::Fcd {
                dest_prefix, files, ..
            } => files
                .iter()
                .map(|file| (join_dest(dest_prefix, &file.path), file.sha256.as_str()))
                .collect(),
            Self::ExternalUrl { files } => files
                .iter()
                .map(|file| (file.dest.clone(), file.sha256.as_str()))
                .collect(),
        }
    }
}

impl Manifest {
    /// Read and parse `VENDOR.json` from `path`, validating its contents.
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path).map_err(|source| Error::io(path, source))?;
        let manifest: Self = serde_json::from_slice(&bytes).map_err(|source| Error::Manifest {
            path: path.to_owned(),
            source,
        })?;
        manifest.validate(path)?;
        Ok(manifest)
    }

    /// Write this manifest back to `path` as pretty-printed, newline-terminated JSON.
    pub fn save(&self, path: &Path) -> Result<()> {
        let mut bytes =
            serde_json::to_vec_pretty(self).expect("Manifest serialization cannot fail");
        bytes.push(b'\n');
        std::fs::write(path, bytes).map_err(|source| Error::io(path, source))
    }

    fn validate(&self, path: &Path) -> Result<()> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(Error::InvalidManifest {
                path: path.to_owned(),
                message: format!(
                    "unsupported schema_version {} (expected {SCHEMA_VERSION})",
                    self.schema_version
                ),
            });
        }
        for source in &self.sources {
            if let Source::Qspec { commit, .. }
            | Source::SelfRepo { commit, .. }
            | Source::Fcd { commit, .. } = source
            {
                if !is_full_commit_sha(commit) {
                    return Err(Error::InvalidManifest {
                        path: path.to_owned(),
                        message: format!(
                            "{commit} is not a full 40-character commit sha; \
                             this command never resolves a short sha or a ref"
                        ),
                    });
                }
            }
            if let Source::Fcd { source_prefix, .. } = source {
                if !is_safe_relative_path(source_prefix) {
                    return Err(Error::InvalidManifest {
                        path: path.to_owned(),
                        message: format!(
                            "{source_prefix}: source_prefix is not a safe tree-relative path; \
                             an absolute path, a backslash and a \".\" or \"..\" component are \
                             refused"
                        ),
                    });
                }
            }
            match source {
                Source::Qspec {
                    dest_prefix, files, ..
                }
                | Source::SelfRepo {
                    dest_prefix, files, ..
                }
                | Source::Fcd {
                    dest_prefix, files, ..
                } => {
                    if !dest_prefix.is_empty() && !is_safe_relative_path(dest_prefix) {
                        return Err(Error::InvalidManifest {
                            path: path.to_owned(),
                            message: format!(
                                "{dest_prefix}: dest_prefix is not a safe tree-relative path; \
                                 an absolute path, a backslash and a \".\" or \"..\" component \
                                 are refused"
                            ),
                        });
                    }
                    for file in files {
                        if !is_safe_relative_path(&file.path) {
                            return Err(Error::InvalidManifest {
                                path: path.to_owned(),
                                message: format!(
                                    "{}: path is not a safe tree-relative path; an absolute \
                                     path, a backslash and a \".\" or \"..\" component are \
                                     refused",
                                    file.path
                                ),
                            });
                        }
                    }
                }
                Source::ExternalUrl { files } => {
                    for file in files {
                        if !is_safe_relative_path(&file.dest) {
                            return Err(Error::InvalidManifest {
                                path: path.to_owned(),
                                message: format!(
                                    "{}: dest is not a safe tree-relative path; an absolute \
                                     path, a backslash and a \".\" or \"..\" component are \
                                     refused",
                                    file.dest
                                ),
                            });
                        }
                    }
                }
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        for dest in self.dest_paths() {
            if !seen.insert(dest.clone()) {
                return Err(Error::InvalidManifest {
                    path: path.to_owned(),
                    message: format!("{dest} is vendored by more than one source entry"),
                });
            }
        }
        Ok(())
    }

    /// Every tree-relative destination path recorded across all sources.
    pub fn dest_paths(&self) -> Vec<String> {
        self.sources.iter().flat_map(Source::dest_paths).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use std::io::Write;
    use std::path::PathBuf;

    fn write_manifest(text: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("VENDOR.json");
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(text.as_bytes()).unwrap();
        (dir, path)
    }

    /// Tracing: TC-150.
    #[trace("TC-150", "NFR-011-AC-2")]
    #[test]
    fn dest_paths_apply_dest_prefix_per_source_kind() {
        let manifest = Manifest {
            schema_version: SCHEMA_VERSION,
            tree: "example".into(),
            sources: vec![
                Source::Qspec {
                    repo: "https://example.invalid/qspec".into(),
                    commit: "a".repeat(40),
                    dest_prefix: "quire-specification".into(),
                    files: vec![PinnedFile {
                        path: "spec/a.md".into(),
                        sha256: "sha256:".to_owned() + &"0".repeat(64),
                    }],
                },
                Source::SelfRepo {
                    commit: "b".repeat(40),
                    dest_prefix: "external/quire-spec-language".into(),
                    files: vec![PinnedFile {
                        path: "src/a.rs".into(),
                        sha256: "sha256:".to_owned() + &"1".repeat(64),
                    }],
                },
                Source::ExternalUrl {
                    files: vec![ExternalFile {
                        dest: "unicode-17.0.0/a.txt".into(),
                        url: "https://example.invalid/a.txt".into(),
                        sha256: "sha256:".to_owned() + &"2".repeat(64),
                    }],
                },
            ],
        };
        assert_eq!(
            manifest.dest_paths(),
            vec![
                "quire-specification/spec/a.md",
                "external/quire-spec-language/src/a.rs",
                "unicode-17.0.0/a.txt",
            ]
        );
    }

    /// Tracing: TC-149.
    #[trace("TC-149", "NFR-011-AC-1")]
    #[test]
    fn load_refuses_a_short_or_non_hex_commit() {
        let text = format!(
            r#"{{"schema_version":1,"tree":"t","sources":[
                {{"kind":"qspec","repo":"r","commit":"abc123","dest_prefix":"","files":[
                    {{"path":"p","sha256":"sha256:{}"}}
                ]}}
            ]}}"#,
            "0".repeat(64)
        );
        let (_dir, path) = write_manifest(&text);
        let error = Manifest::load(&path).unwrap_err();
        assert!(matches!(error, Error::InvalidManifest { .. }));
        assert!(error.to_string().contains("40-character commit sha"));
    }

    /// Tracing: TC-149.
    #[trace("TC-149", "NFR-011-AC-1")]
    #[test]
    fn load_refuses_a_pinned_file_path_that_is_absolute_or_escapes_the_tree() {
        for bad_path in ["/etc/passwd", "../../etc/passwd", "a/../b", "a/./b"] {
            let manifest = Manifest {
                schema_version: SCHEMA_VERSION,
                tree: "t".into(),
                sources: vec![Source::Qspec {
                    repo: "r".into(),
                    commit: "a".repeat(40),
                    dest_prefix: String::new(),
                    files: vec![PinnedFile {
                        path: bad_path.into(),
                        sha256: "sha256:".to_owned() + &"0".repeat(64),
                    }],
                }],
            };
            let error = manifest.validate(Path::new("VENDOR.json")).unwrap_err();
            assert!(
                matches!(error, Error::InvalidManifest { .. }),
                "{bad_path:?}: {error}"
            );
            assert!(
                error.to_string().contains("safe tree-relative path"),
                "{bad_path:?}: {error}"
            );
        }
    }

    /// Tracing: TC-149.
    #[trace("TC-149", "NFR-011-AC-1")]
    #[test]
    fn load_refuses_a_dest_prefix_that_is_absolute_or_escapes_the_tree() {
        let manifest = Manifest {
            schema_version: SCHEMA_VERSION,
            tree: "t".into(),
            sources: vec![Source::SelfRepo {
                commit: "a".repeat(40),
                dest_prefix: "../outside".into(),
                files: vec![PinnedFile {
                    path: "p".into(),
                    sha256: "sha256:".to_owned() + &"0".repeat(64),
                }],
            }],
        };
        let error = manifest.validate(Path::new("VENDOR.json")).unwrap_err();
        assert!(matches!(error, Error::InvalidManifest { .. }));
        assert!(error.to_string().contains("dest_prefix is not a safe"));
    }

    /// Tracing: TC-149.
    #[trace("TC-149", "NFR-011-AC-1")]
    #[test]
    fn load_refuses_an_external_file_dest_that_is_absolute_or_escapes_the_tree() {
        let manifest = Manifest {
            schema_version: SCHEMA_VERSION,
            tree: "t".into(),
            sources: vec![Source::ExternalUrl {
                files: vec![ExternalFile {
                    dest: "/etc/passwd".into(),
                    url: "https://example.invalid/a.txt".into(),
                    sha256: "sha256:".to_owned() + &"0".repeat(64),
                }],
            }],
        };
        let error = manifest.validate(Path::new("VENDOR.json")).unwrap_err();
        assert!(matches!(error, Error::InvalidManifest { .. }));
        assert!(error.to_string().contains("dest is not a safe"));
    }

    /// Tracing: TC-149.
    #[trace("TC-149", "NFR-011-AC-1")]
    #[test]
    fn load_refuses_the_same_destination_from_two_sources() {
        let sha = "0".repeat(64);
        let text = format!(
            r#"{{"schema_version":1,"tree":"t","sources":[
                {{"kind":"qspec","repo":"r","commit":"{a}","dest_prefix":"","files":[
                    {{"path":"p","sha256":"sha256:{sha}"}}
                ]}},
                {{"kind":"self","commit":"{b}","dest_prefix":"","files":[
                    {{"path":"p","sha256":"sha256:{sha}"}}
                ]}}
            ]}}"#,
            a = "a".repeat(40),
            b = "b".repeat(40),
        );
        let (_dir, path) = write_manifest(&text);
        let error = Manifest::load(&path).unwrap_err();
        assert!(matches!(error, Error::InvalidManifest { .. }));
        assert!(error.to_string().contains("more than one source"));
    }

    /// Tracing: TC-149.
    #[trace("TC-149", "NFR-011-AC-1")]
    #[test]
    fn load_refuses_an_unsupported_schema_version() {
        let text = r#"{"schema_version":2,"tree":"t","sources":[]}"#;
        let (_dir, path) = write_manifest(text);
        let error = Manifest::load(&path).unwrap_err();
        assert!(matches!(error, Error::InvalidManifest { .. }));
    }

    /// Tracing: TC-149.
    #[trace("TC-149", "NFR-011-AC-1")]
    #[test]
    fn load_refuses_an_unknown_field() {
        let text = r#"{"schema_version":1,"tree":"t","sources":[],"extra":true}"#;
        let (_dir, path) = write_manifest(text);
        assert!(matches!(Manifest::load(&path), Err(Error::Manifest { .. })));
    }

    /// Tracing: TC-150.
    #[trace("TC-150", "NFR-011-AC-2")]
    #[test]
    fn save_then_load_round_trips() {
        let manifest = Manifest {
            schema_version: SCHEMA_VERSION,
            tree: "example".into(),
            sources: vec![Source::ExternalUrl {
                files: vec![ExternalFile {
                    dest: "a.txt".into(),
                    url: "https://example.invalid/a.txt".into(),
                    sha256: "sha256:".to_owned() + &"3".repeat(64),
                }],
            }],
        };
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("VENDOR.json");
        manifest.save(&path).unwrap();
        let loaded = Manifest::load(&path).unwrap();
        assert_eq!(loaded.dest_paths(), manifest.dest_paths());
    }
}
