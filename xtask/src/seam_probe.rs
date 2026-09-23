// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL#214 (FR-063): `cargo xtask seam-probe` demonstrates ADR-012 §5.1's S1
//! and S7 seams by building the QSL crate under `RUSTFLAGS=--cfg seam_probe`
//! and comparing the `E0004` (non-exhaustive match) locations rustc reports
//! against a checked-in list, exactly as FR-063 requires.
//!
//! **Scope: S1 (one location) and S7 (two locations) (PR #262 review,
//! finding F7; PR #305 review, finding 6).** FR-063-AC-6 names five
//! categories: S1's stage-participation table and prefix arm, S2, S3, S4.
//! S1 originally had two checked-in `match`es over `FamilyKind` in
//! `src/family/mod.rs` -- `catalog_code_prefix`'s prefix arm and
//! `stage_hooks`'s stage-participation table. `stage_hooks` is deleted: its
//! only non-test callers were three `assert_eq!` call sites asserting a
//! hand-written `match`'s own literal result against itself, a fabricated
//! reader manufactured to keep otherwise-dead code alive (see
//! `crate::family`'s module doc in the QSL crate). Deleting the fabricated
//! callers left `stage_hooks` itself with no real reader, so it is gone
//! too, and with it the second S1 seam this tool could check in. Only
//! `catalog_code_prefix`'s prefix arm remains. S2 (the parser's
//! leading-token-kind entry table and the parsed-form-enum check seam) and
//! S3 (the checked-node-enum's evaluator, v2 emitter and
//! requirement-derivation matches) are seams over `token::Kind`,
//! `Expression` and `NodeKind` -- pre-existing, crate-wide enums every
//! `Value` form uses (literals, operators, `let`, `if`, records,
//! collections), not just the two forms this ticket migrates (function
//! declaration and application) -- so covering them means adding a probe
//! arm to every match site across `parser.rs`, `check.rs`, `evaluate.rs`,
//! `facts.rs` and `termination.rs`, correctly, for forms this ticket does
//! not own. That is tracked as its own piece of work (Linear QSL-143), not
//! attempted here. S4 (each family `Cause` enum's `catalog_code()`) has no
//! cause-bearing family to demonstrate it yet -- see
//! `crate::family::outcome`'s own doc in the QSL crate.
//!
//! **S7 (QSL-46/#185, ADR-012 §5.1's row: "requirement derivation per
//! family; registry advertisement check; CG `negotiate_*` capability arm").**
//! QSL owns the first two; `#[cfg(seam_probe)]` on
//! `crate::check::Capability` (defined in `capability.rs` itself, which also
//! carries its own match's probe arm and so is not a seam-probe location)
//! makes both non-exhaustive under the probe build: `requests::families`
//! (requirement derivation per family) and `route.rs`'s `advertises_kind`
//! helper `same_kind` (registry advertisement check). The CG `negotiate_*`
//! arm is quire-contract-codegen#86's own seam, not probed here.
//!
//! This tool's checked-in list therefore covers exactly the one S1 and two
//! S7 locations QSL's own code adds, and reports that scope honestly rather
//! than as "AC-6 satisfied."

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;

use qsl_attrs::string_edge;
use syn::spanned::Spanned;
use syn::visit::Visit;

use crate::error::{Error, Result};

/// One seam-probe location: a file path (relative to the workspace root, as
/// rustc reports it) and the enclosing item's name -- `Type::method` inside
/// an `impl`, or a bare function name at module scope (PR #262 review,
/// finding F14).
///
/// **Not a bare line number.** An earlier version of this struct keyed on
/// `(file, line)`, matching FR-063's own text loosely enough, but a line
/// number moves whenever anything *above* the checked-in `match` in the
/// same file changes -- adding a doc comment, a new item, a `cargo fmt`
/// reflow -- none of which touch the seam itself. That red-lines `make ci`
/// for a reason unrelated to the seam the probe exists to guard. rustc's
/// `E0004` diagnostic carries a primary span inside the enclosing item, not
/// the item's own identity, so this tool resolves that span to the
/// enclosing function's name itself (via a `syn` walk over the same source
/// file, `enclosing_item_name`) and compares on that instead -- stable
/// against everything except actually moving, renaming or removing the
/// seam.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SeamLocation {
    /// The source file, relative to the workspace root, exactly as rustc's
    /// `E0004` diagnostic reports it.
    pub file: String,
    /// The enclosing item's name -- `Type::method` inside an `impl`, or a
    /// bare function name at module scope.
    pub item: String,
}

