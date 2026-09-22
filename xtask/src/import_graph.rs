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
//! because of this). [`value_import_edges`] (TC-175/AC-6), by contrast,
//! excludes `#[cfg(test)]`-gated imports on purpose: AC-6's tier bound
//! constrains `check`'s *shipped* dependency graph, and a test-only import is
//! not part of it -- see [`value_import_edges`]'s own doc for the concrete
//! case (`TextType`) this distinction was written to resolve correctly.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

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
                });
            } else {
                out.push(UseEdge {
                    file: file.to_owned(),
                    path: prefix.to_vec(),
                    leaf: ident,
                    is_glob: false,
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
            });
        }
        syn::UseTree::Glob(_) => {
            out.push(UseEdge {
                file: file.to_owned(),
                path: prefix.to_vec(),
                leaf: String::new(),
                is_glob: true,
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
/// (see [`value_import_edges`]'s own doc for why tier 2 uses this instead of
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

/// FR-068-AC-6's tier-1 allow-list: the nine K-designated siblings X-1 has
/// not yet relocated, unbounded in which items `check` imports from them.
const K_DESIGNATED_MODULES: [&str; 9] = [
    "collection",
    "comparison",
    "composite",
    "decimal",
    "equality",
    "ieee",
    "node",
    "numeric",
    "rational",
];

/// FR-068-AC-6's tier-2 allow-list, amended once by the PR #282 review
/// findings: F3 widened it from five items/two modules to seven/three
/// (`family.rs`'s pre-existing `encode_value_type` needs
/// `QuantityUnit`/`TextProfile`). AC-6 could not pass as originally written
/// against any conforming implementation that actually carries `family.rs`'s
/// production code.
///
/// **`TextType` is deliberately absent (owner ruling, PR #282 review,
/// post-rebase).** `check::family::tests::
/// mint_declaration_identity_matches_a_checked_in_digest`'s moved golden-digest
/// fixture does import `TextType` from `value::text`, but only inside
/// `#[cfg(test)] mod tests` -- [`value_import_edges`] excludes `#[cfg(test)]`
/// imports from this bound by design (see its own doc), because this
/// allow-list constrains `check`'s *shipped* dependency graph, the layering
/// property FR-068-AC-6 actually governs; a test-only import is not part of
/// that graph, and admitting it into tier 2 would permanently license
/// production code to import it too, unchecked, since AC-6's own scan would
/// then have no way to tell the two apart. Widening the list to fit a test
/// import was tried and reverted for exactly this reason: it would have
/// bought a green at the cost of the bound's own meaning, the same shape as
/// an unjustified raised size limit.
const DECLARED_INTERIM_ITEMS: [(&str, &str); 7] = [
    ("enumeration", "EnumDeclaration"),
    ("enumeration", "EnumValue"),
    ("quantity", "check_comparable"),
    ("quantity", "result_unit"),
    ("quantity", "UnitOperation"),
    ("quantity", "QuantityUnit"),
    ("text", "TextProfile"),
];

/// Which of FR-068-AC-6's tiers one `check` -> `value::<submodule>` edge
/// falls into.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueImportTier {
    /// Tier 1: unbounded imports from the nine K-designated siblings.
    KDesignated,
    /// Tier 2: exactly the seven named items (see this module's own
    /// `DECLARED_INTERIM_ITEMS` constant).
    DeclaredInterim,
    /// Tier 3: forbidden -- anything else.
    Forbidden,
}

/// One resolved `check` -> `value::<submodule>` edge, classified into
/// FR-068-AC-6's tiers (TC-175).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueImportEdge {
    /// The file this edge was found in.
    pub file: String,
    /// The `value::` submodule the edge resolves into.
    pub submodule: String,
    /// The bound leaf name.
    pub leaf: String,
    /// Which tier this edge falls into.
    pub tier: ValueImportTier,
    /// Whether this edge is written in crate-absolute, submodule-qualified
    /// form (`crate::value::<submodule>::Name`) -- FR-068-CON-2/AC-6's own
    /// required form (PR #282 review F2) -- rather than a flat
    /// `crate::value::Name` reached only through `value`'s own aggregate.
    pub submodule_qualified: bool,
}

/// Every name `value::mod.rs` re-exports from one of its own *sibling*
/// submodules via a bare (non-`crate`-rooted) `pub use <submodule>::{...}`
/// line -- `crate::check`'s and `crate::forms`'s cross-module re-exports
/// have a two-segment path (`crate::X`) and are excluded by construction,
/// since TC-175 cares which `value::` submodule an item belongs to, not
/// `check`'s or `forms`'s own re-exports.
fn value_submodule_reexports(
    workspace_root: &Path,
) -> Result<std::collections::BTreeMap<String, String>> {
    let parsed = parse_file(workspace_root, "src/value/mod.rs")?;
    let edges = use_edges_in_file(&parsed, "src/value/mod.rs");
    let mut map = std::collections::BTreeMap::new();
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

/// TC-175: every *shipped* `use` edge under `src/check/` resolving into any
/// `value::` submodule at all (not only `expression`/`checking`, which
/// [`check_module_violations`] covers), classified into FR-068-AC-6's
/// tiers. A flat `crate::value::Name` edge is resolved to its owning
/// submodule by scanning `value::mod.rs`'s whole sibling re-export table,
/// not only the two names [`value_reexports`] needs -- TC-175's own
/// criterion is which submodule an item belongs to, not only whether it is
/// `expression`.
///
/// **Shipped imports only (owner ruling, PR #282 review, post-rebase).**
/// Unlike [`check_module_violations`] (TC-172/AC-3, deliberately stricter:
/// `check` must import nothing from `value::expression` at all, in test code
/// or not, so a `check`-side test can never quietly reopen that edge) and
/// [`model_check_edges`] (TC-176), this scan uses this module's own
/// `shipped_use_edges_in_file` helper, not `use_edges_in_file`, and so
/// excludes `#[cfg(test)]`-gated imports. FR-068-AC-6's tier bound constrains `check`'s
/// dependency on `value` as a layering property of the *shipped* crate; a
/// test-only import is not part of that dependency graph, and admitting one
/// into the tier-2 allow-list to make a test pass would permanently license
/// production code to import it too, with no way for this scan to ever catch
/// that widening back. `check::family::tests::
/// mint_declaration_identity_matches_a_checked_in_digest`'s `TextType`
/// import is the concrete case this excludes: real, `#[cfg(test)]`-gated,
/// and out of tier 2's scope by this design choice, not by oversight.
pub fn value_import_edges(workspace_root: &Path) -> Result<Vec<ValueImportEdge>> {
    let submodule_reexports = value_submodule_reexports(workspace_root)?;
    let mut edges_found = Vec::new();
    for file in files_in(workspace_root, "src/check")? {
        let parsed = parse_file(workspace_root, &file)?;
        for edge in shipped_use_edges_in_file(&parsed, &file) {
            if edge.is_glob {
                continue;
            }
            let normalized = strip_leading_crate(&edge.path);
            let (submodule, submodule_qualified) = match normalized {
                [first, second, ..] if first == "value" => (Some(second.clone()), true),
                [first] if first == "value" => {
                    (submodule_reexports.get(&edge.leaf).cloned(), false)
                }
                _ => (None, false),
            };
            let Some(submodule) = submodule else {
                continue;
            };
            let tier = if K_DESIGNATED_MODULES.contains(&submodule.as_str()) {
                ValueImportTier::KDesignated
            } else if DECLARED_INTERIM_ITEMS.contains(&(submodule.as_str(), edge.leaf.as_str())) {
                ValueImportTier::DeclaredInterim
            } else {
                ValueImportTier::Forbidden
            };
            edges_found.push(ValueImportEdge {
                file: edge.file.clone(),
                submodule,
                leaf: edge.leaf.clone(),
                tier,
                submodule_qualified,
            });
        }
    }
    Ok(edges_found)
}

/// Whether `path` (a `use` edge's segments, `crate`-rooted or not) resolves
/// into `crate::package` or `crate::checked_package`. Both are layer-4
/// (ADR-011 §6.1): QSL-182 prep split the former's moving half out into the
/// latter, a sibling module, so a `check` (layer 3) import of either is the
/// same forbidden layer-3-depends-on-layer-4 edge FR-068-AC-6 already named
/// for `package` alone before the split.
fn resolves_into_package(path: &[String]) -> bool {
    matches!(strip_leading_crate(path), [first, ..] if first == "package" || first == "checked_package")
}

/// FR-068-AC-6's last sentence ("No file under `check` imports from ...
/// `package`") had no mechanized test (PR #291 review, finding 3):
/// [`value_import_edges`] only scans edges whose resolved path starts with
/// `value`, so a `crate::package` import -- direct or, unlike `value`,
/// **including a glob**, since `package` has no declared-interim allow-list
/// to check a leaf against -- was invisible to it. This scan closes that
/// gap directly, over `check`'s *shipped* dependency graph only, matching
/// AC-6's own scope for the rest of its bound (see this module's header
/// doc, "`#[cfg(test)]` handling differs by which criterion is being
/// checked"). Widened by QSL-182 prep to also flag `checked_package`, the
/// sibling module the same layer-4 content partly moved into (see
/// `resolves_into_package`'s own doc, private to this module).
pub fn check_package_import_edges(workspace_root: &Path) -> Result<Vec<UseEdge>> {
    let mut edges_found = Vec::new();
    for file in files_in(workspace_root, "src/check")? {
        let parsed = parse_file(workspace_root, &file)?;
        for edge in shipped_use_edges_in_file(&parsed, &file) {
            if resolves_into_package(&edge.path) {
                edges_found.push(edge);
            }
        }
    }
    Ok(edges_found)
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

    /// TC-175 steps 1-2 (Expected Results): a fixture item resolving into a
    /// K-designated module is tier 1, one of the seven named items is tier
    /// 2, and anything else -- including a *different* item from
    /// `enumeration`/`quantity`/`text` -- is tier 3 (forbidden). Written
    /// against the classifier directly (not the real tree) so this test
    /// exercises a synthetic tier-3 case the real, currently-clean tree
    /// does not itself contain.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn tier_classification_matches_ac6s_amended_allow_lists() {
        assert!(K_DESIGNATED_MODULES.contains(&"numeric"));
        assert!(DECLARED_INTERIM_ITEMS.contains(&("quantity", "QuantityUnit")));
        assert!(DECLARED_INTERIM_ITEMS.contains(&("text", "TextProfile")));
        // `TextType` is deliberately NOT tier 2 (owner ruling, PR #282
        // review, post-rebase): its only real dependency is
        // `#[cfg(test)]`-gated, and `value_import_edges` excludes test-only
        // imports from this bound entirely (see its own doc and
        // `shipped_scan_excludes_a_cfg_test_only_forbidden_import` below),
        // so it needs no tier-2 admission at all.
        assert!(!DECLARED_INTERIM_ITEMS.contains(&("text", "TextType")));
        // An item from a tier-2 *module* that is not one of the seven named
        // items (e.g. `quantity::Quantity`, which `check` does not import)
        // must not be silently admitted just because its module is tier 2.
        assert!(!DECLARED_INTERIM_ITEMS.contains(&("quantity", "Quantity")));
    }

    /// Against the real, current tree: every `check` -> `value::<submodule>`
    /// edge is tier 1 or tier 2 (group (c) is empty), and every one is
    /// written in crate-absolute, submodule-qualified form -- PR #282
    /// review F2's rewrite, confirmed at the resolved level rather than
    /// merely by having been the one that wrote it. This includes
    /// `check::family.rs`'s own `#[cfg(test)]`-gated `TextType` import: it
    /// resolves to `Forbidden` under [`shipped_use_edges_in_file`]'s
    /// unfiltered sibling [`use_edges_in_file`] (see
    /// `shipped_scan_excludes_a_cfg_test_only_forbidden_import` immediately
    /// below for that exact assertion), but is invisible to this scan by
    /// design, so it does not appear here at all.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn real_check_value_imports_are_bounded_to_tier_1_and_2_and_submodule_qualified() {
        let edges = value_import_edges(&workspace_root()).expect("scan runs");
        assert!(
            !edges.is_empty(),
            "fixture sanity: check must import something real from value"
        );
        for edge in &edges {
            assert_ne!(
                edge.tier,
                ValueImportTier::Forbidden,
                "tier-3 (forbidden) edge found: {edge:?}"
            );
            assert!(
                edge.submodule_qualified,
                "edge not written in submodule-qualified form: {edge:?}"
            );
        }
    }

    /// FR-068-AC-6's own forbidden list ("No file under `check` imports from
    /// ... `package`"), checked directly (PR #291 review, finding 3):
    /// `value_import_edges` only ever scans `value::`-rooted edges, so this
    /// half of AC-6 had no mechanized backing until now. Against the real,
    /// current tree, `check`'s shipped dependency graph imports nothing from
    /// `package` at all -- confirmed live: a `use crate::package::*;` was
    /// planted temporarily in `src/check/mod.rs` and confirmed to fail this
    /// test before being reverted.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn real_check_has_no_package_import() {
        let edges = check_package_import_edges(&workspace_root()).expect("scan runs");
        assert!(
            edges.is_empty(),
            "check must never import from package or checked_package (FR-068-AC-6): {edges:?}"
        );
    }

    /// A fixture proving `check_package_import_edges` actually rejects a
    /// `package` import, including a glob -- unlike a `value::` import, a
    /// `package` import is forbidden outright with no allow-list to bound
    /// it, so a glob (which binds no named leaf to check against an
    /// allow-list) must still be caught. Pins
    /// `real_check_has_no_package_import`'s fix permanently.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn package_import_is_classified_as_a_violation_including_glob() {
        let source = r#"
            use crate::package::PackageDeclarations;
            use crate::package::*;
        "#;
        let parsed = syn::parse_file(source).expect("fixture parses");
        let edges = shipped_use_edges_in_file(&parsed, "src/check/fixture.rs");
        assert_eq!(edges.len(), 2);
        for edge in &edges {
            assert!(
                resolves_into_package(&edge.path),
                "expected a package-resolving edge: {edge:?}"
            );
        }
    }

    /// QSL-182 prep: `checked_package` is the sibling module the moving
    /// half of layer-4 `package` split into, so a `check` import of it is
    /// the same forbidden layer-3-depends-on-layer-4 edge as a `package`
    /// import, and must be caught the same way, including a glob. Pins
    /// [`resolves_into_package`]'s widened match arm permanently.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn checked_package_import_is_classified_as_a_violation_including_glob() {
        let source = r#"
            use crate::checked_package::CheckedPackage;
            use crate::checked_package::*;
        "#;
        let parsed = syn::parse_file(source).expect("fixture parses");
        let edges = shipped_use_edges_in_file(&parsed, "src/check/fixture.rs");
        assert_eq!(edges.len(), 2);
        for edge in &edges {
            assert!(
                resolves_into_package(&edge.path),
                "expected a checked_package-resolving edge: {edge:?}"
            );
        }
    }

    /// A deliberately-introduced, non-`#[cfg(test)]` (shipped) import of an
    /// unlisted item from a tier-2 module (`value::text::NormalizationForm`,
    /// real in this crate, not one of the seven named tier-2 items) is
    /// classified `Forbidden` -- this is TC-175's own falsifiability
    /// requirement (owner ruling, PR #282 review, post-rebase): the tier
    /// scan must actually reject something, not merely fail to reject
    /// anything the allow-list was widened to admit. A live equivalent of
    /// this fixture was run against the real tree (`src/check/mod.rs`,
    /// temporarily) and confirmed `real_check_value_imports_are_bounded_
    /// to_tier_1_and_2_and_submodule_qualified` fails with exactly this
    /// `Forbidden` classification before being reverted; this test pins that
    /// behavior permanently.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn shipped_forbidden_import_is_classified_forbidden() {
        let source = r#"
            use crate::value::text::NormalizationForm;
        "#;
        let parsed = syn::parse_file(source).expect("fixture parses");
        let edges = shipped_use_edges_in_file(&parsed, "src/check/fixture.rs");
        assert_eq!(edges.len(), 1);
        assert!(!K_DESIGNATED_MODULES.contains(&"text"));
        assert!(!DECLARED_INTERIM_ITEMS.contains(&("text", "NormalizationForm")));
    }

    /// A `#[cfg(test)]`-gated import of the same otherwise-forbidden shape
    /// is excluded entirely by [`shipped_use_edges_in_file`] -- the concrete
    /// mechanism `TextType` relies on to need no tier-2 admission (owner
    /// ruling, PR #282 review, post-rebase). [`use_edges_in_file`], by
    /// contrast, still reports it: this test pins the difference between the
    /// two collectors directly, not only its effect on one real name.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn shipped_scan_excludes_a_cfg_test_only_forbidden_import() {
        let source = r#"
            #[cfg(test)]
            mod tests {
                use crate::value::text::NormalizationForm;
            }
        "#;
        let parsed = syn::parse_file(source).expect("fixture parses");
        let shipped = shipped_use_edges_in_file(&parsed, "src/check/fixture.rs");
        assert!(
            shipped.is_empty(),
            "a #[cfg(test)]-only import must not appear in the shipped scan: {shipped:?}"
        );
        let unfiltered = use_edges_in_file(&parsed, "src/check/fixture.rs");
        assert_eq!(
            unfiltered.len(),
            1,
            "the unfiltered collector (used by TC-172/TC-176) must still see it: {unfiltered:?}"
        );
    }
}
