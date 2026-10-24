// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL#214 (FR-064): the string-edge detector -- the one syntax-level scan for
//! a string comparison or a string `match` outside a `#[string_edge]`-marked
//! function, over the parsed source of one crate.
//!
//! **Scope of what this scan can detect.** This is a syntax-only scan (no
//! type inference): it reports an equality/ordering comparison where at
//! least one operand is a string *literal* (`x == "foo"`) or a named
//! `const NAME: &str` declared in the same crate (`x == NAME`), a `match`
//! whose arm patterns include a string literal or such a const, and a
//! `starts_with`/`ends_with`/`contains`/`eq_ignore_ascii_case`/
//! `strip_prefix`/`trim_start_matches` method call with such an operand.
//! Named consts are resolved by name within one crate (`Self::NAME` and
//! `module::NAME` resolve by their last segment), so a const shadowed by a
//! same-named non-string item can be over-reported, never hidden.
//!
//! It does not detect a comparison between two `&str` bindings with no
//! literal or named const on either side (`x == y`, both variables): that
//! needs a real type checker, not an AST walk, to know either side is a
//! string at all. Two of ADR-010 §4.3's named sites are compared against
//! literals and the others through typed conversions, so the named sites are
//! covered; this is not a claim of completeness against every possible
//! `&str` comparison in the crate.
//!
//! **Test code.** The scan skips a `tests/` directory, a `#[cfg(test)]`
//! item, and a file declared by `#[cfg(test)] mod name;` (with the files
//! below it). A file is never skipped by its name alone.
//!
//! **Branch-gating detection.** A comparison found while walking an
//! `if`/`while` condition or a `match` scrutinee/guard is marked
//! branch-gating; the allow-list rejects any entry at such a location
//! (FR-064-AC-5). This is wider: a comparison that is a term of a
//! `&&`/`||` chain, a comparison that is a match arm's own value, and a
//! `strip_prefix` call (whose `Some` is the dispatch) are also
//! branch-gating, because the combined result is what an outer branch
//! reads. Still structural, not dataflow: a comparison bound to a variable
//! that is later used in a branch is not detected as branch-gating here.

use std::collections::BTreeSet;

use syn::visit::Visit;

/// An [`Occurrence`] with whether a `#[string_edge]`-marked function
/// encloses it.
#[derive(Clone, Debug)]
pub(crate) struct Found {
    pub(crate) occurrence: Occurrence,
    pub(crate) marked: bool,
}

/// One string comparison or string `match` the scan found.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Occurrence {
    /// The source file the occurrence was found in, relative to the base
    /// directory, with `/` separators.
    pub file: String,
    /// The 1-based line the occurrence starts on, kept for the human-facing
    /// report only -- the allow-list itself keys on `item`, not this
    /// (F8; see [`crate::AllowListEntry::item`]'s own doc).
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

/// The crate that owns the marker and the marker's own name.
const MARKER_CRATE: &str = "ix_trace_rs";
const MARKER_NAME: &str = "string_edge";

fn ident_is(ident: &syn::Ident, name: &str) -> bool {
    ident == name
}

/// Whether `attrs` carry the `ix-trace-rs` marker: the path `string_edge`,
/// `ix_trace_rs::string_edge` or `::ix_trace_rs::string_edge`. Any other path,
/// a crate-qualified path of another crate included, is not a marker.
fn has_string_edge_marker(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        let path = attr.path();
        let mut segments = path.segments.iter();
        let bare = |segment: Option<&syn::PathSegment>, name: &str| {
            segment.is_some_and(|segment| ident_is(&segment.ident, name))
        };
        match (path.leading_colon.is_some(), path.segments.len()) {
            (false, 1) => bare(segments.next(), MARKER_NAME),
            (_, 2) => bare(segments.next(), MARKER_CRATE) && bare(segments.next(), MARKER_NAME),
            _ => false,
        }
    })
}

