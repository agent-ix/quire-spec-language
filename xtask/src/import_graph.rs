// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-139 (FR-068) TC-172/TC-176: resolve `check`'s and `model`'s real
//! `use` edges against `value`'s own re-export tables, instead of trusting a
//! textual scan of `use` lines alone.
//!
//! FR-068-CON-4 lets `value::mod.rs` go on re-exporting both `check`'s
//! relocated types and `value::expression`'s evaluation-only types through
//! its own flat aggregate (`pub use crate::check::{...}`, `pub use
//! expression::{...}`). That aggregate is exactly what makes a flat
//! `crate::value::X` import's *true* defining module invisible to a plain
//! grep for `value::expression` or `crate::check` -- the coordinator's own
//! framing in the PR #282 review that asked for this tool. A `use
//! crate::value::Evaluation;` inside `check` never contains the substring
//! `expression` anywhere in its own text; a `use crate::value::
//! DispatchTable;` inside `model` never contains the substring `check`
//! anywhere in its own text. Both compile today (CON-4 permits the
//! aggregate), and both are exactly the hidden edges TC-172 step 5 and
//! TC-176 step 5 require catching at the *resolved* level.
//!
//! This module closes that gap by reading `src/value/mod.rs`'s own two
//! aggregate `use` blocks (`pub use expression::{...}` and `pub use
//! crate::check::{...}`) to build the two name sets that matter, then
//! resolving every `use` edge found under `src/check/` and `src/model/`
//! against them: a flat `crate::value::Name` import whose `Name` is in
//! `value`'s `expression` re-export set resolves into `value::expression`
//! exactly as surely as a direct `crate::value::expression::Name` import
//! would, and the same for `crate::check`'s re-export set inside `model`.
//!
//! **Scope of what this resolves.** This is a two-hop resolution (`use`
//! edge -> `value::mod.rs`'s own aggregate lines), matched to the one
//! concrete mechanism FR-068-CON-4 names, not a general name-resolution
//! engine: it does not follow a re-export chain through a third module.
//!
//! **`#[cfg(test)]` handling differs by which criterion is being checked
//! (owner ruling, PR #282 review, post-rebase).**
//! [`check_module_violations`] (TC-172/AC-3) and [`model_check_edges`]
//! (TC-176) do not evaluate `#[cfg]` attributes at all: a `#[cfg(test)]`-gated
//! `use` is scanned the same as an unconditional one, which only ever makes
//! those two scans *stricter* than a build would be, and is deliberate --
//! `check` must import nothing from `value::expression` even in its own test
//! code (TC-174's test lives in `value::expression` instead, precisely
//! because of this). [`check_layer_edges`] (TC-175/AC-6), by contrast,
//! excludes `#[cfg(test)]`-gated imports and items on purpose: AC-6's bound
//! constrains `check`'s *shipped* dependency graph, and a test-only import is
//! not part of it.
//!
//! **FR-068-AC-6's layer rule (the layer-rule ruling, 2026-09-22;
//! [`check_layer_edges`]).** `check`'s shipped imports are bounded by
//! module and layer, not by item: any item of a permitted module is
//! allowed, and the number imported is never checked. This covers both
//! `use` lines and inline `crate::`/`super::`/`self::` paths under
//! `src/check/`, resolving a flat `crate::value::Name` aggregate import to
//! its real submodule (failing regardless, since the rule requires the
//! qualified form) and a `super::`/`self::` path relative to its own file.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use syn::spanned::Spanned;
use syn::visit::Visit;

use crate::error::{Error, Result};

fn parse_file(workspace_root: &Path, relative: &str) -> Result<syn::File> {
    let path = workspace_root.join(relative);
    let source = fs::read_to_string(&path).map_err(|source| Error::io(&path, source))?;
    syn::parse_file(&source).map_err(|source| Error::ImportGraphParse { path, source })
}

/// One resolved `use` edge found in a source file: the crate-absolute path
/// (segments, `crate` first when the source wrote it `crate`-rooted; kept
/// exactly as written otherwise) and the leaf name it binds.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UseEdge {
    /// The file this edge was found in, relative to the workspace root.
    pub file: String,
    /// The full path's segments, in source order (e.g. `["crate", "value",
    /// "quantity"]` for `use crate::value::quantity::QuantityUnit;`).
    pub path: Vec<String>,
    /// The bound name -- the imported item's own name, not a local rename
    /// (a `use a::b::C as D;` records `leaf: "C"`, the name that matters for
    /// resolving *what* it points at, not what it is called locally).
    pub leaf: String,
    /// Whether this edge came from a glob import (`use a::b::*;`), which
    /// binds no single named leaf; `leaf` is empty for a glob edge.
    pub is_glob: bool,
    /// 1-based source line the bound leaf (or the `*` token, for a glob)
    /// starts on -- FR-068-AC-6's layer rule names a failing edge's file,
    /// line and resolved module, so this edge type needs its own line, not
    /// only the enclosing `use` item's.
    pub line: usize,
}

/// Flatten one `syn::UseTree` into zero or more [`UseEdge`]s, appending to
/// `prefix` as the walk descends and reporting `leaf` as each branch's own
/// terminal name (its rename target if any, otherwise its own ident).
fn flatten_use_tree(tree: &syn::UseTree, prefix: &[String], file: &str, out: &mut Vec<UseEdge>) {
    match tree {
        syn::UseTree::Path(use_path) => {
            let mut next = prefix.to_vec();
            next.push(use_path.ident.to_string());
            flatten_use_tree(&use_path.tree, &next, file, out);
        }
        syn::UseTree::Name(use_name) => {
            let line = use_name.ident.span().start().line;
            let ident = use_name.ident.to_string();
            if ident == "self" {
                // `use a::b::self;` binds `b` itself under prefix `a`; the
                // leaf is the last prefix segment, not the literal "self".
                let leaf = prefix.last().cloned().unwrap_or_default();
                out.push(UseEdge {
                    file: file.to_owned(),
                    path: prefix.to_vec(),
                    leaf,
                    is_glob: false,
                    line,
                });
            } else {
                out.push(UseEdge {
                    file: file.to_owned(),
                    path: prefix.to_vec(),
                    leaf: ident,
                    is_glob: false,
                    line,
                });
            }
        }
        syn::UseTree::Rename(use_rename) => {
            // The name that matters for resolution is the *original*
            // ident, not the local alias `as` renames it to.
            out.push(UseEdge {
                file: file.to_owned(),
                path: prefix.to_vec(),
                leaf: use_rename.ident.to_string(),
                is_glob: false,
                line: use_rename.ident.span().start().line,
            });
        }
        syn::UseTree::Glob(use_glob) => {
            out.push(UseEdge {
                file: file.to_owned(),
                path: prefix.to_vec(),
                leaf: String::new(),
                is_glob: true,
                line: use_glob.star_token.span().start().line,
            });
        }
        syn::UseTree::Group(use_group) => {
            for item in &use_group.items {
                flatten_use_tree(item, prefix, file, out);
            }
        }
    }
}