/// FR-063-AC-6's S1 and S7 categories, checked in.
///
/// S1: the one `match` over `FamilyKind` in `qsl-semantics/src/family/mod.rs` --
/// `catalog_code_prefix`'s prefix arm. `stage_hooks`'s stage-participation
/// table match is deleted (this module's own doc, PR #262 review finding
/// F7), narrowing this list from two locations to one. Exactly one location
/// because that module has exactly one `match` over `FamilyKind` today;
/// adding a second real seam over `FamilyKind` means updating this list in
/// the same change (FR-063-AC-1/AC-2), or `xtask seam-probe` fails.
///
/// S7 (QSL-46/#185): the two `match`es over `crate::check::Capability`
/// outside `capability.rs` itself (which owns the probe variant and its
/// own arm, so is not a seam-probe location) -- `requests::families`
/// (requirement derivation per family, `src/linking/composed/requests.rs`)
/// and `route.rs`'s `advertises_kind` helper `same_kind` (registry
/// advertisement check). Both item names were confirmed empirically by
/// running this tool under `--cfg seam_probe` and reading the reported
/// `E0004` locations, not assumed from the source layout: `same_kind` is a
/// bare `fn` nested inside `advertises_kind`'s body, not an `impl` method,
/// so it resolves unqualified.
///
/// FR-090-AC-6 (TC-387): the `ProtocolClause` snapshot cause's
/// `catalog_code()` match over `crate::check::WrongSnapshotCause`, whose
/// `#[cfg(seam_probe)]` variant has an arm only in `WrongSnapshotCause::
/// as_str`.
///
/// FR-090-AC-4 (TC-385): the S6a seam's `match` over
/// `value::expression::s6a::S6aFamilyKind` (`evaluate_declaration`), its
/// `S6aFamilyKind::family` mapping to `FamilyKind`, and the one
/// `match` over `crate::family::FamilyOutcome` (`ValueFunctionFamily::
/// evaluate`, which passes the evaluator's `FamilyOutcome` back as an
/// `EvalOutcome`). Each enum carries a `#[cfg(seam_probe)]` variant.
pub fn checked_in_locations() -> BTreeSet<SeamLocation> {
    [
        SeamLocation {
            file: "qsl-semantics/src/family/mod.rs".to_owned(),
            item: "FamilyKind::catalog_code_prefix".to_owned(),
        },
        SeamLocation {
            file: "src/linking/composed/requests.rs".to_owned(),
            item: "families".to_owned(),
        },
        SeamLocation {
            file: "src/route.rs".to_owned(),
            item: "same_kind".to_owned(),
        },
        SeamLocation {
            file: "src/value/expression/causes.rs".to_owned(),
            item: "ProtocolClauseSnapshot::catalog_code".to_owned(),
        },
        SeamLocation {
            file: "src/value/expression/mod.rs".to_owned(),
            item: "evaluate_declaration".to_owned(),
        },
        SeamLocation {
            file: "src/value/expression/s6a.rs".to_owned(),
            item: "S6aFamilyKind::family".to_owned(),
        },
        SeamLocation {
            file: "src/value/expression/family.rs".to_owned(),
            item: "ValueFunctionFamily::evaluate".to_owned(),
        },
    ]
    .into_iter()
    .collect()
}

/// A `syn` walk over one parsed file, finding the innermost `fn` (a bare
/// module-level function, or an `impl` method qualified as `Type::method`)
/// whose span contains `target_line`. Picks the smallest enclosing span so
/// a nested item (there are none at seam sites today, but a future match
/// inside a closure or a nested `fn` should still resolve to its own
/// immediate item, not an outer one) wins over a larger ancestor.
struct EnclosingItem {
    target_line: u32,
    current_impl_self: Option<String>,
    found: Option<(String, u32)>,
}

impl EnclosingItem {
    fn consider(&mut self, span: proc_macro2::Span, name: String) {
        let start = span.start().line as u32;
        let end = span.end().line as u32;
        if start <= self.target_line && self.target_line <= end {
            let width = end - start;
            if self.found.as_ref().is_none_or(|(_, best)| width < *best) {
                self.found = Some((name, width));
            }
        }
    }
}

impl<'ast> Visit<'ast> for EnclosingItem {
    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        let previous = self
            .current_impl_self
            .replace(crate::impl_self_name(&node.self_ty));
        syn::visit::visit_item_impl(self, node);
        self.current_impl_self = previous;
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.consider(node.span(), node.sig.ident.to_string());
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        let name = match &self.current_impl_self {
            Some(self_ty) => format!("{self_ty}::{}", node.sig.ident),
            None => node.sig.ident.to_string(),
        };
        self.consider(node.span(), name);
        syn::visit::visit_impl_item_fn(self, node);
    }
}

