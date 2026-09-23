// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-254 steps 3-5 (FR-087-AC-4, ADR-013 O-04/R-06): a signature scan of
//! every file under `src/library/`, the module that builds `ImportView`.
//!
//! It fails on:
//!
//! - any `NodeKey` path anywhere in `library` code, since `library` never
//!   builds or names a checked node key;
//! - any function, method, trait method (required or default), foreign fn
//!   or annotated closure that takes a name and returns a node id: a type
//!   naming `WireNodeId`, `NodeKey`, `PackageNodeKey` or `ImportView`, or
//!   any `library` type that carries one, including `Self` inside such a
//!   type's `impl`, `Self::X` where an `impl` binds `type X` to one, a
//!   generic parameter bounded by one (`T: From<WireNodeId>`), and a
//!   `&mut` out-parameter of such a type;
//! - any name index anywhere in `library`, whether or not a name is taken
//!   to build it: a return type, a struct or enum field, or a type alias
//!   that is a `BTreeMap`, `HashMap` or `IndexMap` (or a reference to or an
//!   iterator over one) from a name to a node id;
//! - any method of `ImportView` that takes a name at all, whether in an
//!   inherent `impl`, a trait `impl`, or a trait implemented for
//!   `ImportView` (including a blanket `impl<T> Trait for T`), since an
//!   importing package's own checker reads the view's
//!   `(name, PackageNodeKey)` entries itself (step 5; `check::imports`).
//!
//! A parameter "takes a name" when its type names `str`, `String` or
//! `QualifiedName`, a `library` type alias or `use ... as` rename of one, a
//! `library` newtype wrapping one, or a generic parameter bounded by one
//! (`N: AsRef<str>`, `where N: Borrow<str>`, `impl Into<String>`) or by
//! `ToString`/`Display`. Function signatures written inside a macro's
//! tokens (a `macro_rules!` body or an invocation) are matched on the
//! token text, since `syn::visit` does not parse macro bodies.
//!
//! This is a `syn` signature scan, not a call graph or a type checker. It
//! does not see:
//!
//! - a node id returned in another representation: raw `[u8; 32]` digest
//!   bytes, a hex `String`, or a `usize` position into a list of ids;
//! - a name passed as bytes (`&[u8]`) rather than as a string type;
//! - a closure without type annotations, including one stored in a
//!   `fn(&str) -> Option<WireNodeId>` variable or constant;
//! - a local variable's type inside a function body (a `let` map from
//!   names to ids that never leaves the function).
//!
//! Every file must read and parse, and each rule is also run against a
//! synthetic violating source, so a scanner that silently matched nothing
//! would fail here too.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use ix_trace_rs::trace;
use syn::spanned::Spanned;
use syn::visit::Visit;

const NAME_TYPES: [&str; 3] = ["str", "String", "QualifiedName"];
/// Bounds that make a generic parameter a name without naming `str` or
/// `String` themselves (`AsRef<str>`, `Borrow<str>` and `Into<String>` do).
const NAME_BOUNDS: [&str; 2] = ["ToString", "Display"];
const KEY_TYPES: [&str; 4] = ["NodeKey", "WireNodeId", "PackageNodeKey", "ImportView"];
const VIEW: &str = "ImportView";
/// Maps, and iterators over one, whose first type argument is the key.
const MAP_TYPES: [&str; 7] = [
    "BTreeMap", "HashMap", "IndexMap", "Iter", "IterMut", "IntoIter", "Range",
];

fn library_files() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/library");
    let mut files = Vec::new();
    let mut pending = vec![root];
    while let Some(dir) = pending.pop() {
        let entries = std::fs::read_dir(&dir)
            .unwrap_or_else(|error| panic!("{}: failed to list: {error}", dir.display()));
        for entry in entries {
            let path = entry.expect("directory entry reads").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// Whether a syntax node names any of `idents` as an exact path segment.
struct Finder<'a> {
    idents: &'a BTreeSet<String>,
    found: bool,
}

impl<'ast> Visit<'ast> for Finder<'_> {
    fn visit_path_segment(&mut self, segment: &'ast syn::PathSegment) {
        if self.idents.contains(&segment.ident.to_string()) {
            self.found = true;
        }
        syn::visit::visit_path_segment(self, segment);
    }
}

fn type_names(ty: &syn::Type, idents: &BTreeSet<String>) -> bool {
    let mut finder = Finder {
        idents,
        found: false,
    };
    finder.visit_type(ty);
    finder.found
}

fn bound_names(bound: &syn::TypeParamBound, idents: &BTreeSet<String>) -> bool {
    let mut finder = Finder {
        idents,
        found: false,
    };
    finder.visit_type_param_bound(bound);
    finder.found
}

