// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL #138: behavioral tests for `revendor` / `revendor_check` against
//! synthetic manifests (fast, offline, no git dependency) plus the
//! acceptance points that need a real repository: idempotency (against a
//! disposable, hermetic git repository this process creates, so it needs no
//! history and passes on CI's shallow checkout), and that the two checked-in
//! manifests actually describe the live vendored trees.
use ix_trace_rs::trace;
use std::path::{Path, PathBuf};

use xtask::manifest::{ExternalFile, Manifest, PinnedFile, Source};
use xtask::{cargo_pin, revendor, revendor_check, Sources, Tree};

/// A placeholder FCD rev for tests whose synthetic manifest carries no
/// `Source::Fcd` entry at all -- `revendor_check` never reads this
/// parameter unless the manifest actually has one.
const NO_FCD_SOURCE: &str = "0000000000000000000000000000000000000000";

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn sha256_of(bytes: &[u8]) -> String {
    quire_spec_language::ByteDigest::of(bytes).to_string()
}

/// Run `git -C dir <args>`, failing the test with the exact command on error.
fn run_git(dir: &Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?} failed in {}", dir.display());
}

fn init_git_repo(dir: &Path) {
    run_git(dir, &["init", "--quiet"]);
}

/// Write `contents` to `dir/rel`, commit it with a fixed, local-only
/// identity (so this never depends on the host's global git config or
/// signing setup), and return the new commit's full 40-character sha.
fn git_commit_file(dir: &Path, rel: &str, contents: &[u8]) -> String {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, contents).unwrap();
    run_git(dir, &["add", rel]);
    run_git(
        dir,
        &[
            "-c",
            "user.name=xtask-test",
            "-c",
            "user.email=xtask-test@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "-m",
            "test commit",
        ],
    );
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

/// Copy every regular file and directory from `src` into `dst`, creating
/// `dst`. Used so a real-clone test never revendors the checked-in
/// `resources/` tree in place: it works on a disposable copy instead, so a
/// failing assertion never leaves that tree dirty.
fn copy_dir_recursive(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let file_type = entry.file_type().unwrap();
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &dst_path);
        } else if file_type.is_file() {
            std::fs::copy(entry.path(), &dst_path).unwrap();
        }
    }
}

/// The two checked-in manifests describe the vendored trees exactly as they
/// stand: no drift, no stray file. This is the offline drift-detection gate
/// (`revendor-check`'s job) and needs no local `quire-specification` clone.
/// Tracing: TC-150, TC-152.
#[trace("TC-150", "TC-152", "NFR-011-AC-2", "NFR-011-AC-4")]
#[test]
fn checked_in_manifests_match_the_vendored_trees_with_no_drift_or_stray_files() {
    let root = workspace_root();
    let expected_fcd_commit = cargo_pin::read_agent_ix_semantic_ir_rev(&root)
        .expect("this workspace's own Cargo.toml/Cargo.lock agree on one agent-ix-semantic-ir rev");
    for tree in Tree::ALL {
        let manifest = Manifest::load(&tree.manifest_path(&root)).unwrap();
        let report = revendor_check(&manifest, &tree.root(&root), &expected_fcd_commit).unwrap();
        assert!(
            report.is_clean(),
            "{}: drifted={:?} stray={:?}",
            tree.dir_name(),
            report.drifted,
            report.stray
        );
    }
}