/// Resolve a 1-based `line` in `source` to its innermost enclosing
/// function's name (see [`EnclosingItem`]). `None` if `source` fails to
/// parse or no function's span contains `line`.
fn enclosing_item_name(source: &str, line: u32) -> Option<String> {
    let parsed = syn::parse_file(source).ok()?;
    let mut visitor = EnclosingItem {
        target_line: line,
        current_impl_self: None,
        found: None,
    };
    visitor.visit_file(&parsed);
    visitor.found.map(|(name, _)| name)
}

/// Build `package`'s `--lib` target with `rustflags` appended to
/// `RUSTFLAGS` (empty for a normal build) and collect every `E0004`
/// diagnostic's primary span, resolved to its enclosing item ([`SeamLocation`]),
/// plus whether the build itself succeeded and its raw stderr (F15: needed
/// to tell a genuine compile failure apart from `--offline` dependency
/// resolution failing before rustc ever runs).
///
/// **Its own `--target-dir` (PR #262 review, finding F7).** This function
/// runs `cargo build` twice with different `RUSTFLAGS` (plain, then `--cfg
/// seam_probe`); without a target dir of its own, it inherited whatever
/// `CARGO_TARGET_DIR` the caller had set -- the same directory `cargo
/// test`/`cargo clippy` use elsewhere in the same `make ci` run. Each
/// RUSTFLAGS flip invalidates that whole dependency graph's incremental
/// cache, and the next unrelated build in the same directory pays to
/// rebuild it again. `target/seam-probe` (the same relative-to-workspace-
/// root idiom `ci-clean-build`'s own `--target-dir target/clean` already
/// uses in the Makefile) keeps this probe's own RUSTFLAGS churn out of the
/// shared one.
fn build_and_collect_e0004(
    workspace_root: &Path,
    package: &str,
    rustflags: &str,
) -> Result<(bool, BTreeSet<SeamLocation>, String)> {
    let mut command = Command::new("cargo");
    command.current_dir(workspace_root).args([
        "build",
        "--offline",
        "-p",
        package,
        "--lib",
        "--message-format=json",
        "--target-dir",
        "target/seam-probe",
    ]);
    if !rustflags.is_empty() {
        command.env("RUSTFLAGS", rustflags);
    }
    let output = command
        .output()
        .map_err(|source| Error::SeamProbeSpawn { source })?;
    let succeeded = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut raw_lines: BTreeSet<(String, u32)> = BTreeSet::new();
    for line in stdout.lines() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if value.get("reason").and_then(serde_json::Value::as_str) != Some("compiler-message") {
            continue;
        }
        let Some(message) = value.get("message") else {
            continue;
        };
        let is_e0004 = message
            .get("code")
            .and_then(|code| code.get("code"))
            .and_then(serde_json::Value::as_str)
            == Some("E0004");
        if !is_e0004 {
            continue;
        }
        let Some(spans) = message.get("spans").and_then(serde_json::Value::as_array) else {
            continue;
        };
        for span in spans {
            if span.get("is_primary").and_then(serde_json::Value::as_bool) != Some(true) {
                continue;
            }
            let file = span.get("file_name").and_then(serde_json::Value::as_str);
            let line_start = span.get("line_start").and_then(serde_json::Value::as_u64);
            if let (Some(file), Some(line_start)) = (file, line_start) {
                raw_lines.insert((file.to_owned(), line_start as u32));
            }
        }
    }
    let mut locations = BTreeSet::new();
    for (file, line) in raw_lines {
        let path = workspace_root.join(&file);
        let source = fs::read_to_string(&path)
            .map_err(|source| Error::SeamProbeReadSource { path, source })?;
        let item = enclosing_item_name(&source, line)
            .unwrap_or_else(|| format!("<unresolved item at line {line}>"));
        locations.insert(SeamLocation { file, item });
    }
    Ok((succeeded, locations, stderr))
}

