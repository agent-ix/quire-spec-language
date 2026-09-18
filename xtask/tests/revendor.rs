// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL #138: behavioral tests for `revendor` / `revendor_check` against
//! synthetic manifests (fast, offline, no git dependency) plus the two
//! acceptance points that need a real repository: idempotency, and that the
//! two checked-in manifests actually describe the live vendored trees.
use std::path::{Path, PathBuf};

use xtask::manifest::{ExternalFile, Manifest, PinnedFile, Source};
use xtask::{revendor, revendor_check, Sources, Tree};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn sha256_of(bytes: &[u8]) -> String {
    quire_spec_language::ByteDigest::of(bytes).to_string()
}

/// The two checked-in manifests describe the vendored trees exactly as they
/// stand: no drift, no stray file. This is the offline drift-detection gate
/// (`revendor-check`'s job) and needs no local `quire-specification` clone.
#[test]
fn checked_in_manifests_match_the_vendored_trees_with_no_drift_or_stray_files() {
    for tree in Tree::ALL {
        let root = workspace_root();
        let manifest = Manifest::load(&tree.manifest_path(&root)).unwrap();
        let report = revendor_check(&manifest, &tree.root(&root)).unwrap();
        assert!(
            report.is_clean(),
            "{}: drifted={:?} stray={:?}",
            tree.dir_name(),
            report.drifted,
            report.stray
        );
    }
}

/// A file changed after vendoring is reported as drift, not silently accepted.
#[test]
fn revendor_check_reports_a_byte_changed_after_vendoring() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), b"original").unwrap();
    let manifest = Manifest {
        schema_version: 1,
        tree: "synthetic".into(),
        sources: vec![Source::ExternalUrl {
            files: vec![ExternalFile {
                dest: "a.txt".into(),
                url: "https://example.invalid/a.txt".into(),
                sha256: sha256_of(b"original"),
            }],
        }],
    };
    let report = revendor_check(&manifest, dir.path()).unwrap();
    assert!(report.is_clean());

    std::fs::write(dir.path().join("a.txt"), b"tampered").unwrap();
    let report = revendor_check(&manifest, dir.path()).unwrap();
    assert!(!report.is_clean());
    assert_eq!(report.drifted.len(), 1);
    assert_eq!(report.drifted[0].dest, "a.txt");
    assert_eq!(report.drifted[0].expected_sha256, sha256_of(b"original"));
    assert_eq!(report.drifted[0].actual_sha256, sha256_of(b"tampered"));
}

/// A file present in the tree but absent from the manifest is stray, not silently
/// adopted; the manifest's own path list is the only thing that can add a path.
#[test]
fn revendor_check_reports_a_file_the_manifest_does_not_mention() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), b"kept").unwrap();
    let manifest = Manifest {
        schema_version: 1,
        tree: "synthetic".into(),
        sources: vec![Source::ExternalUrl {
            files: vec![ExternalFile {
                dest: "a.txt".into(),
                url: "https://example.invalid/a.txt".into(),
                sha256: sha256_of(b"kept"),
            }],
        }],
    };
    assert!(revendor_check(&manifest, dir.path()).unwrap().is_clean());

    std::fs::write(dir.path().join("b.txt"), b"uninvited").unwrap();
    let report = revendor_check(&manifest, dir.path()).unwrap();
    assert!(!report.is_clean());
    assert_eq!(report.stray, vec!["b.txt".to_owned()]);

    // README.md and VENDOR.json are always excused from the stray check.
    std::fs::remove_file(dir.path().join("b.txt")).unwrap();
    std::fs::write(dir.path().join("README.md"), b"docs").unwrap();
    std::fs::write(dir.path().join("VENDOR.json"), b"{}").unwrap();
    assert!(revendor_check(&manifest, dir.path()).unwrap().is_clean());
}

/// `revendor` for an `external_url` source only ever verifies the recorded
/// digest; on mismatch it refuses rather than overwriting the file (it never
/// fetches network bytes).
#[test]
fn revendor_refuses_to_silently_accept_drifted_external_bytes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), b"tampered").unwrap();
    let mut manifest = Manifest {
        schema_version: 1,
        tree: "synthetic".into(),
        sources: vec![Source::ExternalUrl {
            files: vec![ExternalFile {
                dest: "a.txt".into(),
                url: "https://example.invalid/a.txt".into(),
                sha256: sha256_of(b"original"),
            }],
        }],
    };
    let sources = Sources {
        workspace_root: dir.path(),
        qspec_clone: None,
    };
    let error = revendor(&mut manifest, dir.path(), &sources).unwrap_err();
    assert!(matches!(error, xtask::Error::ExternalDrift { .. }));
    // The file on disk is untouched -- no silent "fix" was applied.
    assert_eq!(
        std::fs::read(dir.path().join("a.txt")).unwrap(),
        b"tampered"
    );
}

