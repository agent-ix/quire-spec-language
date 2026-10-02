// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-271 and FR-272 (ADR-032 DT-1 to DT-6): `cargo xtask canonical-types`,
//! the gate that keeps one definition per canonical public type.
//!
//! **The canonical set (FR-271).** A canonical type carries, in its outer doc
//! comment, a line that reads exactly `quire:canonical` once the `///` marker
//! and surrounding whitespace are taken off. The set is exactly the tagged
//! definitions found in the scanned crates; no list of names is kept. A tag on
//! anything but a `pub` `struct`, `enum`, `union`, `trait` or `type` alias,
//! and a second tagged definition of one identifier, are `tag` findings.
//!
//! **Scanned set (DT-4).** From `cargo metadata`: the source root (the
//! directory of the target's `src_path`) of every library, binary and
//! proc-macro target of every workspace member, and the same for every
//! resolved package that FR-059's `graph::classify` assigns to an ecosystem
//! repository. Test, bench, example and build-script targets are test or
//! build code, and test code under a root is excluded as `typestate_scan`
//! excludes it.
//!
//! **Rules.**
//! - `identifier` (DT-2): inside the workspace, a module-level `struct`,
//!   `enum`, `union`, `trait` or `type` alias with a canonical type's
//!   identifier, other than its tagged definition, whatever its visibility or
//!   shape, when the canonical definition is in a workspace member.
//! - `re-export` (DT-2): in a workspace member other than the owner crate,
//!   for a canonical type a workspace member owns, a `pub use` of its
//!   identifier rooted at another scanned crate, or that renames it with
//!   `as`, and a `pub use v::*` from another workspace crate that defines or
//!   re-exports the identifier. A path rooted at anything but a scanned
//!   crate is the crate's own, and its item is a namesake.
//! - `copy` (DT-3): across a repository boundary (the workspace and an
//!   ecosystem repository, or two ecosystem repositories), a definition with
//!   the canonical identifier, the same item kind and the same member names
//!   in the same order (field names, positions of tuple fields, variant names,
//!   trait method names). Documentation, attributes, visibility and member
//!   types are ignored.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

use syn::spanned::Spanned;
use syn::visit::Visit;

use crate::error::{Error, Result};
use crate::typestate_scan::shipped_files;

/// FR-059's ecosystem classification, the one definition `arch-lint` uses.
/// Only `classify` is called here; the rest of the module is `arch-lint`'s.
#[allow(dead_code, reason = "xtask calls only `classify` and `Repo`")]
#[path = "../../tools/arch-lint/graph.rs"]
mod graph;

/// The doc line that marks a canonical type.
const TAG: &str = "quire:canonical";

/// Target kinds whose sources are shipped code.
const SHIPPED_KINDS: [&str; 7] = [
    "lib",
    "rlib",
    "dylib",
    "cdylib",
    "staticlib",
    "proc-macro",
    "bin",
];

/// Which repository a scanned package belongs to.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Repository {
    /// A member of the workspace the gate runs in.
    Workspace,
    /// An ecosystem dependency, by the repository `graph::classify` gives.
    Ecosystem(&'static str),
}

/// One scanned package.
#[derive(Clone, Debug)]
pub struct Package {
    /// The package name.
    pub name: String,
    /// The crate name a path names it by (its library target's name).
    pub crate_name: String,
    /// Which repository it belongs to.
    pub repository: Repository,
    /// The source root of each shipped target.
    pub roots: Vec<PathBuf>,
}

/// The scanned packages, read from `cargo metadata --format-version 1`
/// output: the workspace members and the ecosystem dependencies.
#[qsl_attrs::string_edge]
pub fn packages(metadata: &serde_json::Value) -> Result<Vec<Package>> {
    let malformed = |what: &'static str| Error::CanonicalTypesMetadata { what };
    let members: BTreeSet<&str> = metadata["workspace_members"]
        .as_array()
        .ok_or(malformed("workspace_members"))?
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect();
    let mut packages = Vec::new();
    for package in metadata["packages"]
        .as_array()
        .ok_or(malformed("packages"))?
    {
        let name = package["name"].as_str().ok_or(malformed("package name"))?;
        let id = package["id"].as_str().ok_or(malformed("package id"))?;
        let repository = if members.contains(id) {
            Repository::Workspace
        } else {
            match graph::classify(name, package["source"].as_str()) {
                Some(repo) => Repository::Ecosystem(repo.as_str()),
                None => continue,
            }
        };
        let mut roots = Vec::new();
        let mut crate_name = name.replace('-', "_");
        for target in package["targets"]
            .as_array()
            .ok_or(malformed("package targets"))?
        {
            let kinds: Vec<&str> = target["kind"]
                .as_array()
                .ok_or(malformed("target kind"))?
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect();
            if !kinds.iter().any(|kind| SHIPPED_KINDS.contains(kind)) {
                continue;
            }
            if kinds.iter().any(|kind| *kind != "bin") {
                if let Some(target_name) = target["name"].as_str() {
                    crate_name = target_name.replace('-', "_");
                }
            }
            let src_path = target["src_path"]
                .as_str()
                .ok_or(malformed("target src_path"))?;
            if let Some(root) = Path::new(src_path).parent() {
                roots.push(root.to_path_buf());
            }
        }
        roots.sort();
        roots.dedup();
        // A root under another root of the same package is scanned with it.
        let nested: Vec<PathBuf> = roots
            .iter()
            .filter(|root| {
                roots
                    .iter()
                    .any(|other| other != *root && root.starts_with(other))
            })
            .cloned()
            .collect();
        roots.retain(|root| !nested.contains(root));
        packages.push(Package {
            name: name.to_owned(),
            crate_name,
            repository,
            roots,
        });
    }
    Ok(packages)
}