/// Every `use` edge declared at module (non-test, non-function-body) scope
/// in one already-parsed file. `#[cfg(test)]`-gated `use` items are
/// included deliberately (see this module's own doc, "Scope of what this
/// resolves"): excluding them would only ever make the scan more permissive
/// than the real risk, never less.
fn use_edges_in_file(parsed: &syn::File, file: &str) -> Vec<UseEdge> {
    let mut out = Vec::new();
    collect_use_edges(&parsed.items, file, &mut out);
    out
}

fn collect_use_edges(items: &[syn::Item], file: &str, out: &mut Vec<UseEdge>) {
    for item in items {
        match item {
            syn::Item::Use(item_use) => {
                flatten_use_tree(&item_use.tree, &[], file, out);
            }
            syn::Item::Mod(item_mod) => {
                if let Some((_, inner_items)) = &item_mod.content {
                    collect_use_edges(inner_items, file, out);
                }
            }
            _ => {}
        }
    }
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

/// Every `use` edge declared at module scope in one already-parsed file,
/// excluding any edge that is itself `#[cfg(test)]`-gated or that sits
/// inside a `#[cfg(test)] mod { ... }` block -- i.e. every edge that is part
/// of the crate's *shipped* (non-test) dependency graph. Used where a bound
/// governs a shipped layering property rather than every line of source text
/// (see [`check_layer_edges`]'s own doc for why it uses this instead of
/// [`use_edges_in_file`]).
fn shipped_use_edges_in_file(parsed: &syn::File, file: &str) -> Vec<UseEdge> {
    let mut out = Vec::new();
    collect_shipped_use_edges(&parsed.items, file, &mut out);
    out
}

fn collect_shipped_use_edges(items: &[syn::Item], file: &str, out: &mut Vec<UseEdge>) {
    for item in items {
        match item {
            syn::Item::Use(item_use) if !has_cfg_test(&item_use.attrs) => {
                flatten_use_tree(&item_use.tree, &[], file, out);
            }
            syn::Item::Mod(item_mod) if !has_cfg_test(&item_mod.attrs) => {
                if let Some((_, inner_items)) = &item_mod.content {
                    collect_shipped_use_edges(inner_items, file, out);
                }
            }
            _ => {}
        }
    }
}

/// The two name sets `src/value/mod.rs`'s own aggregate `use` blocks
/// establish (see this module's own doc): every name a flat
/// `crate::value::Name` import can reach that really resolves into
/// `value::expression`, and every name a flat `crate::value::Name` import
/// can reach that really resolves into `crate::check`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ValueReexports {
    /// Names `value::mod.rs`'s `pub use expression::{...};` block re-exports.
    pub expression: BTreeSet<String>,
    /// Names `value::mod.rs`'s `pub use crate::check::{...};` block re-exports.
    pub check: BTreeSet<String>,
}

/// Read `src/value/mod.rs` and extract [`ValueReexports`] by finding the
/// one `use` item whose path is exactly `expression` (a sibling-relative
/// `pub use expression::{...}`) and the one whose path starts `crate::check`.
pub fn value_reexports(workspace_root: &Path) -> Result<ValueReexports> {
    let parsed = parse_file(workspace_root, "src/value/mod.rs")?;
    let edges = use_edges_in_file(&parsed, "src/value/mod.rs");
    let mut reexports = ValueReexports::default();
    for edge in &edges {
        if edge.is_glob {
            continue;
        }
        match edge.path.as_slice() {
            [module] if module == "expression" => {
                reexports.expression.insert(edge.leaf.clone());
            }
            [first, second] if first == "crate" && second == "check" => {
                reexports.check.insert(edge.leaf.clone());
            }
            _ => {}
        }
    }
    Ok(reexports)
}

/// Every `.rs` file directly under `dir` (non-recursive: `check`'s and
/// `model`'s own submodule layouts are both flat, one file per module, so a
/// recursive walk is not needed and would only pick up unrelated sibling
/// trees if either module ever grew a nested one).
fn files_in(workspace_root: &Path, dir: &str) -> Result<Vec<String>> {
    let full = workspace_root.join(dir);
    let mut files = Vec::new();
    let entries = fs::read_dir(&full).map_err(|source| Error::io(&full, source))?;
    for entry in entries {
        let entry = entry.map_err(|source| Error::io(&full, source))?;
        let path = entry.path();
        if path.extension().is_some_and(|extension| extension == "rs") {
            let relative = path.strip_prefix(workspace_root).unwrap_or(&path);
            files.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
    files.sort();
    Ok(files)
}

/// Whether `path` (a `use` edge's segments, `crate`-rooted or not) resolves,
/// directly or through `value`'s flat aggregate, into `value::expression`.
/// `leaf` is the edge's own bound name.
fn resolves_into_value_expression(path: &[String], leaf: &str, reexports: &ValueReexports) -> bool {
    let normalized = strip_leading_crate(path);
    match normalized {
        // Direct: `crate::value::expression::...`.
        [first, second, ..] if first == "value" && second == "expression" => true,
        // Flat, through value's own aggregate: `crate::value::Name` where
        // `Name` is one of the names `value::mod.rs` re-exports from
        // `expression`.
        [first] if first == "value" => reexports.expression.contains(leaf),
        _ => false,
    }
}

/// Whether `path` resolves, directly or through `value`'s flat aggregate,
/// into `crate::checking` (the pre-existing SEAM-1/SEAM-2 module, untouched
/// by QSL-139).
fn resolves_into_checking(path: &[String]) -> bool {
    let normalized = strip_leading_crate(path);
    matches!(normalized, [first, ..] if first == "checking")
}

/// Whether `path` resolves, directly or through `value`'s flat aggregate,
/// into `crate::check`.
fn resolves_into_check(path: &[String], leaf: &str, reexports: &ValueReexports) -> bool {
    let normalized = strip_leading_crate(path);
    match normalized {
        [first, ..] if first == "check" => true,
        [first] if first == "value" => reexports.check.contains(leaf),
        _ => false,
    }
}

fn strip_leading_crate(path: &[String]) -> &[String] {
    match path.first().map(String::as_str) {
        Some("crate") => &path[1..],
        _ => path,
    }
}

/// One finding: an edge that resolves somewhere TC-172/TC-176 forbids it
/// from resolving.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Violation {
    /// The file the offending `use` was found in.
    pub file: String,
    /// The edge's own source-written path, joined with `::`.
    pub path: String,
    /// The bound leaf name.
    pub leaf: String,
    /// What this edge actually resolves into, and why that is forbidden.
    pub reason: &'static str,
}

/// TC-172: every `use` edge under `src/check/` that resolves, directly or
/// through `value`'s flat aggregate, into `value::expression` or into
/// `checking` -- both forbidden regardless of whether the edge's own
/// spelling contains the word `expression` or `checking` at all.
pub fn check_module_violations(workspace_root: &Path) -> Result<Vec<Violation>> {
    let reexports = value_reexports(workspace_root)?;
    let mut violations = Vec::new();
    for file in files_in(workspace_root, "src/check")? {
        let parsed = parse_file(workspace_root, &file)?;
        for edge in use_edges_in_file(&parsed, &file) {
            if edge.is_glob {
                continue;
            }
            if resolves_into_value_expression(&edge.path, &edge.leaf, &reexports) {
                violations.push(Violation {
                    file: edge.file.clone(),
                    path: edge.path.join("::"),
                    leaf: edge.leaf.clone(),
                    reason: "resolves into value::expression (directly or through value's flat aggregate)",
                });
            }
            if resolves_into_checking(&edge.path) {
                violations.push(Violation {
                    file: edge.file.clone(),
                    path: edge.path.join("::"),
                    leaf: edge.leaf.clone(),
                    reason: "resolves into checking (the pre-existing SEAM-1/SEAM-2 module)",
                });
            }
        }
    }
    Ok(violations)
}

/// One resolved `model` -> `check` edge (TC-262/FR-074-AC-3, inverted from
/// TC-176/FR-068-AC-9, which this retires): which file it was found in, the
/// name it binds (`"*"` for a glob import whose own path resolves into
/// `check` -- a glob binds no single leaf, see [`UseEdge::is_glob`]), and
/// whether it was written directly against `crate::check` or reached
/// indirectly through `value`'s flat aggregate. After M-2 this edge set
/// SHALL be empty everywhere under `src/model/`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelCheckEdge {
    /// The file the edge was found in, relative to the workspace root.
    pub file: String,
    /// The bound leaf name, or `"*"` for a glob edge.
    pub leaf: String,
    /// `true` when the edge is a direct `crate::check::...` import; `false`
    /// when it is a flat `crate::value::Name` import that resolves into
    /// `check` only through `value`'s own aggregate re-export.
    pub direct: bool,
}