pub(crate) fn has_cfg_test(attrs: &[syn::Attribute]) -> bool {
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
/// same `has_string_edge_marker`/`has_cfg_test` shape already used above in this
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

fn peel_parens(expr: &syn::Expr) -> &syn::Expr {
    match expr {
        syn::Expr::Paren(paren) => peel_parens(&paren.expr),
        other => other,
    }
}

/// The `Option`-returning prefix methods: a `Some` arm is the dispatch.
const PREFIX_GATE_METHODS: [&str; 1] = ["strip_prefix"];

fn is_prefix_gate_method(method: &syn::Ident) -> bool {
    PREFIX_GATE_METHODS.contains(&method.to_string().as_str())
}

struct Scanner<'c> {
    file: String,
    /// Names of the `const NAME: &str` items of the crate being scanned.
    string_consts: &'c BTreeSet<String>,
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
    found: Vec<Found>,
}

impl Scanner<'_> {
    /// Whether `expr` is a string literal or a named string const.
    fn is_string_operand(&self, expr: &syn::Expr) -> bool {
        is_string_lit(expr)
            || matches!(expr, syn::Expr::Path(path)
            if path.qself.is_none()
                && path.path.segments.last().is_some_and(|segment| {
                    self.string_consts.contains(&segment.ident.to_string())
                }))
    }

    /// Whether `expr` is a comparison with a string operand.
    fn is_string_comparison(&self, expr: &syn::Expr) -> bool {
        matches!(expr, syn::Expr::Binary(binary)
            if is_comparison_op(binary.op)
                && (self.is_string_operand(&binary.left) || self.is_string_operand(&binary.right)))
    }

    /// Whether `pat` is a string literal pattern or a named string const.
    fn pattern_has_string_operand(&self, pat: &syn::Pat) -> bool {
        match pat {
            syn::Pat::Lit(syn::PatLit {
                lit: syn::Lit::Str(_),
                ..
            }) => true,
            syn::Pat::Ident(ident) => {
                ident.subpat.is_none() && self.string_consts.contains(&ident.ident.to_string())
            }
            syn::Pat::Path(path) => path
                .path
                .segments
                .last()
                .is_some_and(|segment| self.string_consts.contains(&segment.ident.to_string())),
            syn::Pat::Or(pat_or) => pat_or
                .cases
                .iter()
                .any(|case| self.pattern_has_string_operand(case)),
            _ => false,
        }
    }

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
        let branch_gating = self.condition_depth > 0;
        self.push(line, branch_gating);
    }

    fn record_forced_branch_gating(&mut self, line: u32) {
        self.push(line, true);
    }

    /// Records the occurrence together with whether a `#[string_edge]`
    /// function encloses it: a marked occurrence is silent in the report but
    /// still counts when an allow-list entry is checked against branch
    /// gating (FR-064: a mark is not a way to admit an allow-list entry).
    fn push(&mut self, line: u32, branch_gating: bool) {
        let occurrence = Occurrence {
            file: self.file.clone(),
            line,
            item: self.current_item(),
            branch_gating,
        };
        self.found.push(Found {
            occurrence,
            marked: self.string_edge_depth > 0,
        });
    }
}

impl<'ast> Visit<'ast> for Scanner<'_> {
    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        let previous = self
            .current_impl_self
            .replace(crate::impl_self_name(&node.self_ty));
        syn::visit::visit_item_impl(self, node);
        self.current_impl_self = previous;
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let marked = has_string_edge_marker(&node.attrs);
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
        let marked = has_string_edge_marker(&node.attrs);
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
            && (self.is_string_operand(&node.receiver)
                || node.args.iter().any(|arg| self.is_string_operand(arg)))
        {
            if is_prefix_gate_method(&node.method) {
                // `strip_prefix` yields `Some` only for a matching string, so
                // its result is itself the branch (the `clock:`
                // dispatch reads it through `ok_or_else`).
                self.record_forced_branch_gating(line_of(node));
            } else {
                self.record(line_of(node));
            }
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
            if self.pattern_has_string_operand(&arm.pat) {
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
            // A comparison that is itself the arm's value: the match's
            // result carries the string decision to whatever reads it,
            // so it is branch-gating.
            let value_is_comparison = self.is_string_comparison(peel_parens(&arm.body));
            if value_is_comparison {
                self.condition_depth += 1;
            }
            self.visit_expr(&arm.body);
            if value_is_comparison {
                self.condition_depth -= 1;
            }
        }
    }

    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        if is_comparison_op(node.op)
            && (self.is_string_operand(&node.left) || self.is_string_operand(&node.right))
        {
            self.record(line_of(node));
        }
        if matches!(node.op, syn::BinOp::And(_) | syn::BinOp::Or(_)) {
            // A term of a boolean combinator: the combined result is what an
            // outer `if`/`match` reads, so every comparison inside
            // is branch-gating.
            self.condition_depth += 1;
            syn::visit::visit_expr_binary(self, node);
            self.condition_depth -= 1;
        } else {
            syn::visit::visit_expr_binary(self, node);
        }
    }
}

