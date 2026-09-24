// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-013 §2 (ADR-013:113, QSL-194): one RFC 8785 implementation.
//!
//! "One RFC 8785 JCS implementation produces every RFC 8785 encoding: the
//! `quire-canonical` crate." QSL once spelled "serialize with `serde_json`,
//! then SHA-256" at eight identity sites, each its own canonical encoder,
//! and a helper in one file (`key.rs`'s `jcs_bytes`) was hashed in others
//! (`population.rs`, `intake.rs`). QSL-194 moved them all to
//! `quire-canonical`; this check refuses a new one.
//!
//! **Two function sets.** Over every shipped function of a crate -- free
//! functions and `impl` methods alike:
//!
//! - a *JSON producer* calls a [`SERIALIZERS`] entry, by path
//!   (`serde_json::to_vec`) or through a `use serde_json::...` import,
//!   renamed or not;
//! - a *hashing function* names a [`HASHERS`] entry.
//!
//! Each set is closed under calls: a function calling a member is a member,
//! to a fixpoint, so a helper wrapping a helper is followed however deep.
//!
//! **The rule.** A shipped source file fails unless it is on [`EXEMPT`] when
//! it both
//!
//! - *hashes*: names a [`HASHERS`] entry, or calls a hashing function
//!   defined in another file; and
//! - *reads as JSON*: names `serde_json` at all -- a serializer call, a
//!   grouped or renamed `use serde_json::{to_vec as tv, Value}`, a
//!   `serde_json::Value` whose `to_string()` it hashes -- or calls a JSON
//!   producer defined in another file.
//!
//! So a `json_bytes` helper hashed in another file fails, and so does its
//! mirror: a file that names `serde_json` and hands the bytes to a hash
//! helper living in a clean file. Shipped production code hashes through
//! `quire-canonical` (not a [`HASHERS`] entry: it is the one encoder) or,
//! for exact bytes, `ByteDigest::of` in a file that does neither.
//!
//! **Exemptions are pinned to functions.** An [`EXEMPT`] entry names the
//! functions its hash sites may sit in: each line naming a [`HASHERS`]
//! entry (outside a `use` item) and each call of another file's hashing
//! function. A hash site in any other function of an exempt file fails, and
//! a listed function with no hash site left fails as stale. An entry may
//! also pin its hash sites to named calls ([`Exemption::calls`]): then a
//! [`HASHERS`] line in the listed function fails, and so does any JSON the
//! function names or produces.
//!
//! **How.** The same token scan FR-060's T12-B/C/D use
//! ([`crate::api_surface`]): patterns match the file's `proc_macro2`
//! tokens, so a comment or string literal never matches, and `#[cfg(test)]`
//! items and out-of-line `#[cfg(test)] mod` files are excluded -- a test may
//! take an independent digest over `serde_json` output to check the encoder
//! against.
//!
//! **Calls are resolved by name, within one crate**, to line granularity:
//!
//! - `module::name(` and `super::name(` / `self::name(` reach free
//!   functions of a module whose last path segment matches -- the caller's
//!   child or sibling module, or a root module, when one of those defines
//!   `name`;
//! - `Type::name(` and `Self::name(` reach methods of `impl Type`;
//! - `.name(` reaches the crate's method named `name` when it has exactly
//!   one;
//! - a bare `name(` reaches a free function of the same file, or one the
//!   file imports by `use`, named, renamed or by glob.
//!
//! An exempt function's digest is sanctioned where it is: a call to it is
//! not a hash site, and hashing does not spread through it to its callers.
//!
//! **Stated limitations.** A function passed as a value, a helper in
//! another crate, a producer that makes JSON only through `Value`'s
//! `Display` (no [`SERIALIZERS`] call), a method call whose name two
//! methods of the crate share, a call into an exempt function, and a
//! macro-generated function are not followed. Name resolution
//! over-approximates: two same-named functions both match, and a line
//! holding two functions attributes its calls to the narrower one. A false
//! match is renamed or named in [`EXEMPT`].

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::{Path, PathBuf},
};

use syn::visit::Visit;

use crate::api_surface::{
    cfg_test_lines, cfg_test_module_declarations, flatten_tokens, module_path_of,
    pattern_match_lines, qsl_scan_src_roots, walk_rs_files, CallPattern, LocatedToken, Role, Token,
};
use crate::error::{Error, Result};

/// What names JSON: the `serde_json` crate, however it is imported.
pub(crate) const JSON: &[&str] = &["serde_json"];

/// The `serde_json` functions that produce JSON text or a JSON tree.
pub(crate) const SERIALIZERS: &[&str] = &[
    "to_vec",
    "to_vec_pretty",
    "to_string",
    "to_string_pretty",
    "to_writer",
    "to_writer_pretty",
    "to_value",
];

/// What hashes bytes in QSL: the SHA-256 hasher and its crate, and the two
/// raw-byte digest constructors that call it.
pub(crate) const HASHERS: &[&str] = &["Sha256", "sha2", "of_preimage(", "ByteDigest::of("];

/// Why a file on [`EXEMPT`] may pair JSON with a hash.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ExemptionKind {
    /// Not a canonical encoding at all: the digest is over exact emitted
    /// or supplied bytes, never over a canonical form of a value.
    NotAnIdentity,
    /// An identity the specification itself carves out of RFC 8785: ADR-013
    /// §2 (ADR-013:124-127) names FR-021's `NativePackageIdentity` a
    /// typed-record encoding, retiring with `NativePackage` at ADR-011 §7.3
    /// M-6c. It leaves this list in that change.
    SpecCarvedIdentity,
}

/// One exemption: the file (its crate's source root and module path), the
/// functions its hash sites may sit in (`name` for a free function,
/// `Type::name` for a method), optionally the only calls those hash sites
/// may be, and why.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Exemption {
    pub(crate) crate_src: &'static str,
    pub(crate) module: &'static str,
    pub(crate) functions: &'static [&'static str],
    /// When non-empty, the exemption is pinned to calls: a hash site in a
    /// listed function is covered only when it is a call of one of these
    /// hashing functions of another file, never a [`HASHERS`] line, and a
    /// listed function naming `serde_json` or calling a JSON producer
    /// fails. So a JSON producer cannot be hashed inside the function even
    /// through the sanctioned call.
    pub(crate) calls: &'static [&'static str],
    pub(crate) kind: ExemptionKind,
    pub(crate) reason: &'static str,
}

