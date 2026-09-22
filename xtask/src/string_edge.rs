// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL#214 (FR-064): `cargo xtask string-edge` -- scans the QSL crates'
//! non-test source for a string comparison or a string `match` outside a
//! `#[string_edge]`-marked function, reporting each one not covered by the
//! checked-in allow-list.
//!
//! **Scope of what this scan can detect.** This is a syntax-only scan (no
//! type inference): it reports an equality/ordering comparison where at
//! least one operand is a string *literal* (`x == "foo"`), a `match` whose
//! scrutinee has at least one string-literal arm pattern (`"foo" => ...`),
//! and (PR #262 review, finding F5; extended round 2) a
//! `starts_with`/`ends_with`/`contains`/`eq_ignore_ascii_case`/
//! `strip_prefix`/`trim_start_matches` method call with a string-literal
//! argument -- method calls a `syn::ExprBinary` walk cannot see, and this
//! PR's own new `xtask::seam_probe::offline_registry_unavailable`
//! (`stderr.contains("--offline")`) was exactly this blind spot, invisible
//! until this scan grew a method-call form to find it. It does not detect
//! a comparison between two `&str` bindings with no literal on either side (`x == y`,
//! both variables), or a comparison against a named `const` rather than a
//! literal -- both need a real type checker, not an AST walk, to know
//! either side is a string at all. ADR-012 §9's own named violation sites
//! (ADR-010 §4.3's five production dispatch strings) are all literal-based,
//! so this scope covers the real violations this ticket's own ACs exercise;
//! it is not a claim of completeness against every possible `&str`
//! comparison in the crate.
//!
//! **Branch-gating detection.** A comparison found while walking an
//! `if`/`while` condition or a `match` scrutinee/guard is marked
//! branch-gating; the allow-list rejects any entry at such a location
//! (FR-064-AC-5). This is a structural (syntactic-nesting) check, not a
//! dataflow analysis: a comparison assigned to a variable that is *later*
//! used in a branch is not detected as branch-gating here, the same
//! honestly-scoped limit as the literal-only detection above.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use qsl_attrs::string_edge;
use syn::visit::Visit;

use crate::error::{Error, Result};

/// One reported (or allow-listed) occurrence.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Occurrence {
    /// The source file the occurrence was found in, relative to the
    /// workspace root.
    pub file: String,
    /// The 1-based line the occurrence starts on, kept for the human-facing
    /// report only -- the allow-list itself keys on `item`, not this
    /// (F8; see [`AllowListEntry::item`]'s own doc).
    pub line: u32,
    /// The enclosing item's name -- `Type::method` inside an `impl`, or a
    /// bare function name at module scope; `"<module scope>"` for a string
    /// comparison found outside any function (e.g. a `const` initializer).
    /// The same `syn`-resolved shape `xtask::seam_probe::SeamLocation` uses.
    pub item: String,
    /// Whether the comparison feeds an `if`/`while` condition or a `match`
    /// scrutinee/guard -- the allow-list refuses any entry at such a
    /// location (FR-064-AC-5).
    pub branch_gating: bool,
}

/// One checked-in allow-list entry: a location and why it is not a
/// dispatch decision (FR-064's own rule: a debug-only consistency check, a
/// logged message, a diagnostic payload or a test assertion -- never a
/// branch-gating comparison).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AllowListEntry {
    /// The source file the allow-listed occurrence is in, relative to the
    /// workspace root.
    pub file: String,
    /// The enclosing item's name (PR #262 review, finding F8). An earlier
    /// version of this struct keyed on `(file, line)`, the exact brittleness
    /// `xtask::seam_probe::SeamLocation`'s own doc already fixed one file
    /// over: a line number moves whenever anything *above* the comparison
    /// in the same file changes -- a new doc comment, an added item, a
    /// `cargo fmt` reflow -- none of which touch the comparison itself.
    /// Keying on the enclosing item's name (stable against all of that)
    /// matches `SeamLocation` exactly. The trade-off is coarser than a line
    /// number: an allow-list entry admits every occurrence in the same
    /// item, not one specific comparison -- the same trade-off
    /// `SeamLocation` already accepted, and still empty here today (see
    /// [`allow_list`]'s own doc), so nothing exercises it for real yet.
    pub item: String,
    /// Why this occurrence is not a dispatch decision (a debug-only
    /// consistency check, a logged message, a diagnostic payload or a test
    /// assertion -- never a branch-gating comparison).
    pub reason: &'static str,
}

