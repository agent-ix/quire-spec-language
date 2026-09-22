// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-251 (FR-088-AC-5, R-06): no function outside the check stage resolves
//! a `QualifiedName` (or a bare string) to a node id or declaration, with
//! the one documented exception being the `replay` facade's own E9 lookup.
//!
//! **Scope of what this scans (step 3's own signature criterion, the part
//! this test can check with `syn` alone -- see the doc below for what step
//! 1/2's call-graph half is not decided here).** This walks every function
//! and method definition in `src/`, outside `src/check/` and `src/replay/`,
//! and fails on any whose signature both (a) takes a parameter naming
//! `QualifiedName` and (b) returns a type naming `NodeKey`, `DeclarationKey`
//! or `ExportIdentity` -- exactly AC-5's own literal criterion for that
//! half, "a function ... accepts a `QualifiedName` ... and returns a node id
//! or declaration." This is a signature scan, not a full call-graph/
//! type-resolution engine (matching this repository's own `xtask::
//! definition_scan`/`tools/arch-lint` convention of scoping a `syn`-only
//! scan to what it can actually decide, and naming what it cannot): it does
//! not resolve which of this crate's five distinct `QualifiedName`-named
//! types a given parameter names (four are unrelated wire/syntax types
//! outside O-11's scope; see `check::identity`'s own module doc), so a
//! hypothetical parameter using one of the *other* four would still be
//! flagged here -- there are none today, so this scan is not vacuous
//! against the live tree.
//!
//! **AC-5's "or a bare string" half is investigated, not enforced here.**
//! Extending [`signature_matches`] to also flag a bare `&str` parameter
//! (not `QualifiedName`) whose function returns `NodeKey`/`DeclarationKey`/
//! `ExportIdentity` is mechanically straightforward and was proven
//! non-vacuous: with that extension in place, this scan flagged
//! `src/model/intake.rs`'s `declaration_key`/`identity_keys`/
//! `read_type_identity` and `src/model/refusal.rs`'s `#[cfg(test)]` `key`
//! helper -- real, existing `&str -> DeclarationKey` signatures, not a
//! synthetic mutant. But every one of them is a plain constructor
//! (`DeclarationKey { package: package.to_owned(), node: node.to_owned() }`
//! or its own callers), never a *lookup*: it wraps the caller's own strings
//! unchanged into the newtype, with no table or graph consulted, which is
//! not the "look up a node by name" R-06 forbids. A signature-only scan
//! cannot tell that apart from a genuine post-check name resolution
//! function with the same shape (this is exactly the limitation this file's
//! own convention note above names for a `syn`-only scan); soundly
//! distinguishing the two needs real call-graph/body analysis, out of this
//! scan's own scope. Extending the enforced check would therefore either
//! false-positive permanently on legitimate intake code, or require
//! touching `model/intake.rs`/`model/refusal.rs` to work around the scan --
//! neither is this test's job. This test enforces only the `QualifiedName`
//! half; the bare-string half stays a documented, investigated gap.
//!
//! **`value::library::LibraryLock::resolve_name` no longer exists.** #299
//! (landed on `main` before this PR) relocated `value::library` into the new
//! `library` module FR-087 defines and, per FR-087's own owner ruling
//! (`spec/functional/FR-087-typestate-and-cross-package-node-key.md`, item
//! 3(b)), deleted `resolve_name`, `NameReference` and `ExportIdentity`
//! outright rather than carrying them forward (`src/library/mod.rs`'s own
//! module doc: "This module names none of `resolve_name`, `ExportIdentity`,
//! `NameReference` or `NameRefusal`."). The gray area this doc used to flag
//! for a human ruling is moot: there is nothing left to rule on, and this
//! scan's directory-based exclusion has no such function to miss.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use ix_trace_rs::trace;
use syn::visit::Visit;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Every regular `.rs` file under `root`, relative to `workspace_root`,
/// excluding `exclude_prefixes` (checked against the relative path).
fn source_files(workspace_root: &Path, exclude_prefixes: &[&str]) -> Vec<String> {
    let root = workspace_root.join("src");
    let mut files = Vec::new();
    let mut pending = vec![root];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
                continue;
            }
            let relative = path
                .strip_prefix(workspace_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            if exclude_prefixes
                .iter()
                .any(|prefix| relative.starts_with(prefix))
            {
                continue;
            }
            files.push(relative);
        }
    }
    files.sort();
    files
}

/// Whether `ty` names `ident` as an exact path segment anywhere in its own
/// structure (a parameter or return type mentioning it, through a
/// reference, generic or path) -- a real AST walk over every path segment,
/// not a `Debug`-string substring search (PR #300 review, L5): this needs
/// none of `syn`'s `extra-traits` feature (which exists only to put `Debug`
/// on `syn::Type`), and an identifier match is exact, so a longer,
/// unrelated identifier that merely contains `ident` as a substring
/// (`MyQualifiedNameWrapper`, say) does not false-positive the way a
/// substring search over the formatted tree could.
fn type_names(ty: &syn::Type, ident: &str) -> bool {
    struct SegmentFinder<'a> {
        ident: &'a str,
        found: bool,
    }

    impl<'a, 'ast> Visit<'ast> for SegmentFinder<'a> {
        fn visit_path_segment(&mut self, segment: &'ast syn::PathSegment) {
            if segment.ident == self.ident {
                self.found = true;
            }
            syn::visit::visit_path_segment(self, segment);
        }
    }

    let mut finder = SegmentFinder {
        ident,
        found: false,
    };
    finder.visit_type(ty);
    finder.found
}

