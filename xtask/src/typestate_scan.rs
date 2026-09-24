// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-158 (ADR-013 §7 S-3): the whole-tree scans behind the typestate,
//! cross-package key and qualified-name criteria that are claims about where
//! an item is defined, what its fields are, and who calls what.
//!
//! [`scan`] parses every shipped `.rs` file under the named directories and
//! records, per item, only what those criteria ask about: each struct's
//! fields (name, whether private, and every type name its type mentions),
//! each enum's variants, each function's return type and every path it
//! names, whether it mints a `NodeKey`, and each `use` item.
//!
//! **Shipped code.** An item under `#[cfg(test)]` is skipped, and so is a
//! file declared by a `#[cfg(test)] mod x;` (with or without `#[path]`),
//! together with every file below that module. A file's name decides
//! nothing: a `forge_tests.rs` no `#[cfg(test)]` declaration reaches is
//! scanned. `tests/` directories (integration tests) are not scanned.
//!
//! **Aliases.** A `use … as X` rename and a `type X = …;` alias, in any
//! scanned file, make `X` stand for every name its target mentions. Every
//! recorded name set is closed over them, so `type Forged = CheckedPackage;`
//! and `use quire_exact::NodeKey as Nk;` are seen through. The alias table is
//! global, not per scope, so a name reused for two aliases over-approximates.
//!
//! **Mints.** A function, method, trait default method, `static` or `const`
//! mints a `NodeKey` if it names `NodeKey::from_digest` through any alias,
//! calls `<NodeKey>::from_digest` through a qualified self type, calls
//! `Self::from_digest` inside an `impl NodeKey`, calls a `node_key_of`
//! helper, or writes any of those inside a macro's tokens.
//!
//! **Known limits.** A macro body is read as flat tokens: the mint check
//! finds `Name::from_digest` there, but no other name set sees inside it,
//! and a macro that builds the path from fragments is invisible. A value
//! handed out through an out-parameter (`&mut Option<T>`) or a callback is
//! not an owned return. A function reached only through a function pointer
//! or trait object is recorded where it is defined, with no call graph. A
//! `#[cfg(test)]` spelled inside `cfg_attr` or `any(...)` is not recognized,
//! so such code is scanned, which errs toward reporting it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use syn::visit::Visit;

use crate::definition_scan::{has_cfg_test, parse_file, source_files};
use crate::error::Result;

/// One struct field or tuple-struct element.
#[derive(Clone, Debug)]
pub struct Field {
    /// The field's name; `None` for a tuple field.
    pub name: Option<String>,
    /// Whether the field has no visibility qualifier (private to its module).
    pub private: bool,
    /// Every path segment name the field's type mentions, at any depth.
    pub type_names: BTreeSet<String>,
    /// Every type name the key type of a `BTreeMap`, `HashMap` or
    /// `IndexMap` in the field's type mentions.
    pub map_keys: BTreeSet<String>,
}

/// One struct definition.
#[derive(Clone, Debug)]
pub struct Struct {
    /// The struct's name.
    pub name: String,
    /// The file it is defined in, relative to the workspace root.
    pub file: String,
    /// Whether it declares a type or const generic parameter (a lifetime is
    /// not a state parameter).
    pub generic: bool,
    /// Its fields, in declaration order.
    pub fields: Vec<Field>,
}

/// One enum definition.
#[derive(Clone, Debug)]
pub struct Enum {
    /// The enum's name.
    pub name: String,
    /// The file it is defined in, relative to the workspace root.
    pub file: String,
    /// Each variant's name and fields, in declaration order.
    pub variants: Vec<(String, Vec<Field>)>,
}

/// One function, method, trait default method, `static` or `const`.
#[derive(Clone, Debug)]
pub struct Function {
    /// The file it is defined in, relative to the workspace root.
    pub file: String,
    /// The `impl` block's `Self` type name, or the trait's name.
    pub self_type: Option<String>,
    /// The item's name.
    pub name: String,
    /// Every type name its return type (a `static`'s or `const`'s own type)
    /// mentions outside a reference: the types it hands back owned.
    pub owned_return: BTreeSet<String>,
    /// Every type name the key type of a map in its return type mentions.
    pub return_map_keys: BTreeSet<String>,
    /// Every path segment name its signature and body mention.
    pub names: BTreeSet<String>,
    /// Whether it mints a `NodeKey` (this module's doc, "Mints").
    pub mints_node_key: bool,
}

/// One `use` item.
#[derive(Clone, Debug)]
pub struct Use {
    /// The file it is in, relative to the workspace root.
    pub file: String,
    /// Whether it is a `pub` (or `pub(..)`) re-export.
    pub public: bool,
    /// Every path segment name it mentions, `*` for a glob.
    pub names: BTreeSet<String>,
}

/// Everything [`scan`] records.
#[derive(Clone, Debug, Default)]
pub struct Items {
    /// Every shipped struct.
    pub structs: Vec<Struct>,
    /// Every shipped enum.
    pub enums: Vec<Enum>,
    /// Every shipped function, method, `static` and `const`.
    pub functions: Vec<Function>,
    /// Every shipped `use` item.
    pub uses: Vec<Use>,
}

impl Items {
    /// Every struct named `name`.
    pub fn structs_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Struct> + 'a {
        self.structs.iter().filter(move |item| item.name == name)
    }
}

/// The alias table, from `use … as X` and `type X = …;`.
#[derive(Default)]
struct Aliases {
    /// Alias name -> every name its target mentions.
    targets: BTreeMap<String, BTreeSet<String>>,
    /// `type` alias name -> every name its target's map key types mention.
    map_keys: BTreeMap<String, BTreeSet<String>>,
}