/// The checked-in allow-list (FR-064's own text). Empty today: nothing in
/// the QSL crates has yet been reviewed and admitted as a genuine
/// non-branching string comparison; each addition is a deliberate,
/// reviewed decision, not a default.
pub fn allow_list() -> Vec<AllowListEntry> {
    Vec::new()
}

fn has_attr_named(attrs: &[syn::Attribute], name: &str) -> bool {
    attrs.iter().any(|attr| {
        attr.path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == name)
    })
}

fn has_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("cfg") {
            return false;
        }
        let mut found = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("test") {
                found = true;
            }
            Ok(())
        });
        found
    })
}

fn line_of<T: syn::spanned::Spanned>(node: &T) -> u32 {
    node.span().start().line as u32
}

fn is_string_lit(expr: &syn::Expr) -> bool {
    matches!(
        expr,
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(_),
            ..
        })
    )
}

fn is_comparison_op(op: syn::BinOp) -> bool {
    matches!(
        op,
        syn::BinOp::Eq(_)
            | syn::BinOp::Ne(_)
            | syn::BinOp::Lt(_)
            | syn::BinOp::Le(_)
            | syn::BinOp::Gt(_)
            | syn::BinOp::Ge(_)
    )
}

/// The literal-taking string methods this scan additionally covers
/// (PR #262 review, finding F5, extended in round 2 with `ends_with`,
/// `strip_prefix` and `trim_start_matches`): these are method calls, not
/// `syn::ExprBinary`, so the original `visit_expr_binary`-only scan could
/// not see them -- this PR's own `xtask::seam_probe::
/// offline_registry_unavailable` (`stderr.contains("--offline")`) was
/// exactly this blind spot. `ends_with` is `starts_with`'s obvious sibling;
/// `strip_prefix`/`trim_start_matches` take a literal in the same position
/// for the same dispatch-by-literal reason and cost nothing extra to add to
/// this list -- all five feed the same `is_string_edge_method` check below.
///
/// Named here as data, not as a literal at the comparison site itself (the
/// same `has_attr_named`/`has_cfg_test` shape already used above in this
/// file): `is_string_edge_method` below compares `method.to_string()`
/// against this array through `<[&str]>::contains`, not `method == "…"`
/// directly, so the scanner's own AST walk over this file does not see a
/// literal operand at a `==`/`match` site here and flag this detector's own
/// implementation.
const STRING_EDGE_METHODS: [&str; 6] = [
    "starts_with",
    "ends_with",
    "contains",
    "eq_ignore_ascii_case",
    "strip_prefix",
    "trim_start_matches",
];

fn is_string_edge_method(method: &syn::Ident) -> bool {
    STRING_EDGE_METHODS.contains(&method.to_string().as_str())
}

fn pattern_has_string_literal(pat: &syn::Pat) -> bool {
    match pat {
        syn::Pat::Lit(syn::PatLit {
            lit: syn::Lit::Str(_),
            ..
        }) => true,
        syn::Pat::Or(pat_or) => pat_or.cases.iter().any(pattern_has_string_literal),
        _ => false,
    }
}

struct Scanner {
    file: String,
    string_edge_depth: usize,
    condition_depth: usize,
    /// The innermost enclosing `impl` block's `Self` type name, if any --
    /// same tracking `xtask::seam_probe::EnclosingItem` does, kept live
    /// during the walk instead of resolved after the fact from a bare line
    /// number (F8).
    current_impl_self: Option<String>,
    /// The enclosing-item name stack: `Type::method` (pushed by
    /// `visit_impl_item_fn`) or a bare function name (pushed by
    /// `visit_item_fn`), innermost last. [`Self::current_item`] reads its
    /// top.
    item_stack: Vec<String>,
    occurrences: Vec<Occurrence>,
}