/// F15: `--offline` on a registry cargo has not yet fetched into makes
/// dependency *resolution* fail before rustc ever runs -- no compiler
/// message, no `E0004`, just a non-zero exit and a cargo-authored error on
/// stderr. That is not "the normal build failed to compile"; reporting it
/// as one hides the real, actionable cause (a cold `--offline` registry,
/// fixable by a warm build first) behind a message that reads like a
/// genuine seam regression. Cargo's own offline-resolution error always
/// names the flag it could not honor; this checks for that literal
/// substring rather than guessing at cargo's exact wording, which changes
/// across versions.
///
/// `#[string_edge]` (PR #262 review, nit): this is exactly the class of
/// unmarked literal-substring branch gate `xtask string-edge` exists to
/// find -- including, previously, inside this tool itself.
#[string_edge]
fn offline_registry_unavailable(stderr: &str) -> bool {
    stderr.contains("--offline")
}

/// One probe build: a workspace package whose seams it reports, and the
/// `RUSTFLAGS` it is built under.
///
/// **One probe build per crate (QSL-181).** A seam `match` makes its own
/// crate fail to compile under `--cfg seam_probe`, and a crate that fails
/// stops every crate that depends on it: rustc never reaches them. Since X-6b
/// moved `check` and `family` into `qsl-semantics`, the S1 seam
/// (`FamilyKind::catalog_code_prefix`) is in that crate, while the root
/// crate's seams match over `qsl-semantics`' probe variants. So the probe
/// builds `qsl-semantics` alone under `--cfg seam_probe`, which reports its
/// own seam, and then the root crate under `--cfg seam_probe --cfg
/// seam_probe_downstream`, where the downstream cfg gives each lower crate's
/// own seam its probe arm so that crate compiles and the root crate's seams
/// are reached. Each build must fail; together they must report exactly the
/// checked-in list.
const PROBE_BUILDS: [(&str, &str); 2] = [
    ("qsl-semantics", "--cfg seam_probe"),
    (
        "quire-spec-language",
        "--cfg seam_probe --cfg seam_probe_downstream",
    ),
];

