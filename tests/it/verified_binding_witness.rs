// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-253 (FR-087-AC-3), ADR-011 §4 condition 1: the witness
//! `library::SupportedV2Wire` that `verify_binding` requires is minted only
//! by the layer-4 v2 reader, after IR admitted the bytes.
//!
//! The witness lives in `library::witness`, whose private field means the
//! compiler already refuses a `SupportedV2Wire(())` built anywhere else,
//! including elsewhere in `library`. Its constructor,
//! `attest_ir_admitted_v2`, must be callable from `qsl-package`'s
//! `checked_v2`, and Rust can restrict visibility only to
//! an ancestor module (E0742), so no visibility confines that call to the
//! reader. This `syn` scan does. Outside `library::witness` and `library`'s
//! test-only `binding_tests.rs`, it requires exactly one reference to the
//! constructor in every workspace crate's `src/`: a direct call inside
//! `read_checked_package_v2`,
//! within a match arm whose pattern names `AdmittedV2`. It also fails on:
//!
//! - any reference inside a macro's tokens, anywhere, since `syn::visit`
//!   does not parse macro bodies, so a call there cannot be placed;
//! - a reference from any other function in `checked_v2.rs`, which would
//!   let a wrapper mint the witness for other callers;
//! - a tuple or struct construction of `SupportedV2Wire` outside
//!   `library::witness`, as a second check on what the compiler refuses.
//!
//! The constructor and `verify_binding` are `pub` for the QSL-181 crate
//! boundary. Two gates confine the constructor within the QSL workspace,
//! and neither is complete alone: arch-lint rule T12-E fails on any
//! reference outside `qsl-package`'s `checked_v2`, but not on a wrapper
//! inside `checked_v2` (a `#[macro_export]` macro, a trait method or a
//! helper fn) that other modules call; this scan fails on exactly those,
//! because it requires the one reference in `checked_v2` to be the direct
//! call in `read_checked_package_v2`'s `AdmittedV2` arm. It scans every
//! workspace crate's `src/`, so it follows `library` into `qsl-semantics`
//! (X-6b) and the reader into `qsl-package` (X-7).
use std::path::{Path, PathBuf};

use ix_trace_rs::trace;
use syn::visit::Visit;

const CONSTRUCTOR: &str = "attest_ir_admitted_v2";
const WITNESS: &str = "SupportedV2Wire";
const READER_FILE: &str = "qsl-package/src/checked_v2.rs";
const READER_FN: &str = "read_checked_package_v2";
const ADMITTED_ARM: &str = "AdmittedV2";
/// The witness's own module, and `library`'s test-only binding tests, both
/// in `qsl-semantics`.
const EXEMPT: [&str; 2] = [
    "qsl-semantics/src/library/witness.rs",
    "qsl-semantics/src/library/binding_tests.rs",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Every Rust source file of every workspace crate: the root crate's
/// `src/` and each `<crate>/src/` beside it, so the scan follows `library`
/// into `qsl-semantics` (QSL-181) and the reader into `qsl-package`
/// (X-7). Each file is named relative to the workspace root, crate
/// directory included (`qsl-semantics/src/library/witness.rs`), which is how
/// [`EXEMPT`] and [`READER_FILE`] name it: a same-named module in another
/// crate is not exempt.
fn source_files() -> Vec<(String, PathBuf)> {
    let root = root();
    let mut crate_roots = vec![root.clone()];
    for entry in std::fs::read_dir(&root).expect("the workspace root lists") {
        let path = entry.expect("directory entry reads").path();
        if path.join("Cargo.toml").is_file() && path.join("src").is_dir() {
            crate_roots.push(path);
        }
    }
    let mut files = Vec::new();
    for crate_root in crate_roots {
        let mut pending = vec![crate_root.join("src")];
        while let Some(dir) = pending.pop() {
            let entries = std::fs::read_dir(&dir)
                .unwrap_or_else(|error| panic!("{}: failed to list: {error}", dir.display()));
            for entry in entries {
                let path = entry.expect("directory entry reads").path();
                if path.is_dir() {
                    pending.push(path);
                } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
                    let relative = path
                        .strip_prefix(&root)
                        .expect("under the workspace root")
                        .to_string_lossy()
                        .replace('\\', "/");
                    files.push((relative, path));
                }
            }
        }
    }
    files.sort();
    files
}

fn last_segment_is(path: &syn::Path, ident: &str) -> bool {
    path.segments
        .last()
        .is_some_and(|segment| segment.ident == ident)
}

fn pattern_names(pat: &syn::Pat, ident: &str) -> bool {
    struct Finder<'a> {
        ident: &'a str,
        found: bool,
    }
    impl<'ast> Visit<'ast> for Finder<'_> {
        fn visit_path_segment(&mut self, segment: &'ast syn::PathSegment) {
            if segment.ident == self.ident {
                self.found = true;
            }
            syn::visit::visit_path_segment(self, segment);
        }
    }
    let mut finder = Finder {
        ident,
        found: false,
    };
    finder.visit_pat(pat);
    finder.found
}