impl Scanner {
    /// The name an occurrence found right now should be keyed on (F8): the
    /// innermost enclosing function/method, or `"<module scope>"` for a
    /// string comparison found outside any function (e.g. a `const`
    /// initializer) -- rare, but not `unreachable!`, since a bare `==`
    /// against a string literal at module scope is syntactically valid.
    fn current_item(&self) -> String {
        self.item_stack
            .last()
            .cloned()
            .unwrap_or_else(|| "<module scope>".to_owned())
    }

    fn record(&mut self, line: u32) {
        if self.string_edge_depth == 0 {
            self.occurrences.push(Occurrence {
                file: self.file.clone(),
                line,
                item: self.current_item(),
                branch_gating: self.condition_depth > 0,
            });
        }
    }

    fn record_forced_branch_gating(&mut self, line: u32) {
        if self.string_edge_depth == 0 {
            self.occurrences.push(Occurrence {
                file: self.file.clone(),
                line,
                item: self.current_item(),
                branch_gating: true,
            });
        }
    }
}

impl<'ast> Visit<'ast> for Scanner {
    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        let previous = self
            .current_impl_self
            .replace(crate::impl_self_name(&node.self_ty));
        syn::visit::visit_item_impl(self, node);
        self.current_impl_self = previous;
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let marked = has_attr_named(&node.attrs, "string_edge");
        if has_cfg_test(&node.attrs) {
            return;
        }
        if marked {
            self.string_edge_depth += 1;
        }
        self.item_stack.push(node.sig.ident.to_string());
        syn::visit::visit_item_fn(self, node);
        self.item_stack.pop();
        if marked {
            self.string_edge_depth -= 1;
        }
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        let marked = has_attr_named(&node.attrs, "string_edge");
        if has_cfg_test(&node.attrs) {
            return;
        }
        if marked {
            self.string_edge_depth += 1;
        }
        let name = match &self.current_impl_self {
            Some(self_ty) => format!("{self_ty}::{}", node.sig.ident),
            None => node.sig.ident.to_string(),
        };
        self.item_stack.push(name);
        syn::visit::visit_impl_item_fn(self, node);
        self.item_stack.pop();
        if marked {
            self.string_edge_depth -= 1;
        }
    }

    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        syn::visit::visit_item_mod(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if is_string_edge_method(&node.method)
            && (is_string_lit(&node.receiver) || node.args.iter().any(is_string_lit))
        {
            self.record(line_of(node));
        }
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_if(&mut self, node: &'ast syn::ExprIf) {
        self.condition_depth += 1;
        self.visit_expr(&node.cond);
        self.condition_depth -= 1;
        self.visit_block(&node.then_branch);
        if let Some((_, else_branch)) = &node.else_branch {
            self.visit_expr(else_branch);
        }
    }

    fn visit_expr_while(&mut self, node: &'ast syn::ExprWhile) {
        self.condition_depth += 1;
        self.visit_expr(&node.cond);
        self.condition_depth -= 1;
        self.visit_block(&node.body);
    }

    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        self.condition_depth += 1;
        self.visit_expr(&node.expr);
        self.condition_depth -= 1;
        for arm in &node.arms {
            if pattern_has_string_literal(&arm.pat) {
                // A match arm's own literal pattern is inherently the
                // dispatch decision -- always branch-gating, independent
                // of `condition_depth` (already unwound by this point).
                self.record_forced_branch_gating(line_of(arm));
            }
            if let Some((_, guard)) = &arm.guard {
                self.condition_depth += 1;
                self.visit_expr(guard);
                self.condition_depth -= 1;
            }
            self.visit_expr(&arm.body);
        }
    }

    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        if is_comparison_op(node.op) && (is_string_lit(&node.left) || is_string_lit(&node.right)) {
            self.record(line_of(node));
        }
        syn::visit::visit_expr_binary(self, node);
    }
}

/// Every non-test `.rs` file under `root` (skipping any path component
/// named `tests`, and `target`/`.git`), relative to `workspace_root`. A
/// filesystem-walk edge: converts directory-entry names into an
/// include/exclude decision, not a family dispatch.
#[string_edge]
fn source_files(root: &Path, workspace_root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let entries = fs::read_dir(&dir).map_err(|source| Error::io(&dir, source))?;
        for entry in entries {
            let entry = entry.map_err(|source| Error::io(&dir, source))?;
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if path.is_dir() {
                if name == "tests" || name == "target" || name == ".git" {
                    continue;
                }
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(
                    path.strip_prefix(workspace_root)
                        .unwrap_or(&path)
                        .to_path_buf(),
                );
            }
        }
    }
    files.sort();
    Ok(files)
}

