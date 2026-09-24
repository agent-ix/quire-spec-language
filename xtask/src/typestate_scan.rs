// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-158 (ADR-013 §7 S-3): the whole-tree scans behind the typestate,
//! cross-package key and qualified-name criteria that are claims about where
//! an item is defined, what its fields are, and who calls what.
//!
//! [`scan`] parses every shipped `.rs` file under the named directories once
//! and records, per item, only what those criteria ask about: each struct's
//! fields (name, whether private, and every type name its type mentions),
//! each enum's variants, each function's return type and every path it
//! names, whether it mints a `NodeKey`, and each `use` item. `#[cfg(test)]`
//! items are excluded, as in [`crate::definition_scan`].
//!
//! **What this does not see.** Names only: a type reached through an alias
//! (`type K = NodeKey;`) is seen as `K`, and a path written inside a macro
//! body is seen only by the mint check, which also reads macro tokens.

use std::collections::BTreeSet;
use std::path::Path;

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

/// One free function or `impl` method.
#[derive(Clone, Debug)]
pub struct Function {
    /// The file it is defined in, relative to the workspace root.
    pub file: String,
    /// The `impl` block's `Self` type name, for a method.
    pub self_type: Option<String>,
    /// The function's name.
    pub name: String,
    /// Every type name its return type mentions outside a reference: the
    /// types it hands back owned.
    pub owned_return: BTreeSet<String>,
    /// Every type name the key type of a map in its return type mentions.
    pub return_map_keys: BTreeSet<String>,
    /// Every path segment name its signature and body mention.
    pub names: BTreeSet<String>,
    /// Whether it calls the kernel `NodeKey` constructor, or passes it as a
    /// value (`NodeKey::from_digest`), or calls a `node_key_of(` helper
    /// (FR-060 T12-B's two patterns), including inside a macro's tokens.
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
    /// Every shipped function and method.
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

fn fields(fields: &syn::Fields) -> Vec<Field> {
    fields
        .iter()
        .map(|field| {
            let mut collector = NameCollector::default();
            collector.visit_type(&field.ty);
            Field {
                name: field.ident.as_ref().map(ToString::to_string),
                private: matches!(field.vis, syn::Visibility::Inherited),
                type_names: collector.names,
                map_keys: collector.map_keys,
            }
        })
        .collect()
}

/// Whether a path is `NodeKey::from_digest` or ends in `node_key_of`.
fn is_mint_path(path: &syn::Path) -> bool {
    let segments: Vec<String> = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    segments
        .windows(2)
        .any(|pair| pair[0] == "NodeKey" && pair[1] == "from_digest")
        || segments.last().is_some_and(|last| last == "node_key_of")
}

#[derive(Default)]
struct BodyScan {
    names: BTreeSet<String>,
    mints: bool,
}

impl<'ast> Visit<'ast> for BodyScan {
    fn visit_path(&mut self, node: &'ast syn::Path) {
        if is_mint_path(node) {
            self.mints = true;
        }
        syn::visit::visit_path(self, node);
    }