impl Aliases {
    /// `names` closed over the alias table.
    fn close(&self, names: &BTreeSet<String>) -> BTreeSet<String> {
        let mut closed = names.clone();
        let mut pending: Vec<String> = names.iter().cloned().collect();
        while let Some(name) = pending.pop() {
            for target in self.targets.get(&name).into_iter().flatten() {
                if closed.insert(target.clone()) {
                    pending.push(target.clone());
                }
            }
        }
        closed
    }

    /// The map key names of a type that mentions `type_names` and whose own
    /// map keys are `direct`: `direct` plus the keys of every map-typed
    /// alias it names, closed over the alias table.
    fn map_keys(
        &self,
        type_names: &BTreeSet<String>,
        direct: &BTreeSet<String>,
    ) -> BTreeSet<String> {
        let mut keys = direct.clone();
        for name in self.close(type_names) {
            keys.extend(self.map_keys.get(&name).into_iter().flatten().cloned());
        }
        self.close(&keys)
    }

    /// Whether `name` is `NodeKey` or an alias of it.
    fn is_node_key(&self, name: &str) -> bool {
        self.close(&BTreeSet::from([name.to_owned()]))
            .contains("NodeKey")
    }
}

impl<'ast> Visit<'ast> for Aliases {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if !has_cfg_test(&node.attrs) {
            syn::visit::visit_item_mod(self, node);
        }
    }

    fn visit_use_rename(&mut self, node: &'ast syn::UseRename) {
        self.targets
            .entry(node.rename.to_string())
            .or_default()
            .insert(node.ident.to_string());
    }

    fn visit_item_type(&mut self, node: &'ast syn::ItemType) {
        if !has_cfg_test(&node.attrs) {
            let mut collector = NameCollector::default();
            collector.visit_type(&node.ty);
            let name = node.ident.to_string();
            self.targets
                .entry(name.clone())
                .or_default()
                .extend(collector.names);
            self.map_keys
                .entry(name)
                .or_default()
                .extend(collector.map_keys);
        }
    }
}

#[derive(Default)]
struct NameCollector {
    names: BTreeSet<String>,
    map_keys: BTreeSet<String>,
    skip_references: bool,
}

impl<'ast> Visit<'ast> for NameCollector {
    fn visit_path_segment(&mut self, node: &'ast syn::PathSegment) {
        self.names.insert(node.ident.to_string());
        if ["BTreeMap", "HashMap", "IndexMap"].contains(&node.ident.to_string().as_str()) {
            if let syn::PathArguments::AngleBracketed(arguments) = &node.arguments {
                if let Some(syn::GenericArgument::Type(key)) = arguments.args.first() {
                    self.map_keys.extend(type_names(key));
                }
            }
        }
        syn::visit::visit_path_segment(self, node);
    }

    fn visit_type_reference(&mut self, node: &'ast syn::TypeReference) {
        if !self.skip_references {
            syn::visit::visit_type_reference(self, node);
        }
    }

    fn visit_use_name(&mut self, node: &'ast syn::UseName) {
        self.names.insert(node.ident.to_string());
    }

    fn visit_use_rename(&mut self, node: &'ast syn::UseRename) {
        self.names.insert(node.ident.to_string());
        self.names.insert(node.rename.to_string());
    }

    fn visit_use_path(&mut self, node: &'ast syn::UsePath) {
        self.names.insert(node.ident.to_string());
        syn::visit::visit_use_path(self, node);
    }

    fn visit_use_glob(&mut self, _: &'ast syn::UseGlob) {
        self.names.insert("*".to_owned());
    }
}

/// Every path segment name `ty` mentions, at any depth.
fn type_names(ty: &syn::Type) -> BTreeSet<String> {
    let mut collector = NameCollector::default();
    collector.visit_type(ty);
    collector.names
}

/// `ty`'s owned type names (references skipped) and its map key names.
fn owned_type_names(ty: &syn::Type) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut owned = NameCollector {
        skip_references: true,
        ..NameCollector::default()
    };
    owned.visit_type(ty);
    let mut keys = NameCollector::default();
    keys.visit_type(ty);
    (owned.names, keys.map_keys)
}

struct BodyScan<'a> {
    aliases: &'a Aliases,
    self_type: Option<&'a str>,
    names: BTreeSet<String>,
    mints: bool,
}

impl BodyScan<'_> {
    /// Whether `path` is `NodeKey::from_digest` under any alias,
    /// `Self::from_digest` in an `impl NodeKey`, or a `node_key_of` helper.
    fn is_mint_path(&self, path: &syn::Path) -> bool {
        let segments: Vec<String> = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect();
        let owner_is_node_key = |owner: &str| {
            self.aliases.is_node_key(owner)
                || (owner == "Self"
                    && self
                        .self_type
                        .is_some_and(|ty| self.aliases.is_node_key(ty)))
        };
        segments
            .windows(2)
            .any(|pair| pair[1] == "from_digest" && owner_is_node_key(&pair[0]))
            || segments.last().is_some_and(|last| last == "node_key_of")
    }
}