/// Every reference to the witness constructor and every construction of
/// the witness type in one file, with where each sits.
#[derive(Default)]
struct Scan {
    fns: Vec<String>,
    admitted_arms: usize,
    /// Direct calls sitting in the reader's `AdmittedV2` arm.
    sanctioned: Vec<usize>,
    /// Every other reference, as "line: what".
    other: Vec<String>,
    /// Constructions of the witness type, as "line: what".
    constructions: Vec<String>,
}

impl Scan {
    fn in_reader_arm(&self) -> bool {
        self.admitted_arms > 0 && self.fns.last().is_some_and(|name| name == READER_FN)
    }

    /// One identifier-shaped word of a macro's tokens, at `text[from..to]`.
    fn macro_word(&mut self, text: &str, from: usize, to: usize, line: usize) {
        let word = &text[from..to];
        if word == CONSTRUCTOR {
            self.other.push(format!("{line}: inside a macro"));
        }
        if word == WITNESS {
            let rest = text[to..].trim_start();
            if rest.starts_with('(') || rest.starts_with('{') {
                self.constructions
                    .push(format!("{line}: {WITNESS} built inside a macro"));
            }
        }
    }
}

impl<'ast> Visit<'ast> for Scan {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        // A nested fn starts a fresh context: an arm around it does not
        // sanction a call inside it.
        let arms = std::mem::take(&mut self.admitted_arms);
        self.fns.push(node.sig.ident.to_string());
        syn::visit::visit_item_fn(self, node);
        self.fns.pop();
        self.admitted_arms = arms;
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        let arms = std::mem::take(&mut self.admitted_arms);
        self.fns.push(node.sig.ident.to_string());
        syn::visit::visit_impl_item_fn(self, node);
        self.fns.pop();
        self.admitted_arms = arms;
    }

    fn visit_arm(&mut self, node: &'ast syn::Arm) {
        let admitted = pattern_names(&node.pat, ADMITTED_ARM);
        if admitted {
            self.admitted_arms += 1;
        }
        syn::visit::visit_arm(self, node);
        if admitted {
            self.admitted_arms -= 1;
        }
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(callee) = node.func.as_ref() {
            let line = callee
                .path
                .segments
                .last()
                .map_or(0, |segment| segment.ident.span().start().line);
            if last_segment_is(&callee.path, CONSTRUCTOR) {
                if self.in_reader_arm() {
                    self.sanctioned.push(line);
                } else {
                    let context = self.fns.last().map_or("<no fn>", String::as_str);
                    self.other.push(format!("{line}: call in fn {context}"));
                }
                // The callee path is recorded; only the arguments remain.
                for argument in &node.args {
                    self.visit_expr(argument);
                }
                return;
            }
            if last_segment_is(&callee.path, WITNESS) {
                self.constructions.push(format!("{line}: {WITNESS}(..)"));
            }
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_struct(&mut self, node: &'ast syn::ExprStruct) {
        if last_segment_is(&node.path, WITNESS) {
            let line = node
                .path
                .segments
                .last()
                .map_or(0, |segment| segment.ident.span().start().line);
            self.constructions.push(format!("{line}: {WITNESS} {{..}}"));
        }
        syn::visit::visit_expr_struct(self, node);
    }

    fn visit_path_segment(&mut self, segment: &'ast syn::PathSegment) {
        // Reached only for references that are not a direct call's callee:
        // a fn-pointer path, a `use`, or a path inside a type.
        if segment.ident == CONSTRUCTOR {
            let line = segment.ident.span().start().line;
            self.other.push(format!("{line}: non-call reference"));
        }
        syn::visit::visit_path_segment(self, segment);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        let line = node
            .path
            .segments
            .last()
            .map_or(0, |segment| segment.ident.span().start().line);
        let text = node.tokens.to_string();
        let mut word_start = None;
        for (index, c) in text.char_indices().chain([(text.len(), ' ')]) {
            let ident_char = c.is_alphanumeric() || c == '_';
            match (word_start, ident_char) {
                (None, true) => word_start = Some(index),
                (Some(from), false) => {
                    self.macro_word(&text, from, index, line);
                    word_start = None;
                }
                _ => {}
            }
        }
        syn::visit::visit_macro(self, node);
    }
}

/// The violations in one file, given its path relative to its own crate
/// root.
fn violations(file: &str, source: &str) -> Vec<String> {
    if EXEMPT.contains(&file) {
        return Vec::new();
    }
    let parsed = syn::parse_file(source)
        .unwrap_or_else(|error| panic!("{file}: failed to parse as Rust: {error}"));
    let mut scan = Scan::default();
    scan.visit_file(&parsed);
    let mut found: Vec<String> = scan
        .other
        .into_iter()
        .chain(scan.constructions)
        .map(|violation| format!("{file}:{violation}"))
        .collect();
    let sanctioned = if file == READER_FILE {
        scan.sanctioned.len()
    } else {
        0
    };
    if file != READER_FILE {
        found.extend(
            scan.sanctioned
                .iter()
                .map(|line| format!("{file}:{line}: call outside {READER_FILE}")),
        );
    } else if sanctioned != 1 {
        found.push(format!(
            "{file}: expected exactly one call in {READER_FN}'s {ADMITTED_ARM} arm, found {sanctioned}"
        ));
    }
    found
}

#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn only_the_v2_reader_mints_the_condition_1_witness() {
    let files = source_files();
    assert_eq!(
        files.iter().filter(|(file, _)| file == READER_FILE).count(),
        1,
        "the scan must reach exactly one {READER_FILE}"
    );
    assert!(
        files.iter().any(|(file, _)| file == EXEMPT[0]),
        "the scan must reach the witness module {}",
        EXEMPT[0]
    );
    let mut found = Vec::new();
    for (file, path) in files {
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{}: failed to read: {error}", path.display()));
        found.extend(violations(&file, &source));
    }
    assert!(
        found.is_empty(),
        "the condition-1 witness is referenced outside the v2 reader's AdmittedV2 arm: {found:?}"
    );
}