/// QSL #131 PR 3, M1: the FCD rev a vendored fixture tree is read from lives
/// in exactly one place -- this workspace's own `Cargo.toml`/`Cargo.lock`
/// pin of `agent-ix-semantic-ir` -- never repeated by hand in `VENDOR.json`.
/// A `Source::Fcd` entry whose own `commit` has drifted from that one place
/// is refused, with a typed error naming both commits, before any per-file
/// digest check runs.
/// Tracing: TC-152.
#[trace("TC-152", "NFR-011-AC-4")]
#[test]
fn revendor_check_refuses_an_fcd_source_whose_commit_disagrees_with_the_cargo_pin() {
    let dir = tempfile::tempdir().unwrap();
    let manifest = Manifest {
        schema_version: 1,
        tree: "synthetic".into(),
        sources: vec![Source::Fcd {
            repo: "https://github.com/agent-ix/filament-core-data".into(),
            commit: "0000000000000000000000000000000000000000".into(),
            source_prefix: "crates/extraction-frontend/fixtures/architecture".into(),
            dest_prefix: String::new(),
            files: vec![PinnedFile {
                path: "PROVENANCE.json".into(),
                sha256: "sha256:".to_owned() + &"0".repeat(64),
            }],
        }],
    };
    let error = revendor_check(
        &manifest,
        dir.path(),
        "7dcb2f2c7466a770b2362561e70ed10a8f941c1f",
    )
    .unwrap_err();
    match error {
        xtask::Error::FcdRevMismatch {
            manifest_commit,
            cargo_rev,
            ..
        } => {
            assert_eq!(manifest_commit, "0000000000000000000000000000000000000000");
            assert_eq!(cargo_rev, "7dcb2f2c7466a770b2362561e70ed10a8f941c1f");
        }
        other => panic!("expected FcdRevMismatch, got {other:?}"),
    }
}

/// A file changed after vendoring is reported as drift, not silently accepted.
/// Tracing: TC-152.
#[trace("TC-152", "NFR-011-AC-4")]
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
    let report = revendor_check(&manifest, dir.path(), NO_FCD_SOURCE).unwrap();
    assert!(report.is_clean());

    std::fs::write(dir.path().join("a.txt"), b"tampered").unwrap();
    let report = revendor_check(&manifest, dir.path(), NO_FCD_SOURCE).unwrap();
    assert!(!report.is_clean());
    assert_eq!(report.drifted.len(), 1);
    assert_eq!(report.drifted[0].dest, "a.txt");
    assert_eq!(report.drifted[0].expected_sha256, sha256_of(b"original"));
    assert_eq!(report.drifted[0].actual_sha256, sha256_of(b"tampered"));
}

/// A vendored file that is missing entirely is reported as drift too, not
/// confused with a byte-level mismatch or silently ignored.
/// Tracing: TC-152.
#[trace("TC-152", "NFR-011-AC-4")]
#[test]
fn revendor_check_reports_a_missing_vendored_file() {
    let dir = tempfile::tempdir().unwrap();
    let manifest = Manifest {
        schema_version: 1,
        tree: "synthetic".into(),
        sources: vec![Source::ExternalUrl {
            files: vec![ExternalFile {
                dest: "a.txt".into(),
                url: "https://example.invalid/a.txt".into(),
                sha256: sha256_of(b"expected"),
            }],
        }],
    };
    // a.txt is never created on disk.
    let report = revendor_check(&manifest, dir.path(), NO_FCD_SOURCE).unwrap();
    assert!(!report.is_clean());
    assert_eq!(report.drifted.len(), 1);
    assert_eq!(report.drifted[0].dest, "a.txt");
    assert_eq!(report.drifted[0].expected_sha256, sha256_of(b"expected"));
    assert_eq!(report.drifted[0].actual_sha256, "missing");
}

/// A file present in the tree but absent from the manifest is stray, not silently
/// adopted; the manifest's own path list is the only thing that can add a path.
/// Tracing: TC-152.
#[trace("TC-152", "NFR-011-AC-4")]
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
    assert!(revendor_check(&manifest, dir.path(), NO_FCD_SOURCE)
        .unwrap()
        .is_clean());

    std::fs::write(dir.path().join("b.txt"), b"uninvited").unwrap();
    let report = revendor_check(&manifest, dir.path(), NO_FCD_SOURCE).unwrap();
    assert!(!report.is_clean());
    assert_eq!(report.stray, vec!["b.txt".to_owned()]);

    // README.md and VENDOR.json are always excused from the stray check.
    std::fs::remove_file(dir.path().join("b.txt")).unwrap();
    std::fs::write(dir.path().join("README.md"), b"docs").unwrap();
    std::fs::write(dir.path().join("VENDOR.json"), b"{}").unwrap();
    assert!(revendor_check(&manifest, dir.path(), NO_FCD_SOURCE)
        .unwrap()
        .is_clean());
}