/// TC-262: every `use` edge under `src/model/` that resolves, directly or
/// through `value`'s flat aggregate, into `crate::check` -- including a
/// glob (`use crate::check::*;`) whose own path resolves into `check`.
/// `resolves_into_check` doesn't need `leaf` for that branch (it only
/// consults `leaf` for a flat `crate::value::Name` import), so a glob's
/// empty leaf resolves correctly without special-casing; only the *skip*
/// used to be the bug (PR #291 review finding 1: an earlier revision
/// skipped every glob edge outright before ever calling
/// `resolves_into_check`, so a `use crate::check::*;` planted under
/// `src/model/` passed `real_model_check_edge_is_empty` completely
/// undetected -- a genuine gate hole, not a false alarm; confirmed live
/// against the real tree and reverted, see
/// `glob_import_into_check_is_recorded_not_skipped` below for the pinned
/// fixture).
pub fn model_check_edges(workspace_root: &Path) -> Result<Vec<ModelCheckEdge>> {
    let reexports = value_reexports(workspace_root)?;
    let mut edges_found = Vec::new();
    for file in files_in(workspace_root, "src/model")? {
        let parsed = parse_file(workspace_root, &file)?;
        for edge in use_edges_in_file(&parsed, &file) {
            if resolves_into_check(&edge.path, &edge.leaf, &reexports) {
                let direct =
                    strip_leading_crate(&edge.path).first().map(String::as_str) == Some("check");
                let leaf = if edge.is_glob {
                    "*".to_string()
                } else {
                    edge.leaf.clone()
                };
                edges_found.push(ModelCheckEdge {
                    file: edge.file.clone(),
                    leaf,
                    direct,
                });
            }
        }
    }
    Ok(edges_found)
}

/// Every name `value::mod.rs` re-exports from one of its own *sibling*
/// submodules via a bare (non-`crate`-rooted) `pub use <submodule>::{...}`
/// line -- `crate::check`'s and `crate::forms`'s cross-module re-exports
/// have a two-segment path (`crate::X`) and are excluded by construction,
/// since TC-175 cares which `value::` submodule an item belongs to, not
/// `check`'s or `forms`'s own re-exports.
fn value_submodule_reexports(workspace_root: &Path) -> Result<BTreeMap<String, String>> {
    let parsed = parse_file(workspace_root, "src/value/mod.rs")?;
    let edges = use_edges_in_file(&parsed, "src/value/mod.rs");
    let mut map = BTreeMap::new();
    for edge in &edges {
        if edge.is_glob {
            continue;
        }
        if let [module] = edge.path.as_slice() {
            map.insert(edge.leaf.clone(), module.clone());
        }
    }
    Ok(map)
}

// ---------------------------------------------------------------------
// FR-068-AC-6, the layer-rule ruling (2026-09-22): `check`'s imports are
// bounded by module and layer, not by item. This retires the former
// tier-1/tier-2 item lists (`K_DESIGNATED_MODULES`, `DECLARED_INTERIM_ITEMS`)
// entirely -- the bound below is per module, and the number of items
// imported from a permitted module is not checked.
// ---------------------------------------------------------------------

/// FR-068-AC-6's permitted-module list (closed): any item of a listed
/// module is allowed. A `value::<submodule>` entry names one specific
/// submodule; every other entry is matched by its own name and all of its
/// descendants (`check` and its descendants; `library` and its
/// descendants; and so on) -- see [`module_or_descendant`].
const LAYER_PERMITTED_MODULES: &[&str] = &[
    // K, F.
    "quire_exact",
    "qsl_foundation",
    // Layer 2.
    "forms",
    // Layer 3, before `check` core (ADR-011 §6.1: `semantic_value < model <
    // library < check core`).
    "value::definition",
    "value::enumeration",
    "value::unit",
    "value::quantity",
    "value::key",
    "value::reference",
    "value::containment",
    "value::declaration",
    "model",
    "value::model_query",
    "library",
    // Layer 3, `check` core itself.
    "check",
    "family",
    // The `value` K-copy modules, each only while it exists (this list
    // only shrinks; a module leaves it in the change that deletes that
    // module's QSL copy).
    "value::collection",
    "value::comparison",
    "value::composite",
    "value::decimal",
    "value::division",
    "value::equality",
    "value::ieee",
    "value::node",
    "value::numeric",
    "value::outcome",
    "value::rational",
    "value::text",
];

/// FR-068-AC-6's MUST NOT list (closed): a later layer, forbidden including
/// any descendant.
const LAYER_FORBIDDEN_MODULES: &[&str] = &[
    "checked_package",
    "package",
    "value::expression",
    "route",
    "replay",
    "lowering",
];