/// The names of every `const NAME: &str` item (module-level, associated or
/// trait) outside `#[cfg(test)]` code in `files`.
fn collect_string_consts<'a>(files: impl Iterator<Item = &'a syn::File>) -> BTreeSet<String> {
    struct Consts(BTreeSet<String>);
    fn is_str_ref(ty: &syn::Type) -> bool {
        matches!(ty, syn::Type::Reference(reference)
            if matches!(&*reference.elem, syn::Type::Path(path) if path.path.is_ident("str")))
    }
    impl<'ast> Visit<'ast> for Consts {
        fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
            if !has_cfg_test(&node.attrs) {
                syn::visit::visit_item_mod(self, node);
            }
        }
        fn visit_item_const(&mut self, node: &'ast syn::ItemConst) {
            if is_str_ref(&node.ty) && !has_cfg_test(&node.attrs) {
                self.0.insert(node.ident.to_string());
            }
        }
        fn visit_impl_item_const(&mut self, node: &'ast syn::ImplItemConst) {
            if is_str_ref(&node.ty) && !has_cfg_test(&node.attrs) {
                self.0.insert(node.ident.to_string());
            }
        }
        fn visit_trait_item_const(&mut self, node: &'ast syn::TraitItemConst) {
            if is_str_ref(&node.ty) && !has_cfg_test(&node.attrs) {
                self.0.insert(node.ident.to_string());
            }
        }
    }
    let mut consts = Consts(BTreeSet::new());
    for file in files {
        consts.visit_file(file);
    }
    consts.0
}