/// `cargo metadata` for the workspace at `workspace_root`.
fn cargo_metadata(workspace_root: &Path) -> Result<serde_json::Value> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .args(["metadata", "--format-version", "1"])
        .current_dir(workspace_root)
        .output()
        .map_err(|source| Error::CanonicalTypesSpawn { source })?;
    if !output.status.success() {
        return Err(Error::CanonicalTypesCargoMetadata {
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|source| Error::CanonicalTypesMetadataJson { source })
}

/// The kind of a type definition.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Kind {
    /// A `struct`.
    Struct,
    /// An `enum`.
    Enum,
    /// A `union`.
    Union,
    /// A `trait`.
    Trait,
    /// A `type` alias.
    Alias,
}

/// A source location.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Site {
    /// The file: relative to the workspace root when under it, else absolute.
    pub file: String,
    /// The 1-based line.
    pub line: usize,
}

impl fmt::Display for Site {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.file, self.line)
    }
}

/// One module-level type definition.
#[derive(Clone, Debug)]
struct Definition {
    ident: String,
    kind: Kind,
    members: Vec<String>,
    public: bool,
    tagged: bool,
    site: Site,
    package: usize,
}

/// One `pub use` leaf.
#[derive(Clone, Debug)]
struct Reexport {
    /// The original identifier; empty for a glob.
    ident: String,
    /// Whether it is a glob (`pub use v::*`).
    glob: bool,
    /// The path's first segment.
    root: String,
    renamed: bool,
    site: Site,
    package: usize,
}

/// Which rule a finding breaks.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Rule {
    /// FR-271: a misplaced or duplicated tag.
    Tag,
    /// DT-2: a second definition inside the workspace.
    Identifier,
    /// DT-2: a re-export not through the owner crate.
    Reexport,
    /// DT-3: a same-shaped copy across a repository boundary.
    Copy,
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Tag => "tag",
            Self::Identifier => "identifier",
            Self::Reexport => "re-export",
            Self::Copy => "copy",
        })
    }
}

/// One violation.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Finding {
    /// The rule broken.
    pub rule: Rule,
    /// The canonical identifier, or for a misplaced tag the tagged item.
    pub ident: String,
    /// The canonical definition (the first tagged one for a duplicate tag);
    /// `None` for a misplaced tag.
    pub canonical: Option<Site>,
    /// The second item.
    pub other: Site,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.canonical {
            Some(canonical) => write!(
                f,
                "{}: {}: `{}` defined at {canonical}, again at {}",
                self.other, self.rule, self.ident, self.other
            ),
            None => write!(
                f,
                "{}: {}: `{TAG}` on `{}`, which is not a pub struct, enum, union, trait or type alias",
                self.other, self.rule, self.ident
            ),
        }
    }
}

/// Whether `attrs` hold the tag as an outer doc line.
#[qsl_attrs::string_edge]
fn tagged(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !matches!(attr.style, syn::AttrStyle::Outer) || !attr.path().is_ident("doc") {
            return false;
        }
        let syn::Meta::NameValue(meta) = &attr.meta else {
            return false;
        };
        let syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(text),
            ..
        }) = &meta.value
        else {
            return false;
        };
        text.value().lines().any(|line| line.trim() == TAG)
    })
}

fn field_names(fields: &syn::Fields) -> Vec<String> {
    fields
        .iter()
        .enumerate()
        .map(|(position, field)| {
            field
                .ident
                .as_ref()
                .map_or_else(|| position.to_string(), ToString::to_string)
        })
        .collect()
}