impl<'ast> Visit<'ast> for BodyScan<'_> {
    fn visit_path(&mut self, node: &'ast syn::Path) {
        if self.is_mint_path(node) {
            self.mints = true;
        }
        syn::visit::visit_path(self, node);
    }

    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        if let Some(qself) = &node.qself {
            let last = node
                .path
                .segments
                .last()
                .map(|segment| segment.ident.to_string());
            if last.as_deref() == Some("from_digest")
                && type_names(&qself.ty)
                    .iter()
                    .any(|name| self.aliases.is_node_key(name))
            {
                self.mints = true;
            }
        }
        syn::visit::visit_expr_path(self, node);
    }

    fn visit_path_segment(&mut self, node: &'ast syn::PathSegment) {
        self.names.insert(node.ident.to_string());
        syn::visit::visit_path_segment(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        let tokens = node.tokens.to_string().replace(' ', "");
        let mut owners = self
            .aliases
            .targets
            .keys()
            .filter(|name| self.aliases.is_node_key(name))
            .map(String::as_str)
            .chain(["NodeKey"]);
        // `NodeKey::from_digest`, and `<…NodeKey>::from_digest`.
        if owners.any(|owner| {
            tokens.contains(&format!("{owner}::from_digest"))
                || tokens.contains(&format!("{owner}>::from_digest"))
        }) || tokens.contains("node_key_of(")
        {
            self.mints = true;
        }
        syn::visit::visit_macro(self, node);
    }
}

struct Scanner<'a> {
    file: &'a str,
    aliases: &'a Aliases,
    items: &'a mut Items,
    self_type: Option<String>,
}

impl Scanner<'_> {
    fn record(
        &mut self,
        name: String,
        output: Option<&syn::Type>,
        visit: impl FnOnce(&mut BodyScan<'_>),
    ) {
        let (mut owned_return, return_map_keys) = output.map(owned_type_names).unwrap_or_default();
        if owned_return.remove("Self") {
            owned_return.extend(self.self_type.clone());
        }
        let mut body = BodyScan {
            aliases: self.aliases,
            self_type: self.self_type.as_deref(),
            names: BTreeSet::new(),
            mints: false,
        };
        visit(&mut body);
        self.items.functions.push(Function {
            file: self.file.to_owned(),
            self_type: self.self_type.clone(),
            name,
            owned_return: self.aliases.close(&owned_return),
            return_map_keys: self.aliases.map_keys(&owned_return, &return_map_keys),
            names: self.aliases.close(&body.names),
            mints_node_key: body.mints,
        });
    }

    fn function(&mut self, sig: &syn::Signature, block: &syn::Block) {
        let output = match &sig.output {
            syn::ReturnType::Default => None,
            syn::ReturnType::Type(_, ty) => Some(&**ty),
        };
        self.record(sig.ident.to_string(), output, |body| {
            body.visit_signature(sig);
            body.visit_block(block);
        });
        // Items declared inside the body are items too.
        let previous = self.self_type.take();
        syn::visit::visit_block(self, block);
        self.self_type = previous;
    }

    fn fields(&self, fields: &syn::Fields) -> Vec<Field> {
        fields
            .iter()
            .map(|field| {
                let mut collector = NameCollector::default();
                collector.visit_type(&field.ty);
                Field {
                    name: field.ident.as_ref().map(ToString::to_string),
                    private: matches!(field.vis, syn::Visibility::Inherited),
                    type_names: self.aliases.close(&collector.names),
                    map_keys: self.aliases.map_keys(&collector.names, &collector.map_keys),
                }
            })
            .collect()
    }
}

impl<'ast> Visit<'ast> for Scanner<'_> {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if !has_cfg_test(&node.attrs) {
            syn::visit::visit_item_mod(self, node);
        }
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        let previous = self.self_type.replace(crate::impl_self_name(&node.self_ty));
        syn::visit::visit_item_impl(self, node);
        self.self_type = previous;
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        let previous = self.self_type.replace(node.ident.to_string());
        syn::visit::visit_item_trait(self, node);
        self.self_type = previous;
    }

    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        if let Some(block) = &node.default {
            self.function(&node.sig, block);
        }
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        if !has_cfg_test(&node.attrs) {
            self.function(&node.sig, &node.block);
        }
    }

    fn visit_impl_item_const(&mut self, node: &'ast syn::ImplItemConst) {
        if !has_cfg_test(&node.attrs) {
            self.record(node.ident.to_string(), Some(&node.ty), |body| {
                body.visit_expr(&node.expr);
            });
        }
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        if !has_cfg_test(&node.attrs) {
            let previous = self.self_type.take();
            self.function(&node.sig, &node.block);
            self.self_type = previous;
        }
    }

    fn visit_item_static(&mut self, node: &'ast syn::ItemStatic) {
        if !has_cfg_test(&node.attrs) {
            self.record(node.ident.to_string(), Some(&node.ty), |body| {
                body.visit_expr(&node.expr);
            });
        }
    }

    fn visit_item_const(&mut self, node: &'ast syn::ItemConst) {
        if !has_cfg_test(&node.attrs) {
            self.record(node.ident.to_string(), Some(&node.ty), |body| {
                body.visit_expr(&node.expr);
            });
        }
    }

    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        let fields = self.fields(&node.fields);
        self.items.structs.push(Struct {
            name: node.ident.to_string(),
            file: self.file.to_owned(),
            generic: node
                .generics
                .params
                .iter()
                .any(|param| !matches!(param, syn::GenericParam::Lifetime(_))),
            fields,
        });
    }

    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        let variants = node
            .variants
            .iter()
            .map(|variant| (variant.ident.to_string(), self.fields(&variant.fields)))
            .collect();
        self.items.enums.push(Enum {
            name: node.ident.to_string(),
            file: self.file.to_owned(),
            variants,
        });
    }

    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        let mut collector = NameCollector::default();
        collector.visit_use_tree(&node.tree);
        self.items.uses.push(Use {
            file: self.file.to_owned(),
            public: !matches!(node.vis, syn::Visibility::Inherited),
            names: collector.names,
        });
    }
}

/// `path` with `.` and `..` components resolved lexically.
fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other),
        }
    }
    normalized
}

