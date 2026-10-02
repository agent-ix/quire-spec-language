// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-270 (ADR-032 CK-1 to CK-5): `cargo xtask checked-input`, the gate that
//! keeps execution, routing and replay stage entries on checked inputs.
//!
//! **Stage entries (CK-1).** Every `pub` function and every `pub` method of
//! an inherent `impl` in the shipped code of `qsl-eval` and `qsl-route`, and
//! the same in `qsl-replay` outside its `spine` module (`spine.rs` and the
//! files under `spine/`). Membership comes from crate and module alone.
//!
//! **Pre-check representations (CK-2).** Derived from the scanned code on
//! every run, never listed:
//! - a type with fields (a struct or union with a field, an enum with a
//!   variant that has one) defined in `qsl-source`, `qsl-cst` or
//!   `qsl-forms`, that the return type of one of those crates' `pub`
//!   functions or `pub` inherent methods reaches, directly or through the
//!   fields and `type` aliases of those crates' types;
//! - the S3 input, the receiver of [`S3_CONSTRUCTOR`];
//! - every type of `quire-contract-model`: any type path rooted at that
//!   crate.
//!
//! A fieldless enum (closed vocabulary such as `StateClauseKind`) is outside
//! the set, and so is configuration: a type a `pub` free function of those
//! crates (a pre-check stage) takes as a parameter and no such function's
//! return type names, even when a stage output holds it (`qsl_cst::Limits`,
//! which `ParsedSource` keeps as its effective limits).
//!
//! **Signature rule (CK-3).** A stage entry whose receiver, parameter types
//! or generic bounds mention a pre-check representation at any depth is a
//! `signature` finding, one per entry and representation.
//!
//! **Reconstruction rule (CK-4).** A shipped function anywhere in those
//! crates (outside `spine`) whose signature or body names a `pub` function
//! or `pub` inherent method of `qsl-source`, `qsl-cst` or `qsl-forms`, or
//! the S3 stage constructor, is a `reconstruction` finding, whatever its own
//! signature. Paths inside a macro are read when the macro's tokens parse as
//! comma-separated expressions (`vec![...]`, `format!(...)`).
//!
//! **Resolution.** A path's first segment is resolved through the `use`
//! items of its file (renames, `self` and globs from a pre-check crate) and
//! through the `type` aliases of the scanned stage crates, so
//! `use qsl_forms::Expression as Expr;` and `type Node = qsl_cst::Token;`
//! are seen through. As in `typestate_scan`, the `use` table is per file,
//! not per scope, and the alias table is global, which over-approximates.
//!
//! **Known limits.** A method call written with method syntax
//! (`declarations.check(limits)`) names no path and is not seen; a value of
//! the receiver type has to be named to be called that way, and its type in
//! a stage entry's signature is the `signature` rule's finding. A pre-check
//! type re-exported by another workspace crate (none is today) is seen only
//! by its defining crate's path, and a glob import from `quire-contract-model`
//! is not expanded.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;

use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::Visit;

use crate::definition_scan::has_cfg_test;
use crate::error::{Error, Result};
use crate::typestate_scan::{shipped_files, S3_CONSTRUCTOR};

/// The pre-check crates (ADR-011 layers I3, 1 and 2): source directory and
/// the crate name as a path spells it.
const PRE_CHECK_CRATES: [(&str, &str); 3] = [
    ("qsl-source/src", "qsl_source"),
    ("qsl-cst/src", "qsl_cst"),
    ("qsl-forms/src", "qsl_forms"),
];

/// The IR crate whose every type is a pre-check representation (CK-2 (c)).
const CONTRACT_MODEL: &str = "quire_contract_model";

/// The stage-entry crates: layer 5 `qsl-eval`, layer R `qsl-route` and the
/// layer-6 `replay` facade `qsl-replay` (minus [`SPINE`]).
const STAGE_CRATES: [&str; 3] = ["qsl-eval/src", "qsl-route/src", "qsl-replay/src"];

/// The `spine` module of `qsl-replay`: its file and its directory.
const SPINE: [&str; 2] = ["qsl-replay/src/spine.rs", "qsl-replay/src/spine/"];

