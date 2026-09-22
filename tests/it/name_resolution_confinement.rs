// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-251 (FR-088-AC-5, R-06): no function outside the check stage resolves
//! a `QualifiedName` (or a bare string) to a node id or declaration, with
//! the one documented exception being the `replay` facade's own E9 lookup.
//!
//! **Scope of what this scans (step 3's own signature criterion, the part
//! this test can check with `syn` alone -- see the doc below for what step
//! 1/2's call-graph half and the known `value::library::LibraryLock::resolve_name`
//! gray area are not decided here).** This walks every function and method
//! definition in `src/`, outside `src/check/` and `src/replay/`, and fails
//! on any whose signature both (a) takes a parameter naming `QualifiedName`
//! and (b) returns a type naming `NodeKey`, `DeclarationKey` or
//! `ExportIdentity` -- exactly AC-5's own literal criterion, "a function
//! ... accepts a `QualifiedName` ... and returns a node id or declaration."
//! This is a signature scan, not a full call-graph/type-resolution engine
//! (matching this repository's own `xtask::definition_scan`/
//! `tools/arch-lint` convention of scoping a `syn`-only scan to what it can
//! actually decide, and naming what it cannot): it does not resolve which
//! of this crate's five distinct `QualifiedName`-named types a given
//! parameter names (four are unrelated wire/syntax types outside O-11's
//! scope; see `check::identity`'s own module doc), so a hypothetical
//! parameter using one of the *other* four would still be flagged here --
//! there are none today, so this scan is not vacuous against the live tree.
//!
//! **`value::library::LibraryLock::resolve_name` is a known, separately
//! reported gray area, not decided by this test.** Its signature takes
//! `&NameReference` (a bare-string-*wrapping* type, not `QualifiedName` or
//! `&str` themselves) and returns `ExportIdentity` (which does carry a node
//! id). FR-087's own owner ruling (`spec/functional/FR-087-typestate-and-cross-package-node-key.md`,
//! item 3(b)) already schedules `resolve_name` and `NameReference` for
//! removal when `value::library` relocates into the new `library` module
//! FR-087 defines -- that relocation has not landed as of this test. This
//! test does not assert either verdict for it (that would be deciding an
//! open question this PR's own brief says to report, not resolve); it is
//! recorded here in prose, and in the PR/ticket, for a human ruling.
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

/// Whether `ty`'s own token text names `ident` anywhere (a parameter or
/// return type mentioning it, through a reference, generic or path).
fn type_names(ty: &syn::Type, ident: &str) -> bool {
    format!("{ty:?}").contains(ident)
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
/// returning a node id or declaration.
#[trace("TC-251", "FR-088-AC-5")]
#[test]
fn no_signature_outside_check_or_replay_resolves_a_qualified_name_to_an_identity() {
    let root = workspace_root();
    let mut violations = Vec::new();
    for file in source_files(&root, &["src/check/", "src/replay/"]) {
        let path = root.join(&file);
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(parsed) = syn::parse_file(&source) else {
            continue;
        };
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

/// Step 2's own exception, positively confirmed rather than only excluded
/// by directory: `crate::check::CheckedGraph::callable`/`function_identity`
/// are the checker's own name -> node id resolution functions R-06/O-11
/// name (reached from outside `check` only through
/// `value::expression::CheckedPackage::call`'s typed `QualifiedName`
/// lookup, itself inside the check stage's own resolution path per that
/// method's doc). This asserts they still exist under `check`, so a future
/// rename or relocation out of `check` is caught here rather than silently
/// making the directory-based exclusion above vacuous.
#[trace("TC-251", "FR-088-AC-5")]
#[test]
fn the_checker_own_name_resolution_functions_still_live_under_check() {
    let path = workspace_root().join("src/check/mod.rs");
    let source = std::fs::read_to_string(&path).expect("src/check/mod.rs reads");
    for function_name in ["fn callable", "fn function_identity", "fn function("] {
        assert!(
            source.contains(function_name),
            "expected {function_name} in src/check/mod.rs -- \
             has the checker's own name resolution moved?"
        );
    }
}