/// One file's definitions, re-exports and misplaced tags.
struct FileScan<'a> {
    file: &'a str,
    package: usize,
    definitions: Vec<Definition>,
    reexports: Vec<Reexport>,
    misplaced: Vec<(String, Site)>,
}

impl FileScan<'_> {
    fn site(&self, node: &impl Spanned) -> Site {
        Site {
            file: self.file.to_owned(),
            line: node.span().start().line,
        }
    }

    fn define(
        &mut self,
        ident: &syn::Ident,
        kind: Kind,
        members: Vec<String>,
        vis: &syn::Visibility,
        attrs: &[syn::Attribute],
    ) {
        let public = matches!(vis, syn::Visibility::Public(_));
        let is_tagged = tagged(attrs);
        if is_tagged && !public {
            self.misplaced.push((ident.to_string(), self.site(ident)));
        }
        self.definitions.push(Definition {
            ident: ident.to_string(),
            kind,
            members,
            public,
            tagged: is_tagged,
            site: self.site(ident),
            package: self.package,
        });
    }

    /// A tag on an item that can never carry one.
    fn misplaced_tag(&mut self, attrs: &[syn::Attribute], name: String, node: &impl Spanned) {
        if tagged(attrs) {
            let site = self.site(node);
            self.misplaced.push((name, site));
        }
    }

    #[qsl_attrs::string_edge]
    fn use_tree(&mut self, tree: &syn::UseTree, prefix: &mut Vec<String>) {
        match tree {
            syn::UseTree::Path(path) => {
                prefix.push(path.ident.to_string());
                self.use_tree(&path.tree, prefix);
                prefix.pop();
            }
            syn::UseTree::Name(name) => {
                let ident = name.ident.to_string();
                if ident != "self" {
                    self.reexport(ident, prefix, false, &name.ident);
                }
            }
            syn::UseTree::Rename(rename) => {
                let ident = rename.ident.to_string();
                if ident != "self" {
                    self.reexport(ident, prefix, true, &rename.ident);
                }
            }
            syn::UseTree::Glob(glob) => {
                let site = self.site(glob);
                self.reexports.push(Reexport {
                    ident: String::new(),
                    glob: true,
                    root: prefix.first().cloned().unwrap_or_default(),
                    renamed: false,
                    site,
                    package: self.package,
                });
            }
            syn::UseTree::Group(group) => {
                for item in &group.items {
                    self.use_tree(item, prefix);
                }
            }
        }
    }

    fn reexport(&mut self, ident: String, prefix: &[String], renamed: bool, at: &syn::Ident) {
        let root = prefix.first().cloned().unwrap_or_default();
        let site = self.site(at);
        self.reexports.push(Reexport {
            ident,
            glob: false,
            root,
            renamed,
            site,
            package: self.package,
        });
    }
}