/// Whether `ty` is a name, possibly behind a reference, a pointer, a slice
/// or an array: `&str`, `Box<str>`, `Cow<'_, str>`, `Box<[String]>`.
fn wraps_a_name(ty: &syn::Type, names: &BTreeSet<String>) -> bool {
    const WRAPPERS: [&str; 4] = ["Box", "Rc", "Arc", "Cow"];
    match ty {
        syn::Type::Reference(reference) => wraps_a_name(&reference.elem, names),
        syn::Type::Slice(slice) => wraps_a_name(&slice.elem, names),
        syn::Type::Array(array) => wraps_a_name(&array.elem, names),
        syn::Type::Paren(paren) => wraps_a_name(&paren.elem, names),
        syn::Type::Group(group) => wraps_a_name(&group.elem, names),
        syn::Type::Path(path) => path.path.segments.last().is_some_and(|segment| {
            let ident = segment.ident.to_string();
            if names.contains(&ident) {
                return true;
            }
            let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
                return false;
            };
            WRAPPERS.contains(&ident.as_str())
                && arguments.args.iter().any(|argument| {
                    matches!(argument, syn::GenericArgument::Type(inner) if wraps_a_name(inner, names))
                })
        }),
        _ => false,
    }
}

fn set(idents: &[&str]) -> BTreeSet<String> {
    idents.iter().map(|ident| (*ident).to_owned()).collect()
}

/// One `library` type definition, as the vocabulary fixpoint reads it.
enum Definition {
    /// `type Ident = Type;`
    Alias(Box<syn::Type>),
    /// `use path::From as Ident;`
    Rename(String),
    /// A struct's or enum's field types; `newtype` for a one-field struct.
    Fields {
        types: Vec<syn::Type>,
        newtype: bool,
    },
}

/// The name types and node-id-carrying types of the scanned sources, and
/// the traits implemented for `ImportView`.
struct Vocabulary {
    names: BTreeSet<String>,
    keys: BTreeSet<String>,
    view_traits: BTreeSet<String>,
    /// Associated types some `impl` binds to a node-id-carrying type
    /// (`type Out = WireNodeId;`), so `Self::Out` carries one.
    associated_keys: BTreeSet<String>,
}

#[derive(Default)]
struct Definitions {
    items: Vec<(String, Definition)>,
    view_traits: BTreeSet<String>,
    associated: Vec<(String, syn::Type)>,
}

impl Definitions {
    fn use_tree(&mut self, tree: &syn::UseTree) {
        match tree {
            syn::UseTree::Path(path) => self.use_tree(&path.tree),
            syn::UseTree::Rename(rename) => self.items.push((
                rename.rename.to_string(),
                Definition::Rename(rename.ident.to_string()),
            )),
            syn::UseTree::Group(group) => group.items.iter().for_each(|tree| self.use_tree(tree)),
            syn::UseTree::Name(_) | syn::UseTree::Glob(_) => {}
        }
    }
}

impl<'ast> Visit<'ast> for Definitions {
    fn visit_impl_item_type(&mut self, node: &'ast syn::ImplItemType) {
        self.associated
            .push((node.ident.to_string(), node.ty.clone()));
        syn::visit::visit_impl_item_type(self, node);
    }

    fn visit_item_type(&mut self, node: &'ast syn::ItemType) {
        self.items
            .push((node.ident.to_string(), Definition::Alias(node.ty.clone())));
        syn::visit::visit_item_type(self, node);
    }

    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        let types: Vec<syn::Type> = node.fields.iter().map(|field| field.ty.clone()).collect();
        let newtype = types.len() == 1;
        self.items.push((
            node.ident.to_string(),
            Definition::Fields { types, newtype },
        ));
        syn::visit::visit_item_struct(self, node);
    }

    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        let types = node
            .variants
            .iter()
            .flat_map(|variant| variant.fields.iter().map(|field| field.ty.clone()))
            .collect();
        self.items.push((
            node.ident.to_string(),
            Definition::Fields {
                types,
                newtype: false,
            },
        ));
        syn::visit::visit_item_enum(self, node);
    }

    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        self.use_tree(&node.tree);
        syn::visit::visit_item_use(self, node);
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        if let Some((_, path, _)) = &node.trait_ {
            let generics: BTreeSet<String> = node
                .generics
                .type_params()
                .map(|param| param.ident.to_string())
                .collect();
            let blanket = matches!(node.self_ty.as_ref(), syn::Type::Path(ty)
                if ty.path.get_ident().is_some_and(|ident| generics.contains(&ident.to_string())));
            if blanket || type_names(&node.self_ty, &set(&[VIEW])) {
                if let Some(segment) = path.segments.last() {
                    self.view_traits.insert(segment.ident.to_string());
                }
            }
        }
        syn::visit::visit_item_impl(self, node);
    }
}

