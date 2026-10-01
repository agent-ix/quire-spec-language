// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL#214 (FR-064): `cargo xtask string-edge` -- scans the QSL crates'
//! non-test source for a string comparison or a string `match` outside a
//! `#[string_edge]`-marked function, reporting each one not covered by the
//! checked-in allow-list.
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
    /// Whether a `#[string_edge]` mark silences its function. `false` only
    /// for the raw scan the real-site test uses, so a site stays visible to
    /// it whether or not the sweep has marked it.
    honor_marks: bool,
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

impl<'ast> Visit<'ast> for Scanner<'_> {
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
        if marked && self.honor_marks {
            self.string_edge_depth += 1;
        }
        self.item_stack.push(node.sig.ident.to_string());
        syn::visit::visit_item_fn(self, node);
        self.item_stack.pop();
        if marked && self.honor_marks {
            self.string_edge_depth -= 1;
        }
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        let marked = has_attr_named(&node.attrs, "string_edge");
        if has_cfg_test(&node.attrs) {
            return;
        }
        if marked && self.honor_marks {
            self.string_edge_depth += 1;
        }
        let name = match &self.current_impl_self {
            Some(self_ty) => format!("{self_ty}::{}", node.sig.ident),
            None => node.sig.ident.to_string(),
        };
        self.item_stack.push(name);
        syn::visit::visit_impl_item_fn(self, node);
        self.item_stack.pop();
        if marked && self.honor_marks {
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

/// Every non-test `.rs` file under `root` (skipping any directory named
/// `tests`, `target` or `.git`, and every file a `#[cfg(test)] mod x;`
/// declares), relative to `workspace_root`. A
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
    let excluded = cfg_test_module_paths(&files, workspace_root)?;
    files.retain(|file| !excluded.iter().any(|test_path| file.starts_with(test_path)));
    files.sort();
    Ok(files)
}

/// The paths of every `#[cfg(test)] mod name;` file module declared by one
/// of `files` (FR-064 "excludes test code"): `<dir>/name.rs` (and its
/// `<dir>/name/` children) or `<dir>/name/mod.rs`'s directory. `<dir>` is
/// the declaring file's directory for `mod.rs`/`lib.rs`/`main.rs`, else the
/// directory named after the declaring file.
fn cfg_test_module_paths(files: &[PathBuf], workspace_root: &Path) -> Result<Vec<PathBuf>> {
    let mut excluded = Vec::new();
    for relative in files {
        let full_path = workspace_root.join(relative);
        let source =
            fs::read_to_string(&full_path).map_err(|source| Error::io(&full_path, source))?;
        let parsed = syn::parse_file(&source).map_err(|source| Error::StringEdgeParse {
            path: full_path.clone(),
            source,
        })?;
        let declared: Vec<String> = parsed
            .items
            .iter()
            .filter_map(|item| match item {
                syn::Item::Mod(module)
                    if module.content.is_none() && has_cfg_test(&module.attrs) =>
                {
                    Some(module.ident.to_string())
                }
                _ => None,
            })
            .collect();
        if declared.is_empty() {
            continue;
        }
        let parent = relative.parent().unwrap_or_else(|| Path::new(""));
        let stem = relative.file_stem().map(PathBuf::from).unwrap_or_default();
        let base = if is_module_root_stem(&stem) {
            parent.to_path_buf()
        } else {
            parent.join(&stem)
        };
        for name in declared {
            excluded.push(base.join(format!("{name}.rs")));
            excluded.push(base.join(&name));
        }
    }
    Ok(excluded)
}

/// `mod.rs`, `lib.rs` and `main.rs` own their own directory; any other file
/// `foo.rs` owns `foo/`. A filesystem-name edge, not a family dispatch.
#[string_edge]
fn is_module_root_stem(stem: &Path) -> bool {
    matches!(stem.to_str(), Some("mod" | "lib" | "main"))
}

