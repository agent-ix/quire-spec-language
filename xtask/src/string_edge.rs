// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL#214 (FR-064): `cargo xtask string-edge` -- QSL's adapter over the
//! standalone `qsl-string-edge` scanner. It supplies QSL's crate source roots
//! and QSL's edge rules and nothing else: the detector, the marker and
//! test-code handling, the allow-list checks and the report all live in
//! `qsl-string-edge`, the one implementation consumer repositories run too.

use std::path::{Path, PathBuf};

use qsl_string_edge::{scan, EdgeRules, ScanRequest};

use crate::error::{Error, Result};

/// Every crate source root this scan covers, relative to the workspace root:
/// the QSL root crate, `xtask`, and `qsl-walk-grow` (ADR-011 §6.1 layer WG),
/// `qsl-foundation` (ADR-011 §7.3 X-2), `qsl-cst` (ADR-011 §7.3 X-3),
/// `qsl-source` (ADR-011 §7.3 X-4), `qsl-forms` (ADR-011 §7.3 X-5),
/// `qsl-semantics` (ADR-011 §7.3 X-6), `qsl-package` (ADR-011 §7.3 X-7),
/// `qsl-eval` (ADR-011 §7.3 X-8), `qsl-route` (ADR-011 §7.3 X-9),
/// `qsl-replay` (ADR-011 §7.3 X-10 -- each extracted §6.1 layer crate adds its
/// own entry here the same way), `qsl-analyze` (ADR-011 §7.3 X-12),
/// `qsl-bench` (the benchmark harness) and `qsl-string-edge` (the scanner
/// scans itself). `qsl-attrs` is excluded -- it is a proc-macro identity
/// transform with no string dispatch of any kind (its own module doc).
fn crate_roots() -> Vec<PathBuf> {
    [
        "src",
        "xtask/src",
        "qsl-walk-grow/src",
        "qsl-foundation/src",
        "qsl-cst/src",
        "qsl-source/src",
        "qsl-forms/src",
        "qsl-semantics/src",
        "qsl-package/src",
        "qsl-eval/src",
        "qsl-route/src",
        "qsl-replay/src",
        "qsl-analyze/src",
        "qsl-bench/src",
        "qsl-string-edge/src",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}

/// QSL's edge rules. The allow-list is empty: nothing in the QSL crates has
/// yet been reviewed and admitted as a genuine non-branching string
/// comparison; each addition is a deliberate, reviewed decision, not a
/// default.
fn rules() -> EdgeRules {
    EdgeRules::default()
}

/// Run the `string-edge` scan over QSL's crate roots under `workspace_root`.
/// Returns the "clean" summary text on success (a clean scan is `Ok`,
/// matching `cargo xtask string-edge`'s own success/failure contract); an
/// occurrence report is returned as [`Error::StringEdgeFound`], not `Ok`, so
/// the caller's exit code reflects the finding, and a scan that could not
/// complete is [`Error::StringEdgeScan`].
pub fn run(workspace_root: &Path) -> Result<String> {
    run_with(&ScanRequest {
        base_dir: workspace_root.to_path_buf(),
        crate_roots: crate_roots(),
        rules: rules(),
    })
}

/// [`run`] over an explicit request: the one place a scanner report becomes
/// this gate's `Ok`/`Err`.
fn run_with(request: &ScanRequest) -> Result<String> {
    let report = scan(request)?;
    if report.is_clean() {
        Ok(report.render())
    } else {
        Err(Error::StringEdgeFound {
            summary: report.render(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Code;
    use ix_trace_rs::trace;
    use std::fs;

    /// Workspace members `crate_roots` does not scan: `qsl-attrs` (see
    /// `crate_roots`' own doc) and `tools/arch-lint`, which the scan has
    /// never covered.
    const UNSCANNED_MEMBERS: [&str; 2] = ["qsl-attrs", "tools/arch-lint"];

    fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask lives one level below the workspace root")
            .to_path_buf()
    }

    /// Every workspace member's `src/` is a scan root, except the
    /// documented [`UNSCANNED_MEMBERS`]. A crate extracted later, or a root
    /// dropped from the list (removing `qsl-route/src`
    /// failed nothing), fails here.
    #[trace("TC-162", "FR-064-AC-11")]
    #[test]
    fn every_workspace_member_is_a_scan_root() {
        let workspace_root = workspace_root();
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
        let output = std::process::Command::new(cargo)
            .current_dir(&workspace_root)
            .args([
                "metadata",
                "--format-version",
                "1",
                "--no-deps",
                "--offline",
            ])
            .output()
            .expect("cargo metadata runs");
        assert!(output.status.success(), "cargo metadata failed");
        let metadata: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("cargo metadata emits JSON");
        let mut expected: Vec<PathBuf> = metadata["packages"]
            .as_array()
            .expect("a package list")
            .iter()
            .map(|package| {
                let manifest = Path::new(package["manifest_path"].as_str().expect("a manifest"));
                manifest
                    .parent()
                    .expect("a package directory")
                    .strip_prefix(&workspace_root)
                    .expect("a member under the workspace root")
                    .to_path_buf()
            })
            .filter(|member| {
                !UNSCANNED_MEMBERS
                    .iter()
                    .any(|unscanned| member == Path::new(unscanned))
            })
            .map(|member| member.join("src"))
            .collect();
        expected.sort();
        let mut roots = crate_roots();
        roots.sort();
        assert_eq!(roots, expected);
    }

    /// FR-064-AC-6: the real `Makefile` names `string-edge` as a prerequisite
    /// of `ci` and its recipe runs the `xtask` binary's `string-edge`
    /// subcommand. One-copy-deps (`make use-local`) moved the `cargo xtask`
    /// alias out of the tracked `.cargo/config.toml`, so the recipe spells
    /// this out as `cargo run --package xtask --`.
    #[trace("TC-162", "FR-064-AC-6")]
    #[test]
    fn the_real_makefile_wires_string_edge_into_ci() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask lives one level below the workspace root");
        let makefile =
            fs::read_to_string(workspace_root.join("Makefile")).expect("the Makefile reads");
        let ci = makefile
            .lines()
            .find_map(|line| line.strip_prefix("CI_CHECKS :="))
            .expect("the Makefile has the CI_CHECKS aggregate list");
        assert!(
            ci.split_whitespace().any(|target| target == "string-edge"),
            "CI_CHECKS must list string-edge as an internal prerequisite: {ci}"
        );
        let mut lines = makefile.lines();
        assert!(
            lines.any(|line| line == "string-edge:"),
            "the Makefile must define a `string-edge:` target"
        );
        assert_eq!(
            lines.next(),
            Some("\tcargo run --package xtask -- string-edge"),
            "the recipe must run `cargo run --package xtask -- string-edge`"
        );
    }

    /// FR-064-AC-6: a `.PHONY` aggregate depending on a prerequisite whose
    /// recipe can fail fails when the prerequisite fails, and not when it
    /// succeeds -- `make`'s ordinary semantics, shown on a fixture of the
    /// same aggregate/prerequisite shape.
    #[trace("TC-162", "FR-064-AC-6")]
    #[test]
    fn a_failed_prerequisite_fails_the_aggregate_target() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let makefile = dir.path().join("Makefile");
        fs::write(
            &makefile,
            ".PHONY: gate-fail gate-ok scan-fail scan-ok\n\
             gate-fail: scan-fail\n\
             gate-ok: scan-ok\n\
             scan-fail:\n\tfalse\n\
             scan-ok:\n\ttrue\n",
        )
        .expect("write the fixture Makefile");
        let run = |target: &str| {
            std::process::Command::new("make")
                .arg("-f")
                .arg(&makefile)
                .arg(target)
                .output()
                .expect("make runs")
                .status
                .success()
        };
        assert!(
            !run("gate-fail"),
            "a failed prerequisite must fail the gate"
        );
        assert!(
            run("gate-ok"),
            "a passing prerequisite must not fail the gate"
        );
    }

    /// A workspace tree holding every root [`crate_roots`] names, each with
    /// one clean production file.
    fn fixture_workspace() -> tempfile::TempDir {
        let dir = crate::test_support::tempdir();
        for root in crate_roots() {
            fs::create_dir_all(dir.path().join(&root)).expect("create a crate root");
            fs::write(
                dir.path().join(root).join("lib.rs"),
                "pub fn clean() -> bool { false }\n",
            )
            .expect("write a clean file");
        }
        dir
    }

    fn request(base: &Path) -> ScanRequest {
        ScanRequest {
            base_dir: base.to_path_buf(),
            crate_roots: crate_roots(),
            rules: rules(),
        }
    }

    /// FR-064-AC-4 and AC-11: the adapter exits non-zero for a non-empty
    /// report and zero for an empty one, and its report text and success or
    /// failure are the library's own on the same tree.
    #[trace("TC-162", "FR-064-AC-4", "FR-064-AC-11")]
    #[test]
    fn a_non_empty_report_exits_non_zero_a_clean_scan_exits_zero() {
        let dir = fixture_workspace();
        let root = dir.path();
        let fixture = root.join("src/string_edge_fixture.rs");

        fs::write(
            &fixture,
            "fn dispatch(kind: &str) -> bool { kind == \"value\" }\n",
        )
        .expect("write a violating fixture file");
        let error = run(root).expect_err("a non-empty report must be an Err, not an Ok");
        assert_eq!(error.code(), Code::StringEdge);
        assert_eq!(error.exit_code(), 1);
        let library = scan(&request(root)).expect("the library completes the same scan");
        assert_eq!(library.occurrences.len(), 1);
        assert_eq!(error.to_string(), library.render());
        assert_eq!(
            error.to_string(),
            "string-edge: 1 unmarked, unlisted occurrence(s):\n  \
             src/string_edge_fixture.rs:1\n"
        );

        fs::write(&fixture, "fn dispatch(kind: &str) -> bool { false }\n")
            .expect("clean the same fixture file, with no change to the tree otherwise");
        let summary = run(root).expect("a clean scan must be Ok");
        assert_eq!(
            summary,
            "string-edge: no unmarked, unlisted string comparison or string match found\n"
        );
        assert_eq!(
            summary,
            scan(&request(root)).expect("clean library scan").render()
        );
    }

    /// FR-064-AC-9 and AC-11: a scan that cannot complete is never clean; the
    /// adapter carries the library's typed error and exits 2.
    #[trace("TC-162", "FR-064-AC-9", "FR-064-AC-11")]
    #[test]
    fn an_invalid_input_fixture_fails_like_the_library_with_exit_two() {
        let dir = fixture_workspace();
        let root = dir.path();
        fs::remove_dir_all(root.join("qsl-route/src")).expect("drop one configured root");
        let error = run(root).expect_err("a missing configured root cannot be clean");
        assert!(
            matches!(
                &error,
                Error::StringEdgeScan(qsl_string_edge::Error::RootUnresolved { path, .. })
                    if path == Path::new("qsl-route/src")
            ),
            "{error:?}"
        );
        assert_eq!(error.code(), Code::StringEdgeScan);
        assert_eq!(error.exit_code(), 2);
        let library = scan(&request(root)).expect_err("the library refuses the same tree");
        assert_eq!(error.to_string(), library.to_string());
    }

    /// FR-064-AC-5's real-site half: runs the real scan over the ADR-010
    /// section 4.3 dispatch strings still compared in the tree, at the one
    /// typed conversion each now lives in: `CanonicalizationDomain::from_str`
    /// (`"filament-canonical-json-1"`), `AdapterArtifact::try_from`
    /// (`"quire.state.authority-adapter"`) and `clock_binding_name` (the
    /// `"clock:"` prefix, a `strip_prefix`). An allow-list entry at each must
    /// be refused as gating a branch, whether or not a `#[string_edge]` mark
    /// covers the site. The `"quire.protocol.finite-global/v1"` profile string
    /// is now only registry data (`RegisteredDefinition::identity`), matched by
    /// the registry's own table, and the `"allocation"` site is gone from the
    /// tree. Fails if a site's file/item vanishes without this list being
    /// updated, and if the detector stops seeing a real site.
    #[trace("TC-162", "FR-064-AC-5")]
    #[test]
    fn real_adr010_sites_are_refused_as_allow_list_entries() {
        let sites = [
            (
                "src",
                "src/state/input.rs",
                "CanonicalizationDomain::from_str",
            ),
            ("src", "src/state/input.rs", "AdapterArtifact::try_from"),
            ("src", "src/protocol_artifact/mod.rs", "clock_binding_name"),
            // A real named-`const NAME: &str` site
            // (`identity == NARROW`, `qsl-semantics/src/check/claims.rs`),
            // not a string literal -- the const-resolving detector is what
            // finds this occurrence.
            (
                "qsl-semantics/src",
                "qsl-semantics/src/check/claims.rs",
                "operation_role",
            ),
        ];
        for (crate_root, file, item) in sites {
            let request = ScanRequest {
                base_dir: workspace_root(),
                crate_roots: vec![PathBuf::from(crate_root)],
                rules: EdgeRules {
                    allow_list: vec![qsl_string_edge::AllowListEntry {
                        file: file.to_owned(),
                        item: item.to_owned(),
                        reason: "test: a real ADR-010 section 4.3 production dispatch site"
                            .to_owned(),
                    }],
                },
            };
            let error = scan(&request).expect_err("the entry must be refused");
            assert!(
                matches!(
                    &error,
                    qsl_string_edge::Error::AllowListGatesABranch { file: f, item: i }
                        if f == file && i == item
                ),
                "{file}::{item} is not refused as an allow-list entry: {error:?}"
            );
        }
    }
}