    fn visit_path_segment(&mut self, node: &'ast syn::PathSegment) {
        self.names.insert(node.ident.to_string());
        syn::visit::visit_path_segment(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        let tokens = node.tokens.to_string().replace(' ', "");
        if tokens.contains("NodeKey::from_digest") || tokens.contains("node_key_of(") {
            self.mints = true;
        }
        syn::visit::visit_macro(self, node);
    }
}

struct Scanner<'a> {
    file: &'a str,
    items: &'a mut Items,
    self_type: Option<String>,
}

impl Scanner<'_> {
    fn function(&mut self, sig: &syn::Signature, block: &syn::Block) {
        let (owned_return, return_map_keys) = match &sig.output {
            syn::ReturnType::Default => (BTreeSet::new(), BTreeSet::new()),
            syn::ReturnType::Type(_, ty) => {
                let mut collector = NameCollector {
                    skip_references: true,
                    ..NameCollector::default()
                };
                collector.visit_type(ty);
                let mut names = collector.names;
                if names.remove("Self") {
                    names.extend(self.self_type.clone());
                }
                let mut keys = NameCollector::default();
                keys.visit_type(ty);
                (names, keys.map_keys)
            }
        };
        let mut body = BodyScan::default();
        body.visit_signature(sig);
        body.visit_block(block);
        self.items.functions.push(Function {
            file: self.file.to_owned(),
            self_type: self.self_type.clone(),
            name: sig.ident.to_string(),
            owned_return,
            return_map_keys,
            names: body.names,
            mints_node_key: body.mints,
        });
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

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        if !has_cfg_test(&node.attrs) {
            self.function(&node.sig, &node.block);
        }
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        if !has_cfg_test(&node.attrs) {
            let previous = self.self_type.take();
            self.function(&node.sig, &node.block);
            self.self_type = previous;
        }
    }

    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        self.items.structs.push(Struct {
            name: node.ident.to_string(),
            file: self.file.to_owned(),
            generic: node
                .generics
                .params
                .iter()
                .any(|param| !matches!(param, syn::GenericParam::Lifetime(_))),
            fields: fields(&node.fields),
        });
    }

    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        self.items.enums.push(Enum {
            name: node.ident.to_string(),
            file: self.file.to_owned(),
            variants: node
                .variants
                .iter()
                .map(|variant| (variant.ident.to_string(), fields(&variant.fields)))
                .collect(),
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

/// Whether `file` is a test module's own file: `tests.rs` or `*_tests.rs`.
/// Every such file in the scanned trees is declared under `#[cfg(test)]`
/// (`#[cfg(test)] mod tests;`), which a per-file parse cannot see.
fn is_test_module_file(file: &str) -> bool {
    Path::new(file)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| stem == "tests" || stem.ends_with("_tests"))
}

/// Parse every shipped `.rs` file under each of `dirs` (relative to
/// `workspace_root`; `tests/` directories and test module files excluded) and record its items.
pub fn scan(workspace_root: &Path, dirs: &[&str]) -> Result<Items> {
    let mut items = Items::default();
    for dir in dirs {
        for file in source_files(workspace_root, dir)? {
            if is_test_module_file(&file) {
                continue;
            }
            let parsed = parse_file(workspace_root, &file)?;
            Scanner {
                file: &file,
                items: &mut items,
                self_type: None,
            }
            .visit_file(&parsed);
        }
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

    fn layer_items() -> Items {
        scan(&workspace_root(), &LAYER_CRATES).expect("scan runs")
    }

    fn root_items() -> Items {
        scan(&workspace_root(), &["src"]).expect("scan runs")
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
    /// type has one defining location in the layer crates, in the module
    /// that owns it; every field is private, so the struct literal is no
    /// constructor outside that module and every outside read goes through
    /// an accessor (step 4); and no type is generic over a state parameter.
    /// In the root crate the only namesakes are the two lane-private ones.
    #[trace("TC-243", "FR-087-AC-1")]
    #[test]
    fn tc_243_stage_types_have_one_owner_and_private_fields() {
        let layers = layer_items();
        let root = root_items();
        for (name, owner) in STAGE_TYPES {
            let definitions: Vec<&Struct> = layers.structs_named(name).collect();
            assert_eq!(definitions.len(), 1, "{name}: {definitions:?}");
            let definition = definitions[0];
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
            for namesake in root.structs_named(name) {
                assert!(
                    LANE_PRIVATE_NAMESAKES.contains(&(name, namesake.file.as_str())),
                    "{name} has a second definition at {}",
                    namesake.file
                );
            }
        }
    }

    /// FR-087-AC-2 (TC-244 step 0): the S3 checker and the S4 link step are
    /// the only functions that hand back an owned `CheckedGraph` or
    /// `CheckedPackage`. A `From` impl, a decoder or any other conversion
    /// into either type returns one and fails here. In the root crate, only
    /// the lane-private checker returns its own `checking::CheckedPackage`.
    #[trace("TC-244", "FR-087-AC-2")]
    #[test]
    fn tc_244_only_the_checker_and_link_step_return_checked_typestate() {
        let returning = |items: &Items| -> Vec<(String, Option<String>, String)> {
            items
                .functions
                .iter()
                .filter(|function| {
                    function.owned_return.contains("CheckedGraph")
                        || function.owned_return.contains("CheckedPackage")
                })
                .map(|function| {
                    (
                        function.file.clone(),
                        function.self_type.clone(),
                        function.name.clone(),
                    )
                })
                .collect()
        };
        let expected = vec![
            (
                "qsl-package/src/checked.rs".to_owned(),
                Some("CheckedPackage".to_owned()),
                "link".to_owned(),
            ),
            (
                "qsl-semantics/src/check/mod.rs".to_owned(),
                Some("PackageDeclarations".to_owned()),
                "check".to_owned(),
            ),
        ];
        let mut found = returning(&layer_items());
        found.sort();
        assert_eq!(found, expected);
        for (file, self_type, name) in returning(&root_items()) {
            assert_eq!(
                file, "src/checking.rs",
                "{self_type:?}::{name} returns checked typestate outside the lane-private checker"
            );
        }
    }

    /// FR-087-AC-8 and AC-10 (TC-247): the two `EmittedPackage`s are two
    /// types at the two named paths with no field in common, and no use
    /// item puts both into one scope; the lane-private
    /// `checking::CheckedPackage` import in `src/package/features.rs` and
    /// `src/package/view.rs` stays, and no file imports both
    /// `CheckedPackage`s.
    #[trace("TC-247", "FR-087-AC-8", "FR-087-AC-10")]
    #[test]
    fn tc_247_namesakes_stay_distinct_and_never_share_a_scope() {
        let layers = layer_items();
        let root = root_items();
        let emitted: Vec<&Struct> = layers
            .structs_named("EmittedPackage")
            .chain(root.structs_named("EmittedPackage"))
            .collect();
        let files: Vec<&str> = emitted.iter().map(|item| item.file.as_str()).collect();
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

        let imports_canonical = |item: &Use| {
            item.names.contains("qsl_package")
                && (item.names.contains("*")
                    || item.names.contains("EmittedPackage")
                    || item.names.contains("CheckedPackage"))
        };
        for item in root.uses.iter().filter(|item| imports_canonical(item)) {
            let same_file_lane_import = root.uses.iter().any(|other| {
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
                root.uses.iter().any(|item| item.file == file
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

    /// FR-087-AC-6 (TC-255 steps 1-5): `library`, `package` (E4) and
    /// `replay` (E9) mint no `NodeKey`; every shipped mint is in `check`
    /// or on T12-B's debt list; and no minting function names a
    /// `WireNodeId`, `PackageNodeKey`, `ImportView` or `VerifiedPackage`,
    /// so none is fed by one. `arch-lint api-surface` (T12-B) states the
    /// same allow-list but is not part of `make ci`; this test is.
    #[trace("TC-255", "FR-087-AC-6")]
    #[test]
    fn tc_255_no_node_key_is_minted_outside_check_or_from_a_wire_id() {
        let mut dirs = LAYER_CRATES.to_vec();
        dirs.push("src");
        let items = scan(&workspace_root(), &dirs).expect("scan runs");
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

    /// FR-087-AC-11 (TC-281 steps 1, 4 and 5): `value::library` and
    /// `value::package_identity` are gone; no `ExportIdentity` exists
    /// anywhere; `library` defines no `resolve_name` and no field or
    /// variant of any type holding a `NodeKey`; and the relocated surface
    /// is defined in `library`.
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

        let mut dirs = LAYER_CRATES.to_vec();
        dirs.push("src");
        let items = scan(&root, &dirs).expect("scan runs");
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

    /// FR-088-AC-6 (TC-258 steps 1-3; step 4 is
    /// `value::expression::family`'s own test): in the layer crates, a
    /// `QualifiedName` is never the only field of a struct or variant (a
    /// newtype over a name would make the name the identity), and no map
    /// field or map-returning function outside `check` is keyed by one.
    #[trace("TC-258", "FR-088-AC-6")]
    #[test]
    fn tc_258_a_qualified_name_is_never_an_identity_on_its_own() {
        let items = layer_items();
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

    /// FR-088-AC-8 (TC-260 steps 1-3): `ValueTypeRef` is defined once, as
    /// exactly `{Native(NativeValueType), Package(DeclarationKey)}`, and no
    /// `model` field named `value_type` is a bare `NodeKey` or a string.
    #[trace("TC-260", "FR-088-AC-8")]
    #[test]
    fn tc_260_value_type_ref_is_the_two_member_union() {
        let mut dirs = LAYER_CRATES.to_vec();
        dirs.push("src");
        let items = scan(&workspace_root(), &dirs).expect("scan runs");
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

        let model_fields = items
            .structs
            .iter()
            .filter(|item| item.file.starts_with("qsl-semantics/src/model/"))
            .flat_map(|item| item.fields.iter().map(move |field| (item, field)))
            .filter(|(_, field)| field.name.as_deref() == Some("value_type"));
        let mut seen = 0_usize;
        for (item, field) in model_fields {
            seen += 1;
            for bare in ["NodeKey", "String", "str"] {
                assert!(
                    !field.type_names.contains(bare),
                    "{}.value_type is a bare {bare}",
                    item.name
                );
            }
        }
        assert!(seen > 0, "the scan sees model's value_type fields");
    }
}