/// Every exemption. A file may carry more than one, of different kinds.
pub(crate) const EXEMPT: &[Exemption] = &[
    Exemption {
        crate_src: "src",
        module: "command",
        functions: &["with_request"],
        calls: &[],
        kind: ExemptionKind::NotAnIdentity,
        reason: "the FR-001 `ByteDigest` of the exact artifact bytes it read from disk, \
                 handed to the command's action; not an identity over a canonical form",
    },
    Exemption {
        crate_src: "src",
        module: "native_model",
        functions: &["NativeModel::new_with_profile"],
        calls: &[],
        kind: ExemptionKind::NotAnIdentity,
        reason: "the FR-001 `ByteDigest` of the native model artifact bytes it has just \
                 emitted; not an identity over a canonical form",
    },
    Exemption {
        crate_src: "src",
        module: "package",
        functions: &["NativePackage::new", "NativePackage::read_verified"],
        calls: &[],
        kind: ExemptionKind::NotAnIdentity,
        reason: "the FR-001 `ByteDigest` of the emitted package bytes, and of supplied \
                 package bytes checked against their expected reference; not an identity \
                 over a canonical form",
    },
    Exemption {
        crate_src: "src",
        module: "package",
        functions: &["NativePackageIdentity::of"],
        calls: &[],
        kind: ExemptionKind::SpecCarvedIdentity,
        reason: "FR-021's `NativePackageIdentity`: ADR-013 §2 (ADR-013:124-127) names it a \
                 typed-record encoding, not RFC 8785; retires with `NativePackage` at \
                 ADR-011 §7.3 M-6c",
    },
    Exemption {
        crate_src: "src",
        module: "protocol_artifact::encoding",
        functions: &["candidate"],
        calls: &[],
        kind: ExemptionKind::NotAnIdentity,
        reason: "the FR-001 `ByteDigest` of a transport candidate's exact bytes; not an \
                 identity over a canonical form",
    },
    Exemption {
        crate_src: "src",
        module: "runtime::reading",
        functions: &["read_selected"],
        calls: &[],
        kind: ExemptionKind::NotAnIdentity,
        reason: "the FR-001 `ByteDigest` of supplied artifact bytes, checked against the \
                 expected digest before decoding; not an identity over a canonical form",
    },
    Exemption {
        crate_src: "src",
        module: "runtime::construction",
        functions: &["Artifact::new"],
        calls: &[],
        kind: ExemptionKind::NotAnIdentity,
        reason: "the FR-001 `ByteDigest` of the artifact bytes it has just emitted: an \
                 exact-byte digest of those bytes, not an identity over a canonical form",
    },
    Exemption {
        crate_src: "qsl-semantics/src",
        module: "model::intake",
        functions: &["check_package_digest"],
        calls: &["raw_bytes_digest"],
        kind: ExemptionKind::NotAnIdentity,
        reason: "the FR-154 raw-byte digest of a package document that did not parse, \
                 reported against its declared digest, through `raw_bytes_digest` only; the \
                 parsed document's `sha256-jcs` digest is `quire-canonical`'s",
    },
    Exemption {
        crate_src: "src",
        module: "protocol_artifact::checked_handoff",
        functions: &["build"],
        calls: &[],
        kind: ExemptionKind::NotAnIdentity,
        reason: "the FR-051 `ByteDigest` of the checked-handoff document bytes it has just \
                 emitted; the document's content identity is `quire-canonical`'s (QSL-220)",
    },
    Exemption {
        crate_src: "src",
        module: "protocol_artifact::native_temporal::common",
        functions: &["raw_digest"],
        calls: &[],
        kind: ExemptionKind::NotAnIdentity,
        reason: "the FR-052 `ByteDigest` of native-temporal document bytes, emitted or \
                 supplied; the documents' content identities are `quire-canonical`'s (QSL-220)",
    },
];

/// One call from a file to a function defined in another file of its crate.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct CrossCall {
    pub(crate) line: usize,
    /// The called function, `name` or `Type::name`.
    pub(crate) function: String,
    pub(crate) defined_in: PathBuf,
}

/// One hash site: a line naming a [`HASHERS`] entry, or calling another
/// file's hashing function, and the function it sits in (empty outside any
/// function).
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct HashSite {
    pub(crate) line: usize,
    pub(crate) function: String,
}

/// One file that both hashes and reads as JSON.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EncoderSite {
    pub(crate) file: PathBuf,
    pub(crate) crate_src: String,
    pub(crate) module: String,
    /// Every line naming a [`HASHERS`] entry.
    pub(crate) hasher_lines: Vec<usize>,
    /// Every line naming `serde_json`.
    pub(crate) json_lines: Vec<usize>,
    /// Every call of a JSON producer defined in another file.
    pub(crate) json_calls: Vec<CrossCall>,
    /// Every call of a hashing function defined in another file.
    pub(crate) hash_calls: Vec<CrossCall>,
    /// Every hash site, for pinning exemptions.
    pub(crate) hash_sites: Vec<HashSite>,
}

/// The check's result over one tree.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Outcome {
    /// Files not on [`EXEMPT`]: each fails the check.
    pub(crate) violations: Vec<EncoderSite>,
    /// Exempt files with hash sites outside every listed function: each
    /// site fails the check.
    pub(crate) unlisted: Vec<(EncoderSite, Vec<HashSite>)>,
    /// Exempt files, with the exemptions their hash sites fall under.
    pub(crate) allowed: Vec<(EncoderSite, Vec<&'static Exemption>)>,
    /// Lines naming `serde_json` or calling a JSON producer inside a
    /// function of a call-pinned exemption (see [`Exemption::calls`]): each
    /// fails the check.
    pub(crate) json_in_pinned: Vec<(PathBuf, HashSite)>,
    /// Listed functions with no remaining hash site, by exemption; `None`
    /// when the file no longer pairs JSON with a hash at all. Each fails.
    pub(crate) stale: Vec<(&'static Exemption, Option<&'static str>)>,
}

impl Outcome {
    pub(crate) fn passed(&self) -> bool {
        self.violations.is_empty()
            && self.unlisted.is_empty()
            && self.json_in_pinned.is_empty()
            && self.stale.is_empty()
    }

    /// The allowed files under exemptions of `kind`.
    pub(crate) fn allowed_of(
        &self,
        kind: ExemptionKind,
    ) -> Vec<(&EncoderSite, &'static Exemption)> {
        self.allowed
            .iter()
            .flat_map(|(site, exemptions)| {
                exemptions
                    .iter()
                    .filter(move |exemption| exemption.kind == kind)
                    .map(move |exemption| (site, *exemption))
            })
            .collect()
    }
}

/// One shipped function: a free function (`self_ty` `None`) or an `impl`
/// or trait method, with its first and last line.
struct FnDef {
    name: String,
    self_ty: Option<String>,
    start: usize,
    end: usize,
}

impl FnDef {
    fn qualified(&self) -> String {
        match &self.self_ty {
            Some(self_ty) => format!("{self_ty}::{}", self.name),
            None => self.name.clone(),
        }
    }
}

/// A call as written, before resolution.
enum Call {
    /// `seg::name(`: a module, `Type`, `Self`, `super` or `self`.
    Path(String, String),
    /// `.name(`.
    Method(String),
    /// `name(`.
    Bare(String),
}

/// What one shipped file names, outside its `#[cfg(test)]` items.
struct ScannedFile {
    path: PathBuf,
    crate_src: String,
    module: String,
    hasher_lines: Vec<usize>,
    json_lines: Vec<usize>,
    serializer_lines: Vec<usize>,
    /// Lines inside `use` items.
    use_lines: BTreeSet<usize>,
    functions: Vec<FnDef>,
    calls: Vec<(usize, Call)>,
    /// `use` imports: each local name and the `(parent module segment,
    /// imported name)` it stands for.
    imports: BTreeMap<String, (String, String)>,
    /// Module segments glob-imported (`use a::b::*` gives `b`).
    glob_imports: BTreeSet<String>,
}

impl ScannedFile {
    /// The innermost function whose lines contain `line`.
    fn enclosing(&self, line: usize) -> Option<usize> {
        self.functions
            .iter()
            .enumerate()
            .filter(|(_, function)| function.start <= line && line <= function.end)
            .min_by_key(|(_, function)| function.end - function.start)
            .map(|(index, _)| index)
    }

    fn enclosing_name(&self, line: usize) -> String {
        self.enclosing(line)
            .map(|index| self.functions[index].qualified())
            .unwrap_or_default()
    }

    /// The module this file's `super` names.
    fn parent_module(&self) -> &str {
        self.module
            .rsplit_once("::")
            .map_or("", |(parent, _)| parent)
    }