impl<'ast> Visit<'ast> for FileScan<'_> {
    fn visit_item(&mut self, node: &'ast syn::Item) {
        let skip = match node {
            syn::Item::Mod(item) => crate::definition_scan::has_cfg_test(&item.attrs),
            syn::Item::Impl(item) => crate::definition_scan::has_cfg_test(&item.attrs),
            syn::Item::Fn(item) => crate::definition_scan::has_cfg_test(&item.attrs),
            _ => false,
        };
        if !skip {
            syn::visit::visit_item(self, node);
        }
    }

    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        if crate::definition_scan::has_cfg_test(&node.attrs) {
            return;
        }
        self.define(
            &node.ident,
            Kind::Struct,
            field_names(&node.fields),
            &node.vis,
            &node.attrs,
        );
        syn::visit::visit_item_struct(self, node);
    }

    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        if crate::definition_scan::has_cfg_test(&node.attrs) {
            return;
        }
        let members = node.variants.iter().map(|v| v.ident.to_string()).collect();
        self.define(&node.ident, Kind::Enum, members, &node.vis, &node.attrs);
        syn::visit::visit_item_enum(self, node);
    }

    fn visit_item_union(&mut self, node: &'ast syn::ItemUnion) {
        if crate::definition_scan::has_cfg_test(&node.attrs) {
            return;
        }
        let members = node
            .fields
            .named
            .iter()
            .filter_map(|field| field.ident.as_ref().map(ToString::to_string))
            .collect();
        self.define(&node.ident, Kind::Union, members, &node.vis, &node.attrs);
        syn::visit::visit_item_union(self, node);
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        if crate::definition_scan::has_cfg_test(&node.attrs) {
            return;
        }
        let members = node
            .items
            .iter()
            .filter_map(|item| match item {
                syn::TraitItem::Fn(method) => Some(method.sig.ident.to_string()),
                _ => None,
            })
            .collect();
        self.define(&node.ident, Kind::Trait, members, &node.vis, &node.attrs);
        syn::visit::visit_item_trait(self, node);
    }

    fn visit_item_type(&mut self, node: &'ast syn::ItemType) {
        if crate::definition_scan::has_cfg_test(&node.attrs) {
            return;
        }
        self.define(&node.ident, Kind::Alias, Vec::new(), &node.vis, &node.attrs);
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.misplaced_tag(&node.attrs, node.sig.ident.to_string(), &node.sig.ident);
        // Items inside a body are not module-level: only tags are read there.
        let mut inner = FileScan {
            file: self.file,
            package: self.package,
            definitions: Vec::new(),
            reexports: Vec::new(),
            misplaced: Vec::new(),
        };
        syn::visit::visit_item_fn(&mut inner, node);
        self.misplaced.extend(inner.misplaced);
        self.misplaced.extend(
            inner
                .definitions
                .into_iter()
                .filter(|definition| definition.tagged && definition.public)
                .map(|definition| (definition.ident, definition.site)),
        );
    }

    fn visit_item_const(&mut self, node: &'ast syn::ItemConst) {
        self.misplaced_tag(&node.attrs, node.ident.to_string(), &node.ident);
        syn::visit::visit_item_const(self, node);
    }

    fn visit_item_static(&mut self, node: &'ast syn::ItemStatic) {
        self.misplaced_tag(&node.attrs, node.ident.to_string(), &node.ident);
        syn::visit::visit_item_static(self, node);
    }

    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        self.misplaced_tag(&node.attrs, node.ident.to_string(), &node.ident);
        syn::visit::visit_item_mod(self, node);
    }

    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        if crate::definition_scan::has_cfg_test(&node.attrs) {
            return;
        }
        if matches!(node.vis, syn::Visibility::Public(_)) {
            self.use_tree(&node.tree, &mut Vec::new());
        }
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        // Items in an `impl` are associated, not module-level: only tags are
        // read there.
        for item in &node.items {
            match item {
                syn::ImplItem::Fn(method) => {
                    self.misplaced_tag(
                        &method.attrs,
                        method.sig.ident.to_string(),
                        &method.sig.ident,
                    );
                }
                syn::ImplItem::Type(ty) => {
                    self.misplaced_tag(&ty.attrs, ty.ident.to_string(), &ty.ident);
                }
                syn::ImplItem::Const(constant) => {
                    self.misplaced_tag(
                        &constant.attrs,
                        constant.ident.to_string(),
                        &constant.ident,
                    );
                }
                _ => {}
            }
        }
    }

    fn visit_field(&mut self, node: &'ast syn::Field) {
        let name = node
            .ident
            .as_ref()
            .map_or_else(|| "<tuple field>".to_owned(), ToString::to_string);
        self.misplaced_tag(&node.attrs, name, node);
        syn::visit::visit_field(self, node);
    }

    fn visit_variant(&mut self, node: &'ast syn::Variant) {
        self.misplaced_tag(&node.attrs, node.ident.to_string(), &node.ident);
        syn::visit::visit_variant(self, node);
    }

    fn visit_trait_item(&mut self, node: &'ast syn::TraitItem) {
        match node {
            syn::TraitItem::Fn(method) => {
                self.misplaced_tag(
                    &method.attrs,
                    method.sig.ident.to_string(),
                    &method.sig.ident,
                );
            }
            syn::TraitItem::Type(ty) => {
                self.misplaced_tag(&ty.attrs, ty.ident.to_string(), &ty.ident);
            }
            syn::TraitItem::Const(constant) => {
                self.misplaced_tag(&constant.attrs, constant.ident.to_string(), &constant.ident);
            }
            _ => {}
        }
    }
}

/// Everything one run finds, with the packages it scanned.
#[derive(Debug)]
pub struct Report {
    /// The scanned packages.
    pub packages: Vec<Package>,
    /// The canonical set: each tagged identifier and its definition.
    pub canonical: BTreeMap<String, Site>,
    /// Every finding, sorted.
    pub findings: Vec<Finding>,
}