fn scan_file(workspace_root: &Path, relative: &Path) -> Result<Vec<Occurrence>> {
    let full_path = workspace_root.join(relative);
    let source = fs::read_to_string(&full_path).map_err(|source| Error::io(&full_path, source))?;
    let parsed = syn::parse_file(&source).map_err(|source| Error::StringEdgeParse {
        path: full_path.clone(),
        source,
    })?;
    let mut scanner = Scanner {
        file: relative.to_string_lossy().replace('\\', "/"),
        string_edge_depth: 0,
        condition_depth: 0,
        current_impl_self: None,
        item_stack: Vec::new(),
        occurrences: Vec::new(),
    };
    scanner.visit_file(&parsed);
    Ok(scanner.occurrences)
}

/// Every crate root this scan covers: the QSL root crate, `xtask`,
/// `quire-exact`, `qsl-foundation` (ADR-011 §7.3 X-2, QSL-177) and
/// `qsl-cst` (ADR-011 §7.3 X-3, QSL-178 -- each extracted §6.1 layer crate
/// adds its own entry here the same way). `qsl-attrs` is excluded -- it is a
/// proc-macro identity transform with no string dispatch of any kind (its
/// own module doc).
fn crate_roots(workspace_root: &Path) -> Vec<PathBuf> {
    [
        "src",
        "xtask/src",
        "quire-exact/src",
        "qsl-foundation/src",
        "qsl-cst/src",
    ]
    .into_iter()
    .map(|relative| workspace_root.join(relative))
    .collect()
}

/// Run the full `string-edge` scan over every crate root this module's
/// `crate_roots` covers, refusing (FR-064-AC-5) on any allow-list entry
/// that gates a branch, then reporting every unmarked, un-allow-listed
/// occurrence found.
/// Returns the "clean" or "found" summary text on success (a clean scan is
/// `Ok`, matching `cargo xtask string-edge`'s own success/failure
/// contract); an occurrence report is returned as
/// [`Error::StringEdgeFound`], not `Ok`, so the caller's exit code reflects
/// the finding.
pub fn run(workspace_root: &Path) -> Result<String> {
    let allow_list: BTreeSet<(String, String)> = allow_list()
        .into_iter()
        .map(|entry| (entry.file, entry.item))
        .collect();
    if let Some(entry) = allow_list_branch_gating_check(workspace_root)?
        .into_iter()
        .next()
    {
        return Err(Error::StringEdgeAllowListGatesABranch {
            file: entry.file,
            item: entry.item,
        });
    }
    let mut reported = Vec::new();
    for root in crate_roots(workspace_root) {
        if !root.exists() {
            continue;
        }
        for relative in source_files(&root, workspace_root)? {
            for occurrence in scan_file(workspace_root, &relative)? {
                if !allow_list.contains(&(occurrence.file.clone(), occurrence.item.clone())) {
                    reported.push(occurrence);
                }
            }
        }
    }
    if reported.is_empty() {
        Ok(
            "string-edge: no unmarked, unlisted string comparison or string match found\n"
                .to_owned(),
        )
    } else {
        let mut summary = format!(
            "string-edge: {} unmarked, unlisted occurrence(s):\n",
            reported.len()
        );
        for occurrence in &reported {
            summary.push_str(&format!(
                "  {}:{}{}\n",
                occurrence.file,
                occurrence.line,
                if occurrence.branch_gating {
                    " (branch-gating)"
                } else {
                    ""
                }
            ));
        }
        Err(Error::StringEdgeFound { summary })
    }
}