/// Whether `module` is exactly `entry` or a `::`-segment descendant of it
/// (`model_query` is not a descendant of `model` by this rule -- only a
/// `::` boundary counts, the same segment-matching FR-060 T12-B/T12-C use).
fn module_or_descendant(module: &str, entry: &str) -> bool {
    module == entry || module.starts_with(&format!("{entry}::"))
}

/// FR-068-AC-6's layer classification of one resolved module.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayerClass {
    /// On the permitted list (or a descendant of a permitted entry).
    Permitted,
    /// On the MUST NOT list (or a descendant of one) -- a later layer.
    Forbidden,
    /// On neither list.
    Unlisted,
}

fn classify_layer_module(module: &str) -> LayerClass {
    if LAYER_FORBIDDEN_MODULES
        .iter()
        .any(|entry| module_or_descendant(module, entry))
    {
        LayerClass::Forbidden
    } else if LAYER_PERMITTED_MODULES
        .iter()
        .any(|entry| module_or_descendant(module, entry))
    {
        LayerClass::Permitted
    } else {
        LayerClass::Unlisted
    }
}

/// Whether `name` is a real top-level module of this crate -- a file or
/// directory directly under `src/` -- as opposed to `std` or a third-party
/// crate, which FR-068-AC-6 does not classify at all.
fn is_real_crate_module(workspace_root: &Path, name: &str) -> bool {
    let src = workspace_root.join("src");
    src.join(format!("{name}.rs")).is_file() || src.join(name).is_dir()
}

/// Whether `name` is a real submodule of `value` -- a file or directory
/// directly under `src/value/` -- used to tell a genuinely nested
/// `crate::value::<submodule>::Name` path apart from a flat
/// `crate::value::Name` one at the inline-path level, where (unlike a `use`
/// line) source syntax alone does not separate a path's module prefix from
/// its bound item.
fn is_real_value_submodule(workspace_root: &Path, name: &str) -> bool {
    let value_dir = workspace_root.join("src/value");
    value_dir.join(format!("{name}.rs")).is_file() || value_dir.join(name).is_dir()
}

/// Whether `top` (a resolved path's first segment) is in FR-068-AC-6's
/// scope at all: `quire_exact`/`qsl_foundation` by name, or any other real
/// module of this crate. `std` and every other third-party crate are out of
/// scope and return `false`.
fn in_layer_rule_scope(workspace_root: &Path, top: &str) -> bool {
    top == "quire_exact" || top == "qsl_foundation" || is_real_crate_module(workspace_root, top)
}

/// Resolve `top` (a resolved path's first segment) and `next` (its second,
/// if any) into FR-068-AC-6's module label and whether the edge names its
/// submodule. Only `value` needs `next` resolved further: every other
/// permitted/forbidden entry is matched on `top` alone (`module_or_descendant`
/// covers the rest of the path).
///
/// `next_is_module_by_syntax` is `true` when the caller already knows `next`
/// is a real module segment because the source syntax itself guarantees it
/// (a `use` edge's own path, where every segment but the bound leaf is a
/// module by construction); it is `false` for an inline path, where syntax
/// alone cannot tell a submodule segment (`composite` in
/// `crate::value::composite::Presence`) apart from a flat aggregate item
/// (`Presence` in `crate::value::Presence::Optional`) -- that case falls
/// back to a filesystem check and then `value::mod.rs`'s own re-export
/// table, the same two-step resolution the flat-`use` case already needed.
fn resolve_layer_module(
    workspace_root: &Path,
    submodule_reexports: &BTreeMap<String, String>,
    top: &str,
    next: Option<&str>,
    next_is_module_by_syntax: bool,
) -> (String, bool) {
    if top != "value" {
        return (top.to_owned(), true);
    }
    let Some(next) = next else {
        return ("value".to_owned(), false);
    };
    if next_is_module_by_syntax || is_real_value_submodule(workspace_root, next) {
        return (format!("value::{next}"), true);
    }
    match submodule_reexports.get(next) {
        Some(target) => (format!("value::{target}"), false),
        None => ("value".to_owned(), false),
    }
}

/// Resolve a `use` edge's or inline path's own written segments against
/// `current_module` (the scanning file's own crate-relative module path),
/// stripping a leading `crate` and resolving a leading `super`/`self`
/// relative to the file (FR-068-AC-6: "A `super::` or `self::` path is
/// resolved relative to its file"). A path rooted at anything else (an
/// extern crate name, `quire_exact`/`qsl_foundation` included) is returned
/// unchanged -- it needs no crate-relative substitution.
fn resolve_relative_path(raw: &[String], current_module: &[String]) -> Vec<String> {
    match raw.first().map(String::as_str) {
        Some("crate") => raw[1..].to_vec(),
        Some("super") => {
            let mut base = current_module.to_vec();
            let mut rest = raw;
            while rest.first().map(String::as_str) == Some("super") {
                base.pop();
                rest = &rest[1..];
            }
            base.extend(rest.iter().cloned());
            base
        }
        Some("self") => {
            let mut base = current_module.to_vec();
            base.extend(raw[1..].iter().cloned());
            base
        }
        _ => raw.to_vec(),
    }
}

/// One FR-068-AC-6 layer-rule edge: a `use` line or inline
/// `crate::`/`super::`/`self::` path under `src/check/`, resolved and
/// classified (TC-175).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayerEdge {
    /// The file this edge was found in.
    pub file: String,
    /// 1-based source line.
    pub line: usize,
    /// The resolved module label (e.g. `"value::composite"`, `"model"`).
    pub module: String,
    /// Permitted, forbidden, or unlisted.
    pub class: LayerClass,
    /// Whether a `value` edge names its submodule (`crate::value::
    /// <submodule>::Name`) rather than reaching it through `value`'s flat
    /// aggregate (`crate::value::Name`). Always `true` for a non-`value`
    /// edge, where the rule does not apply.
    pub submodule_qualified: bool,
}

impl LayerEdge {
    /// Whether this edge violates FR-068-AC-6: its module is not permitted,
    /// or it is a `value` edge that does not name its submodule. The flat
    /// form fails even when the submodule it actually reaches is itself
    /// permitted (TC-175: `crate::value::Rational` still fails, though
    /// `value::rational` is a K-copy module).
    pub fn is_violation(&self) -> bool {
        self.class != LayerClass::Permitted || !self.submodule_qualified
    }
}