/// A symlink (or any other non-regular entry) left in a vendored tree is
/// reported as stray, not silently skipped -- a legitimate vendored file is
/// always a plain file, never a link to one.
/// Tracing: TC-152.
#[trace("TC-152", "NFR-011-AC-4")]
#[test]
fn revendor_check_reports_a_symlink_as_stray() {
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
    assert!(revendor_check(&manifest, dir.path(), NO_FCD_SOURCE)
        .unwrap()
        .is_clean());

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(dir.path().join("a.txt"), dir.path().join("link")).unwrap();
        let report = revendor_check(&manifest, dir.path(), NO_FCD_SOURCE).unwrap();
        assert!(!report.is_clean(), "a symlink must not pass as clean");
        assert_eq!(report.stray, vec!["link".to_owned()]);
    }
}

/// `revendor` for an `external_url` source only ever verifies the recorded
/// digest; on mismatch it refuses rather than overwriting the file (it never
/// fetches network bytes).
/// Tracing: TC-150.
#[trace("TC-150", "NFR-011-AC-2")]
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
        fcd_clone: None,
    };
    let error = revendor(&mut manifest, dir.path(), &sources).unwrap_err();
    assert!(matches!(error, xtask::Error::ExternalDrift { .. }));
    // The file on disk is untouched -- no silent "fix" was applied.
    assert_eq!(
        std::fs::read(dir.path().join("a.txt")).unwrap(),
        b"tampered"
    );
}

/// A new pin replaces the tree wholesale: a file the manifest no longer
/// lists is removed by `revendor`, not left behind as a stray file that only
/// a hand deletion could clear.
/// Tracing: TC-152.
#[trace("TC-152", "NFR-011-AC-5")]
#[test]
fn revendor_removes_a_file_dropped_from_the_manifest() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("kept.txt"), b"kept").unwrap();
    std::fs::write(dir.path().join("dropped.txt"), b"stale").unwrap();
    std::fs::write(dir.path().join("README.md"), b"docs").unwrap();
    let mut manifest = Manifest {
        schema_version: 1,
        tree: "synthetic".into(),
        sources: vec![Source::ExternalUrl {
            files: vec![ExternalFile {
                dest: "kept.txt".into(),
                url: "https://example.invalid/kept.txt".into(),
                sha256: sha256_of(b"kept"),
            }],
        }],
    };
    let sources = Sources {
        workspace_root: dir.path(),
        qspec_clone: None,
        fcd_clone: None,
    };
    let report = revendor(&mut manifest, dir.path(), &sources).unwrap();
    assert_eq!(report.removed, vec!["dropped.txt".to_owned()]);
    assert!(!report.is_noop());
    assert!(!dir.path().join("dropped.txt").exists());
    assert!(dir.path().join("kept.txt").exists());
    assert!(dir.path().join("README.md").exists());
}