impl Vocabulary {
    fn of(files: &[syn::File]) -> Self {
        let mut definitions = Definitions::default();
        for file in files {
            definitions.visit_file(file);
        }
        let mut names = set(&NAME_TYPES);
        let mut keys = set(&KEY_TYPES);
        loop {
            let mut changed = false;
            for (ident, definition) in &definitions.items {
                let (is_name, is_key) = match definition {
                    Definition::Alias(ty) => (type_names(ty, &names), type_names(ty, &keys)),
                    Definition::Rename(from) => (names.contains(from), keys.contains(from)),
                    Definition::Fields { types, newtype } => {
                        let is_key = types.iter().any(|ty| type_names(ty, &keys));
                        // A newtype directly over a name is a name; one
                        // over a container of names (a map, a request) is
                        // not, and neither is one that carries a node id.
                        let is_name =
                            *newtype && !is_key && types.iter().any(|ty| wraps_a_name(ty, &names));
                        (is_name, is_key)
                    }
                };
                if is_name {
                    changed |= names.insert(ident.clone());
                }
                if is_key {
                    changed |= keys.insert(ident.clone());
                }
            }
            if !changed {
                break;
            }
        }
        let associated_keys = definitions
            .associated
            .iter()
            .filter(|(_, ty)| type_names(ty, &keys))
            .map(|(ident, _)| ident.clone())
            .collect();
        Self {
            names,
            keys,
            view_traits: definitions.view_traits,
            associated_keys,
        }
    }

    /// The generic parameters of `generics` that are names
    /// (`N: AsRef<str>`), and those that are node ids (`T: From<WireNodeId>`).
    fn generics(&self, generics: &syn::Generics) -> Generics {
        let mut naming = self.names.clone();
        naming.extend(set(&NAME_BOUNDS));
        Generics {
            names: bounded_by(generics, &naming),
            keys: bounded_by(generics, &self.keys),
        }
    }
}

/// The generic parameters in scope that stand for a name or a node id.
#[derive(Default)]
struct Generics {
    names: BTreeSet<String>,
    keys: BTreeSet<String>,
}

/// The type parameters of `generics` with a bound, inline or in the `where`
/// clause, that names any of `idents`.
fn bounded_by(generics: &syn::Generics, idents: &BTreeSet<String>) -> BTreeSet<String> {
    let bounded = |bounds: &syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>| {
        bounds.iter().any(|bound| bound_names(bound, idents))
    };
    let inline = generics
        .type_params()
        .filter(|param| bounded(&param.bounds))
        .map(|param| param.ident.to_string());
    let clauses = generics
        .where_clause
        .iter()
        .flat_map(|clause| &clause.predicates)
        .filter_map(|predicate| match predicate {
            syn::WherePredicate::Type(predicate) if bounded(&predicate.bounds) => {
                match &predicate.bounded_ty {
                    syn::Type::Path(bounded_ty) => {
                        bounded_ty.path.get_ident().map(ToString::to_string)
                    }
                    _ => None,
                }
            }
            _ => None,
        });
    inline.chain(clauses).collect()
}

/// Whether `ty` is a map, or an iterator over one, from a name to a node
/// id: `BTreeMap<&str, PackageNodeKey>`, `&HashMap<String, WireNodeId>`,
/// `btree_map::Iter<'_, String, WireNodeId>`. That is a name index, which
/// only the checker builds.
fn is_name_index(ty: &syn::Type, names: &BTreeSet<String>, keys: &BTreeSet<String>) -> bool {
    struct Maps<'a> {
        names: &'a BTreeSet<String>,
        keys: &'a BTreeSet<String>,
        found: bool,
    }
    impl<'ast> Visit<'ast> for Maps<'_> {
        fn visit_path_segment(&mut self, segment: &'ast syn::PathSegment) {
            if MAP_TYPES.contains(&segment.ident.to_string().as_str()) {
                if let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments {
                    let types: Vec<&syn::Type> = arguments
                        .args
                        .iter()
                        .filter_map(|argument| match argument {
                            syn::GenericArgument::Type(ty) => Some(ty),
                            _ => None,
                        })
                        .collect();
                    if let [key, values @ ..] = types.as_slice() {
                        if type_names(key, self.names)
                            && values.iter().any(|value| type_names(value, self.keys))
                        {
                            self.found = true;
                        }
                    }
                }
            }
            syn::visit::visit_path_segment(self, segment);
        }
    }
    let mut maps = Maps {
        names,
        keys,
        found: false,
    };
    maps.visit_type(ty);
    maps.found
}