/// Scan the parsed `files` of one crate (each with its base-relative name)
/// against that crate's named string consts, marked and unmarked alike.
pub(crate) fn scan_parsed(files: &[(String, syn::File)]) -> Vec<Found> {
    let string_consts = collect_string_consts(files.iter().map(|(_, file)| file));
    let mut found = Vec::new();
    for (relative, file) in files {
        let mut scanner = Scanner {
            file: relative.clone(),
            string_consts: &string_consts,
            string_edge_depth: 0,
            condition_depth: 0,
            current_impl_self: None,
            item_stack: Vec::new(),
            found: Vec::new(),
        };
        scanner.visit_file(file);
        found.extend(scanner.found);
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    /// Every occurrence in `source`, marked or not.
    fn found_in(source: &str) -> Vec<Found> {
        let parsed = syn::parse_file(source).expect("fixture parses");
        scan_parsed(&[("fixture.rs".to_owned(), parsed)])
    }

    /// The occurrences in `source` outside a `#[string_edge]` function.
    fn scan_source(source: &str) -> Vec<Occurrence> {
        found_in(source)
            .into_iter()
            .filter(|found| !found.marked)
            .map(|found| found.occurrence)
            .collect()
    }

    fn occurrence(line: u32, item: &str, branch_gating: bool) -> Occurrence {
        Occurrence {
            file: "fixture.rs".to_owned(),
            line,
            item: item.to_owned(),
            branch_gating,
        }
    }

    /// FR-064-AC-1: a marked function's comparison is not reported; an
    /// unmarked one is, at its own line and item.
    #[trace("TC-162", "FR-064-AC-1")]
    #[test]
    fn marked_function_is_silent_unmarked_is_reported() {
        let source = "#[string_edge]\n\
                      fn edge(kind: &str) -> bool { kind == \"value\" }\n\
                      fn not_an_edge(kind: &str) -> bool { kind == \"value\" }\n";
        assert_eq!(scan_source(source), [occurrence(3, "not_an_edge", false)]);
        let marked: Vec<_> = found_in(source)
            .into_iter()
            .filter(|found| found.marked)
            .map(|found| found.occurrence)
            .collect();
        assert_eq!(marked, [occurrence(2, "edge", false)]);
    }

    /// FR-064-AC-1 and AC-8: the qualified marker `#[ix_trace_rs::string_edge]`
    /// marks a free function and an `impl` method alike.
    #[trace("TC-162", "FR-064-AC-1", "FR-064-AC-8")]
    #[test]
    fn the_qualified_marker_silences_free_functions_and_methods() {
        let occurrences = scan_source(
            "#[ix_trace_rs::string_edge]\n\
             fn edge(kind: &str) -> bool { kind == \"a\" }\n\
             struct T;\n\
             impl T {\n\
             #[ix_trace_rs::string_edge]\n\
             fn edge(kind: &str) -> bool { kind == \"b\" }\n\
             fn plain(kind: &str) -> bool { kind == \"c\" }\n\
             }\n",
        );
        assert_eq!(occurrences, [occurrence(7, "T::plain", false)]);
    }

    /// FR-064-AC-12: the three accepted marker paths each silence their
    /// function, and the unmarked function beside them is reported.
    #[trace("TC-162", "FR-064-AC-12")]
    #[test]
    fn the_three_accepted_marker_paths_silence_their_functions() {
        let occurrences = scan_source(
            "use ix_trace_rs::string_edge;\n\
             #[string_edge]\n\
             fn imported(kind: &str) -> bool { kind == \"a\" }\n\
             #[ix_trace_rs::string_edge]\n\
             fn qualified(kind: &str) -> bool { kind == \"b\" }\n\
             #[::ix_trace_rs::string_edge]\n\
             fn rooted(kind: &str) -> bool { kind == \"c\" }\n\
             fn plain(kind: &str) -> bool { kind == \"d\" }\n",
        );
        assert_eq!(occurrences, [occurrence(8, "plain", false)]);
    }

    /// FR-064-AC-15: a look-alike path, a comment and a doc comment are not
    /// markers, so each function they decorate is reported.
    #[trace("TC-162", "FR-064-AC-15")]
    #[test]
    fn look_alike_paths_and_comments_are_not_markers() {
        let occurrences = scan_source(
            "#[my_macros::string_edge]\n\
             fn other_crate(kind: &str) -> bool { kind == \"a\" }\n\
             #[other::ix_trace_rs::string_edge]\n\
             fn nested(kind: &str) -> bool { kind == \"b\" }\n\
             #[::string_edge]\n\
             fn rooted_bare(kind: &str) -> bool { kind == \"c\" }\n\
             #[ix_trace_rs::other::string_edge]\n\
             fn deeper(kind: &str) -> bool { kind == \"d\" }\n\
             // string_edge\n\
             fn comment(kind: &str) -> bool { kind == \"e\" }\n\
             /// string_edge\n\
             fn doc_comment(kind: &str) -> bool { kind == \"f\" }\n",
        );
        let items: Vec<_> = occurrences
            .iter()
            .map(|found| found.item.as_str())
            .collect();
        assert_eq!(
            items,
            [
                "other_crate",
                "nested",
                "rooted_bare",
                "deeper",
                "comment",
                "doc_comment"
            ]
        );
    }

    /// FR-064-AC-3: a `#[cfg(test)]` module, function and method is never
    /// scanned.
    #[trace("TC-162", "FR-064-AC-3")]
    #[test]
    fn cfg_test_items_are_not_scanned() {
        let occurrences = scan_source(
            "#[cfg(test)]\n\
             mod tests { fn check(kind: &str) -> bool { kind == \"value\" } }\n\
             #[cfg(test)]\n\
             fn helper(kind: &str) -> bool { kind == \"value\" }\n\
             struct T;\n\
             impl T {\n\
             #[cfg(test)]\n\
             fn helper(kind: &str) -> bool { kind == \"value\" }\n\
             fn live(kind: &str) -> bool { kind == \"value\" }\n\
             }\n",
        );
        assert_eq!(occurrences, [occurrence(9, "T::live", false)]);
    }

    /// FR-064-AC-5's branch-gating/non-branching distinction: a comparison
    /// feeding an `if` condition is branch-gating; one feeding only a
    /// logged message is not.
    #[trace("TC-162", "FR-064-AC-5")]
    #[test]
    fn branch_gating_is_distinguished_from_a_non_branching_sink() {
        let occurrences = scan_source(
            "fn dispatch(kind: &str) {\n\
             if kind == \"allocation\" {\n\
             do_something();\n\
             }\n\
             let flag = kind == \"other\";\n\
             log(flag);\n\
             }\n",
        );
        assert_eq!(
            occurrences,
            [
                occurrence(2, "dispatch", true),
                occurrence(5, "dispatch", false)
            ]
        );
    }

    /// FR-064-AC-1: a string `match` is reported by its literal-pattern
    /// arm's own line, branch-gating.
    #[trace("TC-162", "FR-064-AC-1")]
    #[test]
    fn string_match_is_reported() {
        let occurrences = scan_source(
            "fn dispatch(kind: &str) {\n\
             match kind {\n\
             \"allocation\" => {}\n\
             _ => {}\n\
             }\n\
             }\n",
        );
        assert_eq!(occurrences, [occurrence(3, "dispatch", true)]);
    }

    /// F8: the item anchor survives a line shift that touches nothing about
    /// the comparison itself -- the exact brittleness line-keying had.
    #[trace("TC-162", "FR-064-AC-2")]
    #[test]
    fn item_key_is_stable_across_an_unrelated_line_shift() {
        let before = scan_source("fn dispatch(kind: &str) -> bool { kind == \"value\" }\n");
        let after = scan_source(
            "/// A new doc comment that pushes the comparison below down\n\
             /// by several lines.\n\
             fn dispatch(kind: &str) -> bool { kind == \"value\" }\n",
        );
        assert_eq!(before, [occurrence(1, "dispatch", false)]);
        assert_eq!(after, [occurrence(3, "dispatch", false)]);
    }

    /// F5 (round 2 extends the method list with `ends_with`, `strip_prefix`
    /// and `trim_start_matches`): all six `STRING_EDGE_METHODS` are method
    /// calls, not `syn::ExprBinary`, so they need their own detection path.
    /// A literal receiver, a literal argument, and a non-literal argument
    /// (which reports nothing).
    #[trace("TC-162", "FR-064-AC-1")]
    #[test]
    fn literal_taking_string_methods_are_reported() {
        let occurrences = scan_source(
            "fn a(value: &str) -> bool { value.starts_with(\"prefix\") }\n\
             fn b(value: &str) -> bool { \"literal\".eq_ignore_ascii_case(value) }\n\
             fn c(stderr: &str) -> bool { stderr.contains(\"--offline\") }\n\
             fn d(value: &str) -> bool { value.starts_with(other()) }\n\
             fn e(value: &str) -> bool { value.ends_with(\"suffix\") }\n\
             fn f(value: &str) -> Option<&str> { value.strip_prefix(\"prefix\") }\n\
             fn g(value: &str) -> &str { value.trim_start_matches(\"prefix\") }\n",
        );
        assert_eq!(
            occurrences,
            [
                occurrence(1, "a", false),
                occurrence(2, "b", false),
                occurrence(3, "c", false),
                occurrence(5, "e", false),
                occurrence(6, "f", true),
                occurrence(7, "g", false),
            ]
        );
    }

    /// F5: a method call inside a `#[string_edge]`-marked function is
    /// silent, matching the existing `ExprBinary` behaviour.
    #[trace("TC-162", "FR-064-AC-1")]
    #[test]
    fn marked_function_silences_a_literal_taking_method_call_too() {
        let occurrences = scan_source(
            "#[string_edge]\n\
             fn edge(value: &str) -> bool { value.starts_with(\"prefix\") }\n",
        );
        assert_eq!(occurrences, []);
    }

    /// A comparison that is a term of a `&&`/`||` chain, a match arm's own
    /// value, or a `strip_prefix` call is branch-gating; a bare comparison
    /// bound to a variable stays non-branching.
    #[trace("TC-162", "FR-064-AC-5")]
    #[test]
    fn combinator_terms_arm_values_and_prefix_gates_are_branch_gating() {
        let occurrences = scan_source(
            "fn chain(v: &str) -> bool { v.len() == 1 && v == \"a\" }\n\
             fn either(v: &str) -> bool { !(v == \"a\") || v.is_empty() }\n\
             fn arm(v: &str, k: u8) -> bool { match k { 0 => v == \"a\", _ => false } }\n\
             fn prefix(v: &str) -> Option<&str> { v.strip_prefix(\"p:\") }\n\
             fn bound(v: &str) { let f = v == \"a\"; log(f); }\n",
        );
        assert_eq!(
            occurrences,
            [
                occurrence(1, "chain", true),
                occurrence(2, "either", true),
                occurrence(3, "arm", true),
                occurrence(4, "prefix", true),
                occurrence(5, "bound", false),
            ]
        );
    }

    /// FR-064's Behavior ("a string ... and another `&str`/`String` value"):
    /// a comparison, method call or `match` arm against a named `const NAME:
    /// &str` of the same crate is reported like a literal one; a const of
    /// another type is not.
    #[trace("TC-162", "FR-064-AC-1")]
    #[test]
    fn named_string_consts_are_resolved_like_literals() {
        let occurrences = scan_source(
            "const NARROW: &str = \"narrow\";\n\
             struct T;\n\
             impl T { const KIND: &'static str = \"kind\"; }\n\
             const LIMIT: usize = 3;\n\
             fn eq(v: &str) -> bool { v == NARROW }\n\
             fn assoc(v: &str) -> bool { v != Self::KIND }\n\
             fn method(v: &str) -> bool { v.starts_with(NARROW) }\n\
             fn arm(v: &str) -> u8 { match v { NARROW => 1, _ => 0 } }\n\
             fn not_a_string(n: usize) -> bool { n == LIMIT }\n",
        );
        assert_eq!(
            occurrences,
            [
                occurrence(5, "eq", false),
                occurrence(6, "assoc", false),
                occurrence(7, "method", false),
                occurrence(8, "arm", true),
            ]
        );
    }

    /// FR-064-AC-7: a string constant resolves within its own parsed crate
    /// only; a same-named numeric constant in another crate is not string
    /// dispatch.
    #[trace("TC-162", "FR-064-AC-7")]
    #[test]
    fn constants_do_not_cross_crate_scopes() {
        let parse = |source: &str| syn::parse_file(source).expect("fixture parses");
        let reader = vec![(
            "reader.rs".to_owned(),
            parse("const KIND: &str = \"k\";\nfn read(v: &str) -> bool { v == KIND }\n"),
        )];
        let emitter = vec![(
            "emitter.rs".to_owned(),
            parse("const KIND: usize = 3;\nfn emit(n: usize) -> bool { n == KIND }\n"),
        )];
        let reader_found = scan_parsed(&reader);
        assert_eq!(reader_found.len(), 1);
        assert_eq!(reader_found[0].occurrence.item, "read");
        assert_eq!(scan_parsed(&emitter).len(), 0);
        let mixed: Vec<_> = reader.into_iter().chain(emitter).collect();
        assert_eq!(
            scan_parsed(&mixed).len(),
            2,
            "one scope given both files resolves the constant for the second file too"
        );
    }
}