/// Idempotency (QSL #138 acceptance point): running `revendor` twice at the
/// same pin is a no-op the second time. This uses a disposable git
/// repository this test creates on the spot, so it needs no history beyond
/// the one commit it makes here -- unlike reading a real commit from this
/// repository's own history, it passes under CI's shallow (`fetch-depth: 1`)
/// checkout. A negative control (tampering the vendored bytes, not the
/// immutable pinned commit) proves the "no-op" assertions can actually fail.
/// Tracing: TC-151.
#[trace("TC-151", "NFR-011-AC-3")]
#[test]
fn revendor_is_idempotent_against_a_hermetic_git_repository() {
    let source_repo = tempfile::tempdir().unwrap();
    init_git_repo(source_repo.path());
    let commit = git_commit_file(source_repo.path(), "file.txt", b"v1");

    let dest = tempfile::tempdir().unwrap();
    let mut manifest = Manifest {
        schema_version: 1,
        tree: "synthetic".into(),
        sources: vec![Source::SelfRepo {
            commit,
            dest_prefix: String::new(),
            files: vec![PinnedFile {
                path: "file.txt".into(),
                sha256: "sha256:".to_owned() + &"0".repeat(64), // placeholder, overwritten
            }],
        }],
    };
    let sources = Sources {
        workspace_root: source_repo.path(),
        qspec_clone: None,
        fcd_clone: None,
    };

    let first = revendor(&mut manifest, dest.path(), &sources).unwrap();
    assert_eq!(first.written, vec!["file.txt".to_owned()]);
    assert_eq!(std::fs::read(dest.path().join("file.txt")).unwrap(), b"v1");
    let digest_after_first = {
        let Source::SelfRepo { files, .. } = &manifest.sources[0] else {
            unreachable!()
        };
        files[0].sha256.clone()
    };

    let second = revendor(&mut manifest, dest.path(), &sources).unwrap();
    assert!(
        second.is_noop(),
        "second run wrote {:?} / removed {:?}",
        second.written,
        second.removed
    );
    assert_eq!(second.unchanged, vec!["file.txt".to_owned()]);
    assert_eq!(std::fs::read(dest.path().join("file.txt")).unwrap(), b"v1");
    let Source::SelfRepo { files, .. } = &manifest.sources[0] else {
        unreachable!()
    };
    assert_eq!(
        files[0].sha256, digest_after_first,
        "the manifest's recorded digest changed on a no-op run"
    );

    // Negative control: tamper the destination bytes directly (the pinned
    // commit itself is immutable, so this is the only way to simulate
    // drift). A third run at the same pin must detect and repair it -- if it
    // did not, the "no-op" assertions above would be vacuously true even for
    // a `revendor` that never compares bytes at all.
    std::fs::write(dest.path().join("file.txt"), b"tampered").unwrap();
    let third = revendor(&mut manifest, dest.path(), &sources).unwrap();
    assert_eq!(third.written, vec!["file.txt".to_owned()]);
    assert_eq!(std::fs::read(dest.path().join("file.txt")).unwrap(), b"v1");
}

/// `revendor` refuses a qspec-kind source when no clone was given, rather
/// than silently skipping it or reaching for the network.
/// Tracing: TC-149.
#[trace("TC-149", "NFR-011-AC-1")]
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
        fcd_clone: None,
    };
    let error = revendor(&mut manifest, dir.path(), &sources).unwrap_err();
    match error {
        xtask::Error::MissingClone { owner, flag, .. } => {
            assert_eq!(owner, xtask::CommitOwner::Qspec);
            assert_eq!(flag, "--qspec-clone");
        }
        other => panic!("expected MissingClone, got {other:?}"),
    }
}

/// `revendor` refuses an fcd-kind source when no clone was given, rather than
/// silently skipping it or reaching for the network.
/// Tracing: TC-149.
#[trace("TC-149", "NFR-011-AC-1")]
#[test]
fn revendor_refuses_an_fcd_source_with_no_clone_given() {
    let root = workspace_root();
    let dir = tempfile::tempdir().unwrap();
    let mut manifest = Manifest {
        schema_version: 1,
        tree: "synthetic".into(),
        sources: vec![Source::Fcd {
            repo: "https://github.com/agent-ix/filament-core-data".into(),
            commit: "7dcb2f2c7466a770b2362561e70ed10a8f941c1f".into(),
            source_prefix: "crates/extraction-frontend/fixtures/architecture".into(),
            dest_prefix: String::new(),
            files: vec![PinnedFile {
                path: "PROVENANCE.json".into(),
                sha256: "sha256:".to_owned() + &"0".repeat(64),
            }],
        }],
    };
    let sources = Sources {
        workspace_root: &root,
        qspec_clone: None,
        fcd_clone: None,
    };
    let error = revendor(&mut manifest, dir.path(), &sources).unwrap_err();
    match error {
        xtask::Error::MissingClone { owner, flag, .. } => {
            assert_eq!(owner, xtask::CommitOwner::Fcd);
            assert_eq!(flag, "--fcd-clone");
        }
        other => panic!("expected MissingClone, got {other:?}"),
    }
}