/// Classify one `use` edge already found under `src/check/`. `None` when
/// the edge's resolved root is `std` or a third-party crate other than
/// `quire_exact`/`qsl_foundation` -- out of FR-068-AC-6's scope entirely,
/// not a finding.
fn classify_use_edge(
    workspace_root: &Path,
    submodule_reexports: &BTreeMap<String, String>,
    current_module: &[String],
    edge: &UseEdge,
) -> Option<LayerEdge> {
    let resolved = resolve_relative_path(&edge.path, current_module);
    let top = resolved.first()?.clone();
    if !in_layer_rule_scope(workspace_root, &top) {
        return None;
    }
    let next = if resolved.len() >= 2 {
        Some(resolved[1].as_str())
    } else if edge.is_glob {
        None
    } else {
        Some(edge.leaf.as_str())
    };
    let next_is_module_by_syntax = resolved.len() >= 2;
    let (module, submodule_qualified) = resolve_layer_module(
        workspace_root,
        submodule_reexports,
        &top,
        next,
        next_is_module_by_syntax,
    );
    let class = classify_layer_module(&module);
    Some(LayerEdge {
        file: edge.file.clone(),
        line: edge.line,
        module,
        class,
        submodule_qualified,
    })
}

/// Classify one inline `crate::`/`super::`/`self::` path found under
/// `src/check/`. `None` when out of scope, the same as
/// [`classify_use_edge`].
fn classify_inline_edge(
    workspace_root: &Path,
    submodule_reexports: &BTreeMap<String, String>,
    current_module: &[String],
    file: &str,
    line: usize,
    raw_segments: &[String],
) -> Option<LayerEdge> {
    let resolved = resolve_relative_path(raw_segments, current_module);
    let top = resolved.first()?.clone();
    if !in_layer_rule_scope(workspace_root, &top) {
        return None;
    }
    let next = resolved.get(1).map(String::as_str);
    let (module, submodule_qualified) =
        resolve_layer_module(workspace_root, submodule_reexports, &top, next, false);
    let class = classify_layer_module(&module);
    Some(LayerEdge {
        file: file.to_owned(),
        line,
        module,
        class,
        submodule_qualified,
    })
}

/// The crate-relative module path segments for a `.rs` file, relative to
/// the workspace root (`src/check/check.rs` -> `["check", "check"]`;
/// `src/check/mod.rs` -> `["check"]`; `src/lib.rs` -> `[]`) -- Rust's own
/// `mod.rs`/`foo.rs` file-to-module convention, needed to resolve a
/// `super::`/`self::` path relative to its file.
fn module_segments_of(relative_file: &str) -> Vec<String> {
    let mut segments: Vec<String> = Path::new(relative_file)
        .with_extension("")
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect();
    if segments.first().map(String::as_str) == Some("src") {
        segments.remove(0);
    }
    if segments.last().map(String::as_str) == Some("mod") {
        segments.pop();
    }
    if segments.len() == 1 && matches!(segments[0].as_str(), "lib" | "main") {
        segments.clear();
    }
    segments
}

/// Every `.rs` file under `dir`, recursive -- unlike [`files_in`], which is
/// non-recursive because `check`'s and `model`'s layouts happen to be flat
/// today. FR-068-AC-6's layer rule explicitly covers "`check` and its
/// descendants," so this scan must not stop at the first level if `check`
/// ever grows a nested submodule directory.
fn files_in_recursive(workspace_root: &Path, dir: &str) -> Result<Vec<String>> {
    let mut files = Vec::new();
    walk_rs_files_recursive(&workspace_root.join(dir), workspace_root, &mut files)?;
    files.sort();
    Ok(files)
}

fn walk_rs_files_recursive(dir: &Path, workspace_root: &Path, out: &mut Vec<String>) -> Result<()> {
    let entries = fs::read_dir(dir).map_err(|source| Error::io(dir, source))?;
    for entry in entries {
        let entry = entry.map_err(|source| Error::io(dir, source))?;
        let path = entry.path();
        if path.is_dir() {
            walk_rs_files_recursive(&path, workspace_root, out)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let relative = path.strip_prefix(workspace_root).unwrap_or(&path);
            out.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}

/// One inline `crate::`/`super::`/`self::`-rooted path found in shipped
/// (non-`#[cfg(test)]`) code.
struct InlinePath {
    line: usize,
    segments: Vec<String>,
}

/// Walks a parsed file collecting every inline path rooted at `crate`,
/// `super` or `self`, skipping any item gated `#[cfg(test)]` -- mirroring
/// [`shipped_use_edges_in_file`]'s scope at the expression/type level, which
/// that function's own item-list walk cannot reach. `visit_path` is not
/// triggered by a `use` item's own tree (`syn` models `use` paths as
/// `UseTree`, a distinct grammar), so this never double-counts a `use` edge.
struct InlinePathVisitor {
    found: Vec<InlinePath>,
}

impl InlinePathVisitor {
    fn record(&mut self, path: &syn::Path) {
        let Some(first) = path.segments.first() else {
            return;
        };
        if !matches!(first.ident.to_string().as_str(), "crate" | "super" | "self") {
            return;
        }
        let line = first.ident.span().start().line;
        let segments = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect();
        self.found.push(InlinePath { line, segments });
    }
}

impl<'ast> Visit<'ast> for InlinePathVisitor {
    fn visit_path(&mut self, node: &'ast syn::Path) {
        self.record(node);
        syn::visit::visit_path(self, node);
    }

    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if !has_cfg_test(&node.attrs) {
            syn::visit::visit_item_mod(self, node);
        }
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        if !has_cfg_test(&node.attrs) {
            syn::visit::visit_item_fn(self, node);
        }
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        if !has_cfg_test(&node.attrs) {
            syn::visit::visit_item_impl(self, node);
        }
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        if !has_cfg_test(&node.attrs) {
            syn::visit::visit_impl_item_fn(self, node);
        }
    }

    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        if !has_cfg_test(&node.attrs) {
            syn::visit::visit_item_struct(self, node);
        }
    }

    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        if !has_cfg_test(&node.attrs) {
            syn::visit::visit_item_enum(self, node);
        }
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        if !has_cfg_test(&node.attrs) {
            syn::visit::visit_item_trait(self, node);
        }
    }

    fn visit_item_static(&mut self, node: &'ast syn::ItemStatic) {
        if !has_cfg_test(&node.attrs) {
            syn::visit::visit_item_static(self, node);
        }
    }

    fn visit_item_const(&mut self, node: &'ast syn::ItemConst) {
        if !has_cfg_test(&node.attrs) {
            syn::visit::visit_item_const(self, node);
        }
    }
}

fn shipped_inline_paths_in_file(parsed: &syn::File) -> Vec<InlinePath> {
    let mut visitor = InlinePathVisitor { found: Vec::new() };
    visitor.visit_file(parsed);
    visitor.found
}

