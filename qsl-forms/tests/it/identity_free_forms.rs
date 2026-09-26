// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-398 step 2 (FR-091-AC-11): no parsed form holds a `ValueType` or a
//! `NodeKey`. `qsl-forms` depends on `quire-exact`, which defines both, so
//! the crate boundary does not rule them out. This walks every shipped item
//! in `qsl-forms/src/`, the parsed-form types (`TypeForm`, every
//! `Expression` variant, `FunctionDeclaration`, `ParsedForm`) and their
//! methods included, and refuses any path or `use` that names either type.

use std::path::Path;

use ix_trace_rs::trace;
use syn::visit::Visit;

const FORBIDDEN: [&str; 2] = ["ValueType", "NodeKey"];

/// Every forbidden name found in shipped (non-`#[cfg(test)]`) code, with the
/// 1-based line of the path or `use` that names it.
#[derive(Default)]
struct IdentityNames {
    found: Vec<(String, usize)>,
}

impl IdentityNames {
    fn record(&mut self, ident: &syn::Ident) {
        if FORBIDDEN.iter().any(|name| ident == name) {
            self.found
                .push((ident.to_string(), ident.span().start().line));
        }
    }
}

fn is_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("cfg")
            && attr
                .parse_args::<syn::Meta>()
                .is_ok_and(|meta| meta.path().is_ident("test"))
    })
}

impl<'ast> Visit<'ast> for IdentityNames {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        let attrs: &[syn::Attribute] = match item {
            syn::Item::Mod(item) => &item.attrs,
            syn::Item::Fn(item) => &item.attrs,
            syn::Item::Impl(item) => &item.attrs,
            syn::Item::Use(item) => &item.attrs,
            syn::Item::Struct(item) => &item.attrs,
            syn::Item::Enum(item) => &item.attrs,
            syn::Item::Const(item) => &item.attrs,
            _ => &[],
        };
        if !is_cfg_test(attrs) {
            syn::visit::visit_item(self, item);
        }
    }

    fn visit_path_segment(&mut self, segment: &'ast syn::PathSegment) {
        self.record(&segment.ident);
        syn::visit::visit_path_segment(self, segment);
    }

    fn visit_use_name(&mut self, name: &'ast syn::UseName) {
        self.record(&name.ident);
    }

    fn visit_use_rename(&mut self, rename: &'ast syn::UseRename) {
        self.record(&rename.ident);
    }

    fn visit_use_path(&mut self, path: &'ast syn::UsePath) {
        self.record(&path.ident);
        syn::visit::visit_use_path(self, path);
    }
}

#[trace("FR-091-AC-11", "TC-398")]
#[test]
fn no_shipped_item_in_qsl_forms_names_value_type_or_node_key() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files: Vec<_> = std::fs::read_dir(&src)
        .expect("qsl-forms/src reads")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .collect();
    files.sort();
    let names: Vec<String> = files
        .iter()
        .filter_map(|path| path.file_name()?.to_str().map(str::to_owned))
        .collect();
    for required in ["lib.rs", "dispatch.rs", "syntax.rs"] {
        assert!(
            names.iter().any(|name| name == required),
            "qsl-forms/src has no {required}: {names:?}"
        );
    }
    let mut offending = Vec::new();
    for path in &files {
        let source = std::fs::read_to_string(path).expect("a source file reads");
        let file = syn::parse_file(&source).expect("a source file parses");
        let mut visitor = IdentityNames::default();
        visitor.visit_file(&file);
        offending.extend(
            visitor
                .found
                .into_iter()
                .map(|(name, line)| format!("{}:{line}: {name}", path.display())),
        );
    }
    assert!(
        offending.is_empty(),
        "a parsed form names a check-time type: {offending:#?}"
    );
}

/// Every `match` in shipped code with an arm that names a `Production`
/// variant and a catch-all `_` arm, as `line`.
#[derive(Default)]
struct ProductionCatchAlls {
    found: Vec<usize>,
}

/// Whether `pattern` names a `Production::Variant` path anywhere.
fn names_production(pattern: &syn::Pat) -> bool {
    struct Finder(bool);
    impl<'ast> Visit<'ast> for Finder {
        fn visit_path(&mut self, path: &'ast syn::Path) {
            if path.segments.len() >= 2 && path.segments[0].ident == "Production" {
                self.0 = true;
            }
        }
    }
    let mut finder = Finder(false);
    finder.visit_pat(pattern);
    finder.0
}

impl<'ast> Visit<'ast> for ProductionCatchAlls {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        let attrs: &[syn::Attribute] = match item {
            syn::Item::Mod(item) => &item.attrs,
            syn::Item::Fn(item) => &item.attrs,
            syn::Item::Impl(item) => &item.attrs,
            _ => &[],
        };
        if !is_cfg_test(attrs) {
            syn::visit::visit_item(self, item);
        }
    }

    fn visit_expr_match(&mut self, expression: &'ast syn::ExprMatch) {
        if expression.arms.iter().any(|arm| names_production(&arm.pat)) {
            for arm in &expression.arms {
                if matches!(arm.pat, syn::Pat::Wild(_)) {
                    self.found.push(arm.pat.span_start_line());
                }
            }
        }
        syn::visit::visit_expr_match(self, expression);
    }
}

trait StartLine {
    fn span_start_line(&self) -> usize;
}

impl StartLine for syn::Pat {
    fn span_start_line(&self) -> usize {
        syn::spanned::Spanned::span(self).start().line
    }
}

#[trace("FR-091-AC-11", "TC-398")]
#[test]
fn no_production_match_in_qsl_forms_has_a_catch_all_arm() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offending = Vec::new();
    let mut matches_seen = 0_usize;
    for name in ["value.rs", "dispatch.rs", "syntax.rs"] {
        let path = src.join(name);
        let source = std::fs::read_to_string(&path).expect("a source file reads");
        matches_seen += source.matches("Production::").count();
        let file = syn::parse_file(&source).expect("a source file parses");
        let mut visitor = ProductionCatchAlls::default();
        visitor.visit_file(&file);
        offending.extend(visitor.found.iter().map(|line| format!("{name}:{line}")));
    }
    assert!(matches_seen > 0, "the scan reads matches over Production");
    assert!(
        offending.is_empty(),
        "a match over Production has a `_` arm: {offending:?}"
    );
}
