// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-139 (FR-068) TC-170/TC-171/TC-173: resolve "exactly one defining
//! location" claims against the real, whole source tree, not a claim about
//! four named methods or twelve named types read in isolation.
//!
//! TC-170 is explicit that this must fail on *duplication*, not only on
//! absence: an implementation that adds `check`'s four modules while
//! leaving `value::expression`'s old `mod check;`/`mod facts;`/`mod ir;`/
//! `mod termination;` declarations (and their contents) in place would pass
//! an absence-only check but must fail this one. This module answers that
//! by scanning the whole crate (not only `src/check/`) for every defining
//! location of each name TC-170/TC-173 name, and failing loudly on any name
//! with more than one.
//!
//! **Scope of what this catches (TC-170 step 6).** This scans by *name*:
//! a `struct`, `enum`, `fn`, `const` or `type` item's own identifier, and an
//! `impl` method's `(Self type, method name)` pair. It does not compare two
//! *differently named* items' shapes for structural equivalence (TC-170's
//! own hypothetical "a stray `src/value/expression/typing.rs` still
//! defining a second `Typer`-adjacent type under a different name") --
//! that would need a general structural-similarity engine, which is
//! speculative build cost against a scenario this module's
//! [`mod_declarations`] check already forecloses for the one path that
//! matters here: any such stray file would need its own `mod` declaration
//! somewhere reachable to be part of the compiled crate at all (an
//! undeclared `.rs` file is inert, not a hidden duplicate), and
//! [`mod_declarations`] enumerates *every* `mod` item in a file, not only
//! the four this requirement names, so an unexpected fifth declaration in
//! `value/expression/mod.rs` is itself a finding. This mirrors TC-171's own
//! established precedent in this repository (its Description records why it
//! is Inspection, not a `trybuild`/`compiletest` dependency, for an
//! analogous reason): where a criterion's full literal scope would need new
//! build-time machinery this PR does not otherwise need, the gap is
//! recorded here rather than silently declared closed.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use syn::visit::Visit;

use crate::error::{Error, Result};

fn parse_file(workspace_root: &Path, relative: &str) -> Result<syn::File> {
    let path = workspace_root.join(relative);
    let source = fs::read_to_string(&path).map_err(|source| Error::io(&path, source))?;
    syn::parse_file(&source).map_err(|source| Error::ImportGraphParse { path, source })
}

/// One item's or method's defining location.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Definition {
    /// The file this definition was found in, relative to the workspace root.
    pub file: String,
    /// The 1-based line the definition starts on.
    pub line: u32,
}

fn line_of<T: syn::spanned::Spanned>(node: &T) -> u32 {
    node.span().start().line as u32
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

struct DefScanner {
    file: String,
    items: BTreeMap<String, Vec<Definition>>,
    methods: BTreeMap<(String, String), Vec<Definition>>,
    current_impl_self: Option<String>,
}

impl DefScanner {
    fn record_item(&mut self, name: String, line: u32) {
        self.items.entry(name).or_default().push(Definition {
            file: self.file.clone(),
            line,
        });
    }

    fn record_method(&mut self, self_ty: String, method: String, line: u32) {
        self.methods
            .entry((self_ty, method))
            .or_default()
            .push(Definition {
                file: self.file.clone(),
                line,
            });
    }
}

impl<'ast> Visit<'ast> for DefScanner {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        syn::visit::visit_item_mod(self, node);
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        let previous = self
            .current_impl_self
            .replace(crate::impl_self_name(&node.self_ty));
        syn::visit::visit_item_impl(self, node);
        self.current_impl_self = previous;
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        let line = line_of(node);
        if let Some(self_ty) = &self.current_impl_self {
            self.record_method(self_ty.clone(), node.sig.ident.to_string(), line);
        }
        syn::visit::visit_impl_item_fn(self, node);
    }

    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        self.record_item(node.ident.to_string(), line_of(node));
        syn::visit::visit_item_struct(self, node);
    }

    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        self.record_item(node.ident.to_string(), line_of(node));
        syn::visit::visit_item_enum(self, node);
    }

    fn visit_item_const(&mut self, node: &'ast syn::ItemConst) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        self.record_item(node.ident.to_string(), line_of(node));
        syn::visit::visit_item_const(self, node);
    }

    fn visit_item_type(&mut self, node: &'ast syn::ItemType) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        self.record_item(node.ident.to_string(), line_of(node));
        syn::visit::visit_item_type(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        self.record_item(node.sig.ident.to_string(), line_of(node));
        syn::visit::visit_item_fn(self, node);
    }
}

/// Every non-test `.rs` file under `src/`, relative to `workspace_root`
/// (`tests`, `target` and `.git` directory entries excluded, matching
/// `xtask::string_edge::source_files`'s own exclusion list).
fn source_files(workspace_root: &Path) -> Result<Vec<String>> {
    let root = workspace_root.join("src");
    let mut files = Vec::new();
    let mut pending = vec![root];
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
                let relative = path.strip_prefix(workspace_root).unwrap_or(&path);
                files.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    files.sort();
    Ok(files)
}