/// FR-271 and FR-272 over `packages`; files under `workspace_root` are named
/// relative to it.
pub fn scan(workspace_root: &Path, packages: Vec<Package>) -> Result<Report> {
    let mut definitions = Vec::new();
    let mut reexports = Vec::new();
    let mut misplaced = Vec::new();
    for (index, package) in packages.iter().enumerate() {
        for root in &package.roots {
            let relative = root.strip_prefix(workspace_root).unwrap_or(root);
            let dir = relative.to_string_lossy().into_owned();
            for (file, ast) in shipped_files(workspace_root, &[dir.as_str()])? {
                let mut scan = FileScan {
                    file: &file,
                    package: index,
                    definitions: Vec::new(),
                    reexports: Vec::new(),
                    misplaced: Vec::new(),
                };
                scan.visit_file(&ast);
                definitions.extend(scan.definitions);
                reexports.extend(scan.reexports);
                misplaced.extend(scan.misplaced);
            }
        }
    }
    let mut findings: BTreeSet<Finding> = misplaced
        .into_iter()
        .map(|(ident, other)| Finding {
            rule: Rule::Tag,
            ident,
            canonical: None,
            other,
        })
        .collect();

    // The canonical set: the first tagged public definition of each
    // identifier; each further one is a duplicate tag.
    let mut canonical: BTreeMap<&str, &Definition> = BTreeMap::new();
    for definition in definitions.iter().filter(|d| d.tagged && d.public) {
        match canonical.get(definition.ident.as_str()) {
            Some(first) => {
                findings.insert(Finding {
                    rule: Rule::Tag,
                    ident: definition.ident.clone(),
                    canonical: Some(first.site.clone()),
                    other: definition.site.clone(),
                });
            }
            None => {
                canonical.insert(&definition.ident, definition);
            }
        }
    }

    for definition in &definitions {
        let Some(owner) = canonical.get(definition.ident.as_str()) else {
            continue;
        };
        if definition.tagged {
            continue;
        }
        let home = packages[owner.package].repository;
        let here = packages[definition.package].repository;
        let rule = if home == Repository::Workspace && here == Repository::Workspace {
            Some(Rule::Identifier)
        } else if home != here
            && definition.kind == owner.kind
            && definition.members == owner.members
        {
            Some(Rule::Copy)
        } else {
            None
        };
        if let Some(rule) = rule {
            findings.insert(Finding {
                rule,
                ident: definition.ident.clone(),
                canonical: Some(owner.site.clone()),
                other: definition.site.clone(),
            });
        }
    }

    // A re-export's root is a crate only when it names a scanned package;
    // any other root (`crate`, `self`, `super`, a child module) is local, so
    // the item is the crate's own namesake, which the `identifier` rule
    // reports at its definition. DT-2 applies inside the owning repository
    // only: a canonical type owned by an ecosystem crate is reached through
    // that repository's facade, never reported here (DT-5).
    let crates: BTreeMap<&str, usize> = packages
        .iter()
        .enumerate()
        .map(|(index, package)| (package.crate_name.as_str(), index))
        .collect();
    let in_workspace = |package: usize| packages[package].repository == Repository::Workspace;
    for reexport in &reexports {
        let Some(&root) = crates.get(reexport.root.as_str()) else {
            continue;
        };
        if !in_workspace(reexport.package) {
            continue;
        }
        if reexport.glob {
            // `pub use v::*`: every canonical identifier `v` defines or
            // re-exports by name comes through it.
            for (ident, owner) in &canonical {
                if !in_workspace(owner.package)
                    || owner.package == reexport.package
                    || owner.package == root
                {
                    continue;
                }
                let carried = definitions
                    .iter()
                    .any(|d| d.package == root && d.ident == *ident)
                    || reexports
                        .iter()
                        .any(|r| r.package == root && !r.glob && r.ident == *ident);
                if carried {
                    findings.insert(Finding {
                        rule: Rule::Reexport,
                        ident: (*ident).to_owned(),
                        canonical: Some(owner.site.clone()),
                        other: reexport.site.clone(),
                    });
                }
            }
            continue;
        }
        let Some(owner) = canonical.get(reexport.ident.as_str()) else {
            continue;
        };
        if !in_workspace(owner.package) || reexport.package == owner.package {
            continue;
        }
        if reexport.renamed || root != owner.package {
            findings.insert(Finding {
                rule: Rule::Reexport,
                ident: reexport.ident.clone(),
                canonical: Some(owner.site.clone()),
                other: reexport.site.clone(),
            });
        }
    }

    let canonical = canonical
        .into_iter()
        .map(|(ident, definition)| (ident.to_owned(), definition.site.clone()))
        .collect();
    Ok(Report {
        packages,
        canonical,
        findings: findings.into_iter().collect(),
    })
}

/// `cargo xtask canonical-types [<workspace>]`: every finding over the
/// workspace at `workspace_root`, or a one-line summary.
pub fn run(workspace_root: &Path) -> Result<String> {
    let metadata = cargo_metadata(workspace_root)?;
    summarize(workspace_root, packages(&metadata)?)
}