/// The reader's own shape, which the live tree must keep.
const READER_OK: &str = "
fn read_checked_package_v2() {
    match read() {
        Dispatch::AdmittedV2(package) => {
            let admitted = SupportedV2Wire::attest_ir_admitted_v2();
            verify_binding(admitted);
        }
        Dispatch::Refused(_) => {}
    }
}
";

/// The scan admits the reader's own shape, and flags a plain mint and each
/// of three evasions: a call inside a macro, a direct construction of the
/// witness, and a wrapper fn in the reader's file.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn the_scan_flags_each_evasion() {
    assert_eq!(violations(READER_FILE, READER_OK), Vec::<String>::new());

    let flagged = |file: &str, source: &str| !violations(file, source).is_empty();

    // A plain call from another module.
    assert!(flagged(
        "qsl-semantics/src/check/mod.rs",
        "fn forge() { let _ = SupportedV2Wire::attest_ir_admitted_v2(); }",
    ));
    // Evasion 1: the call inside a macro, in another module and in the
    // reader's own arm.
    assert!(flagged(
        "qsl-semantics/src/check/mod.rs",
        "fn forge() { let _ = vec![crate::library::SupportedV2Wire::attest_ir_admitted_v2()]; }",
    ));
    assert!(flagged(
        READER_FILE,
        "fn read_checked_package_v2() { match read() { Dispatch::AdmittedV2(p) => {
            let admitted = SupportedV2Wire::attest_ir_admitted_v2();
            let _ = vec![SupportedV2Wire::attest_ir_admitted_v2()];
            verify_binding(admitted);
        } } }",
    ));
    // Evasion 2: a tuple, struct or in-macro construction elsewhere in
    // `library` (the compiler also refuses these outside `library::witness`).
    assert!(flagged(
        "qsl-semantics/src/library/package_identity.rs",
        "fn forge() -> SupportedV2Wire { super::SupportedV2Wire(()) }",
    ));
    assert!(flagged(
        "qsl-semantics/src/library/mod.rs",
        "fn forge() -> SupportedV2Wire { SupportedV2Wire { 0: () } }",
    ));
    assert!(flagged(
        "qsl-semantics/src/library/mod.rs",
        "fn forge() { let _ = vec![SupportedV2Wire(())]; }",
    ));
    // A same-named witness module in another crate is not exempt.
    assert!(flagged(
        "src/library/witness.rs",
        "fn forge() -> SupportedV2Wire { SupportedV2Wire(()) }",
    ));
    // Evasion 3: a wrapper fn in the reader's file, called by the arm.
    assert!(flagged(
        READER_FILE,
        "pub(crate) fn zz_mint() -> SupportedV2Wire { SupportedV2Wire::attest_ir_admitted_v2() }
        fn read_checked_package_v2() { match read() { Dispatch::AdmittedV2(p) => {
            verify_binding(zz_mint());
        } } }",
    ));
    // A mint in the reader fn but outside the AdmittedV2 arm, and one in a
    // fn nested inside the arm.
    assert!(flagged(
        READER_FILE,
        "fn read_checked_package_v2() { let _ = SupportedV2Wire::attest_ir_admitted_v2(); }",
    ));
    assert!(flagged(
        READER_FILE,
        "fn read_checked_package_v2() { match read() { Dispatch::AdmittedV2(p) => {
            fn nested() -> SupportedV2Wire { SupportedV2Wire::attest_ir_admitted_v2() }
            verify_binding(nested());
        } } }",
    ));
    // A fn-pointer reference, even in the arm.
    assert!(flagged(
        READER_FILE,
        "fn read_checked_package_v2() { match read() { Dispatch::AdmittedV2(p) => {
            let mint = SupportedV2Wire::attest_ir_admitted_v2;
            verify_binding(mint());
        } } }",
    ));
}