/// Which rule a finding breaks.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Rule {
    /// CK-3: a stage entry's signature mentions a pre-check representation.
    Signature,
    /// CK-4: a shipped function names a pre-check stage function.
    Reconstruction,
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Signature => "signature",
            Self::Reconstruction => "reconstruction",
        })
    }
}

/// One violation.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Finding {
    /// The file, relative to the workspace root.
    pub file: String,
    /// The 1-based line: the entry's name for `signature`, the naming path
    /// for `reconstruction`.
    pub line: usize,
    /// The rule broken.
    pub rule: Rule,
    /// The stage entry or function (`Type::method` for a method).
    pub function: String,
    /// The pre-check type or stage function it names, as resolved.
    pub named: String,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match self.rule {
            Rule::Signature => "takes pre-check representation",
            Rule::Reconstruction => "calls pre-check stage function",
        };
        write!(
            f,
            "{}:{}: {}: `{}` {what} `{}`",
            self.file, self.line, self.rule, self.function, self.named
        )
    }
}

/// Every path segment name `ty` mentions, at any depth.
fn type_names(ty: &syn::Type) -> BTreeSet<String> {
    #[derive(Default)]
    struct Names(BTreeSet<String>);
    impl<'ast> Visit<'ast> for Names {
        fn visit_path_segment(&mut self, node: &'ast syn::PathSegment) {
            self.0.insert(node.ident.to_string());
            syn::visit::visit_path_segment(self, node);
        }
    }
    let mut names = Names::default();
    names.visit_type(ty);
    names.0
}

fn path_segments(path: &syn::Path) -> Vec<String> {
    path.segments.iter().map(|s| s.ident.to_string()).collect()
}

/// What the pre-check crates define and hand out.
#[derive(Debug, Default)]
struct PreCheck {
    /// Crate -> the fielded types a `pub` return reaches.
    types: BTreeMap<&'static str, BTreeSet<String>>,
    /// Crate -> its `pub` free function names.
    functions: BTreeMap<&'static str, BTreeSet<String>>,
    /// Crate -> its `pub` inherent methods, as (`Self` type, name).
    methods: BTreeMap<&'static str, BTreeSet<(String, String)>>,
}

/// One pre-check crate's raw facts, before the reach closure.
#[derive(Default)]
struct PreCheckCollector {
    /// Type or alias name -> (has fields, names its fields or target mention).
    definitions: BTreeMap<String, (bool, BTreeSet<String>)>,
    /// Names the return types of `pub` functions and methods mention.
    returned: BTreeSet<String>,
    /// Names the return types of `pub` free functions mention.
    stage_returns: BTreeSet<String>,
    /// Names the parameter types of `pub` free functions mention.
    stage_parameters: BTreeSet<String>,
    functions: BTreeSet<String>,
    methods: BTreeSet<(String, String)>,
    /// The inherent `impl` block's `Self` type, if inside one.
    inherent_self: Option<String>,
}

impl PreCheckCollector {
    fn define(&mut self, name: String, has_fields: bool, fields: &syn::Fields) {
        let entry = self.definitions.entry(name).or_default();
        entry.0 |= has_fields;
        for field in fields {
            entry.1.extend(type_names(&field.ty));
        }
    }

    fn returns(&mut self, output: &syn::ReturnType, self_type: Option<&str>) {
        if let syn::ReturnType::Type(_, ty) = output {
            let mut names = type_names(ty);
            if names.remove("Self") {
                names.extend(self_type.map(str::to_owned));
            }
            self.returned.extend(names);
        }
    }
}