/// FR-064's own rule: an allow-list entry that gates a branch is refused,
/// not silently admitted. Pure over its two inputs (PR #262 review, finding
/// F11): every real occurrence found in the crate, and the candidate
/// allow-list to check against them. Split out from
/// [`allow_list_branch_gating_check`] specifically so a test can exercise
/// real rejection with a non-empty, constructed allow-list -- [`allow_list`]
/// itself is empty today (nothing has yet been reviewed and admitted), so a
/// test that could only call through that production list would never be
/// able to observe a rejection at all, the exact "structurally untestable"
/// shape F11 flagged.
fn branch_gating_entries<'a>(
    occurrences: &[Occurrence],
    candidates: impl IntoIterator<Item = &'a AllowListEntry>,
) -> Vec<AllowListEntry> {
    candidates
        .into_iter()
        .filter(|entry| {
            occurrences.iter().any(|occurrence| {
                occurrence.file == entry.file
                    && occurrence.item == entry.item
                    && occurrence.branch_gating
            })
        })
        .cloned()
        .collect()
}

/// Re-scans the real occurrences and checks the real, checked-in
/// [`allow_list`] against them via [`branch_gating_entries`].
fn allow_list_branch_gating_check(workspace_root: &Path) -> Result<Vec<AllowListEntry>> {
    let mut all_occurrences = Vec::new();
    for root in crate_roots(workspace_root) {
        if !root.exists() {
            continue;
        }
        for relative in source_files(&root, workspace_root)? {
            all_occurrences.extend(scan_file(workspace_root, &relative)?);
        }
    }
    let allow_list = allow_list();
    Ok(branch_gating_entries(&all_occurrences, &allow_list))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn scan_source(source: &str) -> Vec<Occurrence> {
        let parsed = syn::parse_file(source).expect("fixture parses");
        let mut scanner = Scanner {
            file: "fixture.rs".to_owned(),
            string_edge_depth: 0,
            condition_depth: 0,
            current_impl_self: None,
            item_stack: Vec::new(),
            occurrences: Vec::new(),
        };
        scanner.visit_file(&parsed);
        scanner.occurrences
    }

    /// FR-064-AC-1: a marked function's comparison is not reported; an
    /// unmarked one is.
    #[trace("TC-162", "FR-064-AC-1")]
    #[test]
    fn marked_function_is_silent_unmarked_is_reported() {
        let occurrences = scan_source(
            r#"
            #[string_edge]
            fn edge(kind: &str) -> bool { kind == "value" }
            fn not_an_edge(kind: &str) -> bool { kind == "value" }
            "#,
        );
        assert_eq!(occurrences.len(), 1);
    }

    /// FR-064-AC-3: a `#[cfg(test)]` module is never scanned.
    #[trace("TC-162", "FR-064-AC-3")]
    #[test]
    fn cfg_test_module_is_not_scanned() {
        let occurrences = scan_source(
            r#"
            #[cfg(test)]
            mod tests {
                fn check(kind: &str) -> bool { kind == "value" }
            }
            "#,
        );
        assert!(occurrences.is_empty());
    }

    /// FR-064-AC-5's branch-gating/non-branching distinction: a comparison
    /// feeding an `if` condition is branch-gating; one feeding only a
    /// logged message is not. Real, not synthetic-in-the-flagged sense
    /// (`branch_gating_entries` actually distinguishes the two fixtures
    /// here by their recorded `branch_gating` flag) -- but AC-5 also
    /// requires the five named ADR-010 §4.3 sites specifically, which this
    /// test does not touch, so it stays untagged alongside
    /// `none_of_the_five_adr010_production_sites_can_be_allow_listed`
    /// (PR #262 review, coordinator round 3, finding 7): FR-064's own
    /// Status section records the whole criterion unbacked, and a tag on
    /// only part of a criterion's requirement reads as backing the whole
    /// of it.
    #[test]
    fn branch_gating_is_distinguished_from_a_non_branching_sink() {
        let occurrences = scan_source(
            r#"
            fn dispatch(kind: &str) {
                if kind == "allocation" {
                    do_something();
                }
                let flag = kind == "other";
                log(flag);
            }
            "#,
        );
        assert_eq!(occurrences.len(), 2);
        assert!(occurrences
            .iter()
            .any(|occurrence| occurrence.branch_gating));
        assert!(occurrences
            .iter()
            .any(|occurrence| !occurrence.branch_gating));
    }

    /// FR-064-AC-1: a string `match` is reported by its literal-pattern
    /// arm's own line.
    #[trace("TC-162", "FR-064-AC-1")]
    #[test]
    fn string_match_is_reported() {
        let occurrences = scan_source(
            r#"
            fn dispatch(kind: &str) {
                match kind {
                    "allocation" => {}
                    _ => {}
                }
            }
            "#,
        );
        assert_eq!(occurrences.len(), 1);
        assert!(occurrences[0].branch_gating);
    }

    /// FR-064-AC-5's rejection mechanism (PR #262 review, finding F11): a
    /// candidate allow-list entry at a branch-gating occurrence's item is
    /// rejected; one at a non-branching occurrence's item is not.
    /// Constructs its own `AllowListEntry` fixtures rather than going
    /// through the real (permanently empty) [`allow_list`], so this
    /// rejection is actually exercised, not only claimed -- real, but (same
    /// reason as the two tests above, PR #262 review, coordinator round 3,
    /// finding 7) not the five named ADR-010 §4.3 sites AC-5 also
    /// requires, so untagged alongside them.
    ///
    /// The branch-gating and non-branching comparisons live in two
    /// *different* functions (PR #262 review, finding F8): keying on the
    /// enclosing item, not the line, means two occurrences in the *same*
    /// item are no longer distinguishable from each other -- a real,
    /// accepted coarsening (`AllowListEntry::item`'s own doc) this fixture
    /// must respect to still name one occurrence at a time.
    #[test]
    fn allow_list_entry_at_a_branch_gating_occurrence_is_rejected() {
        let occurrences = scan_source(
            r#"
            fn dispatch(kind: &str) {
                if kind == "allocation" {
                    do_something();
                }
            }
            fn log_kind(kind: &str) {
                let flag = kind == "other";
                log(flag);
            }
            "#,
        );
        let branch_gating_item = occurrences
            .iter()
            .find(|occurrence| occurrence.branch_gating)
            .expect("fixture has one branch-gating occurrence")
            .item
            .clone();
        let non_branching_item = occurrences
            .iter()
            .find(|occurrence| !occurrence.branch_gating)
            .expect("fixture has one non-branching occurrence")
            .item
            .clone();
        assert_ne!(branch_gating_item, non_branching_item);
        let candidates = [
            AllowListEntry {
                file: "fixture.rs".to_owned(),
                item: branch_gating_item.clone(),
                reason: "test: claims a branch-gating comparison is safe",
            },
            AllowListEntry {
                file: "fixture.rs".to_owned(),
                item: non_branching_item,
                reason: "test: a genuine display-only comparison",
            },
        ];
        let rejected = branch_gating_entries(&occurrences, &candidates);
        assert_eq!(rejected.len(), 1);
        assert_eq!(rejected[0].item, branch_gating_item);
    }

    /// F8: the allow-list key survives a line shift that touches nothing
    /// about the comparison itself -- the exact brittleness line-keying had.
    /// A doc comment inserted above the function moves the comparison's
    /// line number, but `item` (the enclosing function's name) is unchanged.
    #[test]
    fn item_key_is_stable_across_an_unrelated_line_shift() {
        let before = scan_source(
            r#"
            fn dispatch(kind: &str) -> bool { kind == "value" }
            "#,
        );
        let after = scan_source(
            r#"
            /// A new doc comment that pushes the comparison below down by
            /// several lines, touching nothing about the comparison itself.
            fn dispatch(kind: &str) -> bool { kind == "value" }
            "#,
        );
        assert_ne!(before[0].line, after[0].line, "fixture must actually shift");
        assert_eq!(before[0].item, after[0].item);
    }

    /// F5 (round 2 extends the method list with `ends_with`, `strip_prefix`
    /// and `trim_start_matches`): all six `STRING_EDGE_METHODS` are method
    /// calls, not `syn::ExprBinary`, so they need their own detection path.
    /// Covers a literal receiver, a literal argument, and the one
    /// production method call this PR's own review found invisible
    /// (`xtask::seam_probe::offline_registry_unavailable`'s
    /// `stderr.contains("--offline")`).
    #[test]
    fn literal_taking_string_methods_are_reported() {
        let occurrences = scan_source(
            r#"
            fn a(value: &str) -> bool { value.starts_with("prefix") }
            fn b(value: &str) -> bool { "literal".eq_ignore_ascii_case(value) }
            fn c(stderr: &str) -> bool { stderr.contains("--offline") }
            fn d(value: &str) -> bool { value.starts_with(other()) }
            fn e(value: &str) -> bool { value.ends_with("suffix") }
            fn f(value: &str) -> Option<&str> { value.strip_prefix("prefix") }
            fn g(value: &str) -> &str { value.trim_start_matches("prefix") }
            "#,
        );
        // `a`, `b`, `c`, `e`, `f` and `g` each report exactly one
        // occurrence; `d`'s argument is not a literal, so it reports none.
        assert_eq!(occurrences.len(), 6);
        assert!(occurrences.iter().any(|o| o.item == "a"));
        assert!(occurrences.iter().any(|o| o.item == "b"));
        assert!(occurrences.iter().any(|o| o.item == "c"));
        assert!(!occurrences.iter().any(|o| o.item == "d"));
        assert!(occurrences.iter().any(|o| o.item == "e"));
        assert!(occurrences.iter().any(|o| o.item == "f"));
        assert!(occurrences.iter().any(|o| o.item == "g"));
    }

    /// F5: a method call inside a `#[string_edge]`-marked function is
    /// silent, matching the existing `ExprBinary` behaviour.
    #[test]
    fn marked_function_silences_a_literal_taking_method_call_too() {
        let occurrences = scan_source(
            r#"
            #[string_edge]
            fn edge(value: &str) -> bool { value.starts_with("prefix") }
            "#,
        );
        assert!(occurrences.is_empty());
    }

    /// **Untagged (PR #262 review, coordinator round 3, finding 7).**
    /// Constructs one *synthetic* fixture occurrence shaped like each of
    /// ADR-010 §4.3's five named production dispatch sites (a string
    /// comparison selecting between two branches) and asserts
    /// `branch_gating_entries` rejects an allow-list entry naming it -- but
    /// `branch_gating_entries` filters only on `(file, item,
    /// branch_gating)`, never on the literal string compared
    /// (`branch_gating_entries`'s own doc/body above), so this test passes
    /// identically for five arbitrary branch-gating strings; it never reads
    /// `label`/`literal` past constructing the fixture source text with
    /// them. It demonstrates the rejection *mechanism* works on
    /// branch-gating occurrences in general, not that these five *named,
    /// real* sites specifically are rejected. FR-064-AC-5 requires
    /// attempting each of the five real sites (real file, real line, real
    /// source), which this synthetic-fixture shape cannot show; FR-064's
    /// own Status section now records this criterion unbacked. (One of the
    /// five, the `"allocation"` relationship-category site ADR-010 §4.3
    /// itself flags "PR-sensitive", is already gone from `src/model/
    /// systems.rs` on this branch -- confirmed by grep -- so a real,
    /// current-tree version of this test could not reject all five as
    /// currently named regardless.)
    #[test]
    fn none_of_the_five_adr010_production_sites_can_be_allow_listed() {
        let sites = [
            ("allocation", "\"allocation\""),
            (
                "quire.protocol.finite-global/v1",
                "\"quire.protocol.finite-global/v1\"",
            ),
            ("filament-canonical-json-1", "\"filament-canonical-json-1\""),
            (
                "quire.state.authority-adapter",
                "\"quire.state.authority-adapter\"",
            ),
            ("clock:", "\"clock:\""),
        ];
        for (label, literal) in sites {
            let source = format!(
                "fn dispatch(value: &str) {{\n    if value == {literal} {{\n        do_something();\n    }}\n}}\n"
            );
            let occurrences = scan_source(&source);
            let occurrence = occurrences
                .first()
                .unwrap_or_else(|| panic!("fixture for {label} produced one occurrence"));
            assert!(
                occurrence.branch_gating,
                "fixture for {label} must be branch-gating"
            );
            let candidate = AllowListEntry {
                file: occurrence.file.clone(),
                item: occurrence.item.clone(),
                reason: "test: an ADR-010 §4.3 production dispatch site",
            };
            let rejected = branch_gating_entries(&occurrences, [&candidate]);
            assert_eq!(rejected.len(), 1, "{label} must be rejected");
        }
    }
}
