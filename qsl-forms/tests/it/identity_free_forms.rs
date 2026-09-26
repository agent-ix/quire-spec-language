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
/// variant and a catch-all arm (`_` or a bare binding), as `line`.
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
                if matches!(arm.pat, syn::Pat::Wild(_) | syn::Pat::Ident(_)) {
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
        "a match over Production has a catch-all arm: {offending:?}"
    );
}

/// TC-398 step 1 (FR-091-AC-11): the family-module `use` and inline-path
/// edges of `value.rs`. It may reach `qsl_cst`, `qsl_foundation`,
/// `quire_exact`, the standard library and the forms core (`dispatch`,
/// `spans`, `syntax`), and nothing else.
const ALLOWED_EXTERNAL: [&str; 6] = [
    "qsl_cst",
    "qsl_foundation",
    "quire_exact",
    "std",
    "core",
    "alloc",
];
const FORMS_CORE: [&str; 3] = ["dispatch", "spans", "syntax"];

/// The disallowed edges found in shipped code, as `line: path`.
#[derive(Default)]
struct ModuleEdges {
    found: Vec<String>,
}

impl ModuleEdges {
    /// Judge a path by its segment names: `[first, second, ...]`.
    fn judge(&mut self, segments: &[String], line: usize) {
        let first = segments.first().map(String::as_str).unwrap_or_default();
        let allowed = match first {
            "super" => segments
                .get(1)
                .is_some_and(|second| FORMS_CORE.contains(&second.as_str())),
            "self" => true,
            "crate" => segments
                .get(1)
                .is_some_and(|second| FORMS_CORE.contains(&second.as_str())),
            other => ALLOWED_EXTERNAL.contains(&other),
        };
        if !allowed {
            self.found.push(format!("{line}: {}", segments.join("::")));
        }
    }

    /// Flatten a `use` tree into full paths and judge each.
    fn use_tree(&mut self, tree: &syn::UseTree, prefix: &mut Vec<String>) {
        match tree {
            syn::UseTree::Path(path) => {
                prefix.push(path.ident.to_string());
                self.use_tree(&path.tree, prefix);
                prefix.pop();
            }
            syn::UseTree::Name(name) => {
                let mut full = prefix.clone();
                full.push(name.ident.to_string());
                self.judge(&full, name.ident.span().start().line);
            }
            syn::UseTree::Rename(rename) => {
                let mut full = prefix.clone();
                full.push(rename.ident.to_string());
                self.judge(&full, rename.ident.span().start().line);
            }
            syn::UseTree::Glob(glob) => {
                self.judge(prefix, glob.star_token.spans[0].start().line);
            }
            syn::UseTree::Group(group) => {
                for item in &group.items {
                    self.use_tree(item, prefix);
                }
            }
        }
    }
}

impl<'ast> Visit<'ast> for ModuleEdges {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        let attrs: &[syn::Attribute] = match item {
            syn::Item::Mod(item) => &item.attrs,
            syn::Item::Use(item) => &item.attrs,
            syn::Item::Fn(item) => &item.attrs,
            syn::Item::Impl(item) => &item.attrs,
            _ => &[],
        };
        if is_cfg_test(attrs) {
            return;
        }
        if let syn::Item::Use(item) = item {
            self.use_tree(&item.tree, &mut Vec::new());
            return;
        }
        syn::visit::visit_item(self, item);
    }

    /// Macro bodies (`matches!`, `format!`, ...) are token streams the parser
    /// does not read as paths, so scan their tokens for `root::name` runs.
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        use proc_macro2::TokenTree;
        fn walk(edges: &mut ModuleEdges, stream: proc_macro2::TokenStream) {
            let tokens: Vec<TokenTree> = stream.into_iter().collect();
            let mut i = 0;
            while i < tokens.len() {
                if let TokenTree::Group(group) = &tokens[i] {
                    walk(edges, group.stream());
                }
                if let TokenTree::Ident(root) = &tokens[i] {
                    let name = root.to_string();
                    let rooted = matches!(name.as_str(), "crate" | "super" | "self")
                        || name.starts_with("qsl_")
                        || name.starts_with("quire_");
                    let mut segments = vec![name];
                    let mut j = i + 1;
                    while let (
                        Some(TokenTree::Punct(a)),
                        Some(TokenTree::Punct(b)),
                        Some(TokenTree::Ident(next)),
                    ) = (tokens.get(j), tokens.get(j + 1), tokens.get(j + 2))
                    {
                        if a.as_char() == ':' && b.as_char() == ':' {
                            segments.push(next.to_string());
                            j += 3;
                        } else {
                            break;
                        }
                    }
                    if rooted && segments.len() > 1 {
                        edges.judge(&segments, root.span().start().line);
                    }
                    i = j.max(i + 1);
                    continue;
                }
                i += 1;
            }
        }
        walk(self, mac.tokens.clone());
        syn::visit::visit_macro(self, mac);
    }

    /// Inline paths: only a rooted path (`crate::`, `super::`, `self::`, or
    /// a crate name written out) is an edge; `Type::item` is not.
    fn visit_path(&mut self, path: &'ast syn::Path) {
        let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        let first = segments.first().map(String::as_str).unwrap_or_default();
        let rooted = path.leading_colon.is_some()
            || matches!(first, "crate" | "super" | "self")
            || first.starts_with("qsl_")
            || first.starts_with("quire_");
        if rooted && segments.len() > 1 {
            self.judge(&segments, path.segments[0].ident.span().start().line);
        }
        syn::visit::visit_path(self, path);
    }
}

#[trace("FR-091-AC-11", "TC-398")]
#[test]
fn value_module_has_edges_only_to_the_forms_core_and_the_lower_crates() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/value.rs");
    let source = std::fs::read_to_string(&path).expect("value.rs reads");
    let file = syn::parse_file(&source).expect("value.rs parses");
    let mut edges = ModuleEdges::default();
    edges.visit_file(&file);
    assert!(
        edges.found.is_empty(),
        "value.rs has an edge outside the forms core, qsl_cst, qsl_foundation and quire_exact: {:#?}",
        edges.found
    );
    assert!(
        source.contains("use qsl_cst::") && source.contains("use super::syntax::"),
        "the scan reads value.rs's real imports"
    );
}
