// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-139 (FR-068) TC-176/TC-262: resolve `model`'s real `use` edges
//! against `value`'s own re-export table, instead of trusting a textual scan
//! of `use` lines alone.
//!
//! TC-172's scan of `check` for edges into `value::expression` or
//! `checking` is retired (QSL-182): both modules are in the root crate,
//! which depends on `qsl-semantics`, so Cargo refuses either edge from
//! `check` and the scan could never fire.
//!
//! FR-068-CON-4 lets `value::mod.rs` go on re-exporting `check`'s
//! relocated types through its own flat aggregate (`pub use
//! crate::check::{...}`). That aggregate is exactly what makes a flat
//! `crate::value::X` import's *true* defining module invisible to a plain
//! grep for `crate::check` -- the coordinator's own framing in the PR #282
//! review that asked for this tool. A `use crate::value::DispatchTable;`
//! inside `model` never contains the substring `check` anywhere in its own
//! text, and it is exactly the hidden edge TC-176 step 5 requires catching
//! at the *resolved* level.
//!
//! This module closes that gap by reading
//! `qsl-semantics/src/value/mod.rs`'s own aggregate `use` block (`pub use
//! crate::check::{...}`) to build the name set that matters, then resolving
//! every `use` edge found under `qsl-semantics/src/model/` against it: a flat
//! `crate::value::Name` import whose `Name` is in that set resolves into
//! `crate::check` exactly as surely as a direct `crate::check::Name` import
//! would.
//!
//! **Scope of what this resolves.** This is a two-hop resolution (`use`
//! edge -> `value::mod.rs`'s own aggregate lines), matched to the one
//! concrete mechanism FR-068-CON-4 names, not a general name-resolution
//! engine: it does not follow a re-export chain through a third module.
//!
//! **`#[cfg(test)]` handling differs by which criterion is being checked
//! (owner ruling, PR #282 review, post-rebase).**
//! [`model_check_edges`] (TC-176/TC-262) does not evaluate `#[cfg]`
//! attributes at all: a `#[cfg(test)]`-gated `use` is scanned the same as an
//! unconditional one, which only ever makes the scan *stricter* than a build
//! would be, and is deliberate. [`check_layer_edges`] (TC-175/AC-6), by contrast,
//! excludes `#[cfg(test)]`-gated imports and items on purpose: AC-6's bound
//! constrains `check`'s *shipped* dependency graph, and a test-only import is
//! not part of it.
//!
//! **FR-068-AC-6's layer rule ([`check_layer_edges`]).** `check`'s shipped
//! imports are bounded by module and layer, not by item: any item of a
//! permitted module is allowed, and the number imported is never checked.
//! This covers every shipped `use` item at any depth, every inline
//! `crate::`/`super::`/`self::` path (including one inside a macro's
//! arguments), and every later path through a module a `use` binds, under
//! `qsl-semantics/src/check/`. A flat `crate::value::Name` aggregate import resolves to its
//! real submodule (failing regardless, since the rule requires the qualified
//! form), and a `super::`/`self::` path resolves relative to its own file.

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
    /// Whether this edge is a `self` import (`use a::b::{self};`), which
    /// binds module `b` itself: `path` ends with `b` and `leaf` is `b`.
    pub is_self: bool,
    /// The local name this edge binds in its scope: the `as` rename if
    /// any, otherwise `leaf`. Empty for a glob edge.
    pub binding: String,
    /// 1-based source line the bound leaf (or the `*` token, for a glob)
    /// starts on -- FR-068-AC-6's layer rule names a failing edge's file,
    /// line and resolved module, so this edge type needs its own line, not
    /// only the enclosing `use` item's.
    pub line: usize,
}

impl UseEdge {
    /// The written path of what this edge binds: `path` followed by `leaf`
    /// for a named import, `path` alone for a glob or a `self` import (whose
    /// last `path` segment is already the bound module).
    fn bound_path(&self) -> Vec<String> {
        let mut bound = self.path.clone();
        if !self.is_glob && !self.is_self {
            bound.push(self.leaf.clone());
        }
        bound
    }
}