/// A full scan of `src/`: every top-level `struct`/`enum`/`const`/`type`
/// item's defining locations by name, and every `impl` method's defining
/// locations by `(Self type, method name)`.
pub struct CrateDefinitions {
    /// Name -> every location defining an item of that name.
    pub items: BTreeMap<String, Vec<Definition>>,
    /// `(Self type, method name)` -> every location defining that method.
    pub methods: BTreeMap<(String, String), Vec<Definition>>,
}

/// Scan the whole crate's `src/` tree once.
pub fn scan_crate(workspace_root: &Path) -> Result<CrateDefinitions> {
    let mut items: BTreeMap<String, Vec<Definition>> = BTreeMap::new();
    let mut methods: BTreeMap<(String, String), Vec<Definition>> = BTreeMap::new();
    for file in source_files(workspace_root)? {
        let parsed = parse_file(workspace_root, &file)?;
        let mut scanner = DefScanner {
            file: file.clone(),
            items: BTreeMap::new(),
            methods: BTreeMap::new(),
            current_impl_self: None,
        };
        scanner.visit_file(&parsed);
        for (name, locations) in scanner.items {
            items.entry(name).or_default().extend(locations);
        }
        for (key, locations) in scanner.methods {
            methods.entry(key).or_default().extend(locations);
        }
    }
    Ok(CrateDefinitions { items, methods })
}