    fn last_segment(&self) -> &str {
        self.module.rsplit("::").next().unwrap_or("")
    }
}

/// Whether a call written as `call` in `caller` can reach free or method
/// function `callee` of `defined`. Both files are of one crate.
fn reaches(call: &Call, caller: &ScannedFile, defined: &ScannedFile, callee: &FnDef) -> bool {
    let same_file = caller.path == defined.path;
    match (call, &callee.self_ty) {
        (Call::Method(name), Some(_)) => *name == callee.name,
        (Call::Path(seg, name), Some(self_ty)) => {
            *name == callee.name && (seg == self_ty || (seg == "Self" && same_file))
        }
        (Call::Path(seg, name), None) => {
            *name == callee.name
                && match seg.as_str() {
                    "super" => defined.module == caller.parent_module(),
                    "self" => defined.module == caller.module,
                    _ => defined.last_segment() == seg,
                }
        }
        (Call::Bare(local), None) => {
            if same_file {
                return *local == callee.name;
            }
            let imported = caller.imports.get(local).is_some_and(|(parent, original)| {
                *original == callee.name && imports_from(caller, parent, defined)
            });
            let globbed = *local == callee.name
                && caller
                    .glob_imports
                    .iter()
                    .any(|parent| imports_from(caller, parent, defined));
            imported || globbed
        }
        _ => false,
    }
}

/// Whether `use` parent segment `parent`, written in `caller`, names
/// `defined`'s module.
fn imports_from(caller: &ScannedFile, parent: &str, defined: &ScannedFile) -> bool {
    match parent {
        "super" => defined.module == caller.parent_module(),
        "self" => defined.module == caller.module,
        _ => defined.last_segment() == parent,
    }
}

/// Every shipped function (free, `impl` method or trait method with a
/// body) in `parsed` outside its `#[cfg(test)]` lines.
fn functions(parsed: &syn::File, excluded: &BTreeSet<usize>) -> Vec<FnDef> {
    struct Visitor<'a> {
        excluded: &'a BTreeSet<usize>,
        self_ty: Option<String>,
        functions: Vec<FnDef>,
    }
    impl Visitor<'_> {
        fn push(&mut self, ident: &syn::Ident, span: proc_macro2::Span, self_ty: Option<String>) {
            if !self.excluded.contains(&ident.span().start().line) {
                self.functions.push(FnDef {
                    name: ident.to_string(),
                    self_ty,
                    start: span.start().line,
                    end: span.end().line,
                });
            }
        }
    }
    impl<'ast> Visit<'ast> for Visitor<'_> {
        fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
            self.push(&node.sig.ident, syn::spanned::Spanned::span(node), None);
            let outer = self.self_ty.take();
            syn::visit::visit_item_fn(self, node);
            self.self_ty = outer;
        }
        fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
            let name = match &*node.self_ty {
                syn::Type::Path(path) => path.path.segments.last().map(|s| s.ident.to_string()),
                _ => None,
            };
            let outer = std::mem::replace(&mut self.self_ty, name.or(Some("<impl>".to_owned())));
            syn::visit::visit_item_impl(self, node);
            self.self_ty = outer;
        }
        fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
            let outer = self.self_ty.replace(node.ident.to_string());
            syn::visit::visit_item_trait(self, node);
            self.self_ty = outer;
        }
        fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
            let self_ty = self.self_ty.clone();
            self.push(&node.sig.ident, syn::spanned::Spanned::span(node), self_ty);
            let outer = self.self_ty.take();
            syn::visit::visit_impl_item_fn(self, node);
            self.self_ty = outer;
        }
        fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
            if node.default.is_some() {
                let self_ty = self.self_ty.clone();
                self.push(&node.sig.ident, syn::spanned::Spanned::span(node), self_ty);
            }
            let outer = self.self_ty.take();
            syn::visit::visit_trait_item_fn(self, node);
            self.self_ty = outer;
        }
    }
    let mut visitor = Visitor {
        excluded,
        self_ty: None,
        functions: Vec::new(),
    };
    visitor.visit_file(parsed);
    visitor.functions
}

/// Every line inside a `use` item.
fn use_lines(parsed: &syn::File) -> BTreeSet<usize> {
    struct Visitor(BTreeSet<usize>);
    impl<'ast> Visit<'ast> for Visitor {
        fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
            let span = syn::spanned::Spanned::span(node);
            self.0.extend(span.start().line..=span.end().line);
        }
    }
    let mut visitor = Visitor(BTreeSet::new());
    visitor.visit_file(parsed);
    visitor.0
}

/// Every call written in `tokens` outside `excluded` lines.
fn calls(tokens: &[LocatedToken], excluded: &BTreeSet<usize>) -> Vec<(usize, Call)> {
    let ident = |index: usize| match tokens.get(index).map(|t| &t.token) {
        Some(Token::Ident(word)) => Some(word.as_str()),
        _ => None,
    };
    let punct =
        |index: usize, c: char| tokens.get(index).map(|t| &t.token) == Some(&Token::Punct(c));
    let mut found = Vec::new();
    for index in 0..tokens.len() {
        let Some(name) = ident(index) else {
            continue;
        };
        let line = tokens[index].line;
        if tokens.get(index + 1).map(|t| &t.token) != Some(&Token::OpenParen)
            || excluded.contains(&line)
        {
            continue;
        }
        let call = if index >= 1 && ident(index - 1) == Some("fn") {
            continue;
        } else if index >= 1 && punct(index - 1, '.') {
            Call::Method(name.to_owned())
        } else if index >= 2 && punct(index - 1, ':') && punct(index - 2, ':') {
            match index.checked_sub(3).and_then(ident) {
                Some(seg) => Call::Path(seg.to_owned(), name.to_owned()),
                None => continue,
            }
        } else {
            Call::Bare(name.to_owned())
        };
        found.push((line, call));
    }
    found
}

/// A file's `use` imports: local name to `(parent module segment,
/// imported name)`, and the module segments it glob-imports.
fn imports(parsed: &syn::File) -> (BTreeMap<String, (String, String)>, BTreeSet<String>) {
    fn walk(
        tree: &syn::UseTree,
        parent: &str,
        names: &mut BTreeMap<String, (String, String)>,
        globs: &mut BTreeSet<String>,
    ) {
        match tree {
            syn::UseTree::Path(path) => walk(&path.tree, &path.ident.to_string(), names, globs),
            syn::UseTree::Name(name) => {
                let name = name.ident.to_string();
                names.insert(name.clone(), (parent.to_owned(), name));
            }
            syn::UseTree::Rename(rename) => {
                names.insert(
                    rename.rename.to_string(),
                    (parent.to_owned(), rename.ident.to_string()),
                );
            }
            syn::UseTree::Glob(_) => {
                globs.insert(parent.to_owned());
            }
            syn::UseTree::Group(group) => {
                for item in &group.items {
                    walk(item, parent, names, globs);
                }
            }
        }
    }
    struct Visitor(BTreeMap<String, (String, String)>, BTreeSet<String>);
    impl<'ast> Visit<'ast> for Visitor {
        fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
            walk(&node.tree, "", &mut self.0, &mut self.1);
        }
    }
    let mut visitor = Visitor(BTreeMap::new(), BTreeSet::new());
    visitor.visit_file(parsed);
    (visitor.0, visitor.1)
}