/// Flatten one `syn::UseTree` into zero or more [`UseEdge`]s, appending to
/// `prefix` as the walk descends and reporting `leaf` as each branch's own
/// terminal name (its original ident, not its rename).
fn flatten_use_tree(tree: &syn::UseTree, prefix: &[String], file: &str, out: &mut Vec<UseEdge>) {
    let named = |ident: &syn::Ident, rename: Option<&syn::Ident>| {
        let written = ident.to_string();
        // `use a::b::self;` binds `b` itself under prefix `a`; the leaf is
        // the last prefix segment, not the literal "self".
        let is_self = written == "self";
        let leaf = if is_self {
            prefix.last().cloned().unwrap_or_default()
        } else {
            written
        };
        let binding = rename.map_or_else(|| leaf.clone(), ToString::to_string);
        UseEdge {
            file: file.to_owned(),
            path: prefix.to_vec(),
            leaf,
            is_glob: false,
            is_self,
            binding,
            line: ident.span().start().line,
        }
    };
    match tree {
        syn::UseTree::Path(use_path) => {
            let mut next = prefix.to_vec();
            next.push(use_path.ident.to_string());
            flatten_use_tree(&use_path.tree, &next, file, out);
        }
        syn::UseTree::Name(use_name) => out.push(named(&use_name.ident, None)),
        syn::UseTree::Rename(use_rename) => {
            out.push(named(&use_rename.ident, Some(&use_rename.rename)));
        }
        syn::UseTree::Glob(use_glob) => {
            out.push(UseEdge {
                file: file.to_owned(),
                path: prefix.to_vec(),
                leaf: String::new(),
                is_glob: true,
                is_self: false,
                binding: String::new(),
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

/// The name set `qsl-semantics/src/value/mod.rs`'s own aggregate `use`
/// block establishes (see this module's own doc): every name a flat
/// `crate::value::Name` import can reach that really resolves into
/// `crate::check`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ValueReexports {
    /// Names `value::mod.rs`'s `pub use crate::check::{...};` block re-exports.
    pub check: BTreeSet<String>,
}

/// Read `qsl-semantics/src/value/mod.rs` and extract [`ValueReexports`] by
/// finding the one `use` item whose path starts `crate::check`.
pub fn value_reexports(workspace_root: &Path) -> Result<ValueReexports> {
    let value_mod = format!("{LAYER3_SRC}/value/mod.rs");
    let parsed = parse_file(workspace_root, &value_mod)?;
    let edges = use_edges_in_file(&parsed, &value_mod);
    let mut reexports = ValueReexports::default();
    for edge in &edges {
        if edge.is_glob {
            continue;
        }
        if let [first, second] = edge.path.as_slice() {
            if first == "crate" && second == "check" {
                reexports.check.insert(edge.leaf.clone());
            }
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
        Some(root) if root == "crate" || root == LAYER3_CRATE => &path[1..],
        _ => path,
    }
}

// TC-172 (FR-068-AC-3) used to scan `check` here for an edge into
// `value::expression` or `checking`. Both are in the root crate, and Cargo
// gates crate direction: the root crate depends on `qsl-semantics`, so
// neither edge compiles from `check` (QSL-182 retired the scan).

/// One resolved `model` -> `check` edge (TC-262/FR-074-AC-3, inverted from
/// TC-176/FR-068-AC-9, which this retires): which file it was found in, the
/// name it binds (`"*"` for a glob import whose own path resolves into
/// `check` -- a glob binds no single leaf, see [`UseEdge::is_glob`]), and
/// whether it was written directly against `crate::check` or reached
/// indirectly through `value`'s flat aggregate. After M-2 this edge set
/// SHALL be empty everywhere under `qsl-semantics/src/model/`.
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

/// TC-262: every `use` edge under `qsl-semantics/src/model/` that resolves, directly or
/// through `value`'s flat aggregate, into `crate::check` -- including a
/// glob (`use crate::check::*;`) whose own path resolves into `check`.
/// `resolves_into_check` doesn't need `leaf` for that branch (it only
/// consults `leaf` for a flat `crate::value::Name` import), so a glob's
/// empty leaf resolves correctly without special-casing; only the *skip*
/// used to be the bug (PR #291 review finding 1: an earlier revision
/// skipped every glob edge outright before ever calling
/// `resolves_into_check`, so a `use crate::check::*;` planted under
/// `qsl-semantics/src/model/` passed `real_model_check_edge_is_empty` completely
/// undetected -- a genuine gate hole, not a false alarm; confirmed live
/// against the real tree and reverted, see
/// `glob_import_into_check_is_recorded_not_skipped` below for the pinned
/// fixture).
pub fn model_check_edges(workspace_root: &Path) -> Result<Vec<ModelCheckEdge>> {
    let reexports = value_reexports(workspace_root)?;
    let mut edges_found = Vec::new();
    for file in files_in(workspace_root, &format!("{LAYER3_SRC}/model"))? {
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
/// line -- `crate::check`'s cross-module re-exports have a two-segment path
/// (`crate::X`) and are excluded by construction, since TC-175 cares which
/// `value::` submodule an item belongs to, not `check`'s own re-exports.
fn value_submodule_reexports(workspace_root: &Path) -> Result<BTreeMap<String, String>> {
    let value_mod = format!("{LAYER3_SRC}/value/mod.rs");
    let parsed = parse_file(workspace_root, &value_mod)?;
    let edges = use_edges_in_file(&parsed, &value_mod);
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

/// The source tree these scans read: the `qsl-semantics` crate's `src/`,
/// where `check`, `model`, `family`, `library` and the layer-3 `value`
/// submodules live since QSL-181 (ADR-011 §7.3 X-6). Every scanned path, and
/// every module FR-068-AC-6 names, is relative to this crate.
const LAYER3_SRC: &str = "qsl-semantics/src";

/// That crate's own Rust name. A path rooted at it (`qsl_semantics::value::
/// quantity::UnitTable`, as a doctest or an `extern crate self` alias would
/// write one) is the same crate-relative path as `crate::value::quantity::
/// UnitTable`. Every scan here treats it as a crate root, the same as
/// `crate`: [`strip_leading_crate`] (TC-262), [`resolve_relative_path`]
/// and inline-path resolution (TC-175).
const LAYER3_CRATE: &str = "qsl_semantics";

// ---------------------------------------------------------------------
// FR-068-AC-6: `check`'s imports are bounded by module and layer, not by
// item. The number of items imported from a permitted module is not checked.
//
// Cargo is the gate for crate direction: every later-layer module
// (`checked_package`, `package`, `value::expression`, `route`, `replay`,
// `lowering`, the SEAM modules) is in a crate that depends on
// `qsl-semantics`, so an import of one from here does not compile. This scan
// checks what Cargo cannot see: the module order inside `qsl-semantics`.
// ---------------------------------------------------------------------

/// FR-068-AC-6's permitted-module list (closed): any item of a listed
/// module is allowed. A `value::<submodule>` entry names one specific
/// submodule; every other entry is matched by its own name and all of its
/// descendants (`check` and its descendants; `library` and its
/// descendants; and so on) -- see [`module_or_descendant`]. The K, F and
/// layer-2 entries are workspace crates, named by their Rust crate name;
/// [`in_layer_rule_scope`] reads them from here.
const LAYER_PERMITTED_MODULES: &[&str] = &[
    // K, F.
    "quire_exact",
    "qsl_foundation",
    // Layer 2: the `qsl-forms` crate (ADR-011 §7.3 X-5).
    "qsl_forms",
    // Layer 3, before `check` core (ADR-011 §6.1: `semantic_value < model <
    // library < check core`).
    "value::definition",
    "value::enumeration",
    "value::unit",
    "value::quantity",
    "value::containment",
    "value::semantic_node",
    "value::declaration",
    "model",
    "value::model_query",
    "library",
    // Layer 3, `check` core itself.
    "check",
    "family",
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
    /// A module of this crate that is not on the permitted list.
    Unlisted,
}

fn classify_layer_module(module: &str) -> LayerClass {
    if LAYER_PERMITTED_MODULES
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
    let src = workspace_root.join(LAYER3_SRC);
    src.join(format!("{name}.rs")).is_file() || src.join(name).is_dir()
}

/// Whether `name` is a real submodule of `value` -- a file or directory
/// directly under `qsl-semantics/src/value/` -- used to tell a genuinely nested
/// `crate::value::<submodule>::Name` path apart from a flat
/// `crate::value::Name` one at the inline-path level, where (unlike a `use`
/// line) source syntax alone does not separate a path's module prefix from
/// its bound item.
fn is_real_value_submodule(workspace_root: &Path, name: &str) -> bool {
    let value_dir = workspace_root.join(LAYER3_SRC).join("value");
    value_dir.join(format!("{name}.rs")).is_file() || value_dir.join(name).is_dir()
}

/// Whether `top` (a resolved path's first segment) is in FR-068-AC-6's
/// scope at all: a crate named on [`LAYER_PERMITTED_MODULES`] (`quire_exact`,
/// `qsl_foundation`, `qsl_forms`), or any real module of this crate. `std`
/// and every other crate are out of scope and return `false`.
fn in_layer_rule_scope(workspace_root: &Path, top: &str) -> bool {
    LAYER_PERMITTED_MODULES.contains(&top) || is_real_crate_module(workspace_root, top)
}

/// Resolve `top` (a resolved path's first segment) and `next` (its second,
/// if any) into FR-068-AC-6's module label and whether the edge names its
/// submodule. Only `value` needs `next` resolved further: every other
/// permitted entry is matched on `top` alone (`module_or_descendant`
/// covers the rest of the path).
///
/// `next_is_module_by_syntax` is `true` when the caller already knows `next`
/// is a real module segment because the source syntax itself guarantees it
/// (a `use` edge's own path, where every segment but the bound leaf is a
/// module by construction); it is `false` for an inline path, where syntax
/// alone cannot tell a submodule segment (`quantity` in
/// `crate::value::quantity::UnitTable`) apart from a flat aggregate item
/// (`UnitTable` in `crate::value::UnitTable`) -- that case falls
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
/// resolved relative to its file"). `self` may be followed by `super`s
/// (`self::super::x`). A path rooted at anything else (an extern crate name,
/// the crates on [`LAYER_PERMITTED_MODULES`] included) is returned unchanged -- it
/// needs no crate-relative substitution.
fn resolve_relative_path(raw: &[String], current_module: &[String]) -> Vec<String> {
    let (mut base, mut rest) = match raw.split_first() {
        Some((first, rest)) if first == "crate" || first == LAYER3_CRATE => (Vec::new(), rest),
        Some((first, rest)) if first == "self" => (current_module.to_vec(), rest),
        Some((first, _)) if first == "super" => (current_module.to_vec(), raw),
        _ => return raw.to_vec(),
    };
    while let Some((first, tail)) = rest.split_first() {
        if first != "super" {
            break;
        }
        base.pop();
        rest = tail;
    }
    base.extend(rest.iter().cloned());
    base
}

/// One FR-068-AC-6 layer-rule edge: a `use` line or inline
/// `crate::`/`super::`/`self::` path under `qsl-semantics/src/check/`, resolved and
/// classified (TC-175).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayerEdge {
    /// The file this edge was found in.
    pub file: String,
    /// 1-based source line.
    pub line: usize,
    /// The resolved module label (e.g. `"value::quantity"`, `"model"`).
    pub module: String,
    /// Permitted or unlisted.
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
    /// permitted (TC-175: `crate::value::UnitTable` still fails, though
    /// `value::quantity` is a permitted `semantic_value` module).
    pub fn is_violation(&self) -> bool {
        self.class != LayerClass::Permitted || !self.submodule_qualified
    }
}

/// Classify one already-resolved path (crate-relative, `crate`/`super`/`self`
/// substituted) found at `file:line` under `qsl-semantics/src/check/`. `None` when the
/// path's root is `std` or a crate not on [`LAYER_PERMITTED_MODULES`] -- out
/// of FR-068-AC-6's scope entirely, not
/// a finding. `next_is_module_by_syntax`: see [`resolve_layer_module`].
fn classify_resolved(
    workspace_root: &Path,
    submodule_reexports: &BTreeMap<String, String>,
    file: &str,
    line: usize,
    resolved: &[String],
    next_is_module_by_syntax: bool,
) -> Option<LayerEdge> {
    let top = resolved.first()?;
    if !in_layer_rule_scope(workspace_root, top) {
        return None;
    }
    let (module, submodule_qualified) = resolve_layer_module(
        workspace_root,
        submodule_reexports,
        top,
        resolved.get(1).map(String::as_str),
        next_is_module_by_syntax,
    );
    let class = classify_layer_module(&module);
    Some(LayerEdge {
        file: file.to_owned(),
        line,
        module,
        class,
        submodule_qualified,
    })
}

/// Classify one `use` edge found under `qsl-semantics/src/check/`, on the resolved path of
/// what it binds -- so `use crate::complete;`, `use
/// crate::{complete, model};` and `use super::super::complete;`, which bind a
/// module by name, are classified on that module. Every resolved segment but
/// a named leaf is a module by syntax; a named leaf may itself be a module
/// (`use crate::value::quantity;`), which [`resolve_layer_module`] tells
/// apart from a flat aggregate item.
fn classify_use_edge(
    workspace_root: &Path,
    submodule_reexports: &BTreeMap<String, String>,
    current_module: &[String],
    edge: &UseEdge,
) -> Option<LayerEdge> {
    let resolved = resolve_relative_path(&edge.bound_path(), current_module);
    let module_segments = if edge.is_glob || edge.is_self {
        resolved.len()
    } else {
        resolved.len().saturating_sub(1)
    };
    classify_resolved(
        workspace_root,
        submodule_reexports,
        &edge.file,
        edge.line,
        &resolved,
        module_segments >= 2,
    )
}

/// The crate-relative module path segments for a `.rs` file given relative
/// to the workspace root, dropping everything up to and including its
/// crate's `src` directory (`qsl-semantics/src/check/check.rs` -> `["check", "check"]`;
/// `src/check/mod.rs` -> `["check"]`; `src/lib.rs` -> `[]`) -- Rust's own
/// `mod.rs`/`foo.rs` file-to-module convention, needed to resolve a
/// `super::`/`self::` path relative to its file.
fn module_segments_of(relative_file: &str) -> Vec<String> {
    let mut segments: Vec<String> = Path::new(relative_file)
        .with_extension("")
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect();
    if let Some(src) = segments.iter().position(|segment| segment == "src") {
        segments.drain(..=src);
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

/// One inline path found in shipped (non-`#[cfg(test)]`) code: either rooted
/// at `crate`, `super` or `self`, or of two or more segments (whose first
/// segment may be a `use`-bound alias, resolved in [`check_layer_edges`]).
struct InlinePath {
    line: usize,
    segments: Vec<String>,
}

impl InlinePath {
    fn is_relevant(segments: &[String]) -> bool {
        segments.len() >= 2
            || segments
                .first()
                .is_some_and(|first| matches!(first.as_str(), "crate" | "super" | "self"))
    }
}

/// Every path-shaped run `ident (:: ident)*` in a macro invocation's
/// arguments, at any group depth: `syn` does not parse a macro's arguments,
/// so `vec![crate::package::X]` is only visible as tokens.
fn macro_token_paths(stream: proc_macro2::TokenStream, out: &mut Vec<InlinePath>) {
    let trees: Vec<proc_macro2::TokenTree> = stream.into_iter().collect();
    let is_colon = |index: usize| matches!(trees.get(index), Some(proc_macro2::TokenTree::Punct(punct)) if punct.as_char() == ':');
    let mut index = 0;
    while let Some(tree) = trees.get(index) {
        match tree {
            proc_macro2::TokenTree::Group(group) => {
                macro_token_paths(group.stream(), out);
                index += 1;
            }
            proc_macro2::TokenTree::Ident(first) => {
                let mut segments = vec![first.to_string()];
                index += 1;
                while is_colon(index) && is_colon(index + 1) {
                    let Some(proc_macro2::TokenTree::Ident(next)) = trees.get(index + 2) else {
                        break;
                    };
                    segments.push(next.to_string());
                    index += 3;
                }
                if InlinePath::is_relevant(&segments) {
                    out.push(InlinePath {
                        line: first.span().start().line,
                        segments,
                    });
                }
            }
            _ => index += 1,
        }
    }
}

/// Walks a parsed file collecting, outside any item gated `#[cfg(test)]`:
/// every `use` edge at any depth (module scope, nested `mod`, and a
/// function body's own block-level `use`), every inline path
/// [`InlinePath::is_relevant`] keeps, and every such path written inside a
/// macro invocation's arguments.
struct ShippedEdgeVisitor<'f> {
    file: &'f str,
    uses: Vec<UseEdge>,
    paths: Vec<InlinePath>,
}

impl<'ast> Visit<'ast> for ShippedEdgeVisitor<'_> {
    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        if !has_cfg_test(&node.attrs) {
            flatten_use_tree(&node.tree, &[], self.file, &mut self.uses);
        }
    }

    fn visit_path(&mut self, node: &'ast syn::Path) {
        let segments: Vec<String> = node
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect();
        if let (Some(first), true) = (node.segments.first(), InlinePath::is_relevant(&segments)) {
            self.paths.push(InlinePath {
                line: first.ident.span().start().line,
                segments,
            });
        }
        syn::visit::visit_path(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        macro_token_paths(node.tokens.clone(), &mut self.paths);
        syn::visit::visit_macro(self, node);
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

/// One file's shipped `use` edges and inline paths, each inline path
/// resolved to zero or more crate-relative paths.
///
/// An inline path is resolved when it is rooted at `crate`, `super` or
/// `self`, or when its first segment is a name some shipped `use` in the
/// same file binds: `use crate::value;` followed by `value::Presence` is
/// resolved as `crate::value::Presence`. A binding is tracked file-wide,
/// whatever block its `use` sits in, and a name bound more than once is
/// resolved against every binding -- both over-approximate, never hide.
struct FileEdges {
    current_module: Vec<String>,
    uses: Vec<UseEdge>,
    inline: Vec<(usize, Vec<String>)>,
}

fn file_edges(workspace_root: &Path, file: &str) -> Result<FileEdges> {
    let parsed = parse_file(workspace_root, file)?;
    let current_module = module_segments_of(file);
    let mut visitor = ShippedEdgeVisitor {
        file,
        uses: Vec::new(),
        paths: Vec::new(),
    };
    visitor.visit_file(&parsed);
    let mut bindings: BTreeMap<&str, Vec<Vec<String>>> = BTreeMap::new();
    for edge in &visitor.uses {
        if !edge.is_glob {
            bindings
                .entry(edge.binding.as_str())
                .or_default()
                .push(resolve_relative_path(&edge.bound_path(), &current_module));
        }
    }
    let mut inline = Vec::new();
    for path in &visitor.paths {
        let Some((first, rest)) = path.segments.split_first() else {
            continue;
        };
        if matches!(first.as_str(), "crate" | "super" | "self") || first == LAYER3_CRATE {
            inline.push((
                path.line,
                resolve_relative_path(&path.segments, &current_module),
            ));
        } else {
            for bound in bindings.get(first.as_str()).into_iter().flatten() {
                inline.push((path.line, bound.iter().chain(rest).cloned().collect()));
            }
        }
    }
    let uses = visitor.uses;
    Ok(FileEdges {
        current_module,
        uses,
        inline,
    })
}

/// FR-068-AC-6/TC-175: every shipped `use` edge and inline path under
/// `qsl-semantics/src/check/`, classified against the module-level layer
/// rule: each resolves into a permitted module of this crate, names its
/// `value` submodule, or is a violation. Inline paths resolve as
/// `file_edges` describes.
pub fn check_layer_edges(workspace_root: &Path) -> Result<Vec<LayerEdge>> {
    let submodule_reexports = value_submodule_reexports(workspace_root)?;
    let mut edges = Vec::new();
    for file in files_in_recursive(workspace_root, &format!("{LAYER3_SRC}/check"))? {
        let file_edges = file_edges(workspace_root, &file)?;
        for edge in &file_edges.uses {
            edges.extend(classify_use_edge(
                workspace_root,
                &submodule_reexports,
                &file_edges.current_module,
                edge,
            ));
        }
        for (line, resolved) in &file_edges.inline {
            edges.extend(classify_resolved(
                workspace_root,
                &submodule_reexports,
                &file,
                *line,
                resolved,
                false,
            ));
        }
    }
    edges.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    Ok(edges)
}

/// One resolved path from a shipped `use` edge or inline path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedPath {
    /// The file the path was found in, relative to the workspace root.
    pub file: String,
    /// 1-based source line.
    pub line: usize,
    /// The resolved segments: crate-relative for a `crate`/`super`/`self`
    /// root, as written otherwise. A `use` edge's segments end with the
    /// name it binds; a glob's end with the module it globs.
    pub segments: Vec<String>,
    /// Whether the path came from a glob `use`.
    pub is_glob: bool,
}

/// FR-090-AC-9/TC-390: every shipped `use` edge and inline path in each of
/// `roots` (a `.rs` file or a directory, scanned recursively, relative to
/// `workspace_root`), resolved as `file_edges` describes.
pub fn resolved_paths(workspace_root: &Path, roots: &[&str]) -> Result<Vec<ResolvedPath>> {
    let mut paths = Vec::new();
    for root in roots {
        let files = if workspace_root.join(root).is_dir() {
            files_in_recursive(workspace_root, root)?
        } else {
            vec![(*root).to_owned()]
        };
        for file in files {
            let file_edges = file_edges(workspace_root, &file)?;
            for edge in &file_edges.uses {
                paths.push(ResolvedPath {
                    file: file.clone(),
                    line: edge.line,
                    segments: resolve_relative_path(&edge.bound_path(), &file_edges.current_module),
                    is_glob: edge.is_glob,
                });
            }
            for (line, segments) in file_edges.inline {
                paths.push(ResolvedPath {
                    file: file.clone(),
                    line,
                    segments,
                    is_glob: false,
                });
            }
        }
    }
    Ok(paths)
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

    /// The resolution mechanism `real_model_check_edge_is_empty` (TC-262)
    /// relies on: a flat `use crate::value::DispatchTable;` inside `model`
    /// would resolve into `check` if `value::mod.rs` re-exported it from
    /// there (the fixture below supplies that re-export; the real
    /// `value::mod.rs` has none since QSL-181 X-6a), and is recorded indirect (`direct: false`). Retagged
    /// from TC-176/FR-068-AC-9 (retired by FR-074): the assertion this test
    /// backs -- that such a form must be absent from `model` -- moved to
    /// TC-262's empty-set requirement, but the resolution logic itself is
    /// unchanged and still needs its own unit coverage.
    #[trace("TC-262", "FR-074-AC-3")]
    #[test]
    fn flat_value_import_of_a_check_reexport_resolves_into_check_indirectly() {
        let reexports = ValueReexports {
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

    /// `value_reexports` against the real tree: layer-3 `value`
    /// (`qsl-semantics/src/value/mod.rs`) re-exports nothing from `check`
    /// (QSL-181 X-6a: `semantic_value` sits before `check` in ADR-011 §6.1's
    /// order). Its own submodule re-exports are non-empty, so the resolver
    /// is reading the real file. This fails if a `check` re-export comes
    /// back.
    #[test]
    fn real_value_reexports_nothing_from_check() {
        let reexports = value_reexports(&workspace_root()).expect("scan runs");
        assert!(reexports.check.is_empty(), "{:?}", reexports.check);
        let submodules = value_submodule_reexports(&workspace_root()).expect("scan runs");
        assert_eq!(
            submodules.get("DefinitionLock").map(String::as_str),
            Some("definition")
        );
    }

    // -------------------------------------------------------------------
    // FR-068-AC-6 (TC-175): the module-level layer rule.
    // -------------------------------------------------------------------

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    /// A minimal `qsl-semantics`-shaped fixture tree: a bare marker file for
    /// each of its real top-level modules (so [`is_real_crate_module`] can
    /// tell each apart from an external crate) and each of its real `value`
    /// submodules. `complete` and `value::member` are real modules of the
    /// crate that the permitted list leaves off, so an import of either is
    /// unlisted. Later-layer modules are not faked: they are in other
    /// crates, and Cargo refuses an import of them.
    fn layer_fixture_root() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for module in ["model", "library", "family", "complete"] {
            write(dir.path(), &format!("{LAYER3_SRC}/{module}.rs"), "");
        }
        for submodule in [
            "definition",
            "enumeration",
            "unit",
            "quantity",
            "containment",
            "semantic_node",
            "model_query",
            "member",
        ] {
            write(
                dir.path(),
                &format!("{LAYER3_SRC}/value/{submodule}.rs"),
                "",
            );
        }
        write(dir.path(), "qsl-semantics/src/value/mod.rs", "");
        dir
    }

    /// Against the real, current tree: every shipped edge under
    /// `qsl-semantics/src/check/` -- `use` line and inline path alike -- is permitted,
    /// and every `value` edge names its submodule. This is TC-175 steps 1-2
    /// against the module-level layer rule.
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

    /// TC-175 step 6: a shipped `use crate::complete::ReaderAuthority;`
    /// under `check/` fails, naming the file, line and resolved module:
    /// `complete` is a module of the crate that is not on the permitted list.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn unlisted_use_edge_is_a_violation() {
        let dir = layer_fixture_root();
        write(
            dir.path(),
            "qsl-semantics/src/check/fixture.rs",
            "use crate::complete::ReaderAuthority;\n",
        );
        let edges = check_layer_edges(dir.path()).expect("scan runs");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].module, "complete");
        assert_eq!(edges[0].class, LayerClass::Unlisted);
        assert_eq!(edges[0].line, 1);
        assert!(edges[0].is_violation());
    }

    /// TC-175: layer 2 is the `qsl-forms` crate, so a shipped
    /// `use qsl_forms::Expression;` under `src/check/` is classified, and
    /// permitted, as the `qsl_forms` module rather than skipped as an
    /// external crate.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn qsl_forms_use_edge_is_permitted() {
        let dir = layer_fixture_root();
        write(
            dir.path(),
            "qsl-semantics/src/check/fixture.rs",
            "use qsl_forms::Expression;\n",
        );
        let edges = check_layer_edges(dir.path()).expect("scan runs");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].module, "qsl_forms");
        assert_eq!(edges[0].class, LayerClass::Permitted);
        assert!(!edges[0].is_violation());
    }

    /// TC-175 (QSL-181): the scanned crate is `qsl-semantics`, so a path
    /// rooted at its own name is classified exactly as a `crate::` one would
    /// be, not skipped as an external crate: `qsl_semantics::value::member`
    /// is unlisted, a flat `qsl_semantics::value::UnitTable` names no
    /// submodule, `qsl_semantics::value::quantity` is permitted, and an
    /// inline `qsl_semantics::complete::..` path is unlisted.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn qsl_semantics_rooted_paths_are_classified_like_crate_paths() {
        let dir = layer_fixture_root();
        write(
            dir.path(),
            "qsl-semantics/src/value/mod.rs",
            "pub use quantity::UnitTable;\n",
        );
        write(
            dir.path(),
            "qsl-semantics/src/check/fixture.rs",
            "use qsl_semantics::value::member::SomeThing;\n\
             use qsl_semantics::value::UnitTable;\n\
             use qsl_semantics::value::quantity::QuantityUnit;\n\
             pub fn f(_: qsl_semantics::complete::ReaderAuthority) {}\n",
        );
        let edges = check_layer_edges(dir.path()).expect("scan runs");
        let summary: Vec<(usize, &str, bool)> = edges
            .iter()
            .map(|edge| (edge.line, edge.module.as_str(), edge.is_violation()))
            .collect();
        assert_eq!(
            summary,
            vec![
                (1, "value::member", true),
                (2, "value::quantity", true),
                (3, "value::quantity", false),
                (4, "complete", true),
            ]
        );
        // TC-262 resolves through `strip_leading_crate`, which strips
        // the crate's own name the same way.
        let path = vec!["qsl_semantics".to_owned(), "check".to_owned()];
        assert!(resolves_into_check(&path, "X", &ValueReexports::default()));
    }

    /// TC-175 step 6: a shipped `use crate::complete::*;` glob fails the
    /// same way a named import would -- a glob binds no leaf to check, so
    /// it must still be caught on its module.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn unlisted_glob_use_edge_is_a_violation() {
        let dir = layer_fixture_root();
        write(
            dir.path(),
            "qsl-semantics/src/check/fixture.rs",
            "use crate::complete::*;\n",
        );
        let edges = check_layer_edges(dir.path()).expect("scan runs");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].module, "complete");
        assert!(edges[0].is_violation());
    }

    /// TC-175 step 6: a shipped inline path `crate::value::member::Member`
    /// with no `use` line fails -- an inline path into an unlisted module
    /// is the same edge a `use` line would be.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn unlisted_inline_path_is_a_violation() {
        let dir = layer_fixture_root();
        write(
            dir.path(),
            "qsl-semantics/src/check/fixture.rs",
            "pub fn f() -> crate::value::member::Member {\n    todo!()\n}\n",
        );
        let edges = check_layer_edges(dir.path()).expect("scan runs");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].module, "value::member");
        assert_eq!(edges[0].class, LayerClass::Unlisted);
        assert!(edges[0].is_violation());
    }

    /// TC-175 step 6: a shipped `use crate::value::UnitTable;` (the flat
    /// aggregate) fails even though `value::quantity` -- the submodule it
    /// actually reaches -- is itself a permitted `semantic_value` module.
    /// The flat form itself is what FR-068-AC-6 forbids, independent of the
    /// target.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn flat_value_use_import_is_a_violation_even_when_the_target_is_permitted() {
        let dir = layer_fixture_root();
        write(
            dir.path(),
            "qsl-semantics/src/value/mod.rs",
            "pub use quantity::UnitTable;\n",
        );
        write(
            dir.path(),
            "qsl-semantics/src/check/fixture.rs",
            "use crate::value::UnitTable;\n",
        );
        let edges = check_layer_edges(dir.path()).expect("scan runs");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].module, "value::quantity");
        assert_eq!(edges[0].class, LayerClass::Permitted);
        assert!(!edges[0].submodule_qualified);
        assert!(edges[0].is_violation());
    }

    /// TC-175's own flat-inline-path fixture: a shipped inline flat path
    /// `crate::value::UnitTable` fails, resolved to `value::quantity` (the
    /// module `UnitTable` actually belongs to) through `value::mod.rs`'s own
    /// re-export table.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn flat_inline_value_path_is_a_violation() {
        let dir = layer_fixture_root();
        write(
            dir.path(),
            "qsl-semantics/src/value/mod.rs",
            "pub use quantity::UnitTable;\n",
        );
        write(
            dir.path(),
            "qsl-semantics/src/check/fixture.rs",
            "pub fn f(_: &crate::value::UnitTable) {}\n",
        );
        let edges = check_layer_edges(dir.path()).expect("scan runs");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].module, "value::quantity");
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
            "qsl-semantics/src/check/fixture.rs",
            "use crate::value::member::SomeThing;\n",
        );
        let edges = check_layer_edges(dir.path()).expect("scan runs");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].module, "value::member");
        assert_eq!(edges[0].class, LayerClass::Unlisted);
        assert!(edges[0].submodule_qualified);
        assert!(edges[0].is_violation());
    }

    /// TC-175 step 6: the same unlisted import inside a `#[cfg(test)]`
    /// item does not fail -- shipped-only scope.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn cfg_test_unlisted_import_is_not_reported() {
        let dir = layer_fixture_root();
        write(
            dir.path(),
            "qsl-semantics/src/check/fixture.rs",
            "#[cfg(test)]\nmod tests {\n    use crate::complete::ReaderAuthority;\n}\n",
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
            "qsl-semantics/src/check/fixture.rs",
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
            "qsl-semantics/src/check/fixture.rs",
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
            module_segments_of("qsl-semantics/src/check/check.rs"),
            vec!["check", "check"]
        );
        assert_eq!(module_segments_of("src/check/mod.rs"), vec!["check"]);
        assert_eq!(module_segments_of("src/lib.rs"), Vec::<String>::new());
    }

    /// Every violation's `(module, line)` in a fixture file, for the
    /// module-binding and macro fixtures below.
    fn violations_of(dir: &Path) -> Vec<(String, usize)> {
        check_layer_edges(dir)
            .expect("scan runs")
            .into_iter()
            .filter(LayerEdge::is_violation)
            .map(|edge| (edge.module, edge.line))
            .collect()
    }

    /// TC-175 step 6: a `use` that binds an unlisted module by name --
    /// `use crate::complete;`, `use crate::{value::member, complete as c};`,
    /// `use super::super::complete as d;` -- fails on that module.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn use_binding_an_unlisted_module_is_a_violation() {
        let dir = layer_fixture_root();
        write(
            dir.path(),
            "qsl-semantics/src/check/fixture.rs",
            "use crate::complete;\nuse crate::{value::member, complete as c};\nuse super::super::complete as d;\n",
        );
        assert_eq!(
            violations_of(dir.path()),
            vec![
                ("complete".to_owned(), 1),
                ("value::member".to_owned(), 2),
                ("complete".to_owned(), 2),
                ("complete".to_owned(), 3),
            ]
        );
    }

    /// TC-175 step 6: `{self}` imports bind the module they name.
    /// `use crate::complete::{self};` fails; `use crate::value::quantity::
    /// {self};` names a permitted `semantic_value` submodule and passes.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn self_import_binds_the_named_module() {
        let dir = layer_fixture_root();
        write(
            dir.path(),
            "qsl-semantics/src/check/fixture.rs",
            "use crate::complete::{self};\nuse crate::value::quantity::{self as q};\n",
        );
        let edges = check_layer_edges(dir.path()).expect("scan runs");
        assert_eq!(edges.len(), 2, "{edges:?}");
        assert_eq!(edges[0].module, "complete");
        assert!(edges[0].is_violation());
        assert_eq!(edges[1].module, "value::quantity");
        assert!(!edges[1].is_violation());
    }

    /// TC-175 step 6: `use crate::value;` binds `value`'s flat aggregate. The
    /// `use` fails (it names no submodule), and so does a later
    /// `value::UnitTable::foo`, resolved through the tracked binding to
    /// `value::quantity` without naming it.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn use_binding_value_and_later_flat_paths_are_violations() {
        let dir = layer_fixture_root();
        write(
            dir.path(),
            "qsl-semantics/src/value/mod.rs",
            "pub use quantity::UnitTable;\n",
        );
        write(
            dir.path(),
            "qsl-semantics/src/check/fixture.rs",
            "use crate::value;\npub fn f() -> bool {\n    matches!(g(), value::UnitTable::Foo)\n}\n",
        );
        assert_eq!(
            violations_of(dir.path()),
            vec![("value".to_owned(), 1), ("value::quantity".to_owned(), 3)]
        );
    }

    /// TC-175 step 6: a module bound by a `use`, renamed or not, is tracked:
    /// `use crate::complete as cp;` then `cp::ReaderAuthority::new()` fails
    /// on both lines, while `use crate::value::quantity;` then
    /// `quantity::UnitTable` resolves to the permitted, named submodule.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn later_paths_through_a_bound_module_are_classified() {
        let dir = layer_fixture_root();
        write(
            dir.path(),
            "qsl-semantics/src/check/fixture.rs",
            "use crate::complete as cp;\nuse crate::value::quantity;\n\
             pub fn f() {\n    let _ = cp::ReaderAuthority::new();\n    let _ = quantity::UnitTable::default();\n}\n",
        );
        let edges = check_layer_edges(dir.path()).expect("scan runs");
        let lines = |module: &str| -> Vec<(usize, bool)> {
            edges
                .iter()
                .filter(|edge| edge.module == module)
                .map(|edge| (edge.line, edge.is_violation()))
                .collect()
        };
        assert_eq!(lines("complete"), vec![(1, true), (4, true)]);
        assert_eq!(lines("value::quantity"), vec![(2, false), (5, false)]);
    }

    /// TC-175 step 6: a `use` inside a function body is shipped code and is
    /// scanned; one inside a `#[cfg(test)]` function is not.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn function_body_use_is_scanned() {
        let dir = layer_fixture_root();
        write(
            dir.path(),
            "qsl-semantics/src/check/fixture.rs",
            "pub fn f() {\n    use crate::complete::ReaderAuthority;\n}\n\
             #[cfg(test)]\nfn t() {\n    use crate::value::member::Member;\n}\n",
        );
        assert_eq!(violations_of(dir.path()), vec![("complete".to_owned(), 2)]);
    }

    /// TC-175 step 6: a `crate::` path inside a macro invocation's arguments
    /// (`vec![crate::complete::X]`, `format!("{}", crate::value::member::M)`) is
    /// scanned from the macro's tokens.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn crate_path_inside_macro_arguments_is_a_violation() {
        let dir = layer_fixture_root();
        write(
            dir.path(),
            "qsl-semantics/src/check/fixture.rs",
            "pub fn f() {\n    let _ = vec![crate::complete::X];\n    let _ = format!(\"{}\", crate::value::member::M);\n}\n",
        );
        assert_eq!(
            violations_of(dir.path()),
            vec![("complete".to_owned(), 2), ("value::member".to_owned(), 3)]
        );
    }

    /// TC-175 step 6 / FR-068-AC-6 ("A `super::` or `self::` path is
    /// resolved relative to its file"): `self::super::super::complete::C`
    /// from `check/fixture.rs` resolves to `complete` and fails.
    #[trace("TC-175", "FR-068-AC-6")]
    #[test]
    fn self_then_super_path_resolves_relative_to_its_file() {
        let dir = layer_fixture_root();
        write(
            dir.path(),
            "qsl-semantics/src/check/fixture.rs",
            "pub fn f() -> self::super::super::complete::C {\n    todo!()\n}\n",
        );
        assert_eq!(violations_of(dir.path()), vec![("complete".to_owned(), 1)]);
        let current = vec!["check".to_owned(), "fixture".to_owned()];
        let raw: Vec<String> = ["self", "super", "family"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        assert_eq!(
            resolve_relative_path(&raw, &current),
            vec!["check", "family"]
        );
    }
}