/// Every non-`#[cfg(test)]` `mod` item's name declared at the top level of
/// one file (not recursing into nested `mod { ... }` blocks) -- used to
/// confirm no unexpected module declaration exists alongside the ones a
/// requirement names explicitly (see this module's own doc, "Scope of what
/// this catches"). A `#[cfg(test)] mod tests { ... }` block is test
/// scaffolding, not a compiled-in production module the requirement is
/// concerned with, so it is excluded the same way this module's own
/// `DefScanner` excludes `#[cfg(test)]` items elsewhere in this file.
pub fn mod_declarations(workspace_root: &Path, relative: &str) -> Result<Vec<String>> {
    let parsed = parse_file(workspace_root, relative)?;
    Ok(parsed
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Mod(item_mod) if !has_cfg_test(&item_mod.attrs) => {
                Some(item_mod.ident.to_string())
            }
            _ => None,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use std::path::PathBuf;

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask has a parent directory")
            .to_path_buf()
    }

    /// TC-170 step 2: `value::expression::mod.rs` declares none of `check`,
    /// `facts`, `ir`, `termination`; step 6's mod-declaration half (see this
    /// module's own doc): its *only* declarations are the two the post-move
    /// tree keeps (`evaluate`, `family` -- `family` stays split, since
    /// `value::expression::family.rs` keeps the evaluation half and
    /// `check::family.rs` keeps the checking half, PR #282 review F4), so an
    /// unexpected fifth module cannot be hiding a renamed leftover here.
    #[trace("TC-170", "FR-068-AC-1")]
    #[test]
    fn value_expression_mod_declares_no_check_stage_module() {
        let mods =
            mod_declarations(&workspace_root(), "src/value/expression/mod.rs").expect("scan runs");
        for forbidden in ["check", "facts", "ir", "termination"] {
            assert!(
                !mods.iter().any(|name| name == forbidden),
                "value::expression::mod.rs still declares mod {forbidden};"
            );
        }
        let expected: std::collections::BTreeSet<&str> =
            ["evaluate", "family"].into_iter().collect();
        let actual: std::collections::BTreeSet<&str> = mods.iter().map(String::as_str).collect();
        assert_eq!(
            actual, expected,
            "value::expression::mod.rs declares an unexpected module beyond evaluate/family"
        );
    }

    /// TC-170 step 1: `check` is declared at the crate root.
    #[trace("TC-170", "FR-068-AC-1")]
    #[test]
    fn crate_root_declares_check() {
        let mods = mod_declarations(&workspace_root(), "src/lib.rs").expect("scan runs");
        assert!(mods.iter().any(|name| name == "check"));
    }

    /// Asserts `name` is defined exactly once under `src/check/` and not at
    /// all under `src/value/expression/` -- TC-170's own actual intent ("no
    /// leftover in `value::expression`, no duplicate under `check`"), not a
    /// literal crate-wide uniqueness claim. Some CON-3/TC-173 names (e.g.
    /// `Obligation`) collide, by plain English word, with unrelated,
    /// pre-existing types elsewhere in the crate (`checking::composed::Obligation`,
    /// a struct about a SEAM obligation site; `temporal::result::Obligation`,
    /// an enum about instance-assessment outcomes) that have nothing to do
    /// with this split and are out of scope for it; a global `locations.len()
    /// == 1` assertion would wrongly fail on those legitimate namesakes, so
    /// this only looks at the two directories the move actually concerns.
    fn assert_defined_exactly_once_under_check(name: &str, locations: &[Definition]) {
        let relevant: Vec<&Definition> = locations
            .iter()
            .filter(|location| {
                location.file.starts_with("src/check/")
                    || location.file.starts_with("src/value/expression/")
            })
            .collect();
        assert_eq!(
            relevant.len(),
            1,
            "{name} has {} defining location(s) under src/check/ or src/value/expression/: {relevant:?} (all locations: {locations:?})",
            relevant.len()
        );
        assert!(
            relevant[0].file.starts_with("src/check/"),
            "{name} is defined at {:?}, not under src/check/",
            relevant[0]
        );
    }

    /// TC-170 steps 4-5: each of the four checking methods, and each of the
    /// twenty-one CON-3-named symbols, has exactly one defining location
    /// under `check`/`value::expression` (see
    /// [`assert_defined_exactly_once_under_check`] for why this is scoped
    /// rather than crate-wide).
    #[trace("TC-170", "FR-068-CON-3")]
    #[test]
    fn con3_methods_and_symbols_each_have_exactly_one_defining_location() {
        let definitions = scan_crate(&workspace_root()).expect("scan runs");
        let methods = [
            ("PackageDeclarations", "check"),
            ("CheckedPackage", "check_expression"),
            ("CheckedPackage", "check_postcondition_expression"),
            ("CheckedPackage", "check_clause_expression"),
        ];
        for (self_ty, method) in methods {
            let key = (self_ty.to_owned(), method.to_owned());
            let locations = definitions.methods.get(&key).cloned().unwrap_or_default();
            assert_eq!(
                locations.len(),
                1,
                "{self_ty}::{method} has {} defining location(s): {locations:?}",
                locations.len()
            );
        }
        let symbols = [
            "CheckCause",
            "CheckRefusal",
            "Obligation",
            "MeasureObligation",
            "CheckingStage",
            "CheckingLimitKind",
            "DispatchFunctionRole",
            "InvalidDispatchDeclaration",
            "Location",
            "Origin",
            "ProvedInterval",
            "WrongSnapshotCause",
            "CheckedPackage",
            "CheckedExpression",
            "CheckedFunction",
            "PackageDeclarations",
            "CheckingLimits",
            "DepthAboveMaximum",
            "DispatchOperation",
            "EnumBinding",
            "MAX_CHECKING_DEPTH",
            "CheckMode",
        ];
        for symbol in symbols {
            let locations = definitions.items.get(symbol).cloned().unwrap_or_default();
            assert_defined_exactly_once_under_check(symbol, &locations);
        }
    }

    /// TC-173 step 1: the twelve check-cause types are all defined under
    /// `check`, none under `value::expression` (see
    /// [`assert_defined_exactly_once_under_check`] for why this is scoped
    /// rather than crate-wide).
    #[trace("TC-173", "FR-068-AC-4")]
    #[test]
    fn twelve_check_cause_types_are_defined_exactly_once_under_check() {
        let definitions = scan_crate(&workspace_root()).expect("scan runs");
        let names = [
            "CheckCause",
            "CheckRefusal",
            "Obligation",
            "MeasureObligation",
            "CheckingStage",
            "CheckingLimitKind",
            "DispatchFunctionRole",
            "InvalidDispatchDeclaration",
            "Location",
            "Origin",
            "ProvedInterval",
            "WrongSnapshotCause",
        ];
        for name in names {
            let locations = definitions.items.get(name).cloned().unwrap_or_default();
            assert_defined_exactly_once_under_check(name, &locations);
        }
    }

    /// TC-175 step 3/FR-068-AC-7: `model::checked_dispatch` and
    /// `model::conformance::check_field_refinement_obligation` -- M-2's own
    /// items -- remain defined in `model`, unmoved, and are absent from
    /// `check`; an implementation that folds either in early (ahead of the
    /// corrected M-5-before-M-2 order) would pass every other criterion in
    /// this requirement while doing work QSL-7 (M-2) owns.
    #[trace("TC-175", "FR-068-AC-7")]
    #[test]
    fn m2_items_stay_in_model_and_are_absent_from_check() {
        let definitions = scan_crate(&workspace_root()).expect("scan runs");
        for name in [
            "checked_dispatch_operation",
            "check_field_refinement_obligation",
        ] {
            let locations = definitions.items.get(name).cloned().unwrap_or_default();
            assert_eq!(locations.len(), 1, "{name}: {locations:?}");
            assert!(
                locations[0].file.starts_with("src/model/"),
                "{name} must stay in src/model/: {locations:?}"
            );
            assert!(
                !locations[0].file.starts_with("src/check/"),
                "{name}: {locations:?}"
            );
        }
    }

    /// TC-173 step 2: `InputRefusal` is defined exactly once, and it is
    /// under `value::expression`, not `check`.
    #[trace("TC-173", "FR-068-AC-4")]
    #[test]
    fn input_refusal_is_defined_exactly_once_under_value_expression() {
        let definitions = scan_crate(&workspace_root()).expect("scan runs");
        let locations = definitions
            .items
            .get("InputRefusal")
            .cloned()
            .unwrap_or_default();
        assert_eq!(locations.len(), 1, "{locations:?}");
        assert!(
            locations[0].file.starts_with("src/value/expression/"),
            "InputRefusal: {locations:?}"
        );
        assert!(!locations[0].file.starts_with("src/check/"));
    }
}