impl<'ast> Visit<'ast> for PreCheckCollector {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if !has_cfg_test(&node.attrs) {
            syn::visit::visit_item_mod(self, node);
        }
    }

    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        if !has_cfg_test(&node.attrs) {
            self.define(
                node.ident.to_string(),
                !node.fields.is_empty(),
                &node.fields,
            );
        }
    }

    fn visit_item_union(&mut self, node: &'ast syn::ItemUnion) {
        if !has_cfg_test(&node.attrs) {
            let fields = syn::Fields::Named(node.fields.clone());
            self.define(node.ident.to_string(), !fields.is_empty(), &fields);
        }
    }

    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        if !has_cfg_test(&node.attrs) {
            let name = node.ident.to_string();
            for variant in &node.variants {
                self.define(name.clone(), !variant.fields.is_empty(), &variant.fields);
            }
            self.definitions.entry(name).or_default();
        }
    }

    fn visit_item_type(&mut self, node: &'ast syn::ItemType) {
        if !has_cfg_test(&node.attrs) {
            self.definitions
                .entry(node.ident.to_string())
                .or_default()
                .1
                .extend(type_names(&node.ty));
        }
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        if matches!(node.vis, syn::Visibility::Public(_)) {
            self.functions.insert(node.sig.ident.to_string());
            self.returns(&node.sig.output, None);
            if let syn::ReturnType::Type(_, ty) = &node.sig.output {
                self.stage_returns.extend(type_names(ty));
            }
            for input in &node.sig.inputs {
                if let syn::FnArg::Typed(typed) = input {
                    self.stage_parameters.extend(type_names(&typed.ty));
                }
            }
        }
        let previous = self.inherent_self.take();
        syn::visit::visit_item_fn(self, node);
        self.inherent_self = previous;
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        let inherent = node
            .trait_
            .is_none()
            .then(|| crate::impl_self_name(&node.self_ty));
        let previous = std::mem::replace(&mut self.inherent_self, inherent);
        syn::visit::visit_item_impl(self, node);
        self.inherent_self = previous;
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        if let (Some(self_type), syn::Visibility::Public(_)) = (&self.inherent_self, &node.vis) {
            let self_type = self_type.clone();
            self.methods
                .insert((self_type.clone(), node.sig.ident.to_string()));
            self.returns(&node.sig.output, Some(&self_type));
        }
        let previous = self.inherent_self.take();
        syn::visit::visit_impl_item_fn(self, node);
        self.inherent_self = previous;
    }
}

impl PreCheck {
    /// CK-2 over the shipped code of the pre-check crates under `root`.
    fn derive(root: &Path) -> Result<Self> {
        let mut collectors = Vec::new();
        for (dir, name) in PRE_CHECK_CRATES {
            let mut collector = PreCheckCollector::default();
            for (_, ast) in shipped_files(root, &[dir])? {
                collector.visit_file(&ast);
            }
            collectors.push((name, collector));
        }
        // Reach closure over the three crates' types and aliases together: a
        // `qsl-forms` return can reach a `qsl-cst` node through a field.
        let mut reached = BTreeSet::new();
        let mut pending: Vec<String> = collectors
            .iter()
            .flat_map(|(_, c)| c.returned.iter().cloned())
            .collect();
        while let Some(name) = pending.pop() {
            if !reached.insert(name.clone()) {
                continue;
            }
            for (_, collector) in &collectors {
                if let Some((_, mentions)) = collector.definitions.get(&name) {
                    pending.extend(mentions.iter().cloned());
                }
            }
        }
        // Configuration: what a stage function takes and no stage function
        // returns, though a stage output may hold it (`qsl_cst::Limits` in
        // `ParsedSource`).
        let stage_returns: BTreeSet<&String> = collectors
            .iter()
            .flat_map(|(_, c)| &c.stage_returns)
            .collect();
        let configuration: BTreeSet<String> = collectors
            .iter()
            .flat_map(|(_, c)| &c.stage_parameters)
            .filter(|name| !stage_returns.contains(name))
            .cloned()
            .collect();
        let mut pre_check = Self::default();
        for (name, collector) in collectors {
            let fielded = collector
                .definitions
                .iter()
                .filter(|(type_name, (has_fields, _))| {
                    *has_fields
                        && reached.contains(*type_name)
                        && !configuration.contains(*type_name)
                })
                .map(|(type_name, _)| type_name.clone())
                .collect();
            pre_check.types.insert(name, fielded);
            pre_check.functions.insert(name, collector.functions);
            pre_check.methods.insert(name, collector.methods);
        }
        Ok(pre_check)
    }