/// Whether `ty` names `Self::X` or `<Self as Trait>::X` for an associated
/// type `X` some `impl` binds to a node-id-carrying type.
fn names_associated_key(ty: &syn::Type, associated: &BTreeSet<String>) -> bool {
    struct Finder<'a> {
        associated: &'a BTreeSet<String>,
        found: bool,
    }
    impl<'ast> Visit<'ast> for Finder<'_> {
        fn visit_type_path(&mut self, node: &'ast syn::TypePath) {
            let segments: Vec<String> = node
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect();
            let through_self = match &node.qself {
                Some(qself) => {
                    matches!(qself.ty.as_ref(), syn::Type::Path(path) if path.path.is_ident("Self"))
                }
                None => segments.first().is_some_and(|first| first == "Self"),
            };
            if through_self
                && segments
                    .last()
                    .is_some_and(|last| self.associated.contains(last))
            {
                self.found = true;
            }
            syn::visit::visit_type_path(self, node);
        }
    }
    let mut finder = Finder {
        associated,
        found: false,
    };
    finder.visit_type(ty);
    finder.found
}

/// The `impl` or trait a signature sits in.
#[derive(Clone, Copy, Default)]
struct Context {
    /// `Self` carries a node id.
    self_is_key: bool,
    /// Its methods are `ImportView`'s methods.
    view: bool,
}

struct Scanner<'v> {
    vocabulary: &'v Vocabulary,
    generics: Vec<Generics>,
    context: Context,
    violations: Vec<String>,
}

impl Scanner<'_> {
    fn names(&self) -> BTreeSet<String> {
        let mut names = self.vocabulary.names.clone();
        names.extend(
            self.generics
                .iter()
                .flat_map(|generics| generics.names.iter().cloned()),
        );
        names
    }

    fn keys(&self) -> BTreeSet<String> {
        let mut keys = self.vocabulary.keys.clone();
        keys.extend(
            self.generics
                .iter()
                .flat_map(|generics| generics.keys.iter().cloned()),
        );
        if self.context.self_is_key {
            keys.insert("Self".to_owned());
        }
        keys
    }

    /// Whether `ty` carries a node id: a key type, or `Self::X` for an
    /// associated type bound to one.
    fn carries_a_key(&self, ty: &syn::Type, keys: &BTreeSet<String>) -> bool {
        type_names(ty, keys) || names_associated_key(ty, &self.vocabulary.associated_keys)
    }

    /// A type written in `library` (a return type, a field, an alias) that
    /// is a name index.
    fn check_type(&mut self, ty: &syn::Type, what: &str, line: usize) {
        if is_name_index(ty, &self.names(), &self.keys()) {
            self.violations
                .push(format!("{line}: {what} is a name -> node id index"));
        }
    }

    fn check(&mut self, sig: &syn::Signature) {
        self.generics.push(self.vocabulary.generics(&sig.generics));
        let (names, keys) = (self.names(), self.keys());
        let typed = || {
            sig.inputs.iter().filter_map(|argument| match argument {
                syn::FnArg::Typed(pat_type) => Some(pat_type.ty.as_ref()),
                syn::FnArg::Receiver(_) => None,
            })
        };
        let takes_a_name = typed().any(|ty| type_names(ty, &names));
        let out_parameter = typed().any(|ty| {
            matches!(ty, syn::Type::Reference(reference)
                if reference.mutability.is_some() && self.carries_a_key(&reference.elem, &keys))
        });
        let returns_a_key = match &sig.output {
            syn::ReturnType::Type(_, ty) => self.carries_a_key(ty, &keys),
            syn::ReturnType::Default => false,
        } || out_parameter;
        let name = &sig.ident;
        let line = name.span().start().line;
        if let syn::ReturnType::Type(_, ty) = &sig.output {
            self.check_type(ty, &format!("fn {name}'s return type"), line);
        }
        if takes_a_name && returns_a_key {
            self.violations
                .push(format!("{line}: fn {name} resolves a name to a node id"));
        }
        if self.context.view && takes_a_name {
            self.violations
                .push(format!("{line}: {VIEW}::{name} takes a name"));
        }
        self.generics.pop();
    }

    fn within(
        &mut self,
        generics: &syn::Generics,
        context: Context,
        visit: impl FnOnce(&mut Self),
    ) {
        self.generics.push(self.vocabulary.generics(generics));
        let outer = std::mem::replace(&mut self.context, context);
        visit(self);
        self.context = outer;
        self.generics.pop();
    }

    /// Function signatures inside a macro's tokens, matched on their text.
    fn macro_text(&mut self, text: &str, line: usize) {
        let text = blank_string_literals(text);
        let words = words(&text);
        if words.iter().any(|(_, word)| *word == "NodeKey") {
            self.violations
                .push(format!("{line}: names NodeKey inside a macro"));
        }
        let mut naming = self.names();
        naming.extend(set(&NAME_BOUNDS));
        let keys = self.keys();
        for &(at, word) in &words {
            if word != "fn" {
                continue;
            }
            let Some(signature) = macro_signature(&text[at..]) else {
                continue;
            };
            let takes_a_name = words_of(&signature.head)
                .chain(words_of(&signature.bounds))
                .any(|word| naming.contains(word));
            let returns_a_key = words_of(&signature.output).any(|word| keys.contains(word));
            if takes_a_name && returns_a_key {
                self.violations.push(format!(
                    "{line}: fn inside a macro resolves a name to a node id"
                ));
            }
            if self.context.view && takes_a_name {
                self.violations
                    .push(format!("{line}: {VIEW} method inside a macro takes a name"));
            }
        }
    }
}