/// The local names a file's `use serde_json::...` items give the
/// [`SERIALIZERS`], renamed (`to_vec as tv`) or not.
fn imported_serializers(parsed: &syn::File) -> BTreeSet<String> {
    fn walk(tree: &syn::UseTree, under_serde_json: bool, out: &mut BTreeSet<String>) {
        match tree {
            syn::UseTree::Path(path) => {
                walk(
                    &path.tree,
                    under_serde_json || path.ident == "serde_json",
                    out,
                );
            }
            syn::UseTree::Name(name) if under_serde_json => {
                if SERIALIZERS.contains(&name.ident.to_string().as_str()) {
                    out.insert(name.ident.to_string());
                }
            }
            syn::UseTree::Rename(rename) if under_serde_json => {
                if SERIALIZERS.contains(&rename.ident.to_string().as_str()) {
                    out.insert(rename.rename.to_string());
                }
            }
            syn::UseTree::Group(group) => {
                for item in &group.items {
                    walk(item, under_serde_json, out);
                }
            }
            _ => {}
        }
    }
    struct Visitor(BTreeSet<String>);
    impl<'ast> Visit<'ast> for Visitor {
        fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
            walk(&node.tree, false, &mut self.0);
        }
    }
    let mut visitor = Visitor(BTreeSet::new());
    visitor.visit_file(parsed);
    visitor.0
}

fn scan_file(path: &Path, crate_src: &str, module: &str) -> Result<ScannedFile> {
    let text = fs::read_to_string(path).map_err(|error| Error::io(path, error))?;
    let parsed = syn::parse_file(&text).map_err(|source| Error::source_parse(path, source))?;
    let stream: proc_macro2::TokenStream = text
        .parse()
        .map_err(|source| Error::source_parse(path, source))?;
    let mut tokens = Vec::new();
    flatten_tokens(stream, &mut tokens);
    let excluded = cfg_test_lines(&parsed);
    let lines = |patterns: &[&str]| -> Vec<usize> {
        let compiled: Vec<CallPattern> = patterns.iter().map(|p| CallPattern::compile(p)).collect();
        pattern_match_lines(&tokens, &compiled)
            .into_iter()
            .filter(|line| !excluded.contains(line))
            .collect()
    };
    let mut serializer_patterns: Vec<String> = SERIALIZERS
        .iter()
        .map(|name| format!("serde_json::{name}("))
        .collect();
    serializer_patterns.extend(
        imported_serializers(&parsed)
            .iter()
            .map(|alias| format!("{alias}(")),
    );
    let patterns: Vec<&str> = serializer_patterns.iter().map(String::as_str).collect();
    let (imports, glob_imports) = imports(&parsed);
    Ok(ScannedFile {
        path: path.to_path_buf(),
        crate_src: crate_src.to_owned(),
        module: module.to_owned(),
        hasher_lines: lines(HASHERS),
        json_lines: lines(JSON),
        serializer_lines: lines(&patterns),
        use_lines: use_lines(&parsed),
        functions: functions(&parsed, &excluded),
        calls: calls(&tokens, &excluded),
        imports,
        glob_imports,
    })
}

/// Every shipped file under every scanned crate root of `qsl_root`.
fn scan_tree(qsl_root: &Path) -> Result<Vec<ScannedFile>> {
    let mut scanned = Vec::new();
    for src_root in qsl_scan_src_roots(Role::Qsl, qsl_root) {
        if !src_root.exists() {
            return Err(Error::new(
                crate::error::Code::Usage,
                format!("source root does not exist: {}", src_root.display()),
            ));
        }
        let crate_src = src_root
            .strip_prefix(qsl_root)
            .map_err(|_| {
                Error::new(
                    crate::error::Code::Usage,
                    format!("{} is not under {}", src_root.display(), qsl_root.display()),
                )
            })?
            .to_string_lossy()
            .replace('\\', "/");
        let mut files = Vec::new();
        walk_rs_files(&src_root, &mut files)?;
        files.sort();
        let mut modules = Vec::with_capacity(files.len());
        for file in files {
            let module = file
                .strip_prefix(&src_root)
                .map(module_path_of)
                .map_err(|_| {
                    Error::new(
                        crate::error::Code::Usage,
                        format!("{} is not under {}", file.display(), src_root.display()),
                    )
                })?;
            modules.push((file, module));
        }
        let test_modules = cfg_test_module_declarations(&modules)?;
        for (file, module) in &modules {
            let under = |ancestor: &String| {
                module == ancestor || module.starts_with(&format!("{ancestor}::"))
            };
            if !test_modules.iter().any(under) {
                scanned.push(scan_file(file, &crate_src, module)?);
            }
        }
    }
    Ok(scanned)
}

/// A function, as `(file index, function index)`.
type FnId = (usize, usize);

/// One resolved call: its line, the calling function, and every function
/// it can reach.
struct Resolved {
    line: usize,
    caller: Option<usize>,
    callees: Vec<FnId>,
}

/// Every file's calls, resolved to the functions they can reach.
fn resolve(files: &[ScannedFile]) -> Vec<Vec<Resolved>> {
    let mut by_name: HashMap<&str, Vec<FnId>> = HashMap::new();
    for (file_index, file) in files.iter().enumerate() {
        for (fn_index, function) in file.functions.iter().enumerate() {
            by_name
                .entry(function.name.as_str())
                .or_default()
                .push((file_index, fn_index));
        }
    }
    files
        .iter()
        .map(|caller| {
            caller
                .calls
                .iter()
                .filter_map(|(line, call)| {
                    let name = match call {
                        Call::Path(_, name) | Call::Method(name) => name.as_str(),
                        Call::Bare(local) => caller
                            .imports
                            .get(local)
                            .map_or(local.as_str(), |(_, original)| original.as_str()),
                    };
                    let callees: Vec<FnId> = by_name
                        .get(name)?
                        .iter()
                        .copied()
                        .filter(|(file_index, fn_index)| {
                            let defined = &files[*file_index];
                            defined.crate_src == caller.crate_src
                                && reaches(call, caller, defined, &defined.functions[*fn_index])
                        })
                        .collect();
                    // `.name(` names no type: it is followed only when one
                    // method of the crate has that name.
                    if matches!(call, Call::Method(_)) && callees.len() > 1 {
                        return None;
                    }
                    // A module segment names the caller's child or sibling
                    // module, or a root module, when one of those defines
                    // the function; a same-named module elsewhere does not.
                    let segment = match call {
                        Call::Path(seg, _) => Some(seg.as_str()),
                        Call::Bare(local) => caller.imports.get(local).map(|(p, _)| p.as_str()),
                        Call::Method(_) => None,
                    };
                    let mut callees = callees;
                    if let Some(segment) = segment {
                        let near = |(file_index, _): &FnId| {
                            let module = files[*file_index].module.as_str();
                            let under = |parent: &str| {
                                module == segment && parent.is_empty()
                                    || module
                                        .strip_prefix(parent)
                                        .and_then(|rest| rest.strip_prefix("::"))
                                        == Some(segment)
                            };
                            under(&caller.module)
                                || under(caller.parent_module())
                                || module == segment
                        };
                        if callees.iter().any(near) {
                            callees.retain(near);
                        }
                    }
                    (!callees.is_empty()).then(|| Resolved {
                        line: *line,
                        caller: caller.enclosing(*line),
                        callees,
                    })
                })
                .collect()
        })
        .collect()
}