    /// The pre-check crate a path rooted at `first` belongs to.
    #[qsl_attrs::string_edge]
    fn crate_named(first: &str) -> Option<&'static str> {
        PRE_CHECK_CRATES
            .iter()
            .map(|(_, name)| *name)
            .find(|name| *name == first)
    }

    /// Whether the resolved type path `segments` is a pre-check
    /// representation.
    #[qsl_attrs::string_edge]
    fn is_representation(&self, segments: &[String]) -> bool {
        let (Some(first), Some(last)) = (segments.first(), segments.last()) else {
            return false;
        };
        last == S3_CONSTRUCTOR.receiver
            || first == CONTRACT_MODEL
            || Self::crate_named(first)
                .is_some_and(|name| self.types.get(name).is_some_and(|t| t.contains(last)))
    }

    /// Whether the resolved value path `segments` names a pre-check stage
    /// function.
    #[qsl_attrs::string_edge]
    fn is_stage_function(&self, segments: &[String]) -> bool {
        let [.., owner, last] = segments else {
            return false;
        };
        if owner == S3_CONSTRUCTOR.receiver && last == S3_CONSTRUCTOR.name {
            return true;
        }
        let Some(name) = segments.first().and_then(|first| Self::crate_named(first)) else {
            return false;
        };
        self.functions.get(name).is_some_and(|f| f.contains(last))
            || self
                .methods
                .get(name)
                .is_some_and(|m| m.contains(&(owner.clone(), last.clone())))
    }
}

/// One stage-crate file's `use` table.
#[derive(Debug, Default)]
struct UseTable {
    /// Local name -> every path it is bound to.
    bindings: BTreeMap<String, Vec<Vec<String>>>,
    /// Every glob import's prefix.
    globs: Vec<Vec<String>>,
}

impl UseTable {
    #[qsl_attrs::string_edge]
    fn add(&mut self, tree: &syn::UseTree, prefix: &mut Vec<String>) {
        match tree {
            syn::UseTree::Path(path) => {
                prefix.push(path.ident.to_string());
                self.add(&path.tree, prefix);
                prefix.pop();
            }
            syn::UseTree::Name(name) => {
                let ident = name.ident.to_string();
                if ident == "self" {
                    if let Some(last) = prefix.last() {
                        self.bind(last.clone(), prefix.clone());
                    }
                } else {
                    let mut path = prefix.clone();
                    path.push(ident.clone());
                    self.bind(ident, path);
                }
            }
            syn::UseTree::Rename(rename) => {
                let mut path = prefix.clone();
                let ident = rename.ident.to_string();
                if ident != "self" {
                    path.push(ident);
                }
                self.bind(rename.rename.to_string(), path);
            }
            syn::UseTree::Glob(_) => self.globs.push(prefix.clone()),
            syn::UseTree::Group(group) => {
                for item in &group.items {
                    self.add(item, prefix);
                }
            }
        }
    }

    fn bind(&mut self, name: String, path: Vec<String>) {
        self.bindings.entry(name).or_default().push(path);
    }
}

impl<'ast> Visit<'ast> for UseTable {
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

    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        if !has_cfg_test(&node.attrs) {
            self.add(&node.tree, &mut Vec::new());
        }
    }
}

/// The stage crates' `use` tables and `type` aliases.
struct Resolver<'a> {
    pre_check: &'a PreCheck,
    /// Per file, by the index [`scan`] gives it.
    uses: Vec<UseTable>,
    /// Alias name -> every (file index, target) it is declared with.
    aliases: BTreeMap<String, Vec<(usize, &'a syn::Type)>>,
}

/// A name that roots a path in the current crate.
#[qsl_attrs::string_edge]
fn is_local_root(first: &str) -> bool {
    matches!(first, "crate" | "self" | "super" | "Self")
}