impl<'ast> Visit<'ast> for Scanner<'_> {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let outer = std::mem::take(&mut self.context);
        self.check(&node.sig);
        syn::visit::visit_item_fn(self, node);
        self.context = outer;
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        let context = Context {
            self_is_key: type_names(&node.self_ty, &self.vocabulary.keys),
            view: type_names(&node.self_ty, &set(&[VIEW])),
        };
        self.within(&node.generics, context, |scanner| {
            syn::visit::visit_item_impl(scanner, node);
        });
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.check(&node.sig);
        syn::visit::visit_impl_item_fn(self, node);
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        let view = self
            .vocabulary
            .view_traits
            .contains(&node.ident.to_string());
        let context = Context {
            self_is_key: view,
            view,
        };
        self.within(&node.generics, context, |scanner| {
            syn::visit::visit_item_trait(scanner, node);
        });
    }

    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        self.check(&node.sig);
        syn::visit::visit_trait_item_fn(self, node);
    }

    fn visit_foreign_item_fn(&mut self, node: &'ast syn::ForeignItemFn) {
        self.check(&node.sig);
        syn::visit::visit_foreign_item_fn(self, node);
    }

    fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
        let names = self.names();
        let takes_a_name = node.inputs.iter().any(
            |input| matches!(input, syn::Pat::Type(pat_type) if type_names(&pat_type.ty, &names)),
        );
        let returns_a_key = match &node.output {
            syn::ReturnType::Type(_, ty) => type_names(ty, &self.keys()),
            syn::ReturnType::Default => false,
        };
        if takes_a_name && returns_a_key {
            let line = node.or1_token.span.start().line;
            self.violations
                .push(format!("{line}: closure resolves a name to a node id"));
        }
        syn::visit::visit_expr_closure(self, node);
    }

    fn visit_field(&mut self, node: &'ast syn::Field) {
        let line = node
            .ident
            .as_ref()
            .map_or(node.ty.span().start().line, |ident| {
                ident.span().start().line
            });
        self.check_type(&node.ty, "a field", line);
        syn::visit::visit_field(self, node);
    }

    fn visit_item_type(&mut self, node: &'ast syn::ItemType) {
        let line = node.ident.span().start().line;
        self.check_type(&node.ty, &format!("type {}", node.ident), line);
        syn::visit::visit_item_type(self, node);
    }

    fn visit_path_segment(&mut self, segment: &'ast syn::PathSegment) {
        if segment.ident == "NodeKey" {
            let line = segment.ident.span().start().line;
            self.violations.push(format!("{line}: names NodeKey"));
        }
        syn::visit::visit_path_segment(self, segment);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        let line = node
            .path
            .segments
            .last()
            .map_or(0, |segment| segment.ident.span().start().line);
        self.macro_text(&node.tokens.to_string(), line);
        syn::visit::visit_macro(self, node);
    }
}

/// `text` with the contents of every `"..."` literal blanked, so a word
/// inside a string is never read as code.
fn blank_string_literals(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_string = false;
    let mut escaped = false;
    for c in text.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
                out.push(c);
                continue;
            }
            out.push(' ');
        } else {
            if c == '"' {
                in_string = true;
            }
            out.push(c);
        }
    }
    out
}

/// Every identifier-shaped word of `text`, with its byte offset.
fn words(text: &str) -> Vec<(usize, &str)> {
    let mut found = Vec::new();
    let mut start = None;
    for (index, c) in text.char_indices().chain([(text.len(), ' ')]) {
        let ident_char = c.is_alphanumeric() || c == '_';
        match (start, ident_char) {
            (None, true) => start = Some(index),
            (Some(from), false) => {
                found.push((from, &text[from..index]));
                start = None;
            }
            _ => {}
        }
    }
    found
}