/// The names of every `const NAME: &str` item (module-level, associated or
/// trait) outside `#[cfg(test)]` code in `files`.
fn collect_string_consts(files: &[syn::File]) -> BTreeSet<String> {
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

/// Scan the parsed `files` of one crate against that crate's named string
/// consts. `honor_marks == false` ignores `#[string_edge]` marks.
fn scan_parsed(files: &[(String, syn::File)], honor_marks: bool) -> Vec<Occurrence> {
    let parsed: Vec<syn::File> = files.iter().map(|(_, file)| file.clone()).collect();
    let string_consts = collect_string_consts(&parsed);
    let mut occurrences = Vec::new();
    for (relative, file) in files {
        let mut scanner = Scanner {
            file: relative.clone(),
            string_consts: &string_consts,
            honor_marks,
            string_edge_depth: 0,
            condition_depth: 0,
            current_impl_self: None,
            item_stack: Vec::new(),
            occurrences: Vec::new(),
        };
        scanner.visit_file(file);
        occurrences.extend(scanner.occurrences);
    }
    occurrences
}

/// Scan every crate root under `workspace_root`.
fn scan_workspace(workspace_root: &Path, honor_marks: bool) -> Result<Vec<Occurrence>> {
    let mut all_occurrences = Vec::new();
    for root in crate_roots(workspace_root) {
        if !root.exists() {
            return Err(Error::StringEdgeMissingRoot { path: root });
        }
        let mut files = Vec::new();
        for relative in source_files(&root, workspace_root)? {
            let full_path = workspace_root.join(&relative);
            let source =
                fs::read_to_string(&full_path).map_err(|source| Error::io(&full_path, source))?;
            let parsed = syn::parse_file(&source).map_err(|source| Error::StringEdgeParse {
                path: full_path.clone(),
                source,
            })?;
            files.push((relative.to_string_lossy().replace('\\', "/"), parsed));
        }
        all_occurrences.extend(scan_parsed(&files, honor_marks));
    }
    Ok(all_occurrences)
}

/// Every crate root this scan covers: the QSL root crate, `xtask`,
/// `quire-exact`, `qsl-foundation` (ADR-011 §7.3 X-2), `qsl-cst`
/// (ADR-011 §7.3 X-3), `qsl-source` (ADR-011 §7.3 X-4),
/// `qsl-forms` (ADR-011 §7.3 X-5), `qsl-semantics` (ADR-011 §7.3
/// X-6), `qsl-package` (ADR-011 §7.3 X-7), `qsl-eval`
/// (ADR-011 §7.3 X-8), `qsl-route` (ADR-011 §7.3 X-9), `qsl-replay` (ADR-011 §7.3 X-10
/// -- each extracted §6.1 layer crate adds its own entry here the same way)
/// and `qsl-bench` (the benchmark harness).
/// `qsl-attrs` is excluded -- it is a proc-macro identity transform with no
/// string dispatch of any kind (its own module doc).
fn crate_roots(workspace_root: &Path) -> Vec<PathBuf> {
    [
        "src",
        "xtask/src",
        "quire-exact/src",
        "qsl-foundation/src",
        "qsl-cst/src",
        "qsl-source/src",
        "qsl-forms/src",
        "qsl-semantics/src",
        "qsl-package/src",
        "qsl-eval/src",
        "qsl-route/src",
        "qsl-replay/src",
        "qsl-bench/src",
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
    let allow_list = allow_list();
    if let Some(entry) = allow_list_branch_gating_check(workspace_root)?
        .into_iter()
        .next()
    {
        return Err(Error::StringEdgeAllowListGatesABranch {
            file: entry.file,
            item: entry.item,
        });
    }
    let all_occurrences = scan_workspace(workspace_root, true)?;
    let reported: Vec<Occurrence> = unreported_occurrences(&all_occurrences, &allow_list)
        .into_iter()
        .cloned()
        .collect();
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

/// Every occurrence in `occurrences` not covered by an entry in
/// `allow_list` (matched by `(file, item)`, the same key
/// [`AllowListEntry::item`]'s own doc explains -- an allow-list entry admits
/// every occurrence in the same item, not one specific comparison). Pure
/// over its two inputs, split out of [`run`] for the same reason
/// [`branch_gating_entries`] below was (PR #262 review, finding F11):
/// [`allow_list`] itself is permanently empty today (nothing has yet been
/// reviewed and admitted), so a test that could only call through that
/// production list could never observe an allow-listed occurrence actually
/// being suppressed, or reappearing once its entry is removed (FR-064-AC-2).
fn unreported_occurrences<'a>(
    occurrences: &'a [Occurrence],
    allow_list: &'a [AllowListEntry],
) -> Vec<&'a Occurrence> {
    let allow_list: BTreeSet<(&str, &str)> = allow_list
        .iter()
        .map(|entry| (entry.file.as_str(), entry.item.as_str()))
        .collect();
    occurrences
        .iter()
        .filter(|occurrence| {
            !allow_list.contains(&(occurrence.file.as_str(), occurrence.item.as_str()))
        })
        .collect()
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
    let all_occurrences = scan_workspace(workspace_root, true)?;
    let allow_list = allow_list();
    Ok(branch_gating_entries(&all_occurrences, &allow_list))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    /// Workspace members `crate_roots` does not scan: `qsl-attrs` (see
    /// `crate_roots`' own doc) and `tools/arch-lint`, which the scan has
    /// never covered.
    const UNSCANNED_MEMBERS: [&str; 2] = ["qsl-attrs", "tools/arch-lint"];

    /// Every workspace member's `src/` is a scan root, except the
    /// documented [`UNSCANNED_MEMBERS`]. A crate extracted later, or a root
    /// dropped from the list (removing `qsl-route/src`
    /// failed nothing), fails here.
    #[test]
    fn every_workspace_member_is_a_scan_root() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask lives one level below the workspace root")
            .to_path_buf();
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
            .map(|member| workspace_root.join(member).join("src"))
            .collect();
        expected.sort();
        let mut roots = crate_roots(&workspace_root);
        roots.sort();
        assert_eq!(roots, expected);
    }

    fn scan_source(source: &str) -> Vec<Occurrence> {
        let parsed = syn::parse_file(source).expect("fixture parses");
        scan_parsed(&[("fixture.rs".to_owned(), parsed)], true)
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
    /// logged message is not. The real ADR-010 §4.3 sites are covered by
    /// `real_adr010_sites_are_flagged_branch_gating_by_the_structural_detector`.
    #[trace("TC-162", "FR-064-AC-5")]
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
    /// rejection is actually exercised, not only claimed.
    ///
    /// The branch-gating and non-branching comparisons live in two
    /// *different* functions (PR #262 review, finding F8): keying on the
    /// enclosing item, not the line, means two occurrences in the *same*
    /// item are no longer distinguishable from each other -- a real,
    /// accepted coarsening (`AllowListEntry::item`'s own doc) this fixture
    /// must respect to still name one occurrence at a time.
    #[trace("TC-162", "FR-064-AC-5")]
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

    /// FR-064-AC-2: an allow-listed occurrence's `(file, item)` key is not
    /// reported; removing that same entry, with no change to the source,
    /// causes it to be reported again. Exercises [`unreported_occurrences`]
    /// directly (`run`'s own real filtering logic) rather than the
    /// permanently empty production [`allow_list`], the same "structurally
    /// untestable otherwise" reason [`branch_gating_entries`]'s own tests
    /// already do this for FR-064-AC-5.
    #[trace("TC-162", "FR-064-AC-2")]
    #[test]
    fn allow_listed_occurrence_is_silent_removing_the_entry_reports_it_again() {
        let occurrences = scan_source(
            r#"
            fn dispatch(kind: &str) -> bool { kind == "value" }
            "#,
        );
        let entry = AllowListEntry {
            file: occurrences[0].file.clone(),
            item: occurrences[0].item.clone(),
            reason: "test: added",
        };
        assert!(
            unreported_occurrences(&occurrences, std::slice::from_ref(&entry)).is_empty(),
            "an allow-listed occurrence must not be reported while its entry exists"
        );
        // The entry removed, with no change to `occurrences` (the source):
        // the same occurrence's key is reported again on the next scan.
        assert_eq!(
            unreported_occurrences(&occurrences, &[]).len(),
            1,
            "removing the allow-list entry must report the occurrence again"
        );
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

    /// FR-064-AC-5's five-site fixture half: one synthetic fixture shaped
    /// like each of ADR-010 §4.3's five named dispatch strings (a string
    /// comparison selecting between two branches), each asserted rejected as
    /// an allow-list entry. `branch_gating_entries` filters on `(file, item,
    /// branch_gating)`, never on the literal, so this shows the rejection
    /// mechanism for each spelling; the real sites are covered by
    /// `real_adr010_sites_are_flagged_branch_gating_by_the_structural_detector`.
    #[trace("TC-162", "FR-064-AC-5")]
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

    /// FR-064-AC-5's real-site half: runs the real scan (not a
    /// synthetic fixture, and ignoring `#[string_edge]` marks so a mark cannot
    /// hide a site) over the ADR-010 section 4.3 dispatch strings still
    /// compared in the tree, at the one typed conversion each now lives in:
    /// `CanonicalizationDomain::from_str` (`"filament-canonical-json-1"`),
    /// `AdapterArtifact::try_from` (`"quire.state.authority-adapter"`) and
    /// `clock_binding_name` (the `"clock:"` prefix, a `strip_prefix`). Each
    /// must come back branch-gating and an allow-list entry at it must be
    /// rejected. The `"quire.protocol.finite-global/v1"` profile string is now
    /// only registry data (`RegisteredDefinition::identity`), matched by the
    /// registry's own table, and the `"allocation"` site is gone from the
    /// tree. Fails if a site's file/item vanishes without this list being
    /// updated, and if the detector stops seeing a real site.
    #[trace("TC-162", "FR-064-AC-5")]
    #[test]
    fn real_adr010_sites_are_flagged_branch_gating_by_the_structural_detector() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask lives one level below the workspace root")
            .to_path_buf();
        let sites = [
            (
                "src/state/input.rs",
                "CanonicalizationDomain::from_str",
                "filament-canonical-json-1",
            ),
            (
                "src/state/input.rs",
                "AdapterArtifact::try_from",
                "quire.state.authority-adapter",
            ),
            (
                "src/protocol_artifact/mod.rs",
                "clock_binding_name",
                "clock:",
            ),
            // A real named-`const NAME: &str` site
            // (`identity == NARROW`, `qsl-semantics/src/check/claims.rs`),
            // not a string literal -- the const-resolving detector, not the
            // literal-only one #485 shipped, is what finds this occurrence.
            (
                "qsl-semantics/src/check/claims.rs",
                "operation_role",
                "quire.op.numeric.narrow",
            ),
        ];
        let all = scan_workspace(&workspace_root, false)
            .unwrap_or_else(|error| panic!("the workspace scan: {error}"));
        for (file, item, label) in sites {
            let occurrences: Vec<Occurrence> = all
                .iter()
                .filter(|occurrence| occurrence.file == file)
                .cloned()
                .collect();
            let site: Vec<&Occurrence> = occurrences
                .iter()
                .filter(|occurrence| occurrence.item == item)
                .collect();
            assert!(
                !site.is_empty(),
                "{file}::{item} ({label}) is no longer found by the scan"
            );
            assert!(
                site.iter().all(|occurrence| occurrence.branch_gating),
                "{file}::{item} ({label}) is not flagged branch-gating -- FR-064-AC-5 \
                 requires a real ADR-010 section 4.3 dispatch site to be rejected"
            );
            let candidate = AllowListEntry {
                file: file.to_owned(),
                item: item.to_owned(),
                reason: "test: a real ADR-010 section 4.3 production dispatch site",
            };
            let rejected = branch_gating_entries(&occurrences, [&candidate]);
            assert_eq!(
                rejected.len(),
                1,
                "{file}::{item} ({label}) is not rejected as an allow-list entry"
            );
        }
    }

    /// A comparison that is a term of a `&&`/`||` chain, a match
    /// arm's own value, or a `strip_prefix` call is branch-gating; a bare
    /// comparison bound to a variable stays non-branching.
    #[trace("TC-162", "FR-064-AC-5")]
    #[test]
    fn combinator_terms_arm_values_and_prefix_gates_are_branch_gating() {
        let occurrences = scan_source(
            r#"
            fn chain(v: &str) -> bool { v.len() == 1 && v == "a" }
            fn either(v: &str) -> bool { !(v == "a") || v.is_empty() }
            fn arm(v: &str, k: u8) -> bool { match k { 0 => v == "a", _ => false } }
            fn prefix(v: &str) -> Option<&str> { v.strip_prefix("p:") }
            fn bound(v: &str) { let f = v == "a"; log(f); }
            "#,
        );
        let gating = |item: &str| {
            occurrences
                .iter()
                .find(|o| o.item == item)
                .unwrap_or_else(|| panic!("{item} found"))
                .branch_gating
        };
        assert!(gating("chain"));
        assert!(gating("either"));
        assert!(gating("arm"));
        assert!(gating("prefix"));
        assert!(!gating("bound"));
    }

    /// FR-064-AC-3: file-level test modules are not scanned -- a file under
    /// `tests/` and a file declared by `#[cfg(test)] mod x;` (with its
    /// children) are skipped. A file is never skipped by its name: a
    /// production `plain_tests.rs` or `emit/tests.rs` is scanned.
    #[trace("TC-162", "FR-064-AC-3")]
    #[test]
    fn file_level_test_modules_are_skipped_only_when_declared_cfg_test() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let root = dir.path();
        let src = root.join("src");
        fs::create_dir_all(src.join("tests")).expect("mkdir");
        fs::create_dir_all(src.join("emit/checks")).expect("mkdir");
        fs::write(
            src.join("lib.rs"),
            "mod real;\nmod plain_tests;\nmod emit;\n#[cfg(test)]\nmod unit;\n#[cfg(test)]\nmod tests;\n",
        )
        .expect("write");
        for file in [
            "real.rs",
            "unit.rs",
            "tests.rs",
            "plain_tests.rs",
            "tests/it.rs",
            "emit/checks.rs",
            "emit/checks/deep.rs",
            "emit/keep.rs",
            "emit/tests.rs",
        ] {
            fs::write(src.join(file), "").expect("write");
        }
        fs::write(
            src.join("emit.rs"),
            "#[cfg(test)]\nmod checks;\nmod tests;\n",
        )
        .expect("write");
        let files = source_files(&src, root).expect("files");
        let names: Vec<String> = files
            .iter()
            .map(|f| f.to_string_lossy().replace('\\', "/"))
            .collect();
        assert_eq!(
            names,
            [
                "src/emit/keep.rs",
                "src/emit/tests.rs",
                "src/emit.rs",
                "src/lib.rs",
                "src/plain_tests.rs",
                "src/real.rs"
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
            r#"
            const NARROW: &str = "narrow";
            struct T;
            impl T { const KIND: &'static str = "kind"; }
            const LIMIT: usize = 3;
            fn eq(v: &str) -> bool { v == NARROW }
            fn assoc(v: &str) -> bool { v != Self::KIND }
            fn method(v: &str) -> bool { v.starts_with(NARROW) }
            fn arm(v: &str) -> u8 { match v { NARROW => 1, _ => 0 } }
            fn not_a_string(n: usize) -> bool { n == LIMIT }
            "#,
        );
        for item in ["eq", "assoc", "method", "arm"] {
            assert!(
                occurrences.iter().any(|o| o.item == item),
                "{item} must be reported"
            );
        }
        assert!(!occurrences.iter().any(|o| o.item == "not_a_string"));
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
            .find_map(|line| line.strip_prefix("ci:"))
            .expect("the Makefile has a `ci:` target");
        assert!(
            ci.split_whitespace().any(|target| target == "string-edge"),
            "`ci:` must list string-edge as a prerequisite: {ci}"
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

    /// FR-064-AC-4: `xtask string-edge` exits non-zero when its report is
    /// non-empty and zero when it is empty. Builds one fixture workspace
    /// tree with every root [`crate_roots`] requires (`run`'s own
    /// `root.exists()` check refuses a missing one), then swaps one file's
    /// content between "one unmarked, un-allow-listed occurrence" and
    /// "none" to get both shapes from the same tree, calling the real
    /// [`run`] end to end: `main` maps its `Ok` to `ExitCode::SUCCESS` (zero)
    /// and its `Err` to `Error::exit_code()` (non-zero for every
    /// `Code::StringEdge` variant), so this test exercises exactly the
    /// function that exit code is derived from.
    #[trace("TC-162", "FR-064-AC-4")]
    #[test]
    fn a_non_empty_report_exits_non_zero_a_clean_scan_exits_zero() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let root = dir.path();
        for member in [
            "src",
            "xtask/src",
            "quire-exact/src",
            "qsl-foundation/src",
            "qsl-cst/src",
            "qsl-source/src",
            "qsl-forms/src",
            "qsl-semantics/src",
            "qsl-package/src",
            "qsl-eval/src",
            "qsl-route/src",
            "qsl-replay/src",
            "qsl-bench/src",
        ] {
            fs::create_dir_all(root.join(member)).expect("create a crate root");
        }
        let fixture = root.join("src/string_edge_fixture.rs");

        fs::write(
            &fixture,
            "fn dispatch(kind: &str) -> bool { kind == \"value\" }\n",
        )
        .expect("write a violating fixture file");
        let error = run(root).expect_err("a non-empty report must be an Err, not an Ok");
        assert_eq!(error.code(), crate::error::Code::StringEdge);
        assert_ne!(
            error.exit_code(),
            0,
            "a non-empty report must map to a non-zero exit code"
        );

        fs::write(&fixture, "fn dispatch(kind: &str) -> bool { false }\n")
            .expect("clean the same fixture file, with no change to the tree otherwise");
        let summary = run(root).expect("a clean scan must be Ok");
        assert!(
            summary.contains("no unmarked, unlisted"),
            "a clean scan's own Ok summary: {summary}"
        );
    }
}