fn signature_matches(sig: &syn::Signature) -> bool {
    let takes_qualified_name = sig.inputs.iter().any(|argument| match argument {
        syn::FnArg::Typed(pat_type) => type_names(&pat_type.ty, "QualifiedName"),
        syn::FnArg::Receiver(_) => false,
    });
    if !takes_qualified_name {
        return false;
    }
    match &sig.output {
        syn::ReturnType::Type(_, ty) => {
            type_names(ty, "NodeKey")
                || type_names(ty, "DeclarationKey")
                || type_names(ty, "ExportIdentity")
        }
        syn::ReturnType::Default => false,
    }
}

/// One matching signature's own location, for a failing assertion to name.
struct Violation {
    file: String,
    line: u32,
    name: String,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{} fn {}", self.file, self.line, self.name)
    }
}

struct SignatureScanner {
    file: String,
    violations: Vec<Violation>,
}

impl<'ast> Visit<'ast> for SignatureScanner {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        if signature_matches(&node.sig) {
            self.violations.push(Violation {
                file: self.file.clone(),
                line: node.sig.ident.span().start().line as u32,
                name: node.sig.ident.to_string(),
            });
        }
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        if signature_matches(&node.sig) {
            self.violations.push(Violation {
                file: self.file.clone(),
                line: node.sig.ident.span().start().line as u32,
                name: node.sig.ident.to_string(),
            });
        }
        syn::visit::visit_impl_item_fn(self, node);
    }
}

/// Steps 3-4 (bounded, see this file's own doc): no function outside
/// `check`/`replay` has a signature accepting a `QualifiedName` and
/// returning a node id or declaration. Every source file under scan must
/// actually read and parse: a file this scan cannot see is a gap in
/// coverage, not a pass, so a read or parse failure fails the test loudly
/// instead of silently skipping the file (an unparseable file would
/// otherwise vacuously admit anything it contains).
#[trace("TC-251", "FR-088-AC-5")]
#[test]
fn no_signature_outside_check_or_replay_resolves_a_qualified_name_to_an_identity() {
    let root = workspace_root();
    let mut violations = Vec::new();
    for file in source_files(&root, &["src/check/", "src/replay/"]) {
        let path = root.join(&file);
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{file}: failed to read: {error}"));
        let parsed = syn::parse_file(&source)
            .unwrap_or_else(|error| panic!("{file}: failed to parse as Rust: {error}"));
        let mut scanner = SignatureScanner {
            file: file.clone(),
            violations: Vec::new(),
        };
        scanner.visit_file(&parsed);
        violations.extend(scanner.violations);
    }
    let report: Vec<String> = violations.iter().map(ToString::to_string).collect();
    assert!(
        violations.is_empty(),
        "found a QualifiedName -> identity resolution signature outside check/replay: {report:?}"
    );
}

/// Collects every function/method item name `syn` actually parses as a
/// definition, so the assertion below tests a real AST fact, not merely
/// that some substring appears in the file's text (which a comment, a
/// string literal or a doc example could satisfy just as well).
struct DefinedNames(BTreeSet<String>);

impl<'ast> Visit<'ast> for DefinedNames {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.0.insert(node.sig.ident.to_string());
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.0.insert(node.sig.ident.to_string());
        syn::visit::visit_impl_item_fn(self, node);
    }
}

/// Step 2's own exception, positively confirmed rather than only excluded
/// by directory: `crate::check::CheckedGraph::callable`/`function_identity`
/// are the checker's own name -> node id resolution functions R-06/O-11
/// name (reached from outside `check` only through
/// `value::expression::CheckedPackage::call`'s typed `QualifiedName`
/// lookup, itself inside the check stage's own resolution path per that
/// method's doc). This asserts they still exist under `check` as real
/// parsed function/method definitions (not a text search that a comment or
/// doc example could also satisfy), so a future rename or relocation out of
/// `check` is caught here rather than silently making the directory-based
/// exclusion above vacuous.
#[trace("TC-251", "FR-088-AC-5")]
#[test]
fn the_checker_own_name_resolution_functions_still_live_under_check() {
    let path = workspace_root().join("src/check/mod.rs");
    let source = std::fs::read_to_string(&path).expect("src/check/mod.rs reads");
    let parsed = syn::parse_file(&source).expect("src/check/mod.rs parses as Rust");
    let mut names = DefinedNames(BTreeSet::new());
    names.visit_file(&parsed);
    for function_name in ["callable", "function_identity", "function"] {
        assert!(
            names.0.contains(function_name),
            "expected a defined `fn {function_name}` in src/check/mod.rs -- \
             has the checker's own name resolution moved?"
        );
    }
}