impl Resolver<'_> {
    /// `segments` (written in file `file`) with its first segment expanded
    /// through the file's `use` table, or through its globs when it is a
    /// lone name; unchanged when nothing binds it.
    fn expand_uses(&self, file: usize, segments: &[String]) -> Vec<Vec<String>> {
        let Some((first, rest)) = segments.split_first() else {
            return Vec::new();
        };
        let table = &self.uses[file];
        if let Some(bound) = table.bindings.get(first) {
            return bound
                .iter()
                .map(|path| path.iter().chain(rest).cloned().collect())
                .collect();
        }
        let mut expanded = vec![segments.to_vec()];
        if rest.is_empty() {
            expanded.extend(table.globs.iter().map(|glob| {
                glob.iter()
                    .cloned()
                    .chain(std::iter::once(first.clone()))
                    .collect()
            }));
        }
        expanded
    }

    /// The `type` alias a resolved path names, if it is local (rooted in
    /// the current crate or a lone name) and its last segment is one.
    fn local_alias<'s>(&'s self, segments: &[String]) -> Option<&'s [(usize, &'s syn::Type)]> {
        let first = segments.first()?;
        if segments.len() > 1 && !is_local_root(first) {
            return None;
        }
        self.aliases.get(segments.last()?).map(Vec::as_slice)
    }

    /// The pre-check representations the type path `path` (in file `file`)
    /// names, seen through `use` items and `type` aliases.
    fn representations(
        &self,
        file: usize,
        path: &syn::Path,
        seen: &mut BTreeSet<String>,
    ) -> BTreeSet<String> {
        let mut found = BTreeSet::new();
        for candidate in self.expand_uses(file, &path_segments(path)) {
            if self.pre_check.is_representation(&candidate) {
                found.insert(candidate.join("::"));
            } else if let Some(targets) = self.local_alias(&candidate) {
                let Some(alias) = candidate.last() else {
                    continue;
                };
                if !seen.insert(alias.clone()) {
                    continue;
                }
                for (alias_file, ty) in targets {
                    let mut paths = PathCollector::default();
                    paths.visit_type(ty);
                    for inner in paths.0 {
                        found.extend(self.representations(*alias_file, inner, seen));
                    }
                }
            }
        }
        found
    }

    /// The value path `path` (in file `file`) resolved through `use` items
    /// and, at its first segment, `type` aliases of a plain path type.
    fn value_paths(
        &self,
        file: usize,
        segments: &[String],
        seen: &mut BTreeSet<String>,
    ) -> Vec<Vec<String>> {
        let mut resolved = Vec::new();
        for candidate in self.expand_uses(file, segments) {
            if candidate.len() > 1 {
                if let Some(targets) = self.aliases.get(&candidate[0]) {
                    if seen.insert(candidate[0].clone()) {
                        for (alias_file, ty) in targets {
                            if let syn::Type::Path(type_path) = ty {
                                let mut spliced = path_segments(&type_path.path);
                                spliced.extend(candidate[1..].iter().cloned());
                                resolved.extend(self.value_paths(*alias_file, &spliced, seen));
                            }
                        }
                    }
                }
            }
            resolved.push(candidate);
        }
        resolved
    }
}

/// Every `syn::Path` a node holds, outermost first.
#[derive(Default)]
struct PathCollector<'ast>(Vec<&'ast syn::Path>);

impl<'ast> Visit<'ast> for PathCollector<'ast> {
    fn visit_path(&mut self, node: &'ast syn::Path) {
        self.0.push(node);
        syn::visit::visit_path(self, node);
    }
}

/// The per-file walk that applies CK-3 and CK-4.
struct StageScanner<'r, 'a> {
    resolver: &'r Resolver<'a>,
    file_index: usize,
    file: &'r str,
    findings: &'r mut BTreeSet<Finding>,
    /// The inherent `impl` block's `Self` type, if inside one.
    inherent_self: Option<String>,
    /// Whether inside a trait `impl` (whose methods are not entries).
    in_trait_impl: bool,
    /// The innermost enclosing function's display name.
    function: Option<String>,
}

impl StageScanner<'_, '_> {
    fn signature(&mut self, name: &str, sig: &syn::Signature) {
        let mut paths = PathCollector::default();
        paths.visit_generics(&sig.generics);
        for input in &sig.inputs {
            paths.visit_fn_arg(input);
        }
        for path in paths.0 {
            for named in self
                .resolver
                .representations(self.file_index, path, &mut BTreeSet::new())
            {
                self.findings.insert(Finding {
                    file: self.file.to_owned(),
                    line: sig.ident.span().start().line,
                    rule: Rule::Signature,
                    function: name.to_owned(),
                    named,
                });
            }
        }
    }

    fn reconstruction(&mut self, path: &syn::Path) {
        let Some(function) = &self.function else {
            return;
        };
        for resolved in
            self.resolver
                .value_paths(self.file_index, &path_segments(path), &mut BTreeSet::new())
        {
            if self.resolver.pre_check.is_stage_function(&resolved) {
                self.findings.insert(Finding {
                    file: self.file.to_owned(),
                    line: path.span().start().line,
                    rule: Rule::Reconstruction,
                    function: function.clone(),
                    named: resolved.join("::"),
                });
            }
        }
    }