/// The functions each of whose lines include a `seed` line, closed under
/// calls: a function calling a member is a member. No `stop` function is a
/// member, so nothing joins through one.
fn closure(
    files: &[ScannedFile],
    calls: &[Vec<Resolved>],
    stop: &BTreeSet<FnId>,
    seed: impl Fn(&ScannedFile) -> Vec<usize>,
) -> BTreeSet<FnId> {
    let mut members = BTreeSet::new();
    for (file_index, file) in files.iter().enumerate() {
        for line in seed(file) {
            if let Some(fn_index) = file.enclosing(line) {
                if !stop.contains(&(file_index, fn_index)) {
                    members.insert((file_index, fn_index));
                }
            }
        }
    }
    loop {
        let mut added = Vec::new();
        for (file_index, file_calls) in calls.iter().enumerate() {
            for call in file_calls {
                let Some(caller) = call.caller else {
                    continue;
                };
                let id = (file_index, caller);
                if !members.contains(&id)
                    && !stop.contains(&id)
                    && call.callees.iter().any(|c| members.contains(c))
                {
                    added.push(id);
                }
            }
        }
        if added.is_empty() {
            return members;
        }
        members.extend(added);
    }
}

/// Every call in file `file_index` of a `members` function defined in
/// another file.
fn cross_calls(
    files: &[ScannedFile],
    calls: &[Resolved],
    file_index: usize,
    members: &BTreeSet<FnId>,
) -> Vec<CrossCall> {
    let mut found: Vec<CrossCall> = calls
        .iter()
        .flat_map(|call| {
            call.callees
                .iter()
                .filter(|callee| callee.0 != file_index && members.contains(callee))
                .map(|(defined, fn_index)| CrossCall {
                    line: call.line,
                    function: files[*defined].functions[*fn_index].qualified(),
                    defined_in: files[*defined].path.clone(),
                })
        })
        .collect();
    found.sort();
    found.dedup();
    found
}