/// The directory a file's `mod x;` children live in: the file's own
/// directory for `lib.rs`, `main.rs` and `mod.rs`, else `<dir>/<stem>/`.
fn child_dir(file: &Path) -> PathBuf {
    let parent = file.parent().unwrap_or_else(|| Path::new(""));
    match file.file_stem().and_then(|stem| stem.to_str()) {
        Some("lib" | "main" | "mod") | None => parent.to_path_buf(),
        Some(stem) => parent.join(stem),
    }
}

/// The `#[path = "..."]` value on a `mod` item, if any.
fn path_attribute(attrs: &[syn::Attribute]) -> Option<String> {
    attrs.iter().find_map(|attr| match &attr.meta {
        syn::Meta::NameValue(meta) if meta.path.is_ident("path") => match &meta.value {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(value),
                ..
            }) => Some(value.value()),
            _ => None,
        },
        _ => None,
    })
}

/// Every file (and directory prefix) a `#[cfg(test)] mod x;` in `items`
/// declares, resolved from `file`'s location; `dir` is the directory the
/// current module's children live in.
fn test_modules(file: &Path, dir: &Path, items: &[syn::Item], out: &mut BTreeSet<PathBuf>) {
    for item in items {
        let syn::Item::Mod(module) = item else {
            continue;
        };
        let name = module.ident.to_string();
        match (&module.content, has_cfg_test(&module.attrs)) {
            (None, true) => {
                if let Some(path) = path_attribute(&module.attrs) {
                    let parent = file.parent().unwrap_or_else(|| Path::new(""));
                    out.insert(normalize(&parent.join(path)));
                } else {
                    out.insert(dir.join(format!("{name}.rs")));
                    out.insert(dir.join(&name));
                }
            }
            (Some((_, inner)), false) => test_modules(file, &dir.join(&name), inner, out),
            _ => {}
        }
    }
}

/// Parse every `.rs` file under each of `dirs` (relative to
/// `workspace_root`; `tests/` directories excluded) and record the items of
/// the shipped ones (this module's doc, "Shipped code").
pub fn scan(workspace_root: &Path, dirs: &[&str]) -> Result<Items> {
    let mut parsed = Vec::new();
    for dir in dirs {
        for file in source_files(workspace_root, dir)? {
            let ast = parse_file(workspace_root, &file)?;
            parsed.push((file, ast));
        }
    }
    let mut test_only = BTreeSet::new();
    for (file, ast) in &parsed {
        let path = Path::new(file);
        test_modules(path, &child_dir(path), &ast.items, &mut test_only);
    }
    let shipped: Vec<&(String, syn::File)> = parsed
        .iter()
        .filter(|(file, _)| {
            let path = Path::new(file);
            !test_only.iter().any(|excluded| path.starts_with(excluded))
        })
        .collect();
    let mut aliases = Aliases::default();
    for (_, ast) in &shipped {
        aliases.visit_file(ast);
    }
    let mut items = Items::default();
    for (file, ast) in shipped {
        Scanner {
            file,
            aliases: &aliases,
            items: &mut items,
            self_type: None,
        }
        .visit_file(ast);
    }
    Ok(items)
}

/// The layer crates ADR-011 §6.1 places the canonical stage types in:
/// layer 3 `qsl-semantics`, layer 4 `qsl-package`, layer 5 `qsl-eval` and
/// layer 6 `qsl-replay`.
pub const LAYER_CRATES: [&str; 4] = [
    "qsl-semantics/src",
    "qsl-package/src",
    "qsl-eval/src",
    "qsl-replay/src",
];

