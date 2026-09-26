// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-062-AC-6, second sentence (QSL-246): a family's `evaluate` hook reads
//! no CST, token or display string. "Display string" means rendered text
//! (`Display`/`Debug` output, diagnostic text, source spelling), per ADR-011
//! FB-01. Declared names carried on checked nodes are checked input:
//! copying one into a failure payload, or using a field name as a record
//! key, is not a display-string read.
//!
//! `CheckedPackage` is a concrete type, so a panicking test double cannot be
//! substituted for it. This file backs the criterion's API-surface half
//! instead: `qsl-eval` has no direct dependency on `qsl-cst` or `qsl-forms`,
//! and the evaluator's own source calls nothing that renders text or
//! reads a string-shaped accessor of the checked package. The behavioural
//! half (renaming declared names changes no result) is
//! `qsl-eval/tests/it/evaluation_ignores_display_strings.rs`.

use ix_trace_rs::trace;
use std::path::Path;
use syn::visit::Visit;

const RENDERING_MACROS: [&str; 9] = [
    "format",
    "format_args",
    "write",
    "writeln",
    "print",
    "println",
    "eprint",
    "eprintln",
    "dbg",
];

/// Methods that render a value to text, parse text, or read a string-shaped
/// accessor of the checked package (`CheckedPackage::function_identity`
/// takes a display name; a function's `slot_names` are its source spellings;
/// `spans` are source offsets).
const TEXT_METHODS: [&str; 6] = [
    "to_string",
    "parse",
    "function_identity",
    "slot_names",
    "measure_slot_names",
    "spans",
];

struct Scan {
    only_fn: Option<&'static str>,
    findings: Vec<String>,
}

fn is_cfg_test(attributes: &[syn::Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        attribute.path().is_ident("cfg")
            && attribute
                .parse_args::<syn::Ident>()
                .is_ok_and(|ident| ident == "test")
    })
}

impl<'ast> Visit<'ast> for Scan {
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if !is_cfg_test(&item.attrs) {
            syn::visit::visit_item_mod(self, item);
        }
    }
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if self.only_fn.is_none() && !is_cfg_test(&item.attrs) {
            syn::visit::visit_item_fn(self, item);
        }
    }
    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        let wanted = self.only_fn.is_none_or(|name| item.sig.ident == name);
        if wanted && !is_cfg_test(&item.attrs) {
            syn::visit::visit_impl_item_fn(self, item);
        }
    }
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if let Some(segment) = mac.path.segments.last() {
            if RENDERING_MACROS.iter().any(|name| segment.ident == name) {
                self.findings.push(format!("{}! macro", segment.ident));
            }
        }
    }
    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        let name = call.method.to_string();
        if TEXT_METHODS.contains(&name.as_str()) {
            // The one allowed rendering: a population's declared name copied
            // into an absent-key payload (a declared name is checked input).
            let allowed = name == "to_string"
                && matches!(&*call.receiver, syn::Expr::Path(path) if path.path.is_ident("population"));
            if !allowed {
                self.findings.push(format!(".{name}() call"));
            }
        }
        syn::visit::visit_expr_method_call(self, call);
    }
}

fn findings(source: &str, only_fn: Option<&'static str>) -> Vec<String> {
    let file = syn::parse_file(source).expect("source parses");
    let mut scan = Scan {
        only_fn,
        findings: Vec::new(),
    };
    scan.visit_file(&file);
    scan.findings
}

/// `qsl-eval`'s `[dependencies]` table text.
fn shipped_dependencies(manifest: &str) -> &str {
    let start = manifest
        .find("\n[dependencies]")
        .expect("qsl-eval has a [dependencies] table");
    let rest = &manifest[start + 1..];
    let body = &rest["[dependencies]".len()..];
    body.find("\n[").map_or(body, |end| &body[..end])
}

#[trace("TC-160", "FR-062-AC-6")]
#[test]
fn the_evaluator_has_no_path_to_source_text_or_rendering() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("qsl-eval");

    // No direct dependency on `qsl-cst` (the S1 reader) or `qsl-forms` (the
    // S2 forms); `qsl-forms` is reachable through `qsl-semantics`.
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).expect("manifest reads");
    let shipped = shipped_dependencies(&manifest);
    for forbidden in ["qsl-cst", "qsl-forms"] {
        assert!(
            !shipped.contains(forbidden),
            "qsl-eval ships a dependency on {forbidden}:\n{shipped}"
        );
    }

    // The evaluator machine and the family's `evaluate` hook call nothing
    // that renders text or reads a string-shaped accessor.
    let expression = root.join("src/value/expression");
    let machine = std::fs::read_to_string(expression.join("evaluate.rs")).expect("reads");
    let hook = std::fs::read_to_string(expression.join("family.rs")).expect("reads");
    let mut found = findings(&machine, None);
    found.extend(findings(&hook, Some("evaluate")));
    assert!(found.is_empty(), "evaluator reads display text: {found:?}");

    // The scan is not vacuous: each forbidden form is flagged, and the
    // allowed name copy and test-only items are not.
    let violating =
        "fn evaluate() { let _a = format!(\"{}\", 1); let _b = x.function_identity(\"f\"); \
                     let _c = x.slot_names(); let _d = y.to_string(); let _e = s.parse::<u8>(); }";
    let flagged = findings(violating, None);
    for needle in [
        "format! macro",
        ".function_identity() call",
        ".slot_names() call",
        ".to_string() call",
        ".parse() call",
    ] {
        assert!(flagged.iter().any(|f| f == needle), "{needle}: {flagged:?}");
    }
    assert!(findings("fn f() { population.to_string(); }", None).is_empty());
    assert!(findings("#[cfg(test)] mod t { fn f() { format!(\"x\"); } }", None).is_empty());
}
