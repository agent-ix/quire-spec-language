// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-254 steps 3-5 (FR-087-AC-4, ADR-013 O-04/R-06): a signature scan of
//! every file under `src/library/`, the module that builds `ImportView`.
//!
//! It fails on:
//!
//! - any `NodeKey` path anywhere in `library` code, since `library` never
//!   builds or names a checked node key;
//! - any function or method that takes a name (`&str`, `String` or
//!   `QualifiedName`) and returns a `NodeKey`, a `PackageNodeKey` or an
//!   `ImportView`;
//! - any `ImportView` method that takes a name at all, since an importing
//!   package's own checker reads the view's `(name, PackageNodeKey)` data
//!   itself (step 5).
//!
//! It does not flag `package_identity::ProjectedDeclarations::node(&str) ->
//! Option<WireNodeId>`, which maps a package's own export names to their
//! wire node ids while `library` admits that package. Whether FR-087 permits
//! that function is not settled: see FR-087's Status section.
//!
//! Like `name_resolution_confinement.rs`, this is a `syn` signature scan,
//! not a call graph. Every file must read and parse, and each rule is also
//! run against a synthetic violating source, so a scanner that silently
//! matched nothing would fail here too.
use std::path::{Path, PathBuf};

use ix_trace_rs::trace;
use syn::visit::Visit;

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

/// Whether `ty` names any of `idents` as an exact path segment.
fn type_names(ty: &syn::Type, idents: &[&str]) -> bool {
    struct Finder<'a> {
        idents: &'a [&'a str],
        found: bool,
    }
    impl<'ast> Visit<'ast> for Finder<'_> {
        fn visit_path_segment(&mut self, segment: &'ast syn::PathSegment) {
            if self.idents.iter().any(|ident| segment.ident == ident) {
                self.found = true;
            }
            syn::visit::visit_path_segment(self, segment);
        }
    }
    let mut finder = Finder {
        idents,
        found: false,
    };
    finder.visit_type(ty);
    finder.found
}

const NAME_TYPES: [&str; 3] = ["str", "String", "QualifiedName"];
const KEYED_RESULTS: [&str; 3] = ["NodeKey", "PackageNodeKey", "ImportView"];

fn takes_a_name(sig: &syn::Signature) -> bool {
    sig.inputs.iter().any(|argument| match argument {
        syn::FnArg::Typed(pat_type) => type_names(&pat_type.ty, &NAME_TYPES),
        syn::FnArg::Receiver(_) => false,
    })
}

fn returns_a_key(sig: &syn::Signature) -> bool {
    match &sig.output {
        syn::ReturnType::Type(_, ty) => type_names(ty, &KEYED_RESULTS),
        syn::ReturnType::Default => false,
    }
}

#[derive(Default)]
struct Scanner {
    violations: Vec<String>,
    in_import_view_impl: bool,
}

impl Scanner {
    fn check(&mut self, sig: &syn::Signature) {
        let name = &sig.ident;
        let line = name.span().start().line;
        if takes_a_name(sig) && returns_a_key(sig) {
            self.violations
                .push(format!("{line}: fn {name} resolves a name to a key"));
        }
        if self.in_import_view_impl && takes_a_name(sig) {
            self.violations
                .push(format!("{line}: ImportView::{name} takes a name"));
        }
    }
}

impl<'ast> Visit<'ast> for Scanner {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.check(&node.sig);
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        let outer = self.in_import_view_impl;
        self.in_import_view_impl = type_names(&node.self_ty, &["ImportView"]);
        syn::visit::visit_item_impl(self, node);
        self.in_import_view_impl = outer;
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.check(&node.sig);
        syn::visit::visit_impl_item_fn(self, node);
    }

    fn visit_path_segment(&mut self, segment: &'ast syn::PathSegment) {
        if segment.ident == "NodeKey" {
            let line = segment.ident.span().start().line;
            self.violations.push(format!("{line}: names NodeKey"));
        }
        syn::visit::visit_path_segment(self, segment);
    }
}

fn violations_in(source: &str) -> Vec<String> {
    let parsed = syn::parse_file(source).expect("source parses as Rust");
    let mut scanner = Scanner::default();
    scanner.visit_file(&parsed);
    scanner.violations
}

/// Steps 3-5 over the live tree.
#[trace("TC-254", "FR-087-AC-4")]
#[test]
fn library_never_resolves_a_name_to_a_key_nor_names_node_key() {
    let files = library_files();
    assert!(
        files.iter().any(|file| file.ends_with("library/mod.rs")),
        "the scan must reach src/library/mod.rs, which defines ImportView"
    );
    let mut report = Vec::new();
    for file in files {
        let source = std::fs::read_to_string(&file)
            .unwrap_or_else(|error| panic!("{}: failed to read: {error}", file.display()));
        report.extend(
            violations_in(&source)
                .into_iter()
                .map(|violation| format!("{}:{violation}", file.display())),
        );
    }
    assert!(report.is_empty(), "library violates TC-254: {report:?}");
}

/// Each rule flags a synthetic violation, so the live-tree pass above is
/// not vacuous.
#[trace("TC-254", "FR-087-AC-4")]
#[test]
fn each_rule_flags_a_synthetic_violation() {
    let cases = [
        (
            "impl ImportView { pub fn get(&self, name: &str) -> Option<PackageNodeKey> { None } }",
            2,
        ),
        (
            "impl ImportView { pub fn node(&self, name: &str) -> Option<WireNodeId> { None } }",
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
    ];
    for (source, expected) in cases {
        assert_eq!(
            violations_in(source).len(),
            expected,
            "{source}: {:?}",
            violations_in(source)
        );
    }
    assert!(violations_in(
        "impl ImportView { pub fn exports(&self) -> impl Iterator<Item = (&str, PackageNodeKey)> { todo!() } }"
    )
    .is_empty());
}