/// Every crate of the QSL build a "whole crate" criterion covers: the root
/// crate and every workspace library crate. Not scanned: `xtask` and
/// `tools/arch-lint` (developer tooling) and `qsl-bench` (the benchmark
/// harness, which builds its fixture keys directly and ships nothing).
pub const QSL_CRATES: [&str; 12] = [
    "src",
    "quire-exact/src",
    "qsl-attrs/src",
    "qsl-foundation/src",
    "qsl-cst/src",
    "qsl-source/src",
    "qsl-forms/src",
    "qsl-semantics/src",
    "qsl-package/src",
    "qsl-route/src",
    "qsl-eval/src",
    "qsl-replay/src",
];

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask has a parent directory")
            .to_path_buf()
    }

    fn qsl_items() -> Items {
        scan(&workspace_root(), &QSL_CRATES).expect("scan runs")
    }

    fn in_layer_crate(file: &str) -> bool {
        LAYER_CRATES.iter().any(|dir| file.starts_with(dir))
    }

    /// A scratch source tree of `files` (path, contents), scanned as one
    /// directory `src`.
    fn scan_tree(files: &[(&str, &str)]) -> Items {
        let root = tempfile::tempdir().expect("a temp dir");
        for (path, contents) in files {
            let path = root.path().join(path);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("dirs");
            std::fs::write(path, contents).expect("write");
        }
        scan(root.path(), &["src"]).expect("scan runs")
    }

    fn minting(items: &Items) -> BTreeSet<String> {
        items
            .functions
            .iter()
            .filter(|function| function.mints_node_key)
            .map(|function| function.name.clone())
            .collect()
    }

    /// ADR-013 T-1's five new stage outputs and the file that owns each.
    const STAGE_TYPES: [(&str, &str); 5] = [
        ("CheckedGraph", "qsl-semantics/src/check/mod.rs"),
        ("CheckedPackage", "qsl-package/src/checked.rs"),
        ("EmittedPackage", "qsl-package/src/checked.rs"),
        ("VerifiedPackage", "qsl-semantics/src/library/mod.rs"),
        ("ImportView", "qsl-semantics/src/library/mod.rs"),
    ];

    /// The root-crate namesakes FR-087-AC-8 and AC-10 name: lane-private
    /// types that retire with their lanes, not second definitions.
    const LANE_PRIVATE_NAMESAKES: [(&str, &str); 2] = [
        ("CheckedPackage", "src/checking.rs"),
        ("EmittedPackage", "src/protocol_artifact/native/mod.rs"),
    ];

    /// FR-087-AC-1 (TC-243 steps 1-3) and ADR-013 T-1: each stage-output
    /// type has one definition in the layer crates, in the file that owns
    /// it, with every field private and no type or const generic
    /// parameter; elsewhere in the QSL crates the only namesakes are the
    /// two lane-private ones. Private fields stay readable by the owning
    /// module's own child modules, so this does not prove step 4's
    /// accessor-only reads; TC-244's constructor allow-list covers the
    /// constructor half.
    #[trace("TC-243", "FR-087-AC-1")]
    #[test]
    fn tc_243_stage_types_have_one_owner_and_private_fields() {
        let items = qsl_items();
        for (name, owner) in STAGE_TYPES {
            let (layer, other): (Vec<&Struct>, Vec<&Struct>) = items
                .structs_named(name)
                .partition(|item| in_layer_crate(&item.file));
            assert_eq!(layer.len(), 1, "{name}: {layer:?}");
            let definition = layer[0];
            assert_eq!(
                definition.file, owner,
                "{name} is defined outside its owner"
            );
            assert!(
                !definition.generic,
                "{name} is generic over a state parameter"
            );
            assert!(
                !definition.fields.is_empty(),
                "{name} has no field to keep private"
            );
            for field in &definition.fields {
                assert!(field.private, "{name}.{:?} is not private", field.name);
            }
            for namesake in other {
                assert!(
                    LANE_PRIVATE_NAMESAKES.contains(&(name, namesake.file.as_str())),
                    "{name} has a second definition at {}",
                    namesake.file
                );
            }
        }
    }

    /// Every item allowed to hand back an owned stage-output type, by
    /// (file, `Self` type, name), and why.
    const STAGE_CONSTRUCTORS: [(&str, Option<&str>, &str); 6] = [
        // S3: the checker.
        (
            "qsl-semantics/src/check/mod.rs",
            Some("PackageDeclarations"),
            "check",
        ),
        // S4: the link step.
        ("qsl-package/src/checked.rs", Some("CheckedPackage"), "link"),
        // S4 wire: the v2 emitter's constructor.
        ("qsl-package/src/checked.rs", Some("EmittedPackage"), "new"),
        // I2: the §4 verified binding.
        ("qsl-semantics/src/library/mod.rs", None, "verify_binding"),
        // I2: `VerifiedPackage` -> `ImportView`.
        (
            "qsl-semantics/src/library/mod.rs",
            Some("VerifiedPackage"),
            "into_import_view",
        ),
        // Lane-private: the native-v1 emitter's own `EmittedPackage`.
        ("src/protocol_artifact/native/mod.rs", None, "emit"),
    ];

    fn stage_returners(items: &Items) -> BTreeSet<(String, Option<String>, String)> {
        items
            .functions
            .iter()
            .filter(|function| {
                STAGE_TYPES
                    .iter()
                    .any(|(name, _)| function.owned_return.contains(*name))
            })
            .map(|function| {
                (
                    function.file.clone(),
                    function.self_type.clone(),
                    function.name.clone(),
                )
            })
            .collect()
    }

    /// FR-087-AC-1's constructor half and FR-087-AC-2 (TC-244 step 0): in
    /// the QSL crates, the only items returning an owned `CheckedGraph`,
    /// `CheckedPackage`, `EmittedPackage`, `VerifiedPackage` or
    /// `ImportView` (the lane-private `checking::CheckedPackage` included)
    /// are the named stage constructors; a `From` impl, decoder or other
    /// conversion into one fails here. Out-parameters are not seen.
    #[trace("TC-244", "FR-087-AC-2", "TC-243", "FR-087-AC-1")]
    #[test]
    fn tc_244_only_the_named_stage_constructors_return_stage_types() {
        let mut expected: BTreeSet<(String, Option<String>, String)> = STAGE_CONSTRUCTORS
            .iter()
            .map(|(file, self_type, name)| {
                (
                    (*file).to_owned(),
                    self_type.map(str::to_owned),
                    (*name).to_owned(),
                )
            })
            .collect();
        // The lane-private checker returns its own `checking::CheckedPackage`.
        expected.insert(("src/checking.rs".to_owned(), None, "check".to_owned()));
        assert_eq!(stage_returners(&qsl_items()), expected);
    }

    /// TC-244 step 0's scan sees through an alias, a nested item and a
    /// trait default method; and an out-parameter is its stated blind spot.
    #[trace("TC-244", "FR-087-AC-2")]
    #[test]
    fn tc_244_scan_sees_aliased_and_nested_returners() {
        let items = scan_tree(&[(
            "src/lib.rs",
            "type Forged = CheckedPackage;\n\
             pub fn by_alias() -> Forged { todo!() }\n\
             pub fn outer() { fn nested() -> Option<ImportView> { None } }\n\
             pub trait Mint { fn made(&self) -> VerifiedPackage { todo!() } }\n\
             pub fn out_param(out: &mut Option<CheckedGraph>) { let _ = out; }\n",
        )]);
        let names: BTreeSet<String> = stage_returners(&items)
            .into_iter()
            .map(|(_, _, name)| name)
            .collect();
        assert_eq!(
            names,
            ["by_alias", "made", "nested"].map(str::to_owned).into(),
            "out_param is the documented blind spot"
        );
    }

    /// FR-087-AC-8 and AC-10 (TC-247 steps 1-3, 6 and 7; step 4 by field
    /// name only): the two `EmittedPackage`s are at the two named paths
    /// with no field name in common; no root-crate file imports a
    /// canonical and a lane-private namesake together; and
    /// `src/package/features.rs` and `src/package/view.rs` still import
    /// `crate::checking::CheckedPackage`.
    #[trace("TC-247", "FR-087-AC-8", "FR-087-AC-10")]
    #[test]
    fn tc_247_namesakes_stay_distinct_and_never_share_a_scope() {
        let items = qsl_items();
        let emitted: Vec<&Struct> = items.structs_named("EmittedPackage").collect();
        let mut files: Vec<&str> = emitted.iter().map(|item| item.file.as_str()).collect();
        files.sort_unstable();
        assert_eq!(
            files,
            [
                "qsl-package/src/checked.rs",
                "src/protocol_artifact/native/mod.rs"
            ]
        );
        let field_names = |item: &Struct| -> BTreeSet<Option<String>> {
            item.fields.iter().map(|field| field.name.clone()).collect()
        };
        assert!(
            field_names(emitted[0]).is_disjoint(&field_names(emitted[1])),
            "the two EmittedPackages share a field"
        );

        let root_uses = || {
            items
                .uses
                .iter()
                .filter(|item| item.file.starts_with("src/"))
        };
        let imports_canonical = |item: &Use| {
            item.names.contains("qsl_package")
                && (item.names.contains("*")
                    || item.names.contains("EmittedPackage")
                    || item.names.contains("CheckedPackage"))
        };
        for item in root_uses().filter(|item| imports_canonical(item)) {
            let same_file_lane_import = root_uses().any(|other| {
                other.file == item.file
                    && (other.names.contains("checking") || other.names.contains("native"))
                    && (other.names.contains("CheckedPackage")
                        || other.names.contains("EmittedPackage")
                        || other.names.contains("*"))
            });
            assert!(
                !same_file_lane_import,
                "{} imports a canonical and a lane-private namesake",
                item.file
            );
        }
        for file in ["src/package/features.rs", "src/package/view.rs"] {
            assert!(
                root_uses().any(|item| item.file == file
                    && item.names.contains("checking")
                    && item.names.contains("CheckedPackage")),
                "{file} no longer imports crate::checking::CheckedPackage"
            );
        }
    }

    /// FR-060 T12-B's debt list: the one shipped mint outside `check`.
    const T12B_DEBT: (&str, &str) = ("qsl-eval/src/value/expression/family.rs", "decode_v2");

    /// The wire-admitted types a `NodeKey` mint must never be fed by.
    const WIRE_TYPES: [&str; 4] = [
        "WireNodeId",
        "PackageNodeKey",
        "ImportView",
        "VerifiedPackage",
    ];

    /// FR-087-AC-6 (TC-255 steps 1, 2 and 5, step 3 for direct feeding, and
    /// step 4's role): across the QSL crates, `library`, `qsl-package` (E4)
    /// and `qsl-replay` (E9) mint no `NodeKey`; every mint is in `check` or
    /// is T12-B's debt entry; and no minting item names a `WireNodeId`,
    /// `PackageNodeKey`, `ImportView` or `VerifiedPackage`. A value that
    /// reaches a mint through a call to another function is not traced.
    /// `arch-lint api-surface` (T12-B) is not part of `make ci`; this test
    /// is.
    #[trace("TC-255", "FR-087-AC-6")]
    #[test]
    fn tc_255_no_node_key_is_minted_outside_check_or_from_a_wire_id() {
        let items = qsl_items();
        let mints: Vec<&Function> = items
            .functions
            .iter()
            .filter(|function| function.mints_node_key)
            .collect();
        assert!(
            mints
                .iter()
                .any(|function| function.file.starts_with("qsl-semantics/src/check/")),
            "the scan sees check's own mints"
        );
        for function in mints {
            let site = format!(
                "{}: {:?}::{}",
                function.file, function.self_type, function.name
            );
            for forbidden in [
                "qsl-semantics/src/library/",
                "qsl-package/src/",
                "qsl-replay/src/",
            ] {
                assert!(!function.file.starts_with(forbidden), "mint at {site}");
            }
            assert!(
                function.file.starts_with("qsl-semantics/src/check/")
                    || (function.file.as_str(), function.name.as_str()) == T12B_DEBT,
                "mint outside check and T12-B's debt list at {site}"
            );
            for wire in WIRE_TYPES {
                assert!(
                    !function.names.contains(wire),
                    "mint fed by {wire} at {site}"
                );
            }
        }
    }

    /// TC-255's scan sees a mint through a `use … as` rename, a `type`
    /// alias, a qualified self type, `Self::` in `impl NodeKey`, a trait
    /// default method, a `static`, a `const`, an associated `const`, a
    /// macro and a `node_key_of` helper; and not a `WireNodeId`'s own
    /// `from_digest`.
    #[trace("TC-255", "FR-087-AC-6")]
    #[test]
    fn tc_255_scan_sees_every_mint_spelling() {
        let items = scan_tree(&[(
            "src/lib.rs",
            "use quire_exact::NodeKey as Nk;\n\
             type Key = quire_exact::NodeKey;\n\
             pub fn by_rename() { Nk::from_digest([0; 32]); }\n\
             pub fn by_type_alias() { Key::from_digest([0; 32]); }\n\
             pub fn by_qself() { <quire_exact::NodeKey>::from_digest([0; 32]); }\n\
             impl NodeKey { fn by_self() -> Self { Self::from_digest([0; 32]) } }\n\
             pub trait T { fn by_default(&self) { NodeKey::from_digest([0; 32]); } }\n\
             static BY_STATIC: fn([u8; 32]) -> Nk = Nk::from_digest;\n\
             const BY_CONST: fn([u8; 32]) -> Key = Key::from_digest;\n\
             impl Holder { const BY_ASSOC: fn([u8; 32]) -> Nk = Nk::from_digest; }\n\
             pub fn by_macro() { let _ = vec![Nk::from_digest([0; 32])]; }\n\
             pub fn by_macro_qself() { let _ = vec![<Key>::from_digest([0; 32])]; }\n\
             pub fn by_helper() { node_key_of(1); }\n\
             pub fn not_a_mint() { WireNodeId::from_digest([0; 32]); }\n",
        )]);
        assert_eq!(
            minting(&items),
            [
                "BY_ASSOC",
                "BY_CONST",
                "BY_STATIC",
                "by_default",
                "by_helper",
                "by_macro",
                "by_macro_qself",
                "by_qself",
                "by_rename",
                "by_self",
                "by_type_alias",
            ]
            .map(str::to_owned)
            .into()
        );
    }

    /// The scan skips exactly the files a `#[cfg(test)] mod` declares,
    /// with or without `#[path]`, and the files below them; a file whose
    /// name only looks like a test module is scanned.
    #[test]
    fn scan_skips_test_modules_by_declaration_not_by_file_name() {
        let items = scan_tree(&[
            (
                "src/lib.rs",
                "mod forge_tests;\n\
                 mod parent;\n\
                 #[cfg(test)] mod tests;\n\
                 #[cfg(test)] #[path = \"support/fixture.rs\"] mod fixture;\n",
            ),
            (
                "src/forge_tests.rs",
                "pub fn shipped() { NodeKey::from_digest([0; 32]); }",
            ),
            (
                "src/tests.rs",
                "pub fn test_only() { NodeKey::from_digest([0; 32]); }",
            ),
            (
                "src/tests/deeper.rs",
                "pub fn below_test() { NodeKey::from_digest([0; 32]); }",
            ),
            (
                "src/support/fixture.rs",
                "pub fn by_path() { NodeKey::from_digest([0; 32]); }",
            ),
            (
                "src/parent.rs",
                "#[cfg(test)] mod child_tests;\npub fn parent() {}",
            ),
            (
                "src/parent/child_tests.rs",
                "pub fn child() { NodeKey::from_digest([0; 32]); }",
            ),
        ]);
        assert_eq!(minting(&items), ["shipped".to_owned()].into());
    }

    /// FR-087-AC-11 (TC-281 steps 1, 4 and 5): `value::library` and
    /// `value::package_identity` are gone; no `ExportIdentity` exists in the
    /// QSL crates; `library` defines no `resolve_name` and no field or
    /// variant holding a `NodeKey`; and the relocated surface is defined in
    /// `library`.
    #[trace("TC-281", "FR-087-AC-11")]
    #[test]
    fn tc_281_library_relocated_with_wire_ids_only() {
        let root = workspace_root();
        for gone in [
            "src/value/library.rs",
            "src/value/package_identity.rs",
            "qsl-semantics/src/value/library.rs",
            "qsl-semantics/src/value/package_identity.rs",
        ] {
            assert!(!root.join(gone).exists(), "{gone} still exists");
        }
        let value_mods =
            crate::definition_scan::mod_declarations(&root, "qsl-semantics/src/value/mod.rs")
                .expect("value/mod.rs parses");
        for gone in ["library", "package_identity"] {
            assert!(
                !value_mods.iter().any(|name| name == gone),
                "value declares mod {gone}"
            );
        }

        let items = qsl_items();
        assert_eq!(items.structs_named("ExportIdentity").count(), 0);
        assert!(!items.enums.iter().any(|item| item.name == "ExportIdentity"));

        let in_library = |file: &str| file.starts_with("qsl-semantics/src/library/");
        assert!(
            !items
                .functions
                .iter()
                .any(|function| in_library(&function.file) && function.name == "resolve_name"),
            "library defines resolve_name"
        );
        for item in items.structs.iter().filter(|item| in_library(&item.file)) {
            for field in &item.fields {
                assert!(
                    !field.type_names.contains("NodeKey"),
                    "{}.{:?} holds a NodeKey",
                    item.name,
                    field.name
                );
            }
        }
        for item in items.enums.iter().filter(|item| in_library(&item.file)) {
            for (variant, fields) in &item.variants {
                assert!(
                    !fields
                        .iter()
                        .any(|field| field.type_names.contains("NodeKey")),
                    "{}::{variant} holds a NodeKey",
                    item.name
                );
            }
        }
        for name in [
            "LibraryName",
            "LibraryPackage",
            "PackageId",
            "ImportDeclaration",
            "LibraryLock",
            "PackageNodeKey",
        ] {
            let files: Vec<&str> = items
                .structs_named(name)
                .map(|item| item.file.as_str())
                .collect();
            assert_eq!(files, ["qsl-semantics/src/library/mod.rs"], "{name}");
        }
        let resolvers: Vec<&str> = items
            .functions
            .iter()
            .filter(|function| function.name == "resolve_libraries")
            .map(|function| function.file.as_str())
            .collect();
        assert_eq!(resolvers, ["qsl-semantics/src/library/mod.rs"]);
    }

    /// FR-088-AC-6 (TC-258, partial: steps 1 and 3 over the layer crates'
    /// struct and variant fields and function return types; step 4 is
    /// `value::expression::family`'s own test): no struct or variant has a
    /// `QualifiedName` as its only field, and no map field or map-returning
    /// function outside `check` is keyed by one, aliases seen through. Not
    /// seen: a name beside a filler field, a hand-written `PartialEq` or
    /// `Hash`, a local map, and the root crate's lane-private
    /// `QualifiedName` types.
    #[trace("TC-258", "FR-088-AC-6")]
    #[test]
    fn tc_258_a_qualified_name_is_never_an_identity_on_its_own() {
        let items = scan(&workspace_root(), &LAYER_CRATES).expect("scan runs");
        let embedding = items
            .structs
            .iter()
            .flat_map(|item| item.fields.iter())
            .filter(|field| field.type_names.contains("QualifiedName"))
            .count();
        assert!(
            embedding > 0,
            "the scan sees the fields that embed a QualifiedName"
        );
        let sole =
            |fields: &[Field]| fields.len() == 1 && fields[0].type_names.contains("QualifiedName");
        for item in &items.structs {
            assert!(
                !sole(&item.fields),
                "{} ({}) is a bare QualifiedName",
                item.name,
                item.file
            );
        }
        for item in &items.enums {
            for (variant, fields) in &item.variants {
                assert!(
                    !sole(fields),
                    "{}::{variant} ({}) is a bare QualifiedName",
                    item.name,
                    item.file
                );
            }
        }
        let outside_check = |file: &str| !file.starts_with("qsl-semantics/src/check/");
        for item in items
            .structs
            .iter()
            .filter(|item| outside_check(&item.file))
        {
            for field in &item.fields {
                assert!(
                    !field.map_keys.contains("QualifiedName"),
                    "{}.{:?} is a map keyed by QualifiedName",
                    item.name,
                    field.name
                );
            }
        }
        for function in items
            .functions
            .iter()
            .filter(|item| outside_check(&item.file))
        {
            assert!(
                !function.return_map_keys.contains("QualifiedName"),
                "{}::{} returns a map keyed by QualifiedName",
                function.file,
                function.name
            );
        }
    }

    /// TC-258's scan sees a map keyed by `QualifiedName` through a `type`
    /// alias of the map and through a `use … as` rename of the name.
    #[trace("TC-258", "FR-088-AC-6")]
    #[test]
    fn tc_258_scan_sees_aliased_maps() {
        let items = scan_tree(&[(
            "src/lib.rs",
            "use crate::QualifiedName as Name;\n\
             type ByName = std::collections::BTreeMap<QualifiedName, u8>;\n\
             pub struct A { by_name: ByName, n: u8 }\n\
             pub struct B { by_alias: BTreeMap<Name, u8>, n: u8 }\n\
             pub fn index() -> ByName { todo!() }\n",
        )]);
        let keyed: BTreeSet<&str> = items
            .structs
            .iter()
            .filter(|item| {
                item.fields
                    .iter()
                    .any(|field| field.map_keys.contains("QualifiedName"))
            })
            .map(|item| item.name.as_str())
            .collect();
        assert_eq!(keyed, ["A", "B"].into());
        assert!(items
            .functions
            .iter()
            .any(|function| function.name == "index"
                && function.return_map_keys.contains("QualifiedName")));
    }

    /// FR-088-AC-8 (TC-260, partial: steps 1 and 2, and step 3 for fields
    /// whose name says they hold a type): `ValueTypeRef` is defined once in
    /// the QSL crates, as exactly `{Native(NativeValueType),
    /// Package(DeclarationKey)}`; in `model`, every field whose name
    /// contains `type` names no `NodeKey` or string, and each of the three
    /// value-type records carries a `ValueTypeRef`.
    #[trace("TC-260", "FR-088-AC-8")]
    #[test]
    fn tc_260_value_type_ref_is_the_two_member_union() {
        let items = qsl_items();
        let definitions: Vec<&Enum> = items
            .enums
            .iter()
            .filter(|item| item.name == "ValueTypeRef")
            .collect();
        assert_eq!(definitions.len(), 1, "{definitions:?}");
        let variants: Vec<(&str, Vec<&BTreeSet<String>>)> = definitions[0]
            .variants
            .iter()
            .map(|(name, fields)| {
                (
                    name.as_str(),
                    fields.iter().map(|field| &field.type_names).collect(),
                )
            })
            .collect();
        let native: BTreeSet<String> = ["NativeValueType".to_owned()].into();
        let package: BTreeSet<String> = ["DeclarationKey".to_owned()].into();
        assert_eq!(
            variants,
            [("Native", vec![&native]), ("Package", vec![&package])]
        );

        let model_structs = || {
            items
                .structs
                .iter()
                .filter(|item| item.file.starts_with("qsl-semantics/src/model/"))
        };
        for item in model_structs() {
            for field in &item.fields {
                let Some(name) = field.name.as_deref() else {
                    continue;
                };
                if !name.contains("type") {
                    continue;
                }
                for bare in ["NodeKey", "String", "str"] {
                    assert!(
                        !field.type_names.contains(bare),
                        "{}.{name} is a bare {bare}",
                        item.name
                    );
                }
            }
        }
        for record in [
            "FieldMemberRecord",
            "OperationParameterRecord",
            "OperationResult",
        ] {
            let holds = model_structs()
                .filter(|item| item.name == record)
                .any(|item| {
                    item.fields.iter().any(|field| {
                        field.name.as_deref() == Some("value_type")
                            && field.type_names.contains("ValueTypeRef")
                    })
                });
            assert!(holds, "{record}.value_type is not a ValueTypeRef");
        }
    }
}