fn words_of(text: &str) -> impl Iterator<Item = &str> {
    words(text).into_iter().map(|(_, word)| word)
}

/// A function signature read from macro token text.
struct MacroSignature {
    /// From the name to the parameter list's closing parenthesis, generics
    /// included.
    head: String,
    /// The return type, without any `where` clause.
    output: String,
    /// The `where` clause, if any.
    bounds: String,
}

/// The signature starting at `text`'s leading `fn`, or `None` when no
/// parameter list follows.
fn macro_signature(text: &str) -> Option<MacroSignature> {
    let open = text.find('(')?;
    if text[..open].contains(['{', ';']) {
        return None;
    }
    let mut depth = 0_usize;
    let mut close = None;
    for (index, c) in text[open..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(open + index);
                    break;
                }
            }
            _ => {}
        }
    }
    let close = close?;
    let rest = &text[close + 1..];
    let end = rest.find(['{', ';']).unwrap_or(rest.len());
    let tail = &rest[..end];
    let (output, bounds) = match words(tail).into_iter().find(|(_, word)| *word == "where") {
        Some((at, _)) => (&tail[..at], &tail[at..]),
        None => (tail, ""),
    };
    let output = output.trim_start();
    Some(MacroSignature {
        head: text[2..=close].to_owned(),
        output: output.strip_prefix("->").unwrap_or("").to_owned(),
        bounds: bounds.to_owned(),
    })
}

fn violations_in_files(files: &[syn::File]) -> Vec<Vec<String>> {
    let vocabulary = Vocabulary::of(files);
    files
        .iter()
        .map(|file| {
            let mut scanner = Scanner {
                vocabulary: &vocabulary,
                generics: Vec::new(),
                context: Context::default(),
                violations: Vec::new(),
            };
            scanner.visit_file(file);
            scanner.violations
        })
        .collect()
}

fn violations_in(source: &str) -> Vec<String> {
    let parsed = syn::parse_file(source).expect("source parses as Rust");
    violations_in_files(&[parsed]).concat()
}

/// Steps 3-5 over the live tree. The vocabulary (aliases, newtypes and
/// id-carrying types) is read from every `library` file together, so a
/// type defined in one file and used in another is still seen.
#[trace("TC-254", "FR-087-AC-4")]
#[test]
fn library_never_resolves_a_name_to_a_node_id_nor_names_node_key() {
    let files = library_files();
    assert!(
        files.iter().any(|file| file.ends_with("library/mod.rs")),
        "the scan must reach src/library/mod.rs, which defines ImportView"
    );
    let parsed: Vec<syn::File> = files
        .iter()
        .map(|file| {
            let source = std::fs::read_to_string(file)
                .unwrap_or_else(|error| panic!("{}: failed to read: {error}", file.display()));
            syn::parse_file(&source)
                .unwrap_or_else(|error| panic!("{}: failed to parse: {error}", file.display()))
        })
        .collect();
    let vocabulary = Vocabulary::of(&parsed);
    assert!(
        vocabulary.keys.contains("ProjectedDeclarations"),
        "the vocabulary must see library's own id-carrying types"
    );
    let report: Vec<String> = files
        .iter()
        .zip(violations_in_files(&parsed))
        .flat_map(|(file, violations)| {
            violations
                .into_iter()
                .map(move |violation| format!("{}:{violation}", file.display()))
        })
        .collect();
    assert!(report.is_empty(), "library violates TC-254: {report:?}");
}

/// Each rule flags a synthetic violation, so the live-tree pass above is
/// not vacuous.
#[trace("TC-254", "FR-087-AC-4")]
#[test]
fn each_rule_flags_a_synthetic_violation() {
    let cases = [
        // A name-taking `ImportView` method that returns a key: both rules.
        (
            "impl ImportView { pub fn get(&self, name: &str) -> Option<PackageNodeKey> { None } }",
            2,
        ),
        (
            "impl ImportView { pub fn node(&self, name: &str) -> Option<WireNodeId> { None } }",
            2,
        ),
        // A name-taking `ImportView` method returning no key.
        (
            "impl ImportView { pub fn has(&self, name: &str) -> bool { false } }",
            1,
        ),
        (
            "fn lookup(view: &ImportView, name: String) -> Option<NodeKey> { None }",
            2,
        ),
        (
            "fn key(bytes: [u8; 32]) -> u8 { NodeKey::from_digest(bytes); 0 }",
            1,
        ),
        (
            "fn resolve(name: &QualifiedName) -> ImportView { todo!() }",
            1,
        ),
        // A free function mapping a name to a `WireNodeId`.
        (
            "fn node(names: &BTreeMap<String, WireNodeId>, name: &str) -> Option<WireNodeId> { None }",
            1,
        ),
    ];
    for (source, expected) in cases {
        assert_eq!(
            violations_in(source).len(),
            expected,
            "{source}: {:?}",
            violations_in(source)
        );
    }
}