    fn within(&mut self, name: String, visit: impl FnOnce(&mut Self)) {
        let previous = self.function.replace(name);
        visit(self);
        self.function = previous;
    }
}

impl<'ast> Visit<'ast> for StageScanner<'_, '_> {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if !has_cfg_test(&node.attrs) {
            syn::visit::visit_item_mod(self, node);
        }
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        let self_name = crate::impl_self_name(&node.self_ty);
        let previous = (
            self.inherent_self.replace(self_name),
            std::mem::replace(&mut self.in_trait_impl, node.trait_.is_some()),
        );
        syn::visit::visit_item_impl(self, node);
        (self.inherent_self, self.in_trait_impl) = previous;
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        let previous = (
            self.inherent_self.replace(node.ident.to_string()),
            std::mem::replace(&mut self.in_trait_impl, true),
        );
        syn::visit::visit_item_trait(self, node);
        (self.inherent_self, self.in_trait_impl) = previous;
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        let name = node.sig.ident.to_string();
        if matches!(node.vis, syn::Visibility::Public(_)) {
            self.signature(&name, &node.sig);
        }
        let outer = (
            self.inherent_self.take(),
            std::mem::take(&mut self.in_trait_impl),
        );
        self.within(name, |this| syn::visit::visit_item_fn(this, node));
        (self.inherent_self, self.in_trait_impl) = outer;
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        let owner = self.inherent_self.clone().unwrap_or_default();
        let name = format!("{owner}::{}", node.sig.ident);
        if !self.in_trait_impl && matches!(node.vis, syn::Visibility::Public(_)) {
            self.signature(&name, &node.sig);
        }
        let outer = (
            self.inherent_self.take(),
            std::mem::take(&mut self.in_trait_impl),
        );
        self.within(name, |this| syn::visit::visit_impl_item_fn(this, node));
        (self.inherent_self, self.in_trait_impl) = outer;
    }

    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        if has_cfg_test(&node.attrs) {
            return;
        }
        let owner = self.inherent_self.clone().unwrap_or_default();
        let name = format!("{owner}::{}", node.sig.ident);
        self.within(name, |this| syn::visit::visit_trait_item_fn(this, node));
    }

    fn visit_path(&mut self, node: &'ast syn::Path) {
        self.reconstruction(node);
        syn::visit::visit_path(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        let parser = Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated;
        if let Ok(expressions) = node.parse_body_with(parser) {
            for expression in &expressions {
                self.visit_expr(expression);
            }
        }
        syn::visit::visit_macro(self, node);
    }
}

/// Whether `file` is in `qsl-replay`'s `spine` module.
fn in_spine(file: &str) -> bool {
    file == SPINE[0] || file.starts_with(SPINE[1])
}

/// Every finding of CK-3 and CK-4 over the shipped code under `root`.
pub fn scan(root: &Path) -> Result<Vec<Finding>> {
    let pre_check = PreCheck::derive(root)?;
    let files: Vec<(String, syn::File)> = shipped_files(root, &STAGE_CRATES)?
        .into_iter()
        .filter(|(file, _)| !in_spine(file))
        .collect();
    let mut uses = Vec::with_capacity(files.len());
    let mut aliases: BTreeMap<String, Vec<(usize, &syn::Type)>> = BTreeMap::new();
    for (index, (_, ast)) in files.iter().enumerate() {
        let mut table = UseTable::default();
        table.visit_file(ast);
        uses.push(table);
        let mut collector = AliasCollector::default();
        collector.visit_file(ast);
        for (name, ty) in collector.0 {
            aliases.entry(name).or_default().push((index, ty));
        }
    }
    let resolver = Resolver {
        pre_check: &pre_check,
        uses,
        aliases,
    };
    let mut findings = BTreeSet::new();
    for (index, (file, ast)) in files.iter().enumerate() {
        StageScanner {
            resolver: &resolver,
            file_index: index,
            file,
            findings: &mut findings,
            inherent_self: None,
            in_trait_impl: false,
            function: None,
        }
        .visit_file(ast);
    }
    Ok(findings.into_iter().collect())
}