/// FR-068-AC-6/TC-175 (the layer-rule ruling, 2026-09-22): every shipped
/// `use` line and inline `crate::`/`super::`/`self::` path under
/// `src/check/`, classified against the module-level layer rule. Replaces
/// the former `value_import_edges`/`check_package_import_edges` item-tier
/// scan entirely -- the bound is per module now, so one scan covers both
/// `value`'s submodules and the `package`/`checked_package`/`route`/
/// `replay`/`lowering` forbidden list together.
///
/// **Known limitation, shared with `tools/arch-lint`'s textual scan:** a
/// path reached only through a `use ... as` rename at the call site, or a
/// path written inside a macro invocation's own token stream (`syn` does
/// not parse an arbitrary macro's arguments into expressions), is not
/// resolved by this scan.
pub fn check_layer_edges(workspace_root: &Path) -> Result<Vec<LayerEdge>> {
    let submodule_reexports = value_submodule_reexports(workspace_root)?;
    let mut edges = Vec::new();
    for file in files_in_recursive(workspace_root, "src/check")? {
        let parsed = parse_file(workspace_root, &file)?;
        let current_module = module_segments_of(&file);
        for edge in shipped_use_edges_in_file(&parsed, &file) {
            if let Some(layer_edge) =
                classify_use_edge(workspace_root, &submodule_reexports, &current_module, &edge)
            {
                edges.push(layer_edge);
            }
        }
        for inline in shipped_inline_paths_in_file(&parsed) {
            if let Some(layer_edge) = classify_inline_edge(
                workspace_root,
                &submodule_reexports,
                &current_module,
                &file,
                inline.line,
                &inline.segments,
            ) {
                edges.push(layer_edge);
            }
        }
    }
    edges.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    Ok(edges)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use std::path::PathBuf;

    fn workspace_root() -> PathBuf {
        // `xtask`'s own manifest dir is `<workspace_root>/xtask`.
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask has a parent directory")
            .to_path_buf()
    }

    /// A flat `use crate::value::Evaluation;` binds a name `value::mod.rs`
    /// re-exports from `expression` -- the exact hidden-edge shape TC-172
    /// step 5 requires catching at the resolved level, invisible to any
    /// scan of the edge's own text (it contains neither `expression` nor
    /// any of the seven flagged example names' shared substring).
    #[trace("TC-172", "FR-068-AC-3")]
    #[test]
    fn flat_value_import_of_an_expression_reexport_resolves_into_expression() {
        let reexports = ValueReexports {
            expression: ["Evaluation".to_owned()].into_iter().collect(),
            check: BTreeSet::new(),
        };
        let path = vec!["crate".to_owned(), "value".to_owned()];
        assert!(resolves_into_value_expression(
            &path,
            "Evaluation",
            &reexports
        ));
    }

    /// The same flat-import shape for an *unrelated* name (one `value`
    /// re-exports from somewhere other than `expression`) must not be
    /// flagged -- this resolver targets the real CON-4 aggregate edge, not
    /// every `crate::value::` import.
    #[trace("TC-172", "FR-068-AC-3")]
    #[test]
    fn flat_value_import_of_a_non_expression_name_does_not_resolve_into_expression() {
        let reexports = ValueReexports {
            expression: ["Evaluation".to_owned()].into_iter().collect(),
            check: BTreeSet::new(),
        };
        let path = vec!["crate".to_owned(), "value".to_owned()];
        assert!(!resolves_into_value_expression(
            &path,
            "CollectionType",
            &reexports
        ));
    }

    /// A direct `use crate::value::expression::Foo;` resolves into
    /// `value::expression` regardless of the reexport table -- the other
    /// half of TC-172 step 3 ("the whole module, not a fixed name list").
    #[trace("TC-172", "FR-068-AC-3")]
    #[test]
    fn direct_value_expression_import_resolves_into_expression() {
        let reexports = ValueReexports::default();
        let path = vec![
            "crate".to_owned(),
            "value".to_owned(),
            "expression".to_owned(),
        ];
        assert!(resolves_into_value_expression(
            &path, "Anything", &reexports
        ));
    }

    /// TC-172 step 4: an edge into the pre-existing `checking` module is
    /// flagged the same way, independent of the reexport table.
    #[trace("TC-172", "FR-068-AC-3")]
    #[test]
    fn checking_module_import_is_flagged() {
        let path = vec!["crate".to_owned(), "checking".to_owned()];
        assert!(resolves_into_checking(&path));
        let unrelated = vec!["crate".to_owned(), "value".to_owned()];
        assert!(!resolves_into_checking(&unrelated));
    }

    /// The resolution mechanism `real_model_check_edge_is_empty` (TC-262)
    /// relies on: a flat `use crate::value::DispatchTable;` inside `model`
    /// resolves into `check` only because `value::mod.rs` re-exports it
    /// from there, and is recorded indirect (`direct: false`). Retagged
    /// from TC-176/FR-068-AC-9 (retired by FR-074): the assertion this test
    /// backs -- that such a form must be absent from `model` -- moved to
    /// TC-262's empty-set requirement, but the resolution logic itself is
    /// unchanged and still needs its own unit coverage.
    #[trace("TC-262", "FR-074-AC-3")]
    #[test]
    fn flat_value_import_of_a_check_reexport_resolves_into_check_indirectly() {
        let reexports = ValueReexports {
            expression: BTreeSet::new(),
            check: ["DispatchTable".to_owned()].into_iter().collect(),
        };
        let path = vec!["crate".to_owned(), "value".to_owned()];
        assert!(resolves_into_check(&path, "DispatchTable", &reexports));
        let edge = ModelCheckEdge {
            file: "src/model/checked_dispatch.rs".to_owned(),
            leaf: "DispatchTable".to_owned(),
            direct: strip_leading_crate(&path).first().map(String::as_str) == Some("check"),
        };
        assert!(!edge.direct, "a flat value import must record as indirect");
    }

    /// The other half of the resolution mechanism `real_model_check_edge_is_empty`
    /// (TC-262) relies on: a direct `use crate::check::DispatchTable;` is
    /// recorded `direct: true`. Retagged from TC-176/FR-068-AC-9 (retired
    /// by FR-074) for the same reason as the test above.
    #[trace("TC-262", "FR-074-AC-3")]
    #[test]
    fn direct_check_import_is_recorded_as_direct() {
        let path = vec![
            "crate".to_owned(),
            "check".to_owned(),
            "DispatchTable".to_owned(),
        ];
        let reexports = ValueReexports::default();
        assert!(resolves_into_check(&path, "DispatchTable", &reexports));
        let direct = strip_leading_crate(&path).first().map(String::as_str) == Some("check");
        assert!(direct);
    }

    /// Against the real, current tree: `check_module_violations` is empty --
    /// this is TC-172's own live assertion, run over `check`'s actual
    /// source rather than a fixture, confirming the resolver finds nothing
    /// where the real crate has nothing after PR #282 review F2's rewrite
    /// to submodule-qualified imports.
    #[trace("TC-172", "FR-068-AC-3")]
    #[test]
    fn real_check_module_has_no_value_expression_or_checking_edge() {
        let violations = check_module_violations(&workspace_root()).expect("scan runs");
        assert!(
            violations.is_empty(),
            "check_module_violations found real edges: {violations:?}"
        );
    }

    /// Against the real, current tree: `model_check_edges` is empty -- M-2
    /// (this ticket, QSL-7) moved `model::checked_dispatch` and
    /// `model::conformance::check_field_refinement_obligation`, the only
    /// code that made `model` depend on `check`, into `check` itself, so
    /// the interim edge FR-068-AC-9/FR-068-CON-5 bounded to two files and
    /// thirteen names is now bounded to zero files and zero names -- TC-262
    /// steps 1-2 run against the real crate, not a fixture. Inverted from
    /// FR-068-AC-9's pre-M-2 bounded-but-nonempty assertion. `model_check_edges`
    /// no longer skips glob edges (see its own doc and the fixture
    /// immediately below), so this assertion covers a glob import under
    /// `src/model/` too, not only named-leaf ones.
    #[trace("TC-262", "FR-074-AC-3")]
    #[test]
    fn real_model_check_edge_is_empty() {
        let edges = model_check_edges(&workspace_root()).expect("scan runs");
        assert!(
            edges.is_empty(),
            "model -> check edge must be fully closed after M-2: {edges:?}"
        );
    }

    /// TC-262 steps 3-4/FR-074-AC-3: `model_check_edges` must not silently
    /// skip a glob edge. PR #291 review finding 1 planted `use
    /// crate::check::*;` under `src/model/` against the real tree and
    /// confirmed `real_model_check_edge_is_empty` stayed green -- a genuine
    /// gate hole, since the pre-fix loop skipped every glob edge before
    /// ever calling `resolves_into_check`. Reverted once the fix (removing
    /// that early skip, above) turned the same planted glob red. This
    /// fixture pins the fix permanently, at the level `model_check_edges`
    /// itself relies on: a glob whose own path resolves into `check`
    /// resolves the same as a named import would, with no special-casing
    /// needed for its empty leaf.
    #[trace("TC-262", "FR-074-AC-3")]
    #[test]
    fn glob_import_into_check_is_recorded_not_skipped() {
        let source = r#"
            use crate::check::*;
        "#;
        let parsed = syn::parse_file(source).expect("fixture parses");
        let edges = use_edges_in_file(&parsed, "src/model/fixture.rs");
        assert_eq!(edges.len(), 1);
        assert!(edges[0].is_glob);
        let reexports = ValueReexports::default();
        assert!(
            resolves_into_check(&edges[0].path, &edges[0].leaf, &reexports),
            "a glob import whose own path resolves into `check` must not be skipped"
        );
    }

    /// `value_reexports` against the real tree finds both aggregate blocks
    /// non-empty, confirming this resolver is reading `value::mod.rs`'s
    /// actual current content, not silently matching nothing.
    #[test]
    fn real_value_reexports_are_non_empty() {
        let reexports = value_reexports(&workspace_root()).expect("scan runs");
        assert!(reexports.expression.contains("Evaluation"));
        assert!(reexports.check.contains("DispatchTable"));
    }

    // -------------------------------------------------------------------
    // FR-068-AC-6 (TC-175): the layer-rule ruling (2026-09-22). The former
    // tier-classification tests above this point are retired along with
    // `value_import_edges`/`check_package_import_edges` themselves -- one
    // scan (`check_layer_edges`) now covers both `value`'s submodules and
    // the `package`/`checked_package`/`route`/`replay`/`lowering` forbidden
    // list together.
    // -------------------------------------------------------------------

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    /// A minimal QSL-shaped fixture tree: a bare marker file for every
    /// non-`value` module FR-068-AC-6 names (so [`is_real_crate_module`]
    /// can tell each apart from an external crate), every named `value`
    /// submodule (K-copy and `semantic_value` alike), plus `value::member`
    /// and `value::expression` -- two real submodules the rule deliberately
    /// leaves off both lists (the first permanently unlisted, the second
    /// forbidden).
    fn layer_fixture_root() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for module in [
            "model",
            "library",
            "forms",
            "family",
            "checked_package",
            "package",
            "route",
            "replay",
            "lowering",
            "checking",
        ] {
            write(dir.path(), &format!("src/{module}.rs"), "");
        }
        for submodule in [
            "definition",
            "enumeration",
            "unit",
            "quantity",
            "key",
            "reference",
            "containment",
            "collection",
            "comparison",
            "composite",
            "decimal",
            "division",
            "equality",
            "ieee",
            "node",
            "numeric",
            "outcome",
            "rational",
            "text",
            "model_query",
            "member",
        ] {
            write(dir.path(), &format!("src/value/{submodule}.rs"), "");
        }
        write(dir.path(), "src/value/expression/mod.rs", "");
        write(dir.path(), "src/value/mod.rs", "");
        dir
    }

    /// Against the real, current tree: every shipped edge under
    /// `src/check/` -- `use` line and inline path alike -- is permitted,
    /// and every `value` edge names its submodule. This is TC-175 steps 1-2
    /// against the module-level layer rule (the former tier assertion is
    /// retired above). Red against `main` (the four flat
    /// `crate::value::Presence::Optional` inline paths at `check.rs` lines
    /// 1620, 1653, 1854 and 1867), green once they are repointed at
    /// `crate::value::composite::Presence`/`Presence` brought into scope.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn real_check_layer_edges_have_no_violation() {
        let edges = check_layer_edges(&workspace_root()).expect("scan runs");
        assert!(
            !edges.is_empty(),
            "fixture sanity: check must import something real"
        );
        for edge in &edges {
            assert!(!edge.is_violation(), "layer violation found: {edge:?}");
        }
    }

    /// TC-175 step 6: a shipped `use crate::checked_package::CheckedPackage;`
    /// under `src/check/` fails, naming the file, line and resolved module.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn forbidden_use_edge_is_a_violation() {
        let dir = layer_fixture_root();
        write(
            dir.path(),
            "src/check/fixture.rs",
            "use crate::checked_package::CheckedPackage;\n",
        );
        let edges = check_layer_edges(dir.path()).expect("scan runs");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].module, "checked_package");
        assert_eq!(edges[0].class, LayerClass::Forbidden);
        assert_eq!(edges[0].line, 1);
        assert!(edges[0].is_violation());
    }

    /// TC-175 step 6: a shipped inline `use crate::package::*;` glob fails
    /// the same way a named import would -- `package` has no allow-list to
    /// check a leaf against, so a glob must still be caught (retiring the
    /// dedicated `check_package_import_edges` glob fixtures this scan now
    /// subsumes).
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn forbidden_glob_use_edge_is_a_violation() {
        let dir = layer_fixture_root();
        write(
            dir.path(),
            "src/check/fixture.rs",
            "use crate::package::*;\n",
        );
        let edges = check_layer_edges(dir.path()).expect("scan runs");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].module, "package");
        assert!(edges[0].is_violation());
    }

    /// TC-175 step 6: a shipped inline path `crate::value::expression::
    /// Evaluation` with no `use` line fails -- an inline path into a later
    /// layer is the same forbidden edge a `use` line would be.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn forbidden_inline_path_is_a_violation() {
        let dir = layer_fixture_root();
        write(
            dir.path(),
            "src/check/fixture.rs",
            "pub fn f() -> crate::value::expression::Evaluation {\n    todo!()\n}\n",
        );
        let edges = check_layer_edges(dir.path()).expect("scan runs");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].module, "value::expression");
        assert_eq!(edges[0].class, LayerClass::Forbidden);
        assert!(edges[0].is_violation());
    }

    /// TC-175 step 6: a shipped `use crate::value::Rational;` (the flat
    /// aggregate) fails even though `value::rational` -- the submodule it
    /// actually reaches -- is itself a permitted K-copy module. The flat
    /// form itself is what FR-068-AC-6 forbids, independent of the target.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn flat_value_use_import_is_a_violation_even_when_the_target_is_permitted() {
        let dir = layer_fixture_root();
        write(
            dir.path(),
            "src/value/mod.rs",
            "pub use rational::Rational;\n",
        );
        write(
            dir.path(),
            "src/check/fixture.rs",
            "use crate::value::Rational;\n",
        );
        let edges = check_layer_edges(dir.path()).expect("scan runs");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].module, "value::rational");
        assert_eq!(edges[0].class, LayerClass::Permitted);
        assert!(!edges[0].submodule_qualified);
        assert!(edges[0].is_violation());
    }

    /// TC-175's own flat-inline-path fixture: a shipped inline flat path
    /// `crate::value::Presence::Optional` fails, resolved to
    /// `value::composite` (the module `Presence` actually belongs to)
    /// through `value::mod.rs`'s own re-export table -- exactly the shape
    /// `src/check/check.rs` carried at lines 1620, 1653, 1854 and 1867
    /// before this change repointed them.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn flat_inline_value_path_is_a_violation() {
        let dir = layer_fixture_root();
        write(
            dir.path(),
            "src/value/mod.rs",
            "pub use composite::Presence;\n",
        );
        write(
            dir.path(),
            "src/check/fixture.rs",
            "pub fn f() -> u8 {\n    match crate::value::Presence::Optional {\n        _ => 0,\n    }\n}\n",
        );
        let edges = check_layer_edges(dir.path()).expect("scan runs");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].module, "value::composite");
        assert!(!edges[0].submodule_qualified);
        assert!(edges[0].is_violation());
    }

    /// TC-175 step 6 / Behavior ("A `value` submodule this section does not
    /// name (today `value::member`) is unlisted"): a shipped
    /// `use crate::value::member::SomeThing;` fails as unlisted, even
    /// though `value::member` is a real, submodule-qualified import (not a
    /// flat one) -- being on neither list is its own failure.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn unlisted_value_submodule_is_a_violation() {
        let dir = layer_fixture_root();
        write(
            dir.path(),
            "src/check/fixture.rs",
            "use crate::value::member::SomeThing;\n",
        );
        let edges = check_layer_edges(dir.path()).expect("scan runs");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].module, "value::member");
        assert_eq!(edges[0].class, LayerClass::Unlisted);
        assert!(edges[0].submodule_qualified);
        assert!(edges[0].is_violation());
    }

    /// TC-175 step 6: the same forbidden import inside a `#[cfg(test)]`
    /// item does not fail -- shipped-only scope.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn cfg_test_forbidden_import_is_not_reported() {
        let dir = layer_fixture_root();
        write(
            dir.path(),
            "src/check/fixture.rs",
            "#[cfg(test)]\nmod tests {\n    use crate::checked_package::CheckedPackage;\n}\n",
        );
        let edges = check_layer_edges(dir.path()).expect("scan runs");
        assert!(edges.is_empty(), "{edges:?}");
    }

    /// TC-175 step 6: an additional item from a permitted module (a new
    /// name from `crate::value::quantity`) does not fail -- the number of
    /// items imported from a permitted module is not checked.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn extra_item_from_a_permitted_module_is_not_a_violation() {
        let dir = layer_fixture_root();
        write(
            dir.path(),
            "src/check/fixture.rs",
            "use crate::value::quantity::BrandNewQuantityItem;\n",
        );
        let edges = check_layer_edges(dir.path()).expect("scan runs");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].module, "value::quantity");
        assert_eq!(edges[0].class, LayerClass::Permitted);
        assert!(edges[0].submodule_qualified);
        assert!(!edges[0].is_violation());
    }

    /// `std` and third-party crates are out of FR-068-AC-6's scope entirely
    /// -- not classified, and never reported, even though they are on
    /// neither list.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn external_crate_use_is_out_of_scope() {
        let dir = layer_fixture_root();
        write(
            dir.path(),
            "src/check/fixture.rs",
            "use std::collections::BTreeMap;\nuse serde_json::Value;\n",
        );
        let edges = check_layer_edges(dir.path()).expect("scan runs");
        assert!(edges.is_empty(), "{edges:?}");
    }

    /// FR-068-AC-6: "A `super::` or `self::` path is resolved relative to
    /// its file." `super::` climbs one module level from the file's own
    /// crate-relative module path; `self::` stays at it; `crate::` strips
    /// to the crate root regardless of the current file.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn resolve_relative_path_resolves_super_self_and_crate() {
        let current = vec!["check".to_owned(), "check".to_owned()];
        assert_eq!(
            resolve_relative_path(&["super".to_owned(), "family".to_owned()], &current),
            vec!["check", "family"]
        );
        assert_eq!(
            resolve_relative_path(&["self".to_owned(), "Foo".to_owned()], &current),
            vec!["check", "check", "Foo"]
        );
        assert_eq!(
            resolve_relative_path(&["crate".to_owned(), "value".to_owned()], &current),
            vec!["value"]
        );
    }

    /// `module_segments_of` mirrors Rust's own `mod.rs`/`foo.rs`
    /// file-to-module convention, the basis [`resolve_relative_path`] needs
    /// to resolve a `super::`/`self::` path relative to its file.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn module_segments_of_matches_the_mod_rs_convention() {
        assert_eq!(
            module_segments_of("src/check/check.rs"),
            vec!["check", "check"]
        );
        assert_eq!(module_segments_of("src/check/mod.rs"), vec!["check"]);
        assert_eq!(module_segments_of("src/lib.rs"), Vec::<String>::new());
    }
}