/// The lookup this scan exists to keep out of `library`: a name-keyed
/// declaration map with a `node(&str)` accessor and a `select` that returns
/// the map itself. The map field and both methods are flagged, the second
/// method through `Self`.
#[trace("TC-254", "FR-087-AC-4")]
#[test]
fn the_scan_flags_a_name_keyed_declaration_map() {
    let violations = violations_in(
        "pub(crate) struct ProjectedDeclarations(BTreeMap<String, WireNodeId>);
         impl ProjectedDeclarations {
             pub(crate) fn node(&self, name: &str) -> Option<WireNodeId> { None }
             pub(crate) fn select<'a>(&self, exports: &'a [String]) -> Result<Self, &'a str> { todo!() }
         }",
    );
    assert_eq!(violations.len(), 3, "{violations:?}");
    // Node ids returned through a wrapper type are still node ids.
    assert_eq!(
        violations_in(
            "struct Decls { ids: Vec<WireNodeId> }
             fn select(exports: &[String]) -> Decls { todo!() }"
        )
        .len(),
        1
    );
}

/// A name index built by `library`, even one that takes no name to build,
/// an associated type bound to a node id, and a generic parameter that is
/// built from one.
#[trace("TC-254", "FR-087-AC-4")]
#[test]
fn the_scan_flags_name_indexes_associated_keys_and_key_generics() {
    let flagged = |source: &str| !violations_in(source).is_empty();

    // A name -> node id map returned by a method that takes no name.
    assert!(flagged(
        "impl ImportView { pub fn index(&self) -> BTreeMap<&str, PackageNodeKey> { todo!() } }"
    ));
    assert!(flagged(
        "fn index(view: &View) -> &HashMap<String, WireNodeId> { todo!() }"
    ));
    assert!(flagged(
        "fn index(view: &View) -> IndexMap<Box<str>, WireNodeId> { todo!() }"
    ));
    // An iterator over one.
    assert!(flagged(
        "fn index(view: &View) -> std::collections::btree_map::Iter<'_, String, WireNodeId> { todo!() }"
    ));
    // A field, in a struct or an enum variant, and a type alias.
    assert!(flagged(
        "struct Index { names: HashMap<String, WireNodeId> }"
    ));
    assert!(flagged(
        "enum Index { Names(BTreeMap<QualifiedName, PackageNodeKey>) }"
    ));
    assert!(flagged("type Index = BTreeMap<String, WireNodeId>;"));
    // Through a name alias and a key alias.
    assert!(flagged(
        "type Name = str; type Id = WireNodeId; struct Index(BTreeMap<Box<Name>, Id>);"
    ));

    // `Self::X` bound to a node id on a type that carries none.
    assert!(flagged(
        "trait Lookup { type Out; fn get(&self, name: &str) -> Option<Self::Out>; }
         struct Plain;
         impl Lookup for Plain { type Out = WireNodeId; fn get(&self, name: &str) -> Option<Self::Out> { None } }"
    ));
    assert!(flagged(
        "trait Lookup { type Out; fn get(&self, name: &str) -> Option<<Self as Lookup>::Out>; }
         struct Plain;
         impl Lookup for Plain { type Out = PackageNodeKey; }"
    ));

    // A generic built from a node id: inline and in a `where` clause.
    assert!(flagged(
        "fn get<T: From<WireNodeId>>(name: &str) -> Option<T> { None }"
    ));
    assert!(flagged(
        "fn get<T>(name: &str) -> Option<T> where T: TryFrom<PackageNodeKey> { None }"
    ));
}