/// Run the check over the QSL checkout at `qsl_root`: every shipped QSL
/// crate's `src/`, the roots FR-060's QSL-side rules scan.
pub(crate) fn evaluate(qsl_root: &Path) -> Result<Outcome> {
    let files = scan_tree(qsl_root)?;
    let calls = resolve(&files);
    // An exempt function's digest is sanctioned where it is: calling it is
    // not a new hash site.
    let exempt_functions: BTreeSet<FnId> = files
        .iter()
        .enumerate()
        .flat_map(|(file_index, file)| {
            file.functions
                .iter()
                .enumerate()
                .filter(move |(_, function)| {
                    let name = function.qualified();
                    EXEMPT.iter().any(|e| {
                        e.crate_src == file.crate_src
                            && e.module == file.module
                            && e.functions.contains(&name.as_str())
                    })
                })
                .map(move |(fn_index, _)| (file_index, fn_index))
        })
        .collect();
    let producers = closure(&files, &calls, &BTreeSet::new(), |file| {
        file.serializer_lines.clone()
    });
    let hashers = closure(&files, &calls, &exempt_functions, |file| {
        file.hasher_lines
            .iter()
            .copied()
            .filter(|line| !file.use_lines.contains(line))
            .collect()
    });
    let mut outcome = Outcome::default();
    let mut seen: BTreeSet<(usize, &str)> = BTreeSet::new();
    for (file_index, file) in files.iter().enumerate() {
        let json_calls = cross_calls(&files, &calls[file_index], file_index, &producers);
        let hash_calls = cross_calls(&files, &calls[file_index], file_index, &hashers);
        let hashes = !file.hasher_lines.is_empty() || !hash_calls.is_empty();
        let reads_json = !file.json_lines.is_empty() || !json_calls.is_empty();
        if !(hashes && reads_json) {
            continue;
        }
        let mut hash_sites: Vec<HashSite> = file
            .hasher_lines
            .iter()
            .copied()
            .filter(|line| !file.use_lines.contains(line))
            .chain(hash_calls.iter().map(|call| call.line))
            .map(|line| HashSite {
                line,
                function: file.enclosing_name(line),
            })
            .collect();
        hash_sites.sort();
        hash_sites.dedup();
        let site = EncoderSite {
            file: file.path.clone(),
            crate_src: file.crate_src.clone(),
            module: file.module.clone(),
            hasher_lines: file.hasher_lines.clone(),
            json_lines: file.json_lines.clone(),
            json_calls,
            hash_calls,
            hash_sites,
        };
        let entries: Vec<(usize, &'static Exemption)> = EXEMPT
            .iter()
            .enumerate()
            .filter(|(_, e)| e.crate_src == file.crate_src && e.module == file.module)
            .collect();
        if entries.is_empty() {
            outcome.violations.push(site);
            continue;
        }
        let mut used = Vec::new();
        let mut unlisted = Vec::new();
        for hash_site in &site.hash_sites {
            let owner = entries.iter().find(|(_, e)| {
                e.functions
                    .iter()
                    .any(|function| *function == hash_site.function)
                    && (e.calls.is_empty() || pinned_call(&site, hash_site.line, e.calls))
            });
            match owner {
                Some((index, exemption)) => {
                    let function = exemption
                        .functions
                        .iter()
                        .find(|function| **function == hash_site.function)
                        .copied()
                        .unwrap_or_default();
                    seen.insert((*index, function));
                    if !used
                        .iter()
                        .any(|e: &&Exemption| std::ptr::eq(*e, *exemption))
                    {
                        used.push(*exemption);
                    }
                }
                None => unlisted.push(hash_site.clone()),
            }
        }
        for (index, _) in &entries {
            seen.insert((*index, ""));
        }
        for (_, exemption) in entries.iter().filter(|(_, e)| !e.calls.is_empty()) {
            // Every way the function can hold JSON: a line naming
            // `serde_json`, a serializer call however it was imported, and
            // a call of any JSON producer, in this file or another.
            let producer_calls = calls[file_index]
                .iter()
                .filter(|call| call.callees.iter().any(|callee| producers.contains(callee)))
                .map(|call| call.line);
            let mut json_lines: Vec<usize> = site
                .json_lines
                .iter()
                .chain(&file.serializer_lines)
                .copied()
                .filter(|line| !file.use_lines.contains(line))
                .chain(producer_calls)
                .collect();
            json_lines.sort_unstable();
            json_lines.dedup();
            for line in json_lines {
                let function = file.enclosing_name(line);
                if exemption.functions.contains(&function.as_str()) {
                    outcome
                        .json_in_pinned
                        .push((site.file.clone(), HashSite { line, function }));
                }
            }
        }
        if !unlisted.is_empty() {
            outcome.unlisted.push((site.clone(), unlisted));
        }
        outcome.allowed.push((site, used));
    }
    for (index, exemption) in EXEMPT.iter().enumerate() {
        if !seen.contains(&(index, "")) {
            outcome.stale.push((exemption, None));
            continue;
        }
        for function in exemption.functions {
            if !seen.contains(&(index, *function)) {
                outcome.stale.push((exemption, Some(function)));
            }
        }
    }
    Ok(outcome)
}

/// Whether the hash site at `line` is exactly a call of one of `calls`: a
/// call of another file's hashing function by that name, on a line naming
/// no [`HASHERS`] entry itself.
fn pinned_call(site: &EncoderSite, line: usize, calls: &[&str]) -> bool {
    !site.hasher_lines.contains(&line)
        && site
            .hash_calls
            .iter()
            .any(|call| call.line == line && calls.contains(&call.function.as_str()))
}

/// The report `main` prints.
pub(crate) fn report(outcome: &Outcome) -> String {
    let lines = |numbers: &[usize]| {
        numbers
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",")
    };
    let site = |site: &EncoderSite| {
        let mut text = format!("{} (module {})", site.file.display(), site.module);
        if !site.hasher_lines.is_empty() {
            text.push_str(&format!(": hash at line {}", lines(&site.hasher_lines)));
        }
        if !site.json_lines.is_empty() {
            text.push_str(&format!(", serde_json at line {}", lines(&site.json_lines)));
        }
        for call in &site.json_calls {
            text.push_str(&format!(
                ", calls JSON-producing {} (defined in {}) at line {}",
                call.function,
                call.defined_in.display(),
                call.line
            ));
        }
        for call in &site.hash_calls {
            text.push_str(&format!(
                ", calls hashing {} (defined in {}) at line {}",
                call.function,
                call.defined_in.display(),
                call.line
            ));
        }
        text
    };
    let mut summary = String::new();
    summary.push_str(
        "ADR-013 §2 one-RFC-8785-encoder check (ADR-013:113, QSL-194)\n  Note: a shipped file \
         fails if it both hashes (names Sha256, sha2, of_preimage or ByteDigest::of, or calls \
         a hashing function of another file) and reads as JSON (names serde_json, or calls a \
         JSON-producing function of another file). Both function sets are closed under \
         calls. Calls resolve by name within one crate; functions passed as values, other \
         crates' helpers, Display-only producers, method names two methods share, calls \
         into exempt functions and macro-generated functions are not followed.\n",
    );
    if outcome.passed() {
        summary.push_str("  PASS: no second canonical encoder beside quire-canonical\n");
    } else {
        summary.push_str("  FAIL\n");
        for violation in &outcome.violations {
            summary.push_str(&format!(
                "    second canonical encoder: {} -- encode through quire-canonical instead\n",
                site(violation)
            ));
        }
        for (exempt, sites) in &outcome.unlisted {
            for hash_site in sites {
                summary.push_str(&format!(
                    "    hash site outside the exempt functions: {} line {} in `{}` -- \
                     encode through quire-canonical instead\n",
                    exempt.file.display(),
                    hash_site.line,
                    hash_site.function
                ));
            }
        }
        for (file, json) in &outcome.json_in_pinned {
            summary.push_str(&format!(
                "    JSON inside a call-pinned exempt function: {} line {} in `{}` -- \
                 hash only through the pinned call\n",
                file.display(),
                json.line,
                json.function
            ));
        }
        for (stale, function) in &outcome.stale {
            match function {
                Some(function) => summary.push_str(&format!(
                    "    stale exemption, `{function}` has no hash site: {} {}\n",
                    stale.crate_src, stale.module
                )),
                None => summary.push_str(&format!(
                    "    stale exemption, no remaining match: {} {}\n",
                    stale.crate_src, stale.module
                )),
            }
        }
    }
    for (label, kind) in [
        ("spec-carved identity", ExemptionKind::SpecCarvedIdentity),
        ("exempt", ExemptionKind::NotAnIdentity),
    ] {
        for (allowed, exemption) in outcome.allowed_of(kind) {
            summary.push_str(&format!(
                "    {label}: {} [{}] -- {}\n",
                site(allowed),
                exemption.functions.join(", "),
                exemption.reason
            ));
        }
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOTS: [&str; 10] = [
        "src",
        "qsl-foundation/src",
        "qsl-cst/src",
        "qsl-source/src",
        "qsl-forms/src",
        "qsl-semantics/src",
        "qsl-package/src",
        "qsl-eval/src",
        "qsl-route/src",
        "qsl-replay/src",
    ];

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    /// The fixture text satisfying every [`EXEMPT`] entry of `module`: one
    /// hashing function per listed function, in a file naming `serde_json`.
    /// A call-pinned entry's functions hash by calling its pinned callees,
    /// which [`PINNED_CALLEES`] defines.
    fn exempt_file(crate_src: &str, module: &str) -> String {
        let mut text = format!("use crate::{PINNED_CALLEES}::*;\nuse serde_json::Value;\n");
        for exemption in EXEMPT
            .iter()
            .filter(|e| e.crate_src == crate_src && e.module == module)
        {
            let body = if exemption.calls.is_empty() {
                "Sha256::digest(b);".to_owned()
            } else {
                exemption
                    .calls
                    .iter()
                    .map(|callee| format!("{callee}(b);"))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            for function in exemption.functions {
                match function.split_once("::") {
                    Some((self_ty, name)) => text.push_str(&format!(
                        "impl {self_ty} {{\n    fn {name}(b: &[u8]) {{\n        {body}\n    }}\n}}\n"
                    )),
                    None => text.push_str(&format!(
                        "fn {function}(b: &[u8]) {{\n    {body}\n}}\n"
                    )),
                }
            }
        }
        text
    }

    /// The module, at each crate's source root, defining every call-pinned
    /// exemption's callees as hashing functions that name no JSON.
    const PINNED_CALLEES: &str = "exempt_pinned_callees";

    /// A QSL tree with every scan root and every [`EXEMPT`] entry matching,
    /// so a test sees only the scenario it plants.
    fn tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for relative in ROOTS {
            fs::create_dir_all(dir.path().join(relative)).unwrap();
        }
        for exemption in EXEMPT {
            write(
                dir.path(),
                &format!(
                    "{}/{}.rs",
                    exemption.crate_src,
                    exemption.module.replace("::", "/")
                ),
                &exempt_file(exemption.crate_src, exemption.module),
            );
            if !exemption.calls.is_empty() {
                let callees: String = exemption
                    .calls
                    .iter()
                    .map(|callee| {
                        format!("pub fn {callee}(b: &[u8]) -> [u8; 32] {{ Sha256::digest(b).into() }}\n")
                    })
                    .collect();
                write(
                    dir.path(),
                    &format!("{}/{PINNED_CALLEES}.rs", exemption.crate_src),
                    &callees,
                );
            }
        }
        dir
    }

    const SECOND_ENCODER: &str = "\
use sha2::{Digest, Sha256};

pub(super) fn jcs_bytes(value: &serde_json::Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}

pub(super) fn digest_of(value: &serde_json::Value) -> [u8; 32] {
    Sha256::digest(jcs_bytes(value)).into()
}
";

    fn outcome_of(dir: &tempfile::TempDir) -> Outcome {
        let outcome = evaluate(dir.path()).unwrap();
        assert_eq!(outcome.stale, Vec::new(), "{}", report(&outcome));
        outcome
    }

    fn violations(dir: &tempfile::TempDir) -> Vec<EncoderSite> {
        let outcome = outcome_of(dir);
        assert!(outcome.unlisted.is_empty(), "{}", report(&outcome));
        outcome.violations
    }

    fn calls(found: &[CrossCall]) -> Vec<(usize, &str)> {
        found
            .iter()
            .map(|call| (call.line, call.function.as_str()))
            .collect()
    }

    /// The deleted `key.rs` pair, serializing in one function and hashing
    /// in another, fails, naming both lines.
    #[test]
    fn a_second_encoder_split_across_functions_fails() {
        let dir = tree();
        write(dir.path(), "qsl-semantics/src/model/key.rs", SECOND_ENCODER);
        let found = violations(&dir);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].module, "model::key");
        assert_eq!(found[0].hasher_lines, vec![1, 8]);
        assert_eq!(found[0].json_lines, vec![3, 4, 7]);
    }

    /// PR #389 review, planted case A: a grouped, renamed serializer import.
    #[test]
    fn a_renamed_serializer_import_fails() {
        let dir = tree();
        write(
            dir.path(),
            "qsl-semantics/src/model/population.rs",
            "use serde_json::{to_vec as tv, Value};\nuse sha2::{Digest, Sha256};\n\
             pub fn id(v: &Value) -> [u8; 32] { Sha256::digest(tv(v).unwrap()).into() }\n",
        );
        let found = violations(&dir);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].json_lines, vec![1]);
        assert_eq!(found[0].hasher_lines, vec![2, 3]);
    }

    /// PR #389 review, planted case B: a hash over `Value::to_string()`.
    #[test]
    fn a_hash_over_value_display_fails() {
        let dir = tree();
        write(
            dir.path(),
            "qsl-semantics/src/model/population.rs",
            "pub fn id(v: &serde_json::Value) -> ByteDigest { \
             ByteDigest::of(v.to_string().as_bytes()) }\n",
        );
        assert_eq!(violations(&dir).len(), 1);
    }

    /// PR #389 review, planted case C: a `json_bytes` helper in one file,
    /// hashed in another that never names `serde_json` itself.
    #[test]
    fn a_json_helper_hashed_in_another_file_fails() {
        let dir = tree();
        write(
            dir.path(),
            "qsl-semantics/src/model/json.rs",
            "pub fn json_bytes<T: serde::Serialize>(v: &T) -> Vec<u8> { \
             serde_json::to_vec(v).unwrap() }\n",
        );
        write(
            dir.path(),
            "qsl-semantics/src/model/key.rs",
            "use super::json::json_bytes;\n\
             pub fn id<T: serde::Serialize>(v: &T) -> [u8; 32] {\n    \
             *ByteDigest::of(&json_bytes(v)).as_bytes()\n}\n",
        );
        let found = violations(&dir);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].module, "model::key");
        assert!(found[0].json_lines.is_empty());
        assert_eq!(calls(&found[0].json_calls), vec![(3, "json_bytes")]);
    }

    /// PR #389 delta review, planted case D: the JSON producer is a method
    /// on a type in another file, called as `.canonical_bytes()`.
    #[test]
    fn a_json_producing_method_hashed_in_another_file_fails() {
        let dir = tree();
        write(
            dir.path(),
            "qsl-semantics/src/model/wire.rs",
            "pub struct Wire;\nimpl Wire {\n    pub fn canonical_bytes(&self) -> Vec<u8> {\n        \
             serde_json::to_vec(self).unwrap()\n    }\n}\n",
        );
        write(
            dir.path(),
            "qsl-semantics/src/model/key.rs",
            "pub fn id(w: &super::wire::Wire) -> ByteDigest {\n    \
             ByteDigest::of(&w.canonical_bytes())\n}\n",
        );
        let found = violations(&dir);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].module, "model::key");
        assert_eq!(
            calls(&found[0].json_calls),
            vec![(2, "Wire::canonical_bytes")]
        );
    }

    /// PR #389 delta review, planted case E: two hops -- a helper calling a
    /// helper in a third file that calls the serializer.
    #[test]
    fn a_two_hop_json_helper_hashed_in_another_file_fails() {
        let dir = tree();
        write(
            dir.path(),
            "qsl-semantics/src/model/inner.rs",
            "pub fn raw<T: serde::Serialize>(v: &T) -> Vec<u8> {\n    \
             serde_json::to_vec(v).unwrap()\n}\n",
        );
        write(
            dir.path(),
            "qsl-semantics/src/model/outer.rs",
            "pub fn bytes<T: serde::Serialize>(v: &T) -> Vec<u8> {\n    \
             super::inner::raw(v)\n}\n",
        );
        write(
            dir.path(),
            "qsl-semantics/src/model/key.rs",
            "use super::outer::bytes;\npub fn id<T: serde::Serialize>(v: &T) -> ByteDigest {\n    \
             ByteDigest::of(&bytes(v))\n}\n",
        );
        let found = violations(&dir);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].module, "model::key");
        assert_eq!(calls(&found[0].json_calls), vec![(3, "bytes")]);
    }

    /// PR #389 delta review, planted case F, the mirror: a file naming
    /// `serde_json` hands its bytes to a hash helper living in a clean file
    /// (`key.rs`'s `raw_bytes_digest`).
    #[test]
    fn serde_json_bytes_handed_to_a_clean_files_hash_helper_fails() {
        let dir = tree();
        write(
            dir.path(),
            "qsl-semantics/src/model/key.rs",
            "pub(super) fn raw_bytes_digest(bytes: &[u8]) -> [u8; 32] {\n    \
             *ByteDigest::of(bytes).as_bytes()\n}\n",
        );
        write(
            dir.path(),
            "qsl-semantics/src/model/population.rs",
            "pub fn id(v: &V) -> [u8; 32] {\n    \
             crate::model::key::raw_bytes_digest(&serde_json::to_vec(v).unwrap())\n}\n",
        );
        let found = violations(&dir);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].module, "model::population");
        assert!(found[0].hasher_lines.is_empty());
        assert_eq!(calls(&found[0].hash_calls), vec![(2, "raw_bytes_digest")]);
    }

    /// PR #389 delta review, planted case G: a new hash helper, fed a
    /// `Value`'s `Display` text from a file naming `serde_json`.
    #[test]
    fn a_value_display_handed_to_a_new_hash_helper_fails() {
        let dir = tree();
        write(
            dir.path(),
            "qsl-semantics/src/model/zz_helper.rs",
            "use sha2::{Digest, Sha256};\npub fn hash(b: &[u8]) -> [u8; 32] {\n    \
             Sha256::digest(b).into()\n}\n",
        );
        write(
            dir.path(),
            "qsl-semantics/src/model/population.rs",
            "use serde_json::Value;\npub fn id(v: &Value) -> [u8; 32] {\n    \
             super::zz_helper::hash(v.to_string().as_bytes())\n}\n",
        );
        let found = violations(&dir);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].module, "model::population");
        assert_eq!(calls(&found[0].hash_calls), vec![(3, "hash")]);
    }

    /// A tree whose hashing files name no JSON, and whose JSON files hash
    /// nothing, passes; the exempt files are still reported.
    #[test]
    fn a_tree_with_no_second_encoder_passes() {
        let dir = tree();
        write(
            dir.path(),
            "qsl-semantics/src/model/key.rs",
            "pub fn digest(value: &V) -> [u8; 32] { \
             *quire_canonical::sha256(value, LIMITS).unwrap().as_bytes() }\n\
             pub fn raw(bytes: &[u8]) -> ByteDigest { ByteDigest::of(bytes) }\n",
        );
        write(
            dir.path(),
            "qsl-semantics/src/model/population.rs",
            "pub fn read(b: &[u8]) -> serde_json::Value { serde_json::from_slice(b).unwrap() }\n",
        );
        let outcome = evaluate(dir.path()).unwrap();
        assert!(outcome.passed(), "{}", report(&outcome));
        for kind in [
            ExemptionKind::SpecCarvedIdentity,
            ExemptionKind::NotAnIdentity,
        ] {
            let listed = EXEMPT.iter().filter(|e| e.kind == kind).count();
            assert_eq!(outcome.allowed_of(kind).len(), listed, "{kind:?}");
        }
    }

    /// A serializer and a hasher inside `#[cfg(test)]` code, in an
    /// out-of-line `#[cfg(test)] mod` file, or named only in a comment or a
    /// string, are not shipped code and do not match.
    #[test]
    fn test_code_comments_and_strings_do_not_match() {
        let dir = tree();
        write(
            dir.path(),
            "qsl-semantics/src/model/population.rs",
            "// serde_json::to_vec then Sha256\n\
             pub const NOTE: &str = \"serde_json::to_vec Sha256\";\n\
             #[cfg(test)]\nmod tests {\n    fn f(v: &V) { Sha256::digest(serde_json::to_vec(v).unwrap()); }\n}\n",
        );
        write(
            dir.path(),
            "qsl-package/src/checked_v2.rs",
            "#[cfg(test)]\nmod tests;\npub fn read() {}\n",
        );
        write(
            dir.path(),
            "qsl-package/src/checked_v2/tests.rs",
            SECOND_ENCODER,
        );
        let outcome = evaluate(dir.path()).unwrap();
        assert!(outcome.passed(), "{}", report(&outcome));
    }

    /// A new hash site in an exempt file, outside its listed functions,
    /// fails, naming the function it sits in.
    #[test]
    fn a_new_hash_site_in_an_exempt_file_fails() {
        let dir = tree();
        let mut text = exempt_file("src", "command");
        text.push_str(
            "fn fresh(v: &V) {\n    Sha256::digest(serde_json::to_vec(v).unwrap());\n}\n",
        );
        write(dir.path(), "src/command.rs", &text);
        let outcome = outcome_of(&dir);
        assert!(!outcome.passed());
        assert!(outcome.violations.is_empty());
        assert_eq!(outcome.unlisted.len(), 1);
        let sites = &outcome.unlisted[0].1;
        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].function, "fresh");
    }

    /// QSL-220: intake's exemption is pinned to its `raw_bytes_digest`
    /// call. The pre-#389 `check_package_digest` (55db8a4c), hashing a
    /// `jcs_bytes` JSON producer with `Sha256` directly, fails on both
    /// counts, and hashing JSON through the pinned call still fails.
    #[test]
    fn intake_exemption_is_pinned_to_its_raw_bytes_digest_call() {
        const INTAKE: &str = "qsl-semantics/src/model/intake.rs";
        let dir = tree();
        write(
            dir.path(),
            "qsl-semantics/src/model/key.rs",
            "pub fn jcs_bytes(v: &Tree) -> Vec<u8> { serde_json::to_vec(v).unwrap() }\n",
        );
        write(
            dir.path(),
            INTAKE,
            "use super::key::jcs_bytes;\nuse serde_json::Value;\n\
             fn check_package_digest(b: &[u8], d: Option<&Doc>) -> [u8; 32] {\n\
             \x20   match d {\n\
             \x20       Some(d) => Sha256::digest(jcs_bytes(&d.tree)).into(),\n\
             \x20       None => Sha256::digest(b).into(),\n\
             \x20   }\n}\n",
        );
        let outcome = evaluate(dir.path()).unwrap();
        assert!(!outcome.passed());
        let unlisted: Vec<usize> = outcome
            .unlisted
            .iter()
            .flat_map(|(_, sites)| sites.iter().map(|site| site.line))
            .collect();
        assert_eq!(unlisted, vec![5, 6], "{}", report(&outcome));
        let json: Vec<(usize, &str)> = outcome
            .json_in_pinned
            .iter()
            .map(|(_, site)| (site.line, site.function.as_str()))
            .collect();
        assert_eq!(json, vec![(5, "check_package_digest")]);

        write(
            dir.path(),
            INTAKE,
            "use super::key::jcs_bytes;\nuse crate::exempt_pinned_callees::*;\n\
             fn check_package_digest(b: &[u8], d: Option<&Doc>) -> [u8; 32] {\n\
             \x20   match d {\n\
             \x20       Some(d) => raw_bytes_digest(&jcs_bytes(&d.tree)),\n\
             \x20       None => raw_bytes_digest(b),\n\
             \x20   }\n}\n",
        );
        let outcome = evaluate(dir.path()).unwrap();
        assert!(outcome.unlisted.is_empty(), "{}", report(&outcome));
        let json: Vec<usize> = outcome
            .json_in_pinned
            .iter()
            .map(|(_, site)| site.line)
            .collect();
        assert_eq!(json, vec![5], "{}", report(&outcome));
        assert!(!outcome.passed());
    }

    /// The JSON lines a call-pinned exemption reports for `intake` text.
    fn pinned_json_lines(intake: &str) -> Vec<(usize, String)> {
        let dir = tree();
        write(dir.path(), "qsl-semantics/src/model/intake.rs", intake);
        let outcome = evaluate(dir.path()).unwrap();
        assert!(outcome.unlisted.is_empty(), "{}", report(&outcome));
        assert!(outcome.stale.is_empty(), "{}", report(&outcome));
        assert_eq!(
            outcome.passed(),
            outcome.json_in_pinned.is_empty(),
            "{}",
            report(&outcome)
        );
        outcome
            .json_in_pinned
            .into_iter()
            .map(|(_, site)| (site.line, site.function))
            .collect()
    }

    /// QSL-220 review M1 (A): a serializer imported by name and hashed
    /// through the pinned call fails.
    #[test]
    fn pinned_exemption_refuses_an_imported_serializer() {
        let found = pinned_json_lines(
            "use crate::exempt_pinned_callees::*;\nuse serde_json::to_vec;\n\
             fn check_package_digest(v: &Tree) -> [u8; 32] {\n\
             \x20   raw_bytes_digest(&to_vec(v).unwrap())\n}\n",
        );
        assert_eq!(found, vec![(4, "check_package_digest".to_owned())]);
    }

    /// QSL-220 review M1 (B): a JSON producer defined in the same file and
    /// hashed through the pinned call fails.
    #[test]
    fn pinned_exemption_refuses_a_same_file_json_producer() {
        let found = pinned_json_lines(
            "use crate::exempt_pinned_callees::*;\n\
             fn json(v: &Tree) -> Vec<u8> {\n    serde_json::to_vec(v).unwrap()\n}\n\
             fn check_package_digest(v: &Tree) -> [u8; 32] {\n\
             \x20   raw_bytes_digest(&json(v))\n}\n",
        );
        assert_eq!(found, vec![(6, "check_package_digest".to_owned())]);
    }

    /// The pinned call over exact bytes, next to JSON elsewhere in the
    /// file, passes.
    #[test]
    fn pinned_exemption_accepts_raw_bytes_beside_json_elsewhere() {
        let found = pinned_json_lines(
            "use crate::exempt_pinned_callees::*;\n\
             fn json(v: &Tree) -> Vec<u8> {\n    serde_json::to_vec(v).unwrap()\n}\n\
             fn check_package_digest(b: &[u8]) -> [u8; 32] {\n\
             \x20   raw_bytes_digest(b)\n}\n",
        );
        assert_eq!(found, Vec::new());
    }

    /// An exemption whose file no longer pairs the two fails as stale, and
    /// so does a listed function with no hash site left.
    #[test]
    fn a_stale_exemption_fails() {
        let dir = tree();
        write(
            dir.path(),
            "src/protocol_artifact/checked_handoff.rs",
            "pub fn encode(v: &V) -> Vec<u8> { quire_canonical::to_vec(v, L).unwrap() }\n",
        );
        write(
            dir.path(),
            "src/package.rs",
            "use serde_json::Value;\nimpl NativePackage {\n    fn new(b: &[u8]) {\n        \
             ByteDigest::of(b);\n    }\n    fn read_verified(b: &[u8]) {\n        \
             ByteDigest::of(b);\n    }\n}\n",
        );
        let outcome = evaluate(dir.path()).unwrap();
        assert!(!outcome.passed());
        let stale: Vec<(&str, Option<&str>)> = outcome
            .stale
            .iter()
            .map(|(exemption, function)| (exemption.module, *function))
            .collect();
        assert_eq!(
            stale,
            vec![
                ("package", Some("NativePackageIdentity::of")),
                ("protocol_artifact::checked_handoff", None),
            ]
        );
    }
}