/// Deep, real-clone-backed round trip: with `QSPEC_CLONE_PATH` pointing at a
/// local `agent-ix/quire-specification` checkout, re-running `revendor`
/// against the pinned commits in the checked-in manifests changes nothing.
/// Skips cleanly when the environment variable is absent (e.g. CI), matching
/// how this repo's other externally dependent tests skip. Works on a
/// disposable copy of each vendored tree, never on the checked-in
/// `resources/` directly, so a failing assertion never leaves it dirty.
/// Only the two qspec-sourced trees are exercised here; the two
/// fcd-sourced trees have their own `FCD_CLONE_PATH`-gated counterpart below.
/// Tracing: TC-151.
#[trace("TC-151", "NFR-011-AC-3")]
#[test]
fn revendor_is_idempotent_against_a_real_qspec_clone() {
    let Ok(clone) = std::env::var("QSPEC_CLONE_PATH") else {
        eprintln!("QSPEC_CLONE_PATH not set; skipping the real-clone idempotency check");
        return;
    };
    let clone = PathBuf::from(clone);
    let root = workspace_root();
    for tree in [Tree::NativeV1, Tree::CompleteValue] {
        let manifest_path = tree.manifest_path(&root);
        let mut manifest = Manifest::load(&manifest_path).unwrap();
        let before: Vec<u8> = std::fs::read(&manifest_path).unwrap();

        // Never revendor the checked-in tree in place: work on a scratch
        // copy so a failing run cannot leave `resources/` dirty.
        let scratch = tempfile::tempdir().unwrap();
        let tree_copy = scratch.path().join(tree.dir_name());
        copy_dir_recursive(&tree.root(&root), &tree_copy);

        let sources = Sources {
            workspace_root: &root,
            qspec_clone: Some(&clone),
            fcd_clone: None,
        };
        let report = revendor(&mut manifest, &tree_copy, &sources).unwrap();
        assert!(
            report.is_noop(),
            "{}: re-vendoring at the recorded pin wrote {:?} / removed {:?}",
            tree.dir_name(),
            report.written,
            report.removed
        );

        let scratch_manifest_path = scratch.path().join("VENDOR.json");
        manifest.save(&scratch_manifest_path).unwrap();
        let after = std::fs::read(&scratch_manifest_path).unwrap();
        assert_eq!(
            before,
            after,
            "{}: manifest byte-changed on a no-op run",
            tree.dir_name()
        );
    }
}

/// The `FCD_CLONE_PATH`-gated counterpart of
/// `revendor_is_idempotent_against_a_real_qspec_clone`, for the two
/// fcd-sourced trees `tests/fixtures/architecture` and
/// `tests/fixtures/modules` (QSL #131 PR 3). Skips cleanly when the
/// environment variable is absent.
/// Tracing: TC-151.
#[trace("TC-151", "NFR-011-AC-3")]
#[test]
fn revendor_is_idempotent_against_a_real_fcd_clone() {
    let Ok(clone) = std::env::var("FCD_CLONE_PATH") else {
        eprintln!("FCD_CLONE_PATH not set; skipping the real-clone idempotency check");
        return;
    };
    let clone = PathBuf::from(clone);
    let root = workspace_root();
    for tree in [Tree::TestFixturesArchitecture, Tree::TestFixturesModules] {
        let manifest_path = tree.manifest_path(&root);
        let mut manifest = Manifest::load(&manifest_path).unwrap();
        let before: Vec<u8> = std::fs::read(&manifest_path).unwrap();

        let scratch = tempfile::tempdir().unwrap();
        let tree_copy = scratch.path().join(tree.dir_name());
        copy_dir_recursive(&tree.root(&root), &tree_copy);

        let sources = Sources {
            workspace_root: &root,
            qspec_clone: None,
            fcd_clone: Some(&clone),
        };
        let report = revendor(&mut manifest, &tree_copy, &sources).unwrap();
        assert!(
            report.is_noop(),
            "{}: re-vendoring at the recorded pin wrote {:?} / removed {:?}",
            tree.dir_name(),
            report.written,
            report.removed
        );

        let scratch_manifest_path = scratch.path().join("VENDOR.json");
        manifest.save(&scratch_manifest_path).unwrap();
        let after = std::fs::read(&scratch_manifest_path).unwrap();
        assert_eq!(
            before,
            after,
            "{}: manifest byte-changed on a no-op run",
            tree.dir_name()
        );
    }
}