/// The evasions a plain signature scan misses, each with its own positive.
#[trace("TC-254", "FR-087-AC-4")]
#[test]
fn the_scan_flags_each_evasion() {
    let flagged = |source: &str| !violations_in(source).is_empty();

    // Trait default method, and a trait implemented for `ImportView`.
    assert!(flagged(
        "trait Lookup { fn get(&self, name: &str) -> Option<PackageNodeKey> { None } }
         impl Lookup for ImportView {}"
    ));
    assert!(flagged(
        "trait Named { fn has(&self, name: &str) -> bool { false } }
         impl Named for ImportView {}"
    ));
    // A blanket impl reaches `ImportView` too.
    assert!(flagged(
        "trait Named { fn has(&self, name: &str) -> bool { false } }
         impl<T> Named for T {}"
    ));
    // A trait method returning `Self`, implemented for `ImportView`.
    assert!(flagged(
        "trait Pick { fn pick(&self, name: &str) -> Self; }
         impl Pick for ImportView { fn pick(&self, name: &str) -> Self { todo!() } }"
    ));
    // A required trait method with no implementor in view.
    assert!(flagged(
        "trait Lookup { fn get(&self, name: &str) -> Option<WireNodeId>; }"
    ));

    // `N: AsRef<str>` generics: inline, `where`, impl-level, `impl Trait`.
    assert!(flagged(
        "fn get<N: AsRef<str>>(map: &Decls, name: N) -> Option<WireNodeId> { None }"
    ));
    assert!(flagged(
        "fn get<N>(name: N) -> Option<WireNodeId> where N: Borrow<str> { None }"
    ));
    assert!(flagged(
        "struct Decls; impl<N: Into<String>> Lookup<N> for Decls { fn get(&self, name: N) -> Option<WireNodeId> { None } }"
    ));
    assert!(flagged(
        "fn get(name: impl AsRef<str>) -> Option<WireNodeId> { None }"
    ));
    assert!(flagged(
        "fn get<N: std::fmt::Display>(name: N) -> Option<WireNodeId> { None }"
    ));

    // `str` type aliases, `use ... as` renames and newtypes.
    assert!(flagged(
        "type Name = str; fn get(name: &Name) -> Option<WireNodeId> { None }"
    ));
    assert!(flagged(
        "type Name = str; type Label = Name; fn get(name: &Label) -> Option<WireNodeId> { None }"
    ));
    assert!(flagged(
        "use std::string::String as Text; fn get(name: Text) -> Option<WireNodeId> { None }"
    ));
    assert!(flagged(
        "struct Label(Box<str>); fn get(name: &Label) -> Option<WireNodeId> { None }"
    ));
    // Key aliases and renames too.
    assert!(flagged(
        "type Id = WireNodeId; fn get(name: &str) -> Option<Id> { None }"
    ));
    assert!(flagged(
        "use qsl_foundation::digest::WireNodeId as Id; fn get(name: &str) -> Option<Id> { None }"
    ));

    // Macros: a `macro_rules!` body, an item-position invocation, a
    // metavariable-named fn, and `NodeKey` inside a macro.
    assert!(flagged(
        "macro_rules! lookup { ($name:ident) => { fn $name(&self, name: &str) -> Option<WireNodeId> { None } }; }"
    ));
    assert!(flagged(
        "define! { fn get(name: &str) -> Option<PackageNodeKey> { None } }"
    ));
    assert!(flagged(
        "define! { fn get<N>(name: N) -> Option<WireNodeId> where N: AsRef<str> { None } }"
    ));
    assert!(flagged(
        "fn f() { let _ = vec![NodeKey::from_digest([0; 32])]; }"
    ));
    assert!(flagged(
        "impl ImportView { define! { fn has(&self, name: &str) -> bool { false } } }"
    ));

    // A `&mut` out-parameter and an annotated closure.
    assert!(flagged(
        "fn get(name: &str, out: &mut WireNodeId) -> bool { false }"
    ));
    assert!(flagged(
        "fn f() { let get = |name: &str| -> Option<WireNodeId> { None }; }"
    ));
}

/// What `library` does keep: name-free entries, a membership test that
/// returns a name, and names inside string literals.
#[trace("TC-254", "FR-087-AC-4")]
#[test]
fn the_scan_admits_name_free_entries_and_membership() {
    for source in [
        "impl ImportView { pub fn exports(&self) -> impl Iterator<Item = (&str, PackageNodeKey)> { todo!() } }",
        "struct Decls(BTreeMap<WireNodeId, String>);
         impl Decls {
             fn undeclared<'a>(&self, exports: &'a [String]) -> Option<&'a str> { None }
             fn entries(&self) -> impl Iterator<Item = (WireNodeId, &str)> { todo!() }
         }",
        "fn names(bytes: &[u8]) -> Result<Vec<String>, Defect> { todo!() }",
        // A newtype over a container of names is not itself a name.
        "struct Pins(BTreeMap<String, Selection>); fn bind(pins: &Pins) -> Option<WireNodeId> { None }",
        // A map keyed by node id, carrying names as data, is not an index.
        "struct Entries(BTreeMap<WireNodeId, String>); fn entries(view: &View) -> &BTreeMap<WireNodeId, String> { todo!() }",
        // An associated type bound to a name, not a node id.
        "trait Named { type Out; fn get(&self, id: WireNodeId) -> Option<Self::Out>; }
         impl Named for Plain { type Out = String; }",
        "fn f() { assert!(x, \"fn get(name: &str) -> WireNodeId NodeKey\"); }",
    ] {
        assert_eq!(violations_in(source), Vec::<String>::new(), "{source}");
    }
}