/// [`scan`], as the command reports it: every finding as an error, or a
/// one-line summary.
pub fn summarize(workspace_root: &Path, packages: Vec<Package>) -> Result<String> {
    let report = scan(workspace_root, packages)?;
    if report.findings.is_empty() {
        return Ok(format!(
            "canonical-types: {} packages scanned; every canonical type has one definition.\n",
            report.packages.len()
        ));
    }
    Err(Error::CanonicalTypesFound {
        summary: report
            .findings
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n"),
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use ix_trace_rs::trace;
    use serde_json::json;

    use super::*;

    const IR_SOURCE: &str = "git+https://github.com/agent-ix/quire-contract-ir?branch=main#ea63488";
    const QSL_SOURCE: &str =
        "git+https://github.com/agent-ix/quire-spec-language?branch=main#e3fef8d";

    /// A fixture workspace: source files in a temporary directory and the
    /// `cargo metadata` output that describes them.
    struct Fixture {
        dir: tempfile::TempDir,
        members: Vec<String>,
        packages: Vec<serde_json::Value>,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                dir: tempfile::tempdir().expect("temporary directory"),
                members: Vec::new(),
                packages: Vec::new(),
            }
        }

        fn file(&mut self, path: &str, source: &str) -> &mut Self {
            let path = self.dir.path().join(path);
            fs::create_dir_all(path.parent().expect("a parent directory")).expect("create");
            fs::write(path, source).expect("write");
            self
        }

        fn package(&mut self, name: &str, source: Option<&str>, kind: &str, src_path: &str) {
            let id = format!("{name} 0.1.0");
            self.packages.push(json!({
                "name": name,
                "id": id,
                "source": source,
                "targets": [{
                    "name": name,
                    "kind": [kind],
                    "src_path": self.dir.path().join(src_path),
                }],
            }));
            if source.is_none() {
                self.members.push(id);
            }
        }

        /// A workspace member with a library target at `<name>/src/lib.rs`.
        fn member(&mut self, name: &str, lib: &str) -> &mut Self {
            let src_path = format!("{name}/src/lib.rs");
            self.file(&src_path, lib);
            self.package(name, None, "lib", &src_path);
            self
        }

        /// A resolved dependency with a library at `<name>/src/lib.rs`.
        fn dependency(&mut self, name: &str, source: &str, lib: &str) -> &mut Self {
            let src_path = format!("{name}/src/lib.rs");
            self.file(&src_path, lib);
            self.package(name, Some(source), "lib", &src_path);
            self
        }

        fn metadata(&self) -> serde_json::Value {
            json!({"workspace_members": self.members, "packages": self.packages})
        }

        fn packages(&self) -> Vec<Package> {
            packages(&self.metadata()).expect("metadata reads")
        }

        fn findings(&self) -> Vec<Finding> {
            scan(self.dir.path(), self.packages())
                .expect("scan runs")
                .findings
        }
    }

    fn site(file: &str, line: usize) -> Site {
        Site {
            file: file.to_owned(),
            line,
        }
    }

    fn finding(rule: Rule, ident: &str, canonical: Option<Site>, other: Site) -> Finding {
        Finding {
            rule,
            ident: ident.to_owned(),
            canonical,
            other,
        }
    }

    const TAGGED_METER: &str =
        "/// A meter.\n/// quire:canonical\n/// Counts work.\npub struct Meter {\n    pub used: u64,\n}\n";

    /// TC-746 steps 1 and 2: the tag line anywhere in the doc comment puts
    /// a type in the set; a removed, plain-comment or longer tag line does
    /// not.
    #[test]
    #[trace("TC-746", "FR-271-AC-1")]
    fn tc_746_the_tag_line_defines_the_canonical_set() {
        let mut fixture = Fixture::new();
        fixture
            .member("a", TAGGED_METER)
            .member("b", "struct Meter;\n");
        assert_eq!(
            fixture.findings(),
            vec![finding(
                Rule::Identifier,
                "Meter",
                Some(site("a/src/lib.rs", 4)),
                site("b/src/lib.rs", 1)
            )]
        );
        for untagged in [
            "/// A meter.\npub struct Meter {\n    pub used: u64,\n}\n",
            "// quire:canonical\npub struct Meter {\n    pub used: u64,\n}\n",
            "/// quire:canonical type\npub struct Meter {\n    pub used: u64,\n}\n",
        ] {
            let mut fixture = Fixture::new();
            fixture.member("a", untagged).member("b", "struct Meter;\n");
            assert_eq!(fixture.findings(), vec![], "{untagged}");
        }
    }

    /// TC-746 steps 3 and 4: a tag on a `pub fn`, a private struct or a
    /// field is a `tag` finding; two tagged definitions are one.
    #[test]
    #[trace("TC-746", "FR-271-AC-2")]
    fn tc_746_misplaced_and_duplicate_tags_are_findings() {
        for (source, item, line) in [
            ("/// quire:canonical\npub fn plant() {}\n", "plant", 2),
            ("/// quire:canonical\nstruct Hidden;\n", "Hidden", 2),
            (
                "pub struct Holder {\n    /// quire:canonical\n    pub field: u8,\n}\n",
                "field",
                2,
            ),
        ] {
            let mut fixture = Fixture::new();
            fixture.member("a", source);
            assert_eq!(
                fixture.findings(),
                vec![finding(Rule::Tag, item, None, site("a/src/lib.rs", line))],
                "{source}"
            );
        }
        let mut fixture = Fixture::new();
        fixture.member("a", TAGGED_METER).member("b", TAGGED_METER);
        assert_eq!(
            fixture.findings(),
            vec![finding(
                Rule::Tag,
                "Meter",
                Some(site("a/src/lib.rs", 4)),
                site("b/src/lib.rs", 4)
            )]
        );
    }

    /// A fixture whose crate K tags `pub enum Value` in a private module
    /// and re-exports it at its root, and tags `pub struct Outcome`.
    fn k_fixture() -> Fixture {
        let mut fixture = Fixture::new();
        fixture
            .member("k", "mod value;\npub use self::value::{Outcome, Value};\n")
            .file(
                "k/src/value.rs",
                "/// quire:canonical\npub enum Value {\n    A,\n    B,\n}\n\
                 /// quire:canonical\npub struct Outcome {\n    pub done: bool,\n}\n",
            );
        fixture
    }

    fn k_value() -> Option<Site> {
        Some(site("k/src/value.rs", 2))
    }

    /// TC-747 step 1: namesakes at any visibility and depth, re-exports
    /// through another crate or under another name, and a `[[bin]]`
    /// member's namesake are findings; an associated type, test code and a
    /// re-export through the owner are not.
    #[test]
    #[trace("TC-747", "FR-272-AC-1")]
    fn tc_747_identifier_and_reexport_rules() {
        let w = "w/src/lib.rs";
        for (source, expected) in [
            (
                "struct Value;\n",
                vec![finding(Rule::Identifier, "Value", k_value(), site(w, 1))],
            ),
            (
                "mod nested {\n    pub(crate) enum Value {\n        X,\n    }\n}\n",
                vec![finding(Rule::Identifier, "Value", k_value(), site(w, 2))],
            ),
            (
                "struct V;\nimpl<'de> Visitor<'de> for V {\n    type Value = T;\n}\n",
                vec![],
            ),
            ("#[cfg(test)]\nmod tests {\n    struct Value;\n}\n", vec![]),
            (
                "pub use v::Value;\n",
                vec![finding(Rule::Reexport, "Value", k_value(), site(w, 1))],
            ),
            (
                "pub use k::Value as Datum;\n",
                vec![finding(Rule::Reexport, "Value", k_value(), site(w, 1))],
            ),
            ("pub use k::Value;\n", vec![]),
            (
                "pub use v::*;\n",
                vec![finding(Rule::Reexport, "Value", k_value(), site(w, 1))],
            ),
        ] {
            let mut fixture = k_fixture();
            fixture
                .member("v", "pub use k::Value;\n")
                .member("w", source);
            assert_eq!(fixture.findings(), expected, "{source}");
        }
        // A crate re-exporting its own namesake from a child module: the
        // namesake is the finding, not the re-export.
        let mut fixture = k_fixture();
        fixture
            .member("w", "mod m;\npub use m::Value;\n")
            .file("w/src/m.rs", "pub struct Value;\n");
        assert_eq!(
            fixture.findings(),
            vec![finding(
                Rule::Identifier,
                "Value",
                k_value(),
                site("w/src/m.rs", 1)
            )]
        );
        let mut fixture = k_fixture();
        fixture.file("tool/main.rs", "struct Outcome;\nfn main() {}\n");
        fixture.package("tool", None, "bin", "tool/main.rs");
        assert_eq!(
            fixture.findings(),
            vec![finding(
                Rule::Identifier,
                "Outcome",
                Some(site("k/src/value.rs", 7)),
                site("tool/main.rs", 1)
            )]
        );
    }

    const TAGGED_OPERATOR: &str =
        "/// quire:canonical\npub enum ComparisonOperator {\n    Less,\n    LessOrEqual,\n    Equal,\n}\n";

    /// TC-747 step 2: a same-shaped copy in an ecosystem package is a
    /// `copy` finding whatever its docs, attributes, visibility and member
    /// types; a different name, order or kind is not, and a package outside
    /// the ecosystem is not scanned.
    #[test]
    #[trace("TC-747", "FR-272-AC-2")]
    fn tc_747_copy_rule_across_the_ecosystem() {
        let e = "quire-contract-model/src/lib.rs";
        let copy = |line| {
            vec![finding(
                Rule::Copy,
                "ComparisonOperator",
                Some(site("k/src/lib.rs", 2)),
                site(e, line),
            )]
        };
        for (source, expected) in [
            (
                "pub enum ComparisonOperator { Less, LessOrEqual, Equal }\n",
                copy(1),
            ),
            (
                "/// Other docs.\n#[derive(Debug)]\nenum ComparisonOperator {\n    \
                 Less(x::Y),\n    LessOrEqual { bound: u8 },\n    Equal,\n}\n",
                copy(3),
            ),
            (
                "pub enum ComparisonOperator { Less, LessEqual, Equal }\n",
                vec![],
            ),
            (
                "pub enum ComparisonOperator { LessOrEqual, Less, Equal }\n",
                vec![],
            ),
            (
                "pub struct ComparisonOperator { Less: u8, LessOrEqual: u8, Equal: u8 }\n",
                vec![],
            ),
        ] {
            let mut fixture = Fixture::new();
            fixture.member("k", TAGGED_OPERATOR).dependency(
                "quire-contract-model",
                IR_SOURCE,
                source,
            );
            assert_eq!(fixture.findings(), expected, "{source}");
        }
        let mut fixture = Fixture::new();
        fixture.member("k", TAGGED_OPERATOR).dependency(
            "unrelated",
            "registry+https://github.com/rust-lang/crates.io-index",
            "pub enum ComparisonOperator { Less, LessOrEqual, Equal }\n",
        );
        assert_eq!(fixture.findings(), vec![]);
    }

    /// TC-747 step 3: a backend run finds the backend's copy of a QSL
    /// canonical type and not its re-export through the QSL facade; over the
    /// QSL workspace, IR's wire `ValueType` is not a copy.
    #[test]
    #[trace("TC-747", "FR-272-AC-3")]
    fn tc_747_backend_run_and_ir_boundary_types() {
        let mut fixture = Fixture::new();
        fixture
            .dependency(
                "quire-exact",
                QSL_SOURCE,
                "/// quire:canonical\npub enum Value {\n    A,\n    B,\n}\n",
            )
            // QSL's facade re-exports it; the backend reaches it there.
            .dependency("qsl-replay", QSL_SOURCE, "pub use quire_exact::Value;\n")
            .member(
                "backend",
                "pub enum Value {\n    A(u8),\n    B,\n}\npub use qsl_replay::Value as Kernel;\n",
            );
        assert_eq!(
            fixture.findings(),
            vec![finding(
                Rule::Copy,
                "Value",
                Some(site("quire-exact/src/lib.rs", 2)),
                site("backend/src/lib.rs", 1)
            )]
        );

        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask has a parent directory");
        let metadata = cargo_metadata(root).expect("cargo metadata runs");
        let report = scan(root, packages(&metadata).expect("metadata reads")).expect("scan runs");
        assert!(report.packages.iter().any(|package| {
            package.name == "quire-contract-model"
                && package.repository == Repository::Ecosystem("quire-contract-ir")
        }));
        // IR's wire `ValueType` is a boundary type: same name as the tagged
        // kernel `ValueType`, different members.
        assert!(report.canonical.contains_key("ValueType"));
        let copies: Vec<&Finding> = report
            .findings
            .iter()
            .filter(|f| f.rule == Rule::Copy && f.ident == "ValueType")
            .collect();
        assert_eq!(copies, Vec::<&Finding>::new());
    }

    /// TC-747 step 4: two namesakes give two findings and a failing exit;
    /// a clean fixture passes.
    #[test]
    #[trace("TC-747", "FR-272-AC-4")]
    fn tc_747_every_finding_is_reported_and_exits() {
        let mut fixture = k_fixture();
        fixture.member(
            "w",
            "struct Value;\nmod nested {\n    pub(crate) enum Value {\n        X,\n    }\n}\n",
        );
        assert_eq!(
            fixture.findings(),
            vec![
                finding(
                    Rule::Identifier,
                    "Value",
                    k_value(),
                    site("w/src/lib.rs", 1)
                ),
                finding(
                    Rule::Identifier,
                    "Value",
                    k_value(),
                    site("w/src/lib.rs", 3)
                ),
            ]
        );
        let error = summarize(fixture.dir.path(), fixture.packages())
            .expect_err("two namesakes fail the gate");
        assert_eq!(error.exit_code(), 1);
        assert_eq!(error.to_string().lines().count(), 2);

        let mut clean = k_fixture();
        clean.member("w", "pub use k::Value;\n");
        assert!(summarize(clean.dir.path(), clean.packages()).is_ok());
    }
}
