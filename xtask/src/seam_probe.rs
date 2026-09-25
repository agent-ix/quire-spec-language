// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL#214 (FR-063): `cargo xtask seam-probe` demonstrates ADR-012 §5.1's S1
//! and S7 seams by building the QSL workspace crates that hold seams under
//! `RUSTFLAGS=--cfg seam_probe` (three normal builds and four probe builds,
//! `PROBE_BUILDS` and `NORMAL_BUILDS`) and comparing the `E0004`
//! (non-exhaustive match) locations rustc reports against a checked-in
//! list, exactly as FR-063 requires.
//!
//! **Scope: S1, S2, S3 (partial) and S7 (PR #262 review, finding F7; PR
//! #305 review, finding 6; QSL-143).** FR-063-AC-6 names five categories:
//! S1's stage-participation table and prefix arm, S2, S3, S4. S1 originally
//! had two checked-in `match`es over `FamilyKind` in `src/family/mod.rs` --
//! `catalog_code_prefix`'s prefix arm and `stage_hooks`'s
//! stage-participation table. `stage_hooks` is deleted: its only non-test
//! callers were three `assert_eq!` call sites asserting a hand-written
//! `match`'s own literal result against itself, a fabricated reader
//! manufactured to keep otherwise-dead code alive (see `crate::family`'s
//! module doc in the QSL crate). Deleting the fabricated callers left
//! `stage_hooks` itself with no real reader, so it is gone too, and with it
//! the second S1 seam this tool could check in. Only `catalog_code_prefix`'s
//! prefix arm remains.
//!
//! S2 (the parser's leading-token-kind entry table and the parsed-form-enum
//! check seam) and S3 (the checked-node-enum's evaluator, v2 emitter and
//! requirement-derivation matches) are seams over `LeadingTokenKind`,
//! `Expression` and `NodeKind` -- pre-existing, crate-wide enums every
//! `Value` form uses (literals, operators, `let`, `if`, records,
//! collections), not just the two forms QSL-25/#214 migrated (function
//! declaration and application). QSL-143 adds `Expression` and `NodeKind`'s
//! `#[cfg(seam_probe)]` probe variants and a protective arm at every other
//! production `match` site the variant would otherwise break
//! (`qsl-forms::syntax`, `qsl-semantics::check::assemble`,
//! `::checked_dispatch`, `::family`), landing one checked-in location for
//! S2 (`Typer::infer_form`, the check seam) and two for S3 (`Machine::
//! apply`, the evaluator; `Lowering::lower_node`, the identity-lowering
//! pass feeding v2 emission) -- see [`checked_in_locations`]'s own doc for
//! why the parser's leading-token-kind table itself (`qsl-forms::dispatch::
//! dispatch`) stays out of this list. S4 (each family `Cause` enum's
//! `catalog_code()`) has no cause-bearing family to demonstrate it yet --
//! see `crate::family::outcome`'s own doc in the QSL crate.
//!
//! **S7 (QSL-46/#185, ADR-012 §5.1's row: "requirement derivation per
//! family; registry advertisement check; CG `negotiate_*` capability arm").**
//! QSL owns the first two; `#[cfg(seam_probe)]` on
//! `crate::check::Capability` (defined in `capability.rs` itself, which also
//! carries its own match's probe arm and so is not a seam-probe location)
//! makes both non-exhaustive under the probe build: `requests::families`
//! (requirement derivation per family) and `qsl-route`'s `advertises_kind`
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
/// and `qsl-route`'s `advertises_kind` helper `same_kind` (registry
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
/// `qsl-eval`'s `value::expression::s6a::S6aFamilyKind` (`evaluate_declaration`), its
/// `S6aFamilyKind::family` mapping to `FamilyKind`, and the one
/// `match` over `crate::family::FamilyOutcome` (`ValueFunctionFamily::
/// evaluate`, which passes the evaluator's `FamilyOutcome` back as an
/// `EvalOutcome`). Each enum carries a `#[cfg(seam_probe)]` variant.
///
/// **S2 and S3 (QSL-143): the check seam over the parsed form enum, and the
/// checked node enum's evaluator and identity-lowering pass.** `Expression`
/// (`qsl-forms::syntax::Expression`) and `NodeKind`
/// (`qsl-semantics::check::ir::NodeKind`) each carry a `#[cfg(seam_probe)]`
/// probe variant now too:
///
/// - S2: `Typer::infer_form` (`qsl-semantics/src/check/check/typing.rs`),
///   the check-time dispatch over `Expression` that already carried
///   `#[deny(clippy::wildcard_enum_match_arm)]` before this ticket (the
///   FR-063-AC-7 lint, satisfied in advance of the probe variant that makes
///   it a real seam).
/// - S3: `Machine::apply` (`qsl-eval/src/value/expression/evaluate.rs`),
///   the evaluator's own exhaustive dispatch over `NodeKind`; and
///   `Lowering::lower_node` (`qsl-semantics/src/check/lowering.rs`, also
///   already `#[deny(clippy::wildcard_enum_match_arm)]`), the FR-093
///   identity-lowering pass whose `SemanticNode`/`SemanticTerm` output feeds
///   `qsl-package`'s v2 emission (ADR-012 §5.1 row S3's "v2 emitter").
///
/// **What is not covered, and why.** ADR-012 §5.1 row S2 also names "the
/// parser's leading-token-kind entry table" -- `qsl-forms::dispatch::
/// dispatch`'s `match` over `LeadingTokenKind`. That table lives in
/// `qsl-forms` (ADR-011 §6.1 layer 2), a dependency of every one of
/// [`PROBE_BUILDS`]'s four packages but never itself one of them: giving its
/// `match` the same "no arm under plain `seam_probe`" treatment as
/// `Typer::infer_form` would make `qsl-forms` -- and therefore every crate
/// above it -- fail to compile in *every* probe build, permanently hiding
/// `FamilyKind`'s S1 seam, the S6a/S7 seams and the two S2/S3 seams above,
/// not just this one; giving it an unconditional arm instead would make the
/// variant inert (never actually non-exhaustive anywhere), which is not a
/// seam at all. Probing it needs a build of `qsl-forms` itself, which
/// [`PROBE_BUILDS`]'s fixed four-crate list (FR-063's own Behavior text)
/// does not include; that is a spec gap for a future ticket, not something
/// this checked-in list can honestly claim today. FR-063-AC-6 requires only
/// *one* checked-in entry per category, and `Typer::infer_form` already
/// supplies S2's.
///
/// S4 (each family `Cause` enum's `catalog_code()`) still has no
/// cause-bearing family to demonstrate it (`crate::family::outcome`'s own
/// doc).
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
            file: "qsl-route/src/lib.rs".to_owned(),
            item: "same_kind".to_owned(),
        },
        SeamLocation {
            file: "qsl-eval/src/value/expression/causes.rs".to_owned(),
            item: "ProtocolClauseSnapshot::catalog_code".to_owned(),
        },
        SeamLocation {
            file: "qsl-eval/src/value/expression/mod.rs".to_owned(),
            item: "evaluate_declaration".to_owned(),
        },
        SeamLocation {
            file: "qsl-eval/src/value/expression/s6a.rs".to_owned(),
            item: "S6aFamilyKind::family".to_owned(),
        },
        SeamLocation {
            file: "qsl-eval/src/value/expression/family.rs".to_owned(),
            item: "ValueFunctionFamily::evaluate".to_owned(),
        },
        SeamLocation {
            file: "qsl-semantics/src/check/check/typing.rs".to_owned(),
            item: "Typer::infer_form".to_owned(),
        },
        SeamLocation {
            file: "qsl-eval/src/value/expression/evaluate.rs".to_owned(),
            item: "Machine::apply".to_owned(),
        },
        SeamLocation {
            file: "qsl-semantics/src/check/lowering.rs".to_owned(),
            item: "Lowering::lower_node".to_owned(),
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
/// runs `cargo build` seven times with different `RUSTFLAGS` (three plain
/// builds, then four under `--cfg seam_probe`); without a target dir of its own, it inherited whatever
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
/// are reached. `qsl-route` (QSL-184) gets a downstream build of its own:
/// the root crate names it only as a dev dependency, so building the root
/// crate's `--lib` never compiles it. `qsl-eval` (QSL-183) gets one for
/// the same reason: the root crate does not depend on it at all, so its
/// S6a seams (`value::expression`) are reached only by a build of its own.
/// When a crate above `qsl-eval` comes to depend on it, `qsl-eval`'s seams
/// need a downstream probe arm of their own, as `qsl-semantics`' have, or
/// that crate's build stops inside `qsl-eval`. Each build must fail;
/// together they must report exactly the checked-in list.
const PROBE_BUILDS: [(&str, &str); 4] = [
    ("qsl-semantics", "--cfg seam_probe"),
    (
        "quire-spec-language",
        "--cfg seam_probe --cfg seam_probe_downstream",
    ),
    ("qsl-route", "--cfg seam_probe --cfg seam_probe_downstream"),
    ("qsl-eval", "--cfg seam_probe --cfg seam_probe_downstream"),
];

/// The normal builds, with no probe cfg: the root crate, which builds every
/// crate it depends on, and `qsl-route` and `qsl-eval`, which it does not
/// (see [`PROBE_BUILDS`]).
const NORMAL_BUILDS: [&str; 3] = ["quire-spec-language", "qsl-route", "qsl-eval"];

/// One probe build's result: whether it compiled, and the `E0004`
/// locations it reported.
struct ProbeBuild {
    package: &'static str,
    succeeded: bool,
    locations: BTreeSet<SeamLocation>,
}

/// Compare every probe build's result against the checked-in list.
///
/// A probe build that compiled is an error, but not an early one: the
/// comparison runs over every build first, so the error names the package
/// that compiled and the checked-in locations no build reported
/// (FR-063-AC-2, TC-161 step 4). `qsl-semantics` and `qsl-route` each hold
/// one seam, so adding a probe arm to that seam makes the whole build
/// compile, and the missing location is the only thing that says which seam
/// it was.
fn compare_probe_builds(builds: &[ProbeBuild], checked_in: &BTreeSet<SeamLocation>) -> Result<()> {
    let reported: BTreeSet<SeamLocation> = builds
        .iter()
        .flat_map(|build| build.locations.iter().cloned())
        .collect();
    let unexpected_but_present: Vec<_> = reported.difference(checked_in).cloned().collect();
    let expected_but_missing: Vec<_> = checked_in.difference(&reported).cloned().collect();
    let compiled: Vec<&str> = builds
        .iter()
        .filter(|build| build.succeeded)
        .map(|build| build.package)
        .collect();
    if !compiled.is_empty() {
        return Err(Error::SeamProbeBuildUnexpectedlySucceeded {
            packages: compiled.join(", "),
            expected_but_missing: format!("{expected_but_missing:?}"),
        });
    }
    if !unexpected_but_present.is_empty() || !expected_but_missing.is_empty() {
        return Err(Error::SeamProbeMismatch {
            unexpected_but_present: format!("{unexpected_but_present:?}"),
            expected_but_missing: format!("{expected_but_missing:?}"),
        });
    }
    Ok(())
}

/// `cargo xtask seam-probe`: FR-063's two required assertions (probe builds
/// that fail with exactly the checked-in `E0004` locations, and a normal
/// build that succeeds with none), then the checked-in-list comparison
/// itself.
pub fn run(workspace_root: &Path) -> Result<String> {
    // FR-063 constraint: assert the probe build fails AND the normal build
    // succeeds -- one without the other is half a test. If `--cfg
    // seam_probe` silently stopped being applied, only checking the normal
    // build would never notice.
    for package in NORMAL_BUILDS {
        let (normal_succeeded, normal_locations, normal_stderr) =
            build_and_collect_e0004(workspace_root, package, "")?;
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
    }
    let mut probe_results = Vec::new();
    for (package, rustflags) in PROBE_BUILDS {
        let (probe_succeeded, locations, probe_stderr) =
            build_and_collect_e0004(workspace_root, package, rustflags)?;
        if !probe_succeeded && locations.is_empty() && offline_registry_unavailable(&probe_stderr) {
            return Err(Error::SeamProbeOfflineRegistryUnavailable {
                stderr: probe_stderr,
            });
        }
        probe_results.push(ProbeBuild {
            package,
            succeeded: probe_succeeded,
            locations,
        });
    }
    let checked_in = checked_in_locations();
    compare_probe_builds(&probe_results, &checked_in)?;
    Ok(format!(
        "seam-probe: {} checked-in S1/S2/S3/S7/TC-385/TC-387 locations confirmed under RUSTFLAGS=--cfg seam_probe \
         (qsl-semantics, then quire-spec-language, qsl-route and qsl-eval); normal builds have none. `stage_hooks`'s former \
         second S1 location is deleted (PR #262 review F7). The parser's leading-token-kind table (S2, QSL-143) and S4 (no \
         cause-bearing family yet) are not covered by this checked-in list -- see `checked_in_locations`'s own doc.\n",
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

    fn route_seam() -> SeamLocation {
        SeamLocation {
            file: "qsl-route/src/lib.rs".to_owned(),
            item: "same_kind".to_owned(),
        }
    }

    /// TC-161 step 4 (FR-063-AC-2): a probe arm added at the one seam of a
    /// single-seam build makes that build compile. The failure names the
    /// package and the seam's location as expected-but-missing, instead of
    /// stopping at the first build that compiled.
    #[test]
    #[ix_trace_rs::trace("TC-161", "FR-063-AC-2")]
    fn a_probe_build_that_compiles_names_its_package_and_missing_location() {
        let checked_in = checked_in_locations();
        let everything_but_route: BTreeSet<SeamLocation> = checked_in
            .iter()
            .filter(|location| **location != route_seam())
            .cloned()
            .collect();
        let builds = [
            ProbeBuild {
                package: "quire-spec-language",
                succeeded: false,
                locations: everything_but_route,
            },
            ProbeBuild {
                package: "qsl-route",
                succeeded: true,
                locations: BTreeSet::new(),
            },
        ];
        let message = compare_probe_builds(&builds, &checked_in)
            .expect_err("a probe build that compiled fails the gate")
            .to_string();
        assert!(
            message.contains("the probe build of qsl-route succeeded"),
            "{message}"
        );
        assert!(
            message.contains(r#"expected-but-missing: [SeamLocation { file: "qsl-route/src/lib.rs", item: "same_kind" }]"#),
            "{message}"
        );
    }

    /// Every build failing with exactly the checked-in locations passes,
    /// and a location missing from a failing build is a mismatch.
    #[test]
    #[ix_trace_rs::trace("TC-161", "FR-063-AC-1")]
    fn failing_builds_compare_their_union_against_the_checked_in_list() {
        let checked_in = checked_in_locations();
        let (route, rest): (BTreeSet<_>, BTreeSet<_>) = checked_in
            .iter()
            .cloned()
            .partition(|location| *location == route_seam());
        let builds = [
            ProbeBuild {
                package: "quire-spec-language",
                succeeded: false,
                locations: rest.clone(),
            },
            ProbeBuild {
                package: "qsl-route",
                succeeded: false,
                locations: route,
            },
        ];
        compare_probe_builds(&builds, &checked_in).expect("the union equals the list");
        let missing = [ProbeBuild {
            package: "quire-spec-language",
            succeeded: false,
            locations: rest,
        }];
        let message = compare_probe_builds(&missing, &checked_in)
            .expect_err("a checked-in location no build reported")
            .to_string();
        assert!(message.contains("qsl-route/src/lib.rs"), "{message}");
    }

    /// FR-063-AC-4: `xtask seam-probe` exits non-zero when the checked-in
    /// list and the actual `E0004` location set differ, and only zero when
    /// they are equal. `compare_probe_builds` is the function `run` (the
    /// CLI's own entry point) returns straight through as its `Result`, and
    /// `main` maps `Err` to `error.exit_code()` (non-zero for every
    /// `Code::SeamProbe` variant) and `Ok` to `ExitCode::SUCCESS` -- so this
    /// test exercises the exact function the exit code is derived from, with
    /// a concrete "one entry removed" checked-in list (this criterion's own
    /// text), not only an assertion that the tool "checks" the list.
    #[ix_trace_rs::trace("TC-161", "FR-063-AC-4")]
    #[test]
    fn a_wrong_checked_in_list_maps_to_a_non_zero_exit_code() {
        let checked_in = checked_in_locations();
        let builds = [ProbeBuild {
            package: "quire-spec-language",
            succeeded: false,
            locations: checked_in.clone(),
        }];
        // Equal sets: zero exit (`Ok`, so `main` reaches `ExitCode::SUCCESS`
        // -- there is no `Error::exit_code()` to call at all).
        assert!(
            compare_probe_builds(&builds, &checked_in).is_ok(),
            "a build that reports exactly the checked-in set must not fail the gate"
        );
        // One entry removed from the checked-in list, with no change to
        // what the build reports (this criterion's own "concrete example"):
        // the sets now differ, so the gate must fail with a non-zero exit.
        let mut wrong_checked_in = checked_in.clone();
        wrong_checked_in.remove(&route_seam());
        let error = compare_probe_builds(&builds, &wrong_checked_in)
            .expect_err("a checked-in list missing an entry the build reports must fail the gate");
        assert_eq!(error.code(), crate::error::Code::SeamProbe);
        assert_ne!(
            error.exit_code(),
            0,
            "a checked-in/build mismatch must exit non-zero, not zero"
        );
    }

    /// FR-063-AC-3: nothing outside `xtask seam-probe`'s own build
    /// invocation can set the `seam_probe` cfg -- no `[features]` table
    /// entry, no `build.rs`, no `.cargo/config.toml` `rustflags`, and no
    /// `RUSTFLAGS`/`rustflags` setting in the Makefile or a CI workflow. A
    /// grep-shaped check over those specific files (this criterion's own
    /// text), not a proof that no code path anywhere could set the cfg: a
    /// `build.rs` added later, for instance, would need this check re-run,
    /// not exempt it from the pattern it greps for.
    #[ix_trace_rs::trace("TC-161", "FR-063-AC-3")]
    #[test]
    fn nothing_outside_xtask_seam_probe_can_set_the_seam_probe_cfg() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask lives one level below the workspace root");
        const MEMBERS: [&str; 15] = [
            ".",
            "xtask",
            "tools/arch-lint",
            "quire-exact",
            "qsl-attrs",
            "qsl-foundation",
            "qsl-cst",
            "qsl-replay",
            "qsl-source",
            "qsl-forms",
            "qsl-semantics",
            "qsl-package",
            "qsl-route",
            "qsl-eval",
            "qsl-bench",
        ];

        // No `build.rs`: the only other place a crate could inject a `--cfg`
        // outside `xtask`'s own build invocation.
        for member in MEMBERS {
            let build_rs = workspace_root.join(member).join("build.rs");
            assert!(
                !build_rs.exists(),
                "{} exists: a build.rs can set RUSTFLAGS/cfg outside xtask's own build \
                 invocation (FR-063-AC-3)",
                build_rs.display()
            );
        }

        // No `[features]` table entry named `seam_probe`/`seam-probe`: a
        // grep-shaped check over each member's own `Cargo.toml`, not a full
        // TOML parse (this criterion's own text), since a feature-table key
        // is spelled `name = [...]` at the start of a line regardless of
        // which `[features]` block it sits under.
        let feature_definition = |line: &str| -> bool {
            line.starts_with("seam_probe") || line.starts_with("seam-probe")
        };
        for member in MEMBERS {
            let manifest = workspace_root.join(member).join("Cargo.toml");
            let Ok(source) = fs::read_to_string(&manifest) else {
                continue;
            };
            for line in source.lines() {
                let trimmed = line.trim_start();
                if trimmed.starts_with('#') {
                    continue;
                }
                assert!(
                    !feature_definition(trimmed),
                    "{} defines a seam_probe/seam-probe feature (FR-063-AC-3): {line}",
                    manifest.display()
                );
            }
        }

        // `.cargo/config.toml`: no `rustflags` key at all (the only key that
        // could inject `--cfg seam_probe` process-wide).
        let cargo_config = workspace_root.join(".cargo/config.toml");
        if let Ok(source) = fs::read_to_string(&cargo_config) {
            for line in source.lines() {
                let trimmed = line.trim_start();
                if trimmed.starts_with('#') {
                    continue;
                }
                assert!(
                    !trimmed.starts_with("rustflags"),
                    "{} sets rustflags (FR-063-AC-3): {line}",
                    cargo_config.display()
                );
            }
        }

        // The Makefile: no non-comment line assigns `RUSTFLAGS` outside this
        // module's own doc/comments about it.
        let makefile = workspace_root.join("Makefile");
        let source =
            fs::read_to_string(&makefile).unwrap_or_else(|error| panic!("{makefile:?}: {error}"));
        for line in source.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                continue;
            }
            assert!(
                !trimmed.contains("RUSTFLAGS=") && !trimmed.contains("rustflags="),
                "{} sets RUSTFLAGS/rustflags outside xtask's own build invocation \
                 (FR-063-AC-3): {line}",
                makefile.display()
            );
        }

        // Every CI workflow: no non-comment line sets `RUSTFLAGS`.
        let workflows_dir = workspace_root.join(".github/workflows");
        let entries = fs::read_dir(&workflows_dir)
            .unwrap_or_else(|error| panic!("{workflows_dir:?}: {error}"));
        for entry in entries {
            let path = entry
                .unwrap_or_else(|error| panic!("{workflows_dir:?}: {error}"))
                .path();
            if path
                .extension()
                .is_none_or(|extension| extension != "yml" && extension != "yaml")
            {
                continue;
            }
            let source =
                fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path:?}: {error}"));
            for line in source.lines() {
                let trimmed = line.trim_start();
                if trimmed.starts_with('#') {
                    continue;
                }
                assert!(
                    !trimmed.contains("RUSTFLAGS"),
                    "{} sets RUSTFLAGS (FR-063-AC-3): {line}",
                    path.display()
                );
            }
        }
    }

    /// One closed enum's definition, parsed from `file`, asserting it
    /// carries no `#[non_exhaustive]` attribute (FR-063's own rule: such an
    /// attribute would let a cross-crate `match` over it accept a new
    /// variant through a forced `_` arm with no `E0004`, defeating the seam
    /// entirely).
    fn assert_enum_is_not_non_exhaustive(workspace_root: &Path, file: &str, name: &str) {
        let path = workspace_root.join(file);
        let source = fs::read_to_string(&path).unwrap_or_else(|error| panic!("{file}: {error}"));
        let parsed = syn::parse_file(&source).unwrap_or_else(|error| panic!("{file}: {error}"));
        let item_enum = parsed
            .items
            .iter()
            .find_map(|item| match item {
                syn::Item::Enum(item_enum) if item_enum.ident == name => Some(item_enum),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{file} names no enum {name}"));
        assert!(
            !item_enum
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("non_exhaustive")),
            "{file}::{name} carries #[non_exhaustive] (FR-063): a cross-crate match over it \
             could accept a new variant through a forced `_` arm with no E0004, defeating the \
             seam entirely"
        );
    }

    /// FR-063-AC-7 (second test): none of `FamilyKind`, the parsed form
    /// enum (`Expression`), the checked node enum (`NodeKind`) or a family
    /// `Cause` enum carries `#[non_exhaustive]`. `WrongSnapshotCause`
    /// (`ProtocolClauseSnapshot`'s own cause, already a checked-in S4-shaped
    /// location above) is the one cause-bearing type this repository has
    /// today; no migrated family yet has its own top-level `Cause` (this
    /// module's own doc, and `crate::family::outcome`'s), so there is no
    /// other family `Cause` enum to check yet.
    #[ix_trace_rs::trace("TC-161", "FR-063-AC-7")]
    #[test]
    fn closed_enums_carry_no_non_exhaustive_attribute() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask lives one level below the workspace root");
        for (file, name) in [
            ("qsl-semantics/src/family/mod.rs", "FamilyKind"),
            ("qsl-forms/src/syntax.rs", "Expression"),
            ("qsl-semantics/src/check/ir.rs", "NodeKind"),
            ("qsl-semantics/src/check/refusal.rs", "WrongSnapshotCause"),
        ] {
            assert_enum_is_not_non_exhaustive(workspace_root, file, name);
        }
    }

    /// FR-063-AC-7 (first test): a `_ => ...` fallback arm on a closed enum
    /// produces no `E0004` under `--cfg seam_probe` and is therefore
    /// invisible to `xtask seam-probe` alone; `cargo clippy`'s
    /// `clippy::wildcard_enum_match_arm` is the mechanism that catches it
    /// instead. Reintroduces exactly such a fallback arm in a standalone
    /// fixture crate (a temp directory, not this workspace, so it cannot be
    /// caught by inspecting this repository's own source) and asserts the
    /// lint fires and the build fails.
    #[ix_trace_rs::trace("TC-161", "FR-063-AC-7")]
    #[test]
    fn a_reintroduced_wildcard_arm_trips_the_clippy_lint() {
        let dir = tempfile::tempdir().expect("a temp dir");
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"seam-probe-wildcard-fixture\"\nversion = \"0.1.0\"\n\
             edition = \"2021\"\n\n[workspace]\n",
        )
        .expect("write a fixture Cargo.toml");
        fs::create_dir_all(dir.path().join("src")).expect("create fixture src/");
        fs::write(
            dir.path().join("src/lib.rs"),
            "#![deny(clippy::wildcard_enum_match_arm)]\n\n\
             /// Stands in for one of FR-063's S1-S4 closed enums.\n\
             pub enum Sample {\n    A,\n    B,\n    C,\n}\n\n\
             /// A `_ => unsupported(...)`-shaped fallback arm (FR-063's own \
             forbidden shape): `clippy::wildcard_enum_match_arm`, not \
             `xtask seam-probe`, must be what catches this.\n\
             pub fn unsupported(value: Sample) -> i32 {\n    match value {\n        \
             Sample::A => 1,\n        _ => 0,\n    }\n}\n",
        )
        .expect("write a fixture src/lib.rs");
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
        let output = std::process::Command::new(cargo)
            .current_dir(dir.path())
            .args(["clippy", "--offline", "--message-format=json"])
            .env("CARGO_TARGET_DIR", dir.path().join("target"))
            .output()
            .expect("cargo clippy runs");
        assert!(
            !output.status.success(),
            "a reintroduced wildcard arm must fail clippy's deny, not compile cleanly"
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout
                .lines()
                .any(|line| line.contains("clippy::wildcard_enum_match_arm")),
            "clippy::wildcard_enum_match_arm did not fire: {stdout}"
        );
    }

    /// FR-063-AC-5 (QSL-155 correction), part one: the real gate invokes
    /// `xtask seam-probe`. A grep-shaped check over the real `Makefile` --
    /// `ci:`'s own prerequisite list names `seam-probe`, and the
    /// `seam-probe:` target's own recipe runs `cargo xtask seam-probe` --
    /// not a stub of a Rust-level gate-target-list abstraction the merged
    /// spec presumed and that does not exist (QSL-155's own correction).
    #[ix_trace_rs::trace("TC-161", "FR-063-AC-5")]
    #[test]
    fn the_full_gate_invokes_seam_probe() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask lives one level below the workspace root");
        let makefile = fs::read_to_string(workspace_root.join("Makefile"))
            .expect("read the workspace Makefile");
        let ci_line = makefile
            .lines()
            .find(|line| line.starts_with("ci:"))
            .expect("Makefile has a ci: target line");
        assert!(
            ci_line
                .split_whitespace()
                .any(|prereq| prereq == "seam-probe"),
            "ci: does not name seam-probe as a prerequisite: {ci_line}"
        );
        let recipe_runs_xtask = makefile
            .lines()
            .skip_while(|line| !line.starts_with("seam-probe:"))
            .skip(1)
            .take_while(|line| line.starts_with('\t'))
            .any(|line| line.trim() == "cargo xtask seam-probe");
        assert!(
            recipe_runs_xtask,
            "the seam-probe: target's own recipe does not run `cargo xtask seam-probe`"
        );
    }

    /// FR-063-AC-5 (QSL-155 correction), part two: `xtask seam-probe`'s
    /// non-zero exit propagates to the full gate's own exit. This is
    /// `make`'s own prerequisite-failure semantics -- a `.PHONY` aggregate
    /// target depending on a target whose recipe can fail -- demonstrated
    /// on a minimal fixture `Makefile` with exactly that shape (the real
    /// `ci:`/`seam-probe:` pair, per `the_full_gate_invokes_seam_probe`
    /// above), not by running the real, several-minutes seam-probe build
    /// with a deliberately broken checked-in list.
    #[ix_trace_rs::trace("TC-161", "FR-063-AC-5")]
    #[test]
    fn a_failed_prerequisite_fails_the_aggregate_gate_target() {
        let dir = tempfile::tempdir().expect("a temp dir");
        fs::write(
            dir.path().join("Makefile"),
            ".PHONY: ci probe\nci: probe\nprobe:\n\ttest \"$$PROBE_OK\" = 1\n",
        )
        .expect("write a fixture Makefile");
        let failing = std::process::Command::new("make")
            .arg("-C")
            .arg(dir.path())
            .arg("ci")
            .env("PROBE_OK", "0")
            .output()
            .expect("make runs");
        assert!(
            !failing.status.success(),
            "a failed prerequisite must fail the aggregate gate target"
        );
        let passing = std::process::Command::new("make")
            .arg("-C")
            .arg(dir.path())
            .arg("ci")
            .env("PROBE_OK", "1")
            .output()
            .expect("make runs");
        assert!(
            passing.status.success(),
            "a succeeding prerequisite must not fail the aggregate gate target"
        );
    }
}