/// Idempotency (QSL #138 acceptance point): running `revendor` twice at the
/// same pin is a no-op the second time. This uses the `self` source kind
/// against this repository's own git history, which is always available
/// (no external clone, no network) -- the same mechanism
/// `resources/native-v1/VENDOR.json` uses for its
/// `external/quire-spec-language` historical selection.
#[test]
fn revendor_is_idempotent_against_this_repositorys_own_history() {
    let root = workspace_root();
    // A real, small, long-settled file at a real commit in this repo's history.
    let commit = "bf9960e8d818e4d72182c4691c8aaf4e2cd7dffd";
    let dir = tempfile::tempdir().unwrap();
    let mut manifest = Manifest {
        schema_version: 1,
        tree: "synthetic".into(),
        sources: vec![Source::SelfRepo {
            commit: commit.to_owned(),
            dest_prefix: String::new(),
            files: vec![PinnedFile {
                path: "docs/native-error-codes.md".into(),
                sha256: "sha256:".to_owned() + &"0".repeat(64), // placeholder, overwritten
            }],
        }],
    };
    let sources = Sources {
        workspace_root: &root,
        qspec_clone: None,
    };

    let first = revendor(&mut manifest, dir.path(), &sources).unwrap();
    assert_eq!(first.written, vec!["docs/native-error-codes.md".to_owned()]);
    let bytes_after_first = std::fs::read(dir.path().join("docs/native-error-codes.md")).unwrap();

    let second = revendor(&mut manifest, dir.path(), &sources).unwrap();
    assert!(second.is_noop(), "second run wrote: {:?}", second.written);
    assert_eq!(
        second.unchanged,
        vec!["docs/native-error-codes.md".to_owned()]
    );
    let bytes_after_second = std::fs::read(dir.path().join("docs/native-error-codes.md")).unwrap();
    assert_eq!(bytes_after_first, bytes_after_second);

    // Re-running also leaves the manifest's own recorded digest stable.
    let Source::SelfRepo { files, .. } = &manifest.sources[0] else {
        unreachable!()
    };
    assert_eq!(files[0].sha256, sha256_of(&bytes_after_second));
}

/// `revendor` refuses a qspec-kind source when no clone was given, rather
/// than silently skipping it or reaching for the network.
#[test]
fn revendor_refuses_a_qspec_source_with_no_clone_given() {
    let root = workspace_root();
    let dir = tempfile::tempdir().unwrap();
    let mut manifest = Manifest {
        schema_version: 1,
        tree: "synthetic".into(),
        sources: vec![Source::Qspec {
            repo: "https://github.com/agent-ix/quire-specification".into(),
            commit: "4d6230eb8aa9766ff3017360962f2d6368d74cb3".into(),
            dest_prefix: String::new(),
            files: vec![PinnedFile {
                path: "proposals/quire-v1/shared-grammar.md".into(),
                sha256: "sha256:".to_owned() + &"0".repeat(64),
            }],
        }],
    };
    let sources = Sources {
        workspace_root: &root,
        qspec_clone: None,
    };
    let error = revendor(&mut manifest, dir.path(), &sources).unwrap_err();
    assert!(matches!(error, xtask::Error::MissingClone { .. }));
}

/// Deep, real-clone-backed round trip: with `QSPEC_CLONE_PATH` pointing at a
/// local `agent-ix/quire-specification` checkout, re-running `revendor`
/// against the pinned commits in the checked-in manifests changes nothing.
/// Skips cleanly when the environment variable is absent (e.g. CI), matching
/// how this repo's other externally dependent tests skip.
#[test]
fn revendor_is_idempotent_against_a_real_qspec_clone() {
    let Ok(clone) = std::env::var("QSPEC_CLONE_PATH") else {
        eprintln!("QSPEC_CLONE_PATH not set; skipping the real-clone idempotency check");
        return;
    };
    let clone = PathBuf::from(clone);
    let root = workspace_root();
    for tree in Tree::ALL {
        let manifest_path = tree.manifest_path(&root);
        let mut manifest = Manifest::load(&manifest_path).unwrap();
        let before: Vec<u8> = std::fs::read(&manifest_path).unwrap();
        let sources = Sources {
            workspace_root: &root,
            qspec_clone: Some(&clone),
        };
        let report = revendor(&mut manifest, &tree.root(&root), &sources).unwrap();
        assert!(
            report.is_noop(),
            "{}: re-vendoring at the recorded pin wrote {:?}",
            tree.dir_name(),
            report.written
        );
        manifest.save(&manifest_path).unwrap();
        let after = std::fs::read(&manifest_path).unwrap();
        assert_eq!(
            before,
            after,
            "{}: manifest byte-changed on a no-op run",
            tree.dir_name()
        );
    }
}