/// `cargo xtask seam-probe`: FR-063's two required assertions (probe builds
/// that fail with exactly the checked-in `E0004` locations, and a normal
/// build that succeeds with none), then the checked-in-list comparison
/// itself.
pub fn run(workspace_root: &Path) -> Result<String> {
    // FR-063 constraint: assert the probe build fails AND the normal build
    // succeeds -- one without the other is half a test. If `--cfg
    // seam_probe` silently stopped being applied, only checking the normal
    // build would never notice. The normal build of the root crate builds
    // every crate below it too.
    let (normal_succeeded, normal_locations, normal_stderr) =
        build_and_collect_e0004(workspace_root, "quire-spec-language", "")?;
    if !normal_succeeded {
        if offline_registry_unavailable(&normal_stderr) {
            return Err(Error::SeamProbeOfflineRegistryUnavailable {
                stderr: normal_stderr,
            });
        }
        return Err(Error::SeamProbeNormalBuildFailed {
            stderr: normal_stderr,
        });
    }
    if !normal_locations.is_empty() {
        return Err(Error::SeamProbeNormalBuildHasE0004 {
            locations: format!("{normal_locations:?}"),
        });
    }
    let mut probe_locations = BTreeSet::new();
    for (package, rustflags) in PROBE_BUILDS {
        let (probe_succeeded, locations, probe_stderr) =
            build_and_collect_e0004(workspace_root, package, rustflags)?;
        if probe_succeeded {
            return Err(Error::SeamProbeBuildUnexpectedlySucceeded);
        }
        if locations.is_empty() && offline_registry_unavailable(&probe_stderr) {
            return Err(Error::SeamProbeOfflineRegistryUnavailable {
                stderr: probe_stderr,
            });
        }
        probe_locations.extend(locations);
    }
    let checked_in = checked_in_locations();
    let unexpected_but_present: Vec<_> = probe_locations.difference(&checked_in).cloned().collect();
    let expected_but_missing: Vec<_> = checked_in.difference(&probe_locations).cloned().collect();
    if !unexpected_but_present.is_empty() || !expected_but_missing.is_empty() {
        return Err(Error::SeamProbeMismatch {
            unexpected_but_present: format!("{unexpected_but_present:?}"),
            expected_but_missing: format!("{expected_but_missing:?}"),
        });
    }
    Ok(format!(
        "seam-probe: {} checked-in S1/S7/TC-385/TC-387 locations confirmed under RUSTFLAGS=--cfg seam_probe \
         (qsl-semantics, then quire-spec-language); normal build has none. `stage_hooks`'s former \
         second S1 location is deleted (PR #262 review F7). S2/S3 (QSL-143) and S4 (no \
         cause-bearing family yet) are not covered by this checked-in list.\n",
        checked_in.len()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Records the qualified name of every function that holds a `match`
    /// expression in one parsed file (the naming [`EnclosingItem`] uses).
    #[derive(Default)]
    struct FunctionsWithMatch {
        current_impl_self: Option<String>,
        current_fn: Vec<String>,
        found: BTreeSet<String>,
    }

    impl<'ast> Visit<'ast> for FunctionsWithMatch {
        fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
            let previous = self
                .current_impl_self
                .replace(crate::impl_self_name(&node.self_ty));
            syn::visit::visit_item_impl(self, node);
            self.current_impl_self = previous;
        }

        fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
            self.current_fn.push(node.sig.ident.to_string());
            syn::visit::visit_item_fn(self, node);
            self.current_fn.pop();
        }

        fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
            let name = match &self.current_impl_self {
                Some(self_ty) => format!("{self_ty}::{}", node.sig.ident),
                None => node.sig.ident.to_string(),
            };
            self.current_fn.push(name);
            syn::visit::visit_impl_item_fn(self, node);
            self.current_fn.pop();
        }

        fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
            if let Some(name) = self.current_fn.last() {
                self.found.insert(name.clone());
            }
            syn::visit::visit_expr_match(self, node);
        }
    }

    /// Every checked-in location names a function that exists in its file
    /// and holds a `match`, so a renamed, moved or deleted seam fails here
    /// as well as in the probe build.
    #[test]
    fn checked_in_locations_name_functions_that_hold_a_match() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask lives one level below the workspace root");
        for location in checked_in_locations() {
            let source = fs::read_to_string(workspace_root.join(&location.file))
                .unwrap_or_else(|error| panic!("{}: {error}", location.file));
            let parsed = syn::parse_file(&source)
                .unwrap_or_else(|error| panic!("{}: {error}", location.file));
            let mut visitor = FunctionsWithMatch::default();
            visitor.visit_file(&parsed);
            assert!(
                visitor.found.contains(&location.item),
                "{location:?} names no function holding a match; functions with one: {:?}",
                visitor.found
            );
        }
    }

    /// F14: the checked-in key is the enclosing item, not a line number --
    /// resolving a line elsewhere inside the same function's body (not just
    /// its `match` keyword's own line) must still name that function.
    ///
    /// **Untagged (PR #262 review, coordinator round 3, finding 6).** This
    /// exercises `enclosing_item_name`, the F14 line-to-item-name helper --
    /// a real mechanism test, but not AC-6's subject (the checked-in list's
    /// category coverage), so it carries no `FR-063-AC-6` tag.
    #[test]
    fn enclosing_item_name_resolves_any_line_inside_the_function_body() {
        let source = r#"
            struct FamilyKind;
            impl FamilyKind {
                fn catalog_code_prefix(self) -> &'static str {
                    match self {
                        _ => "value",
                    }
                }
            }
        "#;
        // Line 5 is the `match` keyword's own line; line 6 is one line
        // into its body -- both resolve to the same enclosing method.
        assert_eq!(
            enclosing_item_name(source, 5).as_deref(),
            Some("FamilyKind::catalog_code_prefix")
        );
        assert_eq!(
            enclosing_item_name(source, 6).as_deref(),
            Some("FamilyKind::catalog_code_prefix")
        );
    }

    /// F14: a bare module-level function (no enclosing `impl`) resolves to
    /// its own unqualified name.
    ///
    /// **Untagged (PR #262 review, coordinator round 3, finding 6).** Same
    /// reason as `enclosing_item_name_resolves_any_line_inside_the_function_
    /// body` above: exercises the F14 helper, not AC-6's own subject.
    #[test]
    fn enclosing_item_name_resolves_a_bare_function() {
        let source = "fn free_function() {\n    let x = 1;\n}\n";
        assert_eq!(
            enclosing_item_name(source, 2).as_deref(),
            Some("free_function")
        );
    }

    /// F15: cargo's own `--offline` dependency-resolution failure is
    /// distinguished from a genuine compile failure by the literal `--offline`
    /// substring it always includes; ordinary rustc compile-error stderr
    /// does not contain that flag at all.
    #[test]
    fn offline_registry_unavailable_detects_cargos_own_offline_error() {
        assert!(offline_registry_unavailable(
            "error: failed to get `serde` as a dependency of package `xtask`\n\n\
             Caused by:\n  attempting to make an HTTP request, but --offline was specified\n"
        ));
        assert!(!offline_registry_unavailable(
            "error[E0308]: mismatched types\n --> src/lib.rs:1:1\n"
        ));
    }
}