/// Every shipped module-level `type` alias in a file.
#[derive(Default)]
struct AliasCollector<'ast>(Vec<(String, &'ast syn::Type)>);

impl<'ast> Visit<'ast> for AliasCollector<'ast> {
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

    fn visit_item_type(&mut self, node: &'ast syn::ItemType) {
        if !has_cfg_test(&node.attrs) {
            self.0.push((node.ident.to_string(), &node.ty));
        }
    }
}

/// `cargo xtask checked-input`: every finding, or a one-line summary.
pub fn run(root: &Path) -> Result<String> {
    let findings = scan(root)?;
    if findings.is_empty() {
        return Ok(
            "checked-input: no stage entry takes, and no stage crate rebuilds, a pre-check \
             representation.\n"
                .to_owned(),
        );
    }
    Err(Error::CheckedInputFound {
        summary: findings
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n"),
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use ix_trace_rs::trace;

    use super::*;

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask has a parent directory")
            .to_path_buf()
    }

    fn copy_tree(from: &Path, to: &Path) {
        fs::create_dir_all(to).expect("create directory");
        for entry in fs::read_dir(from).expect("read directory") {
            let entry = entry.expect("directory entry");
            let target = to.join(entry.file_name());
            if entry.path().is_dir() {
                copy_tree(&entry.path(), &target);
            } else {
                fs::copy(entry.path(), &target).expect("copy file");
            }
        }
    }

    /// A copy of the scanned trees with `plants` written into it.
    fn planted_copy(plants: &[(&str, &str)]) -> tempfile::TempDir {
        let copy = tempfile::tempdir().expect("temporary directory");
        let root = workspace_root();
        let dirs = PRE_CHECK_CRATES
            .iter()
            .map(|(dir, _)| *dir)
            .chain(STAGE_CRATES);
        for dir in dirs {
            copy_tree(&root.join(dir), &copy.path().join(dir));
        }
        for (file, source) in plants {
            let path = copy.path().join(file);
            fs::create_dir_all(path.parent().expect("planted file has a parent"))
                .expect("create directory");
            fs::write(path, source).expect("write plant");
        }
        copy
    }

    fn scan_planted(plants: &[(&str, &str)]) -> Vec<Finding> {
        scan(planted_copy(plants).path()).expect("scan runs")
    }

    fn finding(file: &str, line: usize, rule: Rule, function: &str, named: &str) -> Finding {
        Finding {
            file: file.to_owned(),
            line,
            rule,
            function: function.to_owned(),
            named: named.to_owned(),
        }
    }

    const EVAL_PLANT: &str = "qsl-eval/src/plant.rs";
    const ROUTE_PLANT: &str = "qsl-route/src/plant.rs";
    const REPLAY_PLANT: &str = "qsl-replay/src/plant.rs";

    /// TC-745 step 1: each planted signature gives one `signature` finding
    /// naming the planted file and line, the entry and the type.
    #[test]
    #[trace("TC-745", "FR-270-AC-1")]
    fn tc_745_planted_signatures_fail_the_gate() {
        let cases = [
            (
                EVAL_PLANT,
                "pub fn plant(e: &qsl_forms::Expression) {}\n",
                finding(EVAL_PLANT, 1, Rule::Signature, "plant", "qsl_forms::Expression"),
            ),
            (
                EVAL_PLANT,
                "use qsl_forms::Expression as Expr;\npub fn plant(e: &Expr) {}\n",
                finding(EVAL_PLANT, 2, Rule::Signature, "plant", "qsl_forms::Expression"),
            ),
            (
                EVAL_PLANT,
                "type PlantedNode = qsl_cst::CstNode;\npub fn plant(n: &PlantedNode) {}\n",
                finding(EVAL_PLANT, 2, Rule::Signature, "plant", "qsl_cst::CstNode"),
            ),
            (
                ROUTE_PLANT,
                "use qsl_semantics::check::PackageDeclarations;\npub struct Planted;\n\
                 impl Planted {\n    pub fn plant(&self, d: Option<Vec<&PackageDeclarations>>) {}\n}\n",
                finding(
                    ROUTE_PLANT,
                    4,
                    Rule::Signature,
                    "Planted::plant",
                    "qsl_semantics::check::PackageDeclarations",
                ),
            ),
            (
                REPLAY_PLANT,
                "pub fn plant(p: &quire_contract_model::ContractPackage) {}\n",
                finding(
                    REPLAY_PLANT,
                    1,
                    Rule::Signature,
                    "plant",
                    "quire_contract_model::ContractPackage",
                ),
            ),
        ];
        for (file, source, expected) in cases {
            assert_eq!(scan_planted(&[(file, source)]), vec![expected], "{source}");
        }
    }

    /// TC-745 step 2: a stage-crate function that calls a pre-check stage
    /// function is a `reconstruction` finding whatever its signature; the
    /// same call in `spine` is not.
    #[test]
    #[trace("TC-745", "FR-270-AC-2")]
    fn tc_745_planted_reconstruction_fails_the_gate() {
        let parse = "fn plant(graph: &qsl_semantics::check::CheckedGraph) {\n    \
                     let _ = qsl_cst::parse_source(source, limits);\n}\n";
        assert_eq!(
            scan_planted(&[(EVAL_PLANT, parse)]),
            vec![finding(
                EVAL_PLANT,
                2,
                Rule::Reconstruction,
                "plant",
                "qsl_cst::parse_source"
            )]
        );
        let check = "fn plant(limits: CheckingLimits) {\n    \
                     let _ = PackageDeclarations::check(declarations, limits);\n}\n";
        assert_eq!(
            scan_planted(&[(ROUTE_PLANT, check)]),
            vec![finding(
                ROUTE_PLANT,
                2,
                Rule::Reconstruction,
                "plant",
                "PackageDeclarations::check"
            )]
        );
        assert_eq!(
            scan_planted(&[("qsl-replay/src/spine/plant.rs", parse)]),
            vec![]
        );
    }

    /// TC-745 step 3: closed vocabulary, configuration and test code give no
    /// finding; two planted signatures give two.
    #[test]
    #[trace("TC-745", "FR-270-AC-3")]
    fn tc_745_no_false_findings_and_every_finding_reported() {
        let look_alikes = [
            (
                EVAL_PLANT,
                "use qsl_forms::StateClauseKind;\npub fn plant(k: StateClauseKind) {}\n",
            ),
            (
                REPLAY_PLANT,
                "fn plant() -> qsl_cst::Limits {\n    \
                 qsl_cst::Limits { source_bytes: 1, ..qsl_cst::Limits::default() }\n}\n",
            ),
            (
                EVAL_PLANT,
                "#[cfg(test)]\nmod t {\n    pub fn t(e: qsl_forms::Expression) {}\n}\n",
            ),
        ];
        for (file, source) in look_alikes {
            assert_eq!(scan_planted(&[(file, source)]), vec![], "{source}");
        }
        let second = "qsl-eval/src/plant_two.rs";
        assert_eq!(
            scan_planted(&[
                (EVAL_PLANT, "pub fn plant(e: &qsl_forms::Expression) {}\n"),
                (
                    second,
                    "use qsl_forms::Expression as Expr;\npub fn plant(e: &Expr) {}\n"
                ),
            ]),
            vec![
                finding(
                    EVAL_PLANT,
                    1,
                    Rule::Signature,
                    "plant",
                    "qsl_forms::Expression"
                ),
                finding(second, 2, Rule::Signature, "plant", "qsl_forms::Expression"),
            ]
        );
    }

    /// TC-745 step 4: the workspace as it is passes, and the gate's command
    /// fails with a finding exit code once a signature is planted in
    /// `qsl-eval`.
    #[test]
    #[trace("TC-745", "FR-270-AC-4")]
    fn tc_745_the_workspace_passes_and_a_plant_fails_the_command() {
        assert_eq!(scan(&workspace_root()).expect("scan runs"), vec![]);
        let copy = planted_copy(&[(EVAL_PLANT, "pub fn plant(e: &qsl_forms::Expression) {}\n")]);
        let error = run(copy.path()).expect_err("the planted signature fails the gate");
        assert_eq!(error.code(), crate::error::Code::CheckedInput);
        assert_eq!(error.exit_code(), 1);
        assert!(error
            .to_string()
            .contains("qsl-eval/src/plant.rs:1: signature"));
    }
}
