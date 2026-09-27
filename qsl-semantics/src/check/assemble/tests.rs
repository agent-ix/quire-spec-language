// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-091's assembler over real complete-V1 source: S1 (`qsl_cst::parse`),
//! S2 (`qsl_forms::build_unit`), then [`PackageDeclarations::assemble`].

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_forms::{build_unit, FormsLimits};
use qsl_foundation::{SourceIdentity, Span};
use quire_exact::{
    CardinalityBound, CollectionKind, CollectionType, EffectiveId, IeeeWidth, IntegerInterval,
    Presence, RoundingMode, ValueType,
};

use super::{model_field, AssemblyCause, AssemblyError, AssemblyRefusal, TopologyFault, Unmapped};
use crate::check::{
    CheckCause, CheckingLimits, Location, Origin, PackageDeclarations, TypeFormFault,
};
use crate::model::domain_package::{
    DomainPackageRecord, FieldMemberRecord, Multiplicity, NativeValueType, ValueTypeRef,
};
use crate::model::key::DeclarationKey;
use crate::value::declaration::FieldDeclaration;

const PROFILE_V: &str = "profile v = \"quire.value.complete/v1\" version \"1\" digest \
    \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n";

/// The unit text: the header, `profiles`, then `declarations`.
fn text(profiles: &str, declarations: &str) -> String {
    format!("language \"ix:native\" edition \"1-draft\";\n{profiles}{declarations}\n")
}

/// S1, S2 and the assembler over `text`, under owner (`authority`,
/// `identity`).
fn assemble_as(
    authority: &str,
    identity: &str,
    text: &str,
) -> Result<PackageDeclarations, AssemblyRefusal> {
    let parsed = qsl_cst::parse(
        SourceIdentity::new(authority, identity, "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed, FormsLimits::default()).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        Vec::new(),
        Vec::new(),
    )
}

fn assemble(declarations: &str) -> (String, Result<PackageDeclarations, AssemblyRefusal>) {
    let text = text(PROFILE_V, declarations);
    let assembled = assemble_as("a", "u", &text);
    (text, assembled)
}

fn errors(declarations: &str) -> (String, Vec<AssemblyError>) {
    let (text, assembled) = assemble(declarations);
    match assembled {
        Err(refusal) => (text, refusal.errors),
        Ok(_) => panic!("{declarations}: the assembler refuses"),
    }
}

/// The span of the last occurrence of `needle` in `text`.
fn last(text: &str, needle: &str) -> Span {
    let start = text.rfind(needle).expect("the needle is in the unit");
    Span {
        start,
        end: start + needle.len(),
    }
}

/// The span of the first occurrence of `needle` in `text` at or after
/// `from`.
fn after(text: &str, from: &str, needle: &str) -> Span {
    let from = text.find(from).expect("the anchor is in the unit");
    let start = from + text[from..].find(needle).expect("the needle follows");
    Span {
        start,
        end: start + needle.len(),
    }
}

fn int(lower: i64, upper: i64) -> ValueType {
    ValueType::Int(IntegerInterval::new(lower.into(), upper.into()).expect("a nonempty interval"))
}

#[trace("FR-091-AC-12", "TC-399")]
#[test]
fn the_assembler_resolves_aliases_and_signatures() {
    let (_, assembled) = assemble(
        "type Digit = Int[0, 9];\n\
         function inc using v(x: Digit): Int[0, 10] pure { x + 1 }\n\
         function two using v(): Int[0, 10] pure { inc(1) }",
    );
    let package = assembled.expect("the unit assembles");
    assert_eq!(package.aliases, [("Digit".to_owned(), int(0, 9))]);
    let names: Vec<&str> = package
        .functions
        .iter()
        .map(|function| function.name.as_str())
        .collect();
    assert_eq!(names, ["inc", "two"]);
    let (parameters, result) = package
        .resolved_signatures
        .get(0)
        .expect("inc's resolved signature");
    assert_eq!(parameters, &[("x".to_owned(), int(0, 9))]);
    assert_eq!(result, &int(0, 10));
    let (parameters, result) = package
        .resolved_signatures
        .get(1)
        .expect("two's resolved signature");
    assert!(parameters.is_empty());
    assert_eq!(result, &int(0, 10));
    package
        .check(CheckingLimits::default())
        .expect("the package checks");
}

#[trace(
    "FR-091-AC-14",
    "FR-091-AC-15",
    "FR-091-AC-16",
    "FR-091-AC-17",
    "TC-400"
)]
#[test]
fn the_assembler_reports_every_error() {
    let (text, found) = errors(
        "function f using v(x: Missing): Boolean pure { true }\n\
         function g using v(y: Absent): Boolean pure { true }",
    );
    assert_eq!(
        found,
        [
            AssemblyError {
                cause: AssemblyCause::UnresolvedTypeName {
                    name: "Missing".into()
                },
                span: last(&text, "Missing"),
            },
            AssemblyError {
                cause: AssemblyCause::UnresolvedTypeName {
                    name: "Absent".into()
                },
                span: last(&text, "Absent"),
            },
        ]
    );

    let (text, found) = errors(
        "type A = Int[0, 1];\ntype A = Int[0, 2];\n\
         function f using v(x: A): Boolean pure { true }",
    );
    let candidates = vec![
        after(&text, "type A = Int[0, 1]", "A"),
        after(&text, "type A = Int[0, 2]", "A"),
    ];
    assert_eq!(
        found,
        [
            AssemblyError {
                cause: AssemblyCause::AmbiguousTypeName {
                    name: "A".into(),
                    candidates: candidates.clone(),
                },
                span: candidates[1],
            },
            AssemblyError {
                cause: AssemblyCause::AmbiguousTypeName {
                    name: "A".into(),
                    candidates,
                },
                span: after(&text, "(x: A)", "A"),
            },
        ]
    );

    let (text, found) = errors(
        "function f using v(x: Int[9, 0]): Boolean pure { true }\n\
         function g using v(x: Rational[0, 1; 0, 5]): Boolean pure { true }\n\
         function h using v(x: Text[5, 1; nfc]): Boolean pure { true }",
    );
    assert_eq!(
        found,
        [
            AssemblyError {
                cause: AssemblyCause::IllFormedBounds(TypeFormFault::EmptyInterval),
                span: last(&text, "Int[9, 0]"),
            },
            AssemblyError {
                cause: AssemblyCause::IllFormedBounds(TypeFormFault::DenominatorBelowOne),
                span: last(&text, "Rational[0, 1; 0, 5]"),
            },
            AssemblyError {
                cause: AssemblyCause::IllFormedBounds(TypeFormFault::EmptyTextBounds),
                span: last(&text, "Text[5, 1; nfc]"),
            },
        ]
    );

    let (_, found) = errors("type A = B;\ntype B = A;");
    let [AssemblyError {
        cause: AssemblyCause::AliasCycle { edges },
        ..
    }] = found.as_slice()
    else {
        panic!("one alias cycle, not {found:?}");
    };
    assert_eq!(edges, &[("A".into(), "B".into()), ("B".into(), "A".into())]);
    assert_eq!(
        found[0].cause.catalog_code().to_string(),
        "invalid_package/definition-cycle"
    );
    let (_, found) = errors("type C = Option<C>;");
    assert_eq!(
        found
            .iter()
            .map(|error| error.cause.clone())
            .collect::<Vec<_>>(),
        [AssemblyCause::AliasCycle {
            edges: vec![("C".into(), "C".into())]
        }]
    );
}

/// The key `check` minted for the record or tuple named `name`.
fn composite_key(package: &PackageDeclarations, name: &str) -> quire_exact::NodeKey {
    package
        .types
        .composites()
        .find(|declaration| declaration.name() == name)
        .map(|declaration| declaration.key())
        .unwrap_or_else(|| panic!("{name} is declared"))
}

#[trace("FR-091-AC-18", "TC-401")]
#[test]
fn records_and_tuples_are_keyed_over_the_units_owner() {
    let text = text(
        PROFILE_V,
        "record Point { x: Int[0, 9]; y: Int[0, 9]; }\n\
         tuple Pair(Int[0, 9], Int[0, 9]);\n\
         function px using v(p: Point): Int[0, 9] pure { p.x }",
    );
    let keys = |identity: &str| {
        let package = assemble_as("a", identity, &text).expect("the unit assembles");
        let point = composite_key(&package, "Point");
        let pair = composite_key(&package, "Pair");
        assert_ne!(point, pair);
        let (parameters, _) = package
            .resolved_signatures
            .get(0)
            .expect("px's resolved signature");
        assert_eq!(parameters, &[("p".to_owned(), ValueType::Composite(point))]);
        let graph = package
            .check(CheckingLimits::default())
            .expect("the package checks");
        let px = graph.function_identity("px").expect("px is checked");
        (point, pair, px)
    };
    let first = keys("u");
    assert_eq!(keys("u"), first);
    let other = keys("w");
    assert_ne!(other.0, first.0);
    assert_ne!(other.1, first.1);
    assert_ne!(other.2, first.2);
}

#[trace("FR-091-AC-19", "FR-091-AC-23", "TC-405")]
#[test]
fn floating_types_are_admitted_and_reference_types_are_refused() {
    let admitted = |parameter: &str| {
        let (_, assembled) = assemble(&format!(
            "function f using v(x: {parameter}): Boolean pure {{ true }}"
        ));
        assembled.expect("a floating type is admitted")
    };
    // Resolution to `ValueType::Float` is TC-405's type_form test; here the
    // assembler admits every spelling without a floating-type refusal.
    admitted("Float64[nearest-even]");
    admitted("Float32[toward-zero]");
    admitted("Float64");
    let (text, found) = errors("function g using v(r: Reference<M::T>): Boolean pure { true }");
    assert_eq!(
        found,
        [AssemblyError {
            cause: AssemblyCause::UnresolvedTypeName {
                name: "M::T".into()
            },
            span: last(&text, "M::T"),
        }]
    );
}

#[trace("FR-091-AC-8", "TC-396")]
#[test]
fn nested_constructs_of_other_families_refuse_with_their_own_causes() {
    let checked = |declarations: &str| {
        let (_, assembled) = assemble(declarations);
        let refusals = assembled
            .expect("the unit assembles")
            .check(CheckingLimits::default())
            .expect_err("check refuses");
        assert!(
            refusals
                .iter()
                .all(|refusal| refusal.cause.code() != qsl_foundation::Code::UnsupportedConstruct),
            "no step-3 refusal is unsupported_construct: {refusals:?}"
        );
        refusals
    };
    for declarations in [
        "function f using v(x: Int[0, 9]): Boolean pure { pre(x) }",
        "function g using v(x: Int[0, 9]): Boolean pure decreases(pre(x)) { true }",
    ] {
        let refusals = checked(declarations);
        assert!(
            refusals.iter().any(|refusal| refusal.cause.code()
                == qsl_foundation::Code::WrongSnapshot
                && refusal.cause.cause() == Some("forbidden-pre-read")),
            "{declarations}: {refusals:?}"
        );
    }
    let refusals = checked("function f using v(x: Int[0, 9]): Boolean pure { deref(x) }");
    assert!(
        refusals.iter().any(|refusal| matches!(
            refusal.cause,
            CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch)
        )),
        "{refusals:?}"
    );
    let (_, found) =
        errors("function f using v(x: Int[0, 9]): Boolean pure { allInstances<M::T>(x) }");
    assert!(matches!(
        found.as_slice(),
        [AssemblyError {
            cause: AssemblyCause::UnresolvedTypeName { name },
            ..
        }] if name == "M::T"
    ));
    assert_eq!(
        found[0].cause.catalog_code().to_string(),
        "missing_declaration/missing-name"
    );
}

#[trace("FR-091-AC-21", "TC-406")]
#[test]
fn each_assembler_cause_has_its_catalog_code() {
    let span = Span { start: 0, end: 0 };
    let cases = [
        (
            AssemblyCause::UnresolvedTypeName { name: "X".into() },
            "missing_declaration/missing-name",
        ),
        (
            AssemblyCause::AmbiguousTypeName {
                name: "X".into(),
                candidates: vec![span],
            },
            "ambiguous_declaration/ambiguous-name",
        ),
        (
            AssemblyCause::IllFormedBounds(TypeFormFault::EmptyInterval),
            "ill_typed/type-mismatch",
        ),
        (
            AssemblyCause::FloatingType {
                width: IeeeWidth::Binary32,
                rounding: RoundingMode::Exact,
                profile: None,
            },
            "unknown_required_feature/unsupported-feature",
        ),
        (
            AssemblyCause::AliasCycle { edges: Vec::new() },
            "invalid_package/definition-cycle",
        ),
        (
            AssemblyCause::UndeclaredAlias { alias: "w".into() },
            "missing_declaration/missing-selection",
        ),
        (
            AssemblyCause::DuplicateAlias {
                alias: "v".into(),
                spans: vec![span],
            },
            "ambiguous_declaration/ambiguous-name",
        ),
        (
            AssemblyCause::UnsupportedStateClause { name: "X".into() },
            "unknown_required_feature/unsupported-feature",
        ),
    ];
    for (cause, code) in cases {
        assert_eq!(cause.catalog_code().to_string(), code, "{cause:?}");
    }
}

/// SR-722 FND-009: a state clause builds at S2 (FR-102) but has no checker
/// or declaration until FR-104 (QSL-277), so the assembler refuses it at its
/// own declaration span rather than silently dropping it (FND-001's fix).
/// This guards that refusal directly, since going back to a silent drop
/// would otherwise still leave every other gate green.
#[trace("FR-091-AC-21")]
#[test]
fn a_state_clause_refuses_unsupported_until_fr_104() {
    let clause = "invariant Foo using v on Config::ConfigVersion at current { true }";
    let (text, found) = errors(&format!(
        "{clause}\nfunction ok using v(): Boolean pure {{ true }}\n"
    ));
    assert_eq!(
        found,
        [AssemblyError {
            cause: AssemblyCause::UnsupportedStateClause { name: "Foo".into() },
            span: last(&text, clause),
        }]
    );
    assert_eq!(
        found[0].cause.catalog_code().to_string(),
        "unknown_required_feature/unsupported-feature"
    );
}

#[trace("FR-091-AC-22", "TC-412")]
#[test]
fn using_aliases_resolve_to_the_units_profile_selections() {
    let (unit_text, assembled) = assemble("function f using v(): Boolean pure { true }");
    let package = assembled.expect("f's alias names the profile selection v");
    assert_eq!(
        package.functions[0]
            .using()
            .map(|using| using.alias.as_str()),
        Some("v")
    );
    let selection = package
        .function_selections
        .get(&0)
        .expect("f's alias resolved to a recorded selection");
    assert_eq!(selection.alias, "v");
    assert_eq!(selection.definition.identity(), "quire.value.complete/v1");
    assert_eq!(selection.definition.version(), "1");
    assert_eq!(
        &unit_text[selection.span.start..selection.span.end],
        PROFILE_V.trim_end()
    );
    assert_eq!(
        &unit_text[selection.identity_span.start..selection.identity_span.end],
        "\"quire.value.complete/v1\""
    );
    let (text, found) = errors(
        "function f using v(): Boolean pure { true }\n\
         function g using w(): Boolean pure { true }",
    );
    assert_eq!(
        found,
        [AssemblyError {
            cause: AssemblyCause::UndeclaredAlias { alias: "w".into() },
            span: after(&text, "function g using", "w"),
        }]
    );

    let second = PROFILE_V.replace("aaaa", "bbbb");
    let text = text_with(
        &[PROFILE_V, &second],
        "function f using v(): Boolean pure { true }",
    );
    match assemble_as("a", "u", &text) {
        Err(refusal) => {
            let [AssemblyError {
                cause: AssemblyCause::DuplicateAlias { alias, spans },
                ..
            }] = refusal.errors.as_slice()
            else {
                panic!("one duplicate-alias error, not {refusal:?}");
            };
            assert_eq!(alias, "v");
            let spelled: Vec<&str> = spans
                .iter()
                .map(|span| &text[span.start..span.end])
                .collect();
            assert_eq!(spelled, [PROFILE_V.trim_end(), second.trim_end()]);
            assert_eq!(
                refusal.errors[0].cause.catalog_code().to_string(),
                "ambiguous_declaration/ambiguous-name"
            );
        }
        Ok(_) => panic!("a duplicate alias refuses"),
    }
}

fn text_with(profiles: &[&str], declarations: &str) -> String {
    text(&profiles.concat(), declarations)
}

/// Every path segment and `use` name in shipped items that is `qsl_cst`.
#[derive(Default)]
struct CstEdges(usize);

impl<'ast> syn::visit::Visit<'ast> for CstEdges {
    fn visit_path_segment(&mut self, segment: &'ast syn::PathSegment) {
        if segment.ident == "qsl_cst" {
            self.0 += 1;
        }
        syn::visit::visit_path_segment(self, segment);
    }

    fn visit_use_path(&mut self, path: &'ast syn::UsePath) {
        if path.ident == "qsl_cst" {
            self.0 += 1;
        }
        syn::visit::visit_use_path(self, path);
    }

    fn visit_use_name(&mut self, name: &'ast syn::UseName) {
        if name.ident == "qsl_cst" {
            self.0 += 1;
        }
    }
}

#[trace("FR-091-AC-20", "TC-402")]
#[test]
fn the_assembler_reads_no_cst() {
    use syn::visit::Visit as _;
    assert!(module_path!().starts_with("qsl_semantics::check::assemble"));
    for (name, source) in [
        ("assemble.rs", include_str!("../assemble.rs")),
        ("assemble/units.rs", include_str!("units.rs")),
    ] {
        let file = syn::parse_file(source).expect("an assembler file parses");
        let mut edges = CstEdges::default();
        edges.visit_file(&file);
        assert_eq!(
            edges.0, 0,
            "{name}: the assembler's shipped code names qsl_cst"
        );
    }
}

/// Whether a `use` tree names `qsl_cst` anywhere.
fn use_tree_names_cst(tree: &syn::UseTree) -> bool {
    match tree {
        syn::UseTree::Path(path) => path.ident == "qsl_cst" || use_tree_names_cst(&path.tree),
        syn::UseTree::Name(name) => name.ident == "qsl_cst",
        syn::UseTree::Rename(rename) => rename.ident == "qsl_cst",
        syn::UseTree::Glob(_) => false,
        syn::UseTree::Group(group) => group.items.iter().any(use_tree_names_cst),
    }
}

fn rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("a source directory reads") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.push(path);
        }
    }
}

/// The `qsl_cst` re-exports in `dir`: a non-private `use` of a path rooted at
/// `qsl_cst`, or a non-private `type` alias whose type names a `qsl_cst`
/// path. Returns `path:line` findings.
fn cst_re_exports(dir: &std::path::Path) -> Vec<String> {
    /// Whether a type names a path rooted at `qsl_cst`.
    struct NamesCst(bool);
    impl<'ast> syn::visit::Visit<'ast> for NamesCst {
        fn visit_path(&mut self, path: &'ast syn::Path) {
            if path
                .segments
                .first()
                .is_some_and(|first| first.ident == "qsl_cst")
            {
                self.0 = true;
            }
            syn::visit::visit_path(self, path);
        }
    }
    struct ReExports(Vec<usize>);
    impl<'ast> syn::visit::Visit<'ast> for ReExports {
        fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
            if !matches!(item.vis, syn::Visibility::Inherited) && use_tree_names_cst(&item.tree) {
                self.0.push(item.use_token.span.start().line);
            }
        }
        fn visit_item_type(&mut self, item: &'ast syn::ItemType) {
            let mut names = NamesCst(false);
            syn::visit::Visit::visit_type(&mut names, &item.ty);
            if !matches!(item.vis, syn::Visibility::Inherited) && names.0 {
                self.0.push(item.type_token.span.start().line);
            }
        }
    }
    use syn::visit::Visit as _;
    let mut files = Vec::new();
    rust_files(dir, &mut files);
    assert!(files.len() > 3, "the scan reads {}", dir.display());
    let mut found = Vec::new();
    for path in &files {
        let source = std::fs::read_to_string(path).expect("a source file reads");
        let file = syn::parse_file(&source).expect("a source file parses");
        let mut visitor = ReExports(Vec::new());
        visitor.visit_file(&file);
        found.extend(
            visitor
                .0
                .iter()
                .map(|line| format!("{}:{line}", path.display())),
        );
    }
    found
}

/// TC-402 step 2, re-exported types: neither `qsl-forms` (through which the
/// assembler reaches the forms types) nor `qsl-semantics` re-exports a
/// `qsl_cst` item, by `pub use` or by a `pub type` alias. With no such
/// re-export in either crate, the assembler has no path to a CST type
/// except `qsl_cst` itself, which `the_assembler_reads_no_cst` refuses.
#[trace("FR-091-AC-20", "TC-402")]
#[test]
fn no_qsl_cst_type_is_re_exported_to_the_assembler() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut found = Vec::new();
    for dir in [manifest.join("src"), manifest.join("../qsl-forms/src")] {
        found.extend(cst_re_exports(&dir));
    }
    assert!(found.is_empty(), "qsl_cst is re-exported: {found:?}");
}

/// TC-402 step 3: the assembler's test code reaches `qsl_cst` only to run
/// S1 (`parse`, `parse_source`) and to build the `Limits` argument of that
/// call.
#[trace("FR-091-AC-20", "TC-402")]
#[test]
fn the_assembler_tests_reach_qsl_cst_only_to_run_s1() {
    struct Reach(Vec<String>, usize);
    impl Reach {
        fn judge(&mut self, names: &[String]) {
            if names.first().is_some_and(|first| first == "qsl_cst") {
                self.1 += 1;
                match names.get(1).map(String::as_str) {
                    Some("parse" | "parse_source" | "Limits") => {}
                    _ => self.0.push(names.join("::")),
                }
            }
        }
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
                    self.judge(&full);
                }
                syn::UseTree::Rename(rename) => {
                    let mut full = prefix.clone();
                    full.push(rename.ident.to_string());
                    self.judge(&full);
                }
                syn::UseTree::Glob(_) => self.judge(prefix),
                syn::UseTree::Group(group) => {
                    for item in &group.items {
                        self.use_tree(item, prefix);
                    }
                }
            }
        }
        fn tokens(&mut self, stream: proc_macro2::TokenStream) {
            use proc_macro2::TokenTree;
            let tokens: Vec<TokenTree> = stream.into_iter().collect();
            let mut i = 0;
            while i < tokens.len() {
                match &tokens[i] {
                    TokenTree::Group(group) => self.tokens(group.stream()),
                    TokenTree::Ident(root) => {
                        let mut names = vec![root.to_string()];
                        let mut j = i + 1;
                        while let (
                            Some(TokenTree::Punct(a)),
                            Some(TokenTree::Punct(b)),
                            Some(TokenTree::Ident(next)),
                        ) = (tokens.get(j), tokens.get(j + 1), tokens.get(j + 2))
                        {
                            if a.as_char() != ':' || b.as_char() != ':' {
                                break;
                            }
                            names.push(next.to_string());
                            j += 3;
                        }
                        self.judge(&names);
                        i = j;
                        continue;
                    }
                    _ => {}
                }
                i += 1;
            }
        }
    }
    impl<'ast> syn::visit::Visit<'ast> for Reach {
        fn visit_path(&mut self, path: &'ast syn::Path) {
            let names: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
            self.judge(&names);
            syn::visit::visit_path(self, path);
        }
        fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
            self.use_tree(&item.tree, &mut Vec::new());
        }
        fn visit_macro(&mut self, mac: &'ast syn::Macro) {
            self.tokens(mac.tokens.clone());
            syn::visit::visit_macro(self, mac);
        }
    }
    use syn::visit::Visit as _;
    let file = syn::parse_file(include_str!("tests.rs")).expect("tests.rs parses");
    let mut reach = Reach(Vec::new(), 0);
    reach.visit_file(&file);
    assert!(reach.1 > 0, "the scan sees the S1 call");
    assert!(
        reach.0.is_empty(),
        "a test reaches qsl_cst beyond S1: {:?}",
        reach.0
    );
}

/// FR-096: a record read from the unit places its `declaration`
/// occurrence at its declared name.
#[trace("FR-096-AC-1", "FR-091-AC-18")]
#[test]
fn a_declared_type_resolves_to_its_names_region() {
    let text = text(
        PROFILE_V,
        "record Point { x: Int[0, 9]; }\n\
         function px using v(p: Point): Int[0, 9] pure { p.x }",
    );
    let package = assemble_as("a", "u", &text).expect("the unit assembles");
    let location = Location {
        origin: Origin::TypeDeclaration {
            name: "Point".into(),
        },
        path: Vec::new(),
    };
    let region = package.region(&location).expect("Point has a region");
    let span = last(&text, "Point {");
    assert_eq!(
        (region.start(), region.end()),
        (span.start as u64, span.start as u64 + 5)
    );
    let graph = package
        .check(CheckingLimits::default())
        .expect("the package checks");
    assert_eq!(graph.region(&location), Some(region));
}

/// A declared type name binds one declaration whichever kind comes first:
/// every declaration after the first is refused, in either order.
#[trace("FR-091-AC-15", "TC-400")]
#[test]
fn a_duplicate_type_name_is_refused_in_either_order() {
    for declarations in [
        "record P { x: Integer; }\ntype P = Boolean;",
        "type P = Boolean;\nrecord P { x: Integer; }",
    ] {
        let (text, found) = errors(declarations);
        let first = after(&text, declarations.lines().next().unwrap(), "P");
        let second = after(&text, declarations.lines().nth(1).unwrap(), "P");
        assert_eq!(
            found,
            [AssemblyError {
                cause: AssemblyCause::AmbiguousTypeName {
                    name: "P".into(),
                    candidates: vec![first, second],
                },
                span: second,
            }],
            "{declarations}"
        );
    }
}

/// A type form inside a body resolves at the assembler, so its ill-formed
/// bounds refuse there as a signature's do.
#[trace("FR-091-AC-16", "TC-400")]
#[test]
fn a_body_type_forms_bounds_refuse_at_the_assembler() {
    let (text, found) =
        errors("function f using v(x: Int[0, 9]): Boolean pure { convert<Int[9, 0]>(x) = x }");
    assert_eq!(
        found,
        [AssemblyError {
            cause: AssemblyCause::IllFormedBounds(TypeFormFault::EmptyInterval),
            span: last(&text, "Int[9, 0]"),
        }]
    );
}

/// The named types of `fold`, `count` and `sum` are refused at their own
/// names' spans, measure before body, in source order.
#[trace("FR-091-AC-14", "TC-400")]
#[test]
fn accumulator_type_names_are_refused_at_their_own_spans_in_source_order() {
    let (text, found) = errors(
        "function f using v(c: Integer): Integer pure decreases(sum<Early>(x in c: x)) \
         { count<Late>(x in c: true) + fold<Missing>(acc, y in c: acc, identity: 0) }",
    );
    let unresolved = |name: &str| AssemblyError {
        cause: AssemblyCause::UnresolvedTypeName { name: name.into() },
        span: last(&text, name),
    };
    assert_eq!(
        found,
        [
            unresolved("Early"),
            unresolved("Late"),
            unresolved("Missing")
        ]
    );
    assert_eq!(&text[found[2].span.start..found[2].span.end], "Missing");
}

/// Every FR-143 refusal is reported, not only the first.
#[trace("FR-091-AC-18", "TC-401")]
#[test]
fn every_record_refusal_is_reported() {
    let (text, found) = errors(
        "record P { x: Integer; x: Integer; }\n\
         record Q { y: Integer; y: Integer; }",
    );
    let names: Vec<&str> = found
        .iter()
        .map(|error| &text[error.span.start..error.span.end])
        .collect();
    assert_eq!(names, ["P", "Q"]);
    for error in &found {
        assert!(
            matches!(
                &error.cause,
                AssemblyCause::InvalidTypeDeclaration(invalid)
                    if matches!(invalid.cause, crate::value::declaration::DeclarationCause::DuplicateMember(_))
            ),
            "{error:?}"
        );
    }
}

// QSL-252: `model_field` follows QSpec's own Presence row
// (`model-complete.md`:158) and FR-322's "Model-owned members" step 4:
// multiplicity alone gives the value type, and `presence` alone -- never a
// lower bound of `0` -- gives `Option`.

/// A native-`Integer` field member at `multiplicity`/`presence`, with no
/// redefinition. `model_field` never consults `records`/`identities` for a
/// native value type, so [`model_field_of`] passes empty maps.
fn presence_field(multiplicity: Multiplicity, presence: Presence) -> FieldMemberRecord {
    FieldMemberRecord {
        key: DeclarationKey::fixture("model.A/f"),
        owner: DeclarationKey::fixture("model.A"),
        value_type: ValueTypeRef::Native(NativeValueType::Integer),
        multiplicity,
        presence,
        subsets: Vec::new(),
        redefines: None,
    }
}

fn model_field_of(
    multiplicity: Multiplicity,
    presence: Presence,
) -> Result<FieldDeclaration, Unmapped> {
    let field = presence_field(multiplicity, presence);
    let records: BTreeMap<&DeclarationKey, &DomainPackageRecord> = BTreeMap::new();
    let identities: BTreeMap<DeclarationKey, EffectiveId> = BTreeMap::new();
    model_field(&field, &records, &identities)
}

fn mult(lower: u64, upper: Option<u64>, ordered: bool, unique: bool) -> Multiplicity {
    Multiplicity {
        lower,
        upper,
        ordered,
        unique,
    }
}

fn bounded(kind: CollectionKind, lower: u64, upper: u64) -> ValueType {
    ValueType::collection(CollectionType::new(
        kind,
        ValueType::Integer,
        Some(CardinalityBound::new(lower, upper).unwrap()),
    ))
}

/// FR-322's table: `[1, 1]` gives the element type `E` outright, whatever
/// `ordered`/`unique` says.
#[trace("TC-443", "FR-056-AC-10")]
#[test]
fn one_one_required_gives_the_element_type() {
    let declaration = model_field_of(mult(1, Some(1), false, true), Presence::Required).unwrap();
    assert_eq!(declaration.value_type(), &ValueType::Integer);
    assert_eq!(declaration.presence(), Presence::Required);
}

/// A lower bound of `0` makes an empty collection legal; it never makes a
/// field optional (QSpec's Presence row). `[0, 1]` required is the bounded
/// collection `K<E>[0, 1]`, never `Option<E>`.
#[trace("TC-443", "FR-056-AC-10")]
#[test]
fn zero_one_required_gives_the_bounded_collection_not_option() {
    let declaration = model_field_of(mult(0, Some(1), false, true), Presence::Required).unwrap();
    assert_eq!(
        declaration.value_type(),
        &bounded(CollectionKind::Set, 0, 1)
    );
    assert_eq!(declaration.presence(), Presence::Required);
}

#[trace("TC-443", "FR-056-AC-10")]
#[test]
fn zero_five_required_gives_the_bounded_collection() {
    let declaration = model_field_of(mult(0, Some(5), false, true), Presence::Required).unwrap();
    assert_eq!(
        declaration.value_type(),
        &bounded(CollectionKind::Set, 0, 5)
    );
    assert_eq!(declaration.presence(), Presence::Required);
}

/// `[0, unbounded]` gives the unbounded collection `K<E>`, with no
/// `collection_bounds` node -- distinct from an unbounded upper with a
/// lower bound above `0`, which has no kernel type at all.
#[trace("TC-443", "FR-056-AC-10")]
#[test]
fn zero_unbounded_gives_the_unbounded_collection() {
    let declaration = model_field_of(mult(0, None, false, true), Presence::Required).unwrap();
    assert_eq!(
        declaration.value_type(),
        &ValueType::collection(CollectionType::new(
            CollectionKind::Set,
            ValueType::Integer,
            None
        ))
    );
    assert_eq!(declaration.presence(), Presence::Required);
}

/// An unbounded upper bound with a lower bound above `0` has no kernel
/// type at all (FR-322's table); the assembler refuses it. QSpec's own
/// merged member-type vectors (`model-member-type-vectors.json` MA-07) pin
/// this same refusal at `[1, unbounded]`.
#[trace("TC-443", "FR-056-AC-10")]
#[test]
fn one_unbounded_refuses() {
    let outcome = model_field_of(mult(1, None, false, true), Presence::Required);
    assert!(matches!(outcome, Err(Unmapped::Unsupported)), "{outcome:?}");
}

/// Optional presence over `[1, 1]` leaves `model_field`'s own value type as
/// the plain element `E` -- `check`'s `attribute`/`field` readers wrap an
/// `Optional`-presence declaration's value type in `Option` from
/// `presence()` alone (`check/check.rs`'s `attribute`/`field`), exactly as
/// they already do for every other `FieldDeclaration`, giving FR-322's
/// table's `Option<E>` without `model_field` baking `Option` in itself.
#[trace("TC-443", "FR-056-AC-10")]
#[test]
fn optional_presence_over_one_one_is_optional_not_multiplicity_driven() {
    let declaration = model_field_of(mult(1, Some(1), false, true), Presence::Optional).unwrap();
    assert_eq!(declaration.value_type(), &ValueType::Integer);
    assert_eq!(declaration.presence(), Presence::Optional);
}

/// Optional presence composes with a bounded collection exactly the same
/// way: the collection type is unaffected, and only `presence()` carries
/// the `Option` wrapping `check`'s readers apply at read time.
#[trace("TC-443", "FR-056-AC-10")]
#[test]
fn optional_presence_over_zero_three_wraps_the_collection_not_the_bound() {
    let declaration = model_field_of(mult(0, Some(3), false, true), Presence::Optional).unwrap();
    assert_eq!(
        declaration.value_type(),
        &bounded(CollectionKind::Set, 0, 3)
    );
    assert_eq!(declaration.presence(), Presence::Optional);
}

#[trace("TC-443", "FR-056-AC-10")]
#[test]
fn optional_presence_over_zero_one_wraps_the_collection_not_the_bound() {
    let declaration = model_field_of(mult(0, Some(1), false, true), Presence::Optional).unwrap();
    assert_eq!(
        declaration.value_type(),
        &bounded(CollectionKind::Set, 0, 1)
    );
    assert_eq!(declaration.presence(), Presence::Optional);
}

/// Each `ordered`/`unique` combination names its own collection kind
/// (FR-322's table), over a finite multiplicity distinct from the
/// `[1, 1]`/`[0, 1]` special cases above.
#[trace("TC-443", "FR-056-AC-10")]
#[test]
fn ordered_and_unique_selects_ordered_set() {
    let declaration = model_field_of(mult(2, Some(4), true, true), Presence::Required).unwrap();
    assert_eq!(
        declaration.value_type(),
        &bounded(CollectionKind::OrderedSet, 2, 4)
    );
}

#[trace("TC-443", "FR-056-AC-10")]
#[test]
fn ordered_and_not_unique_selects_sequence() {
    let declaration = model_field_of(mult(2, Some(4), true, false), Presence::Required).unwrap();
    assert_eq!(
        declaration.value_type(),
        &bounded(CollectionKind::Sequence, 2, 4)
    );
}

#[trace("TC-443", "FR-056-AC-10")]
#[test]
fn unordered_and_unique_selects_set() {
    let declaration = model_field_of(mult(2, Some(4), false, true), Presence::Required).unwrap();
    assert_eq!(
        declaration.value_type(),
        &bounded(CollectionKind::Set, 2, 4)
    );
}

#[trace("TC-443", "FR-056-AC-10")]
#[test]
fn unordered_and_not_unique_selects_bag() {
    let declaration = model_field_of(mult(2, Some(4), false, false), Presence::Required).unwrap();
    assert_eq!(
        declaration.value_type(),
        &bounded(CollectionKind::Bag, 2, 4)
    );
}

// ----------------------------------------------------------------------
// QSL-275: enums and predicates (TC-481)
// ----------------------------------------------------------------------

/// FR-091 vectors N1 to N4, the SHA-256 of the RFC 8785 preimage bytes.
const N1: &str = "e5e7c1d5b51c76e84c928b616d266d45a570e8211404c302b47dbec62ae00d27";
const N2: &str = "499f4989da1b6790fcda8c1e6e64ed04041d8cd304f336133c48f58c424d65ec";
const N3: &str = "0757650a7514f2f86e2101a0d02e5055d1152c36fc7e01ea2dfc8eabc21dae62";
const N4: &str = "239e86987c45808a71cb0d23e7b25f8f70d6adb426288280c7eaf566f0f145e9";

const STATUS: &str = "ordered enum Status { READY, DONE }\n";

fn binding<'a>(package: &'a PackageDeclarations, name: &str) -> &'a crate::check::EnumBinding {
    package
        .enums
        .iter()
        .find(|binding| binding.name == name)
        .unwrap_or_else(|| panic!("{name} is declared"))
}

fn cases(binding: &crate::check::EnumBinding) -> Vec<&str> {
    binding.members.iter().map(|member| member.case()).collect()
}

#[trace("FR-091-AC-27", "TC-481")]
#[test]
fn enums_are_admitted_with_nominal_keys_over_the_units_owner() {
    let (text, assembled) =
        assemble("ordered enum Status { READY, DONE }\nenum Color { RED, BLUE = \"Blue\" }");
    let package = assembled.expect("the unit assembles");
    let names: Vec<&str> = package.enums.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(names, ["Status", "Color"]);
    let status = binding(&package, "Status");
    assert_eq!(status.declaration.key().to_string(), N1);
    assert_eq!(cases(status), ["READY", "DONE"]);
    assert_eq!(status.members[0].member().to_string(), N2);
    let color = binding(&package, "Color");
    assert_eq!(color.declaration.key().to_string(), N3);
    assert_eq!(cases(color), ["BLUE", "RED"]);
    assert_eq!(
        package.declared_type_spans.get("Status"),
        Some(&last(&text, "Status"))
    );
    assert_eq!(
        package.declared_type_spans.get("Color"),
        Some(&last(&text, "Color"))
    );

    // Neither a display string nor the order of an unordered enum's cases
    // enters its key.
    let (_, other) = assemble("enum Color { RED, BLUE }");
    let other = other.expect("the unit assembles");
    assert_eq!(binding(&other, "Color").declaration.key().to_string(), N3);

    // The owner enters the key.
    let moved =
        assemble_as("a", "w", &text_with(&[PROFILE_V], STATUS)).expect("the unit assembles");
    assert_eq!(binding(&moved, "Status").declaration.key().to_string(), N4);
}

#[trace("FR-091-AC-28", "TC-481")]
#[test]
fn enum_types_and_members_check_and_a_case_the_enum_lacks_refuses() {
    let checked = |declarations: &str| {
        let (_, assembled) = assemble(declarations);
        assembled
            .expect("the unit assembles")
            .check(CheckingLimits::default())
    };
    let package = assemble(&format!(
        "{STATUS}\
         function isReady using v(s: Status): Boolean pure {{ s = Status::READY }}\n\
         function ok using v(): Boolean pure {{ isReady(Status::READY) }}\n\
         function later using v(): Boolean pure {{ Status::READY < Status::DONE }}"
    ))
    .1
    .expect("the unit assembles");
    let shape = binding(&package, "Status").shape();
    let (parameters, _) = package
        .resolved_signatures
        .get(0)
        .expect("isReady's resolved signature");
    assert_eq!(parameters, &[("s".to_owned(), ValueType::Enum(shape))]);
    package
        .check(CheckingLimits::default())
        .expect("the package checks");

    let refusals = checked(
        "enum Color { RED, BLUE }\n\
         function bad using v(): Boolean pure { Color::RED < Color::BLUE }",
    )
    .expect_err("an unordered enum has no order");
    assert!(
        refusals
            .iter()
            .any(|refusal| refusal.cause.code() == qsl_foundation::Code::IllTyped),
        "{refusals:?}"
    );

    let refusals = checked(&format!(
        "{STATUS}function gone using v(): Boolean pure {{ Status::GONE = Status::READY }}"
    ))
    .expect_err("Status has no GONE");
    assert!(
        refusals.iter().any(|refusal| {
            refusal.cause.code() == qsl_foundation::Code::MissingDeclaration
                && refusal.cause.cause() == Some("missing-name")
                && matches!(&refusal.cause, CheckCause::MissingName(name) if name == "Status::GONE")
        }),
        "{refusals:?}"
    );
}

#[trace("FR-091-AC-29", "TC-481")]
#[test]
fn enum_and_type_name_errors_are_all_reported() {
    let (text, found) = errors(
        "enum E { A, B, A }\n\
         enum F { X }\n\
         record F { y: Boolean; }\n\
         function f using v(p: F): Boolean pure { true }\n\
         function g using v(p: Shade): Boolean pure { true }",
    );
    let first_a = after(&text, "enum E", "A");
    let second_a = last(&text, "A }");
    let second_a = Span {
        start: second_a.start,
        end: second_a.start + 1,
    };
    let duplicate = found
        .iter()
        .find(|error| matches!(error.cause, AssemblyCause::DuplicateEnumMember { .. }))
        .expect("a duplicate-enum-member error");
    assert_eq!(
        duplicate.cause,
        AssemblyCause::DuplicateEnumMember {
            enumeration: "E".to_owned(),
            case: "A".to_owned(),
            spans: vec![first_a, second_a],
        }
    );
    assert_eq!(
        duplicate.cause.catalog_code().to_string(),
        "ambiguous_declaration/ambiguous-name"
    );
    let enum_f = after(&text, "enum F", "F");
    let record_f = after(&text, "record F", "F");
    let type_form = after(&text, "function f", "F)");
    let type_form = Span {
        start: type_form.start,
        end: type_form.start + 1,
    };
    assert!(
        found.iter().any(|error| error.span == type_form
            && error.cause
                == AssemblyCause::AmbiguousTypeName {
                    name: "F".to_owned(),
                    candidates: vec![enum_f, record_f],
                }),
        "{found:?}"
    );
    assert!(
        found.iter().any(|error| matches!(
            &error.cause,
            AssemblyCause::UnresolvedTypeName { name } if name == "Shade"
        )),
        "{found:?}"
    );
}

#[trace("FR-091-AC-21", "TC-406")]
#[test]
fn the_enum_causes_have_their_catalog_codes() {
    let span = Span { start: 0, end: 0 };
    assert_eq!(
        AssemblyCause::DuplicateEnumMember {
            enumeration: "E".into(),
            case: "A".into(),
            spans: vec![span],
        }
        .catalog_code()
        .to_string(),
        "ambiguous_declaration/ambiguous-name"
    );
    let fault = crate::value::semantic_node::InvalidSemanticGraph {
        cause: crate::value::semantic_node::SemanticGraphCause::StaleKey,
    };
    assert_eq!(
        AssemblyCause::NominalAdmission(fault)
            .catalog_code()
            .to_string(),
        "runtime_invariant/established-invariant-broken"
    );
}

#[trace("FR-091-AC-30", "TC-481")]
#[test]
fn a_predicate_assembles_as_a_function_of_kind_predicate() {
    let package = assemble(
        "predicate Positive using v(x: Int[0, 9]): Boolean { x > 0 }\n\
         function three using v(): Boolean pure { Positive(3) }",
    )
    .1
    .expect("the unit assembles");
    let kinds: Vec<(&str, qsl_forms::DeclarationKind)> = package
        .functions
        .iter()
        .map(|function| (function.name.as_str(), function.kind()))
        .collect();
    assert_eq!(
        kinds,
        [
            ("Positive", qsl_forms::DeclarationKind::Predicate),
            ("three", qsl_forms::DeclarationKind::Function)
        ]
    );
    let (parameters, result) = package
        .resolved_signatures
        .get(0)
        .expect("Positive's resolved signature");
    assert_eq!(parameters, &[("x".to_owned(), int(0, 9))]);
    assert_eq!(result, &ValueType::Boolean);
    assert_eq!(package.function_selections[&0].alias, "v");
    package
        .check(CheckingLimits::default())
        .expect("the package checks");

    let (text, found) = errors("predicate Q using w(x: Boolean): Boolean { x }");
    assert!(matches!(
        found.as_slice(),
        [AssemblyError {
            cause: AssemblyCause::UndeclaredAlias { alias },
            span,
        }] if alias == "w" && *span == after(&text, "predicate Q", "w")
    ));

    let refusals = assemble("predicate R using v(x: Boolean): Boolean { R(x) }")
        .1
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect_err("a recursive predicate has no measure");
    assert!(
        refusals.iter().any(|refusal| matches!(
            refusal.cause,
            CheckCause::UnprovedDecrease {
                obligation: crate::check::MeasureObligation::MissingMeasure,
                ..
            }
        )),
        "{refusals:?}"
    );
}

#[trace("FR-092-AC-13", "TC-481")]
#[test]
fn predicate_and_enum_nodes_lower_to_their_fr_092_forms() {
    use crate::check::{NodeTag, Owner, SourceOwner};

    let predicate = assemble("predicate Positive using v(x: Int[0, 9]): Boolean { x > 0 }")
        .1
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect("the package checks");
    let function = assemble("function Positive using v(x: Int[0, 9]): Boolean pure { x > 0 }")
        .1
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect("the package checks");
    let node_of = |graph: &crate::check::CheckedGraph| {
        let key = graph
            .function_identity("Positive")
            .expect("Positive is checked");
        graph
            .semantic_graph()
            .node(key)
            .expect("the node is lowered")
            .clone()
    };
    let (predicate, function) = (node_of(&predicate), node_of(&function));
    assert_eq!(predicate.node_tag(), NodeTag::Function);
    assert_eq!(predicate.semantic_form(), "predicate");
    let names: Vec<&str> = predicate
        .declaration()
        .expect("a declaration")
        .iter()
        .map(|segment| segment.as_str())
        .collect();
    assert_eq!(names, ["Positive"]);
    assert_eq!(
        predicate.owner(),
        Some(&Owner::Source(
            SourceOwner::new("a", "u").expect("an owner")
        ))
    );
    assert_eq!(function.semantic_form(), "pure_function");
    assert_ne!(predicate.key(), function.key());

    let graph = assemble(STATUS)
        .1
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect("the package checks");
    let by_key = |key: &str| {
        graph
            .semantic_graph()
            .nodes()
            .find(|node| node.key().to_string() == key)
            .unwrap_or_else(|| panic!("the checked graph holds {key}"))
    };
    let declaration = by_key(N1);
    assert_eq!(declaration.node_tag(), NodeTag::ScalarType);
    assert_eq!(declaration.semantic_form(), "enum");
    let member = by_key(N2);
    assert_eq!(member.node_tag(), NodeTag::Value);
    assert_eq!(member.semantic_form(), "enum_value");
}

#[trace("FR-091-AC-27", "TC-481")]
#[test]
fn the_preimage_constructors_apply_the_reader_checks() {
    use crate::value::enumeration::{EnumDeclarationPreimage, EnumMemberPreimage};
    use crate::value::semantic_node::{NodeOwner, OwnerSubject};

    let owner = || {
        NodeOwner::Source(OwnerSubject {
            authority: "a".to_owned(),
            identity: "u".to_owned(),
        })
    };
    let name = || vec!["E".to_owned()];
    let cases = |cases: &[&str]| cases.iter().map(|case| (*case).to_owned()).collect();
    assert!(EnumDeclarationPreimage::new(owner(), name(), false, cases(&["A", "B"])).is_ok());
    for refused in [
        EnumDeclarationPreimage::new(owner(), name(), false, cases(&[])),
        EnumDeclarationPreimage::new(owner(), name(), false, cases(&["A", "A"])),
        EnumDeclarationPreimage::new(owner(), name(), false, cases(&["1x"])),
        EnumDeclarationPreimage::new(owner(), Vec::new(), false, cases(&["A"])),
    ] {
        assert!(refused.is_err());
    }
    let key = quire_exact::NodeKey::from_digest([7; 32]);
    assert!(EnumMemberPreimage::new(key, "A").is_ok());
    assert!(EnumMemberPreimage::new(key, "not an identifier").is_err());
}

// ----------------------------------------------------------------------
// QSL-275: dimensions and units (TC-483)
// ----------------------------------------------------------------------

/// FR-091 vectors Q1 to Q10.
const Q: [&str; 10] = [
    "ff856699d6710bab9b9a893db4b7cf079f203258f543f02f990191d0e4834028",
    "e277e2dae0f9bae883f80b677af21b3ab95e9ae4c5406864d1dd307d0024f927",
    "3843d2d5c9906e126f8310cf148fa9b8f7366e0d5bdb762db7b8567e0968c10e",
    "7d3bb56f6ff61b76783c608273c6c9b773376958449b3b1ace348ff582fa08d7",
    "3e5ab5ca1538a74d998648ec764c1dc6081acbc0dcdce0c8e81011fd7603730d",
    "4c5aa30d2878d6197f7292492907082ccc857c60da88979b0a99eee5a5aecb0b",
    "700256858942ebde86f19abdfa12423cfc5f7e58d830ba3c5903c2d189cd2edd",
    "83b004fb06bbc9fbe56999ac263d2f3eafe87a25c3182c90acbf1131c0d260ee",
    "5cc7dfb7cedf23b3dff8012718e3cea37586ed90c6b824affb6fafee10db49db",
    "12b65987f369f8908be0d690d372e2b3fc3bae7623672a83a942a950663c67fa",
];

const Q_SOURCES: &str = "dimension Length;\n\
    dimension Time;\n\
    dimension Speed = Length / Time;\n\
    dimension Accel = Speed / Time;\n\
    dimension Temperature;\n\
    unit m : Length = rational(1, 1);\n\
    unit km : Length = rational(2000, 2) * m;\n\
    unit cm : Length = decimal(1, 2) * m;\n\
    unit K : Temperature = rational(1, 1);\n\
    unit C : Temperature = rational(1, 1) * K + decimal(27315, 2);";

fn key_of(hex: &str, graph: &crate::value::unit::UnitGraph) -> quire_exact::NodeKey {
    graph
        .units()
        .map(|unit| unit.key())
        .find(|key| key.to_string() == hex)
        .unwrap_or_else(|| panic!("the graph holds unit {hex}"))
}

/// `Q_SOURCES` with its lines reversed: every declaration comes before
/// the ones it depends on.
fn reversed_sources() -> String {
    Q_SOURCES.split('\n').rev().collect::<Vec<_>>().join("\n")
}

#[trace("FR-091-AC-32", "TC-483")]
#[test]
fn dependencies_are_keyed_before_their_dependents_whatever_the_source_order() {
    let (_, assembled) = assemble(&reversed_sources());
    let package = assembled.expect("the reversed unit assembles");
    let graph = &package.units;
    for hex in &Q[5..] {
        key_of(hex, graph);
    }
    for hex in &Q[..5] {
        assert!(package
            .nominal_spans
            .keys()
            .any(|key| key.to_string() == *hex));
    }
    let km = graph.unit(key_of(Q[6], graph)).expect("km");
    assert_eq!(km.root(), key_of(Q[5], graph));
}

#[trace("FR-091-AC-32", "TC-483")]
#[test]
fn dimensions_and_units_are_admitted_with_the_vector_keys() {
    let (text, assembled) = assemble(Q_SOURCES);
    let package = assembled.expect("the unit assembles");
    let graph = &package.units;
    // Units Q6 to Q10 are admitted under their vector keys.
    let km = graph.unit(key_of(Q[6], graph)).expect("km");
    assert_eq!(km.canonical().scale().to_string(), "1000/1");
    let cm = graph.unit(key_of(Q[7], graph)).expect("cm");
    assert_eq!(cm.canonical().scale().to_string(), "1/100");
    let celsius = graph.unit(key_of(Q[9], graph)).expect("C");
    assert_eq!(celsius.canonical().offset().to_string(), "5463/20");
    assert!(celsius.is_affine());
    assert!(!km.is_affine());
    let metre = graph.unit(key_of(Q[5], graph)).expect("m");
    assert_eq!(km.root(), metre.key());
    key_of(Q[8], graph);
    // Dimensions Q1 to Q5: each key is a dimension the units name, and the
    // derived ones normalize to the base terms of the vectors.
    let dimension = |hex: &str| {
        let key = package
            .nominal_spans
            .keys()
            .find(|key| key.to_string() == hex)
            .copied()
            .unwrap_or_else(|| panic!("nominal_spans holds {hex}"));
        graph
            .dimension(key)
            .unwrap_or_else(|| panic!("the graph holds dimension {hex}"))
    };
    let exponents = |hex: &str| -> Vec<(String, String)> {
        dimension(hex)
            .exponents()
            .map(|(base, exponent)| (base.to_string(), exponent.to_string()))
            .collect()
    };
    assert_eq!(exponents(Q[0]), [(Q[0].to_owned(), "1".to_owned())]);
    assert_eq!(exponents(Q[1]), [(Q[1].to_owned(), "1".to_owned())]);
    assert_eq!(
        exponents(Q[2]),
        [
            (Q[1].to_owned(), "-1".to_owned()),
            (Q[0].to_owned(), "1".to_owned())
        ]
    );
    assert_eq!(
        exponents(Q[3]),
        [
            (Q[1].to_owned(), "-2".to_owned()),
            (Q[0].to_owned(), "1".to_owned())
        ]
    );
    assert_eq!(exponents(Q[4]), [(Q[4].to_owned(), "1".to_owned())]);
    // `nominal_spans` holds the span of each declared name by its key.
    assert_eq!(package.nominal_spans.len(), 10);
    for (hex, name) in Q.iter().zip([
        "Length",
        "Time",
        "Speed",
        "Accel",
        "Temperature",
        "m",
        "km",
        "cm",
        "K",
        "C",
    ]) {
        let span = package
            .nominal_spans
            .iter()
            .find(|(key, _)| key.to_string() == *hex)
            .map(|(_, span)| *span)
            .unwrap_or_else(|| panic!("a span for {name}"));
        assert_eq!(&text[span.start..span.end], name);
    }
    // A unit with no dimension or unit form has the empty graph.
    let empty = assemble("type Digit = Int[0, 9];").1.expect("assembles");
    assert_eq!(empty.units.units().count(), 0);
    assert!(empty.nominal_spans.is_empty());
}

#[trace("FR-091-AC-32", "TC-483")]
#[test]
fn the_decimal_scale_bound_refuses_with_a_work_budget_limit() {
    let assemble_with = |scale: u64| {
        let text = text(
            PROFILE_V,
            &format!(
                "dimension Length; unit m : Length = rational(1, 1);\n\
                 unit c : Length = decimal(1, {scale}) * m;"
            ),
        );
        let parsed = qsl_cst::parse(
            SourceIdentity::new("a", "u", "git", "1"),
            "unit.native",
            text.as_bytes(),
            qsl_cst::Limits::default(),
        )
        .expect("S1 reads the unit");
        let unit = build_unit(&parsed, FormsLimits::default()).expect("S2 builds the unit");
        let assembled = PackageDeclarations::assemble_with_limits(
            parsed.source().reference().clone(),
            unit,
            Vec::new(),
            Vec::new(),
            crate::check::AssemblyLimits { decimal_scale: 4 },
        );
        (text, assembled)
    };
    let (text, assembled) = assemble_with(5);
    let refusal = assembled.expect_err("scale 5 is above the bound 4");
    let [error] = refusal.errors.as_slice() else {
        panic!("one error, not {:?}", refusal.errors)
    };
    let AssemblyCause::DecimalScaleLimit(limit) = &error.cause else {
        panic!("a decimal-scale limit, not {:?}", error.cause)
    };
    assert_eq!(limit.configured_bound(), 4);
    assert_eq!(limit.actual(), 5);
    assert_eq!(error.span, last(&text, "decimal(1, 5)"));
    assert_eq!(
        error.cause.catalog_code().to_string(),
        "stage_limit_exceeded/work-budget-exceeded"
    );
    assert!(assemble_with(4).1.is_ok());
}

#[trace("FR-091-AC-33", "TC-483")]
#[test]
fn a_unit_name_is_not_a_type() {
    let (_, found) = errors(
        "dimension Length;\n\
         unit m : Length = rational(1, 1);\n\
         function f using v(x: m): Boolean pure { true }",
    );
    assert!(matches!(
        found.as_slice(),
        [AssemblyError {
            cause: AssemblyCause::UnresolvedTypeName { name },
            ..
        }] if name == "m"
    ));
}

#[trace("FR-091-AC-34", "TC-483")]
#[test]
fn every_dimension_and_unit_source_error_is_reported() {
    let (text, found) = errors(
        "dimension Length;\n\
         dimension Mass;\n\
         dimension Mass;\n\
         dimension Area = Width^2;\n\
         dimension P = Q;\n\
         dimension Q = P;\n\
         unit a : Length = rational(1, 0);\n\
         unit b : Length = rational(2, 1) * c;\n\
         unit c : Length = rational(1, 2) * b;",
    );
    let codes: Vec<String> = found
        .iter()
        .map(|error| error.cause.catalog_code().to_string())
        .collect();
    assert_eq!(found.len(), 5, "{found:?}");
    let has = |predicate: &dyn Fn(&AssemblyCause) -> bool| {
        found.iter().any(|error| predicate(&error.cause))
    };
    let mass_span = |start: usize| Span {
        start: start + "dimension ".len(),
        end: start + "dimension Mass".len(),
    };
    let first = text.find("dimension Mass").expect("first Mass");
    let second = text.rfind("dimension Mass").expect("second Mass");
    assert!(has(&|cause| *cause
        == AssemblyCause::DuplicateQuantityName {
            name: "Mass".to_owned(),
            spans: vec![mass_span(first), mass_span(second)],
        }));
    assert!(has(&|cause| matches!(
        cause,
        AssemblyCause::UnresolvedQuantityName { name } if name == "Width"
    )));
    let edge = |a: &str, b: &str| (a.to_owned(), b.to_owned());
    assert!(has(&|cause| *cause
        == AssemblyCause::QuantityCycle {
            edges: vec![edge("P", "Q"), edge("Q", "P")]
        }));
    assert!(has(&|cause| *cause
        == AssemblyCause::QuantityCycle {
            edges: vec![edge("b", "c"), edge("c", "b")]
        }));
    assert!(found
        .iter()
        .any(|error| error.cause == AssemblyCause::ZeroDenominator
            && error.span == last(&text, "rational(1, 0)")));
    for expected in [
        "ambiguous_declaration/ambiguous-name",
        "missing_declaration/missing-name",
        "invalid_package/definition-cycle",
        "undefined_expression/unproved-nonzero",
    ] {
        assert!(
            codes.iter().any(|code| code == expected),
            "{expected}: {codes:?}"
        );
    }
}

#[trace("FR-091-AC-35", "TC-483")]
#[test]
fn each_unit_graph_topology_error_refuses_alone() {
    use super::TopologyFault as Fault;
    for (source, fault, declarations) in [
        (
            "dimension L; dimension N = L / L;",
            Fault::EmptyDerivedDimension,
            vec!["N"],
        ),
        (
            "dimension L; unit r : L = rational(1, 1); unit z : L = rational(0, 1) * r;",
            Fault::ZeroScale,
            vec!["z"],
        ),
        (
            "dimension L; unit r : L = rational(2, 1);",
            Fault::NonIdentityRoot,
            vec!["r"],
        ),
        (
            "dimension L; dimension T; unit r : L = rational(1, 1); \
             unit s : T = rational(1, 1) * r;",
            Fault::CrossDimensionTarget,
            vec!["s", "r"],
        ),
        (
            "dimension L; unit r : L = rational(1, 1); unit q : L = rational(1, 1);",
            Fault::TwoRoots,
            vec!["L", "r", "q"],
        ),
    ] {
        let (_, found) = errors(source);
        let [error] = found.as_slice() else {
            panic!("{source}: one error, not {found:?}")
        };
        assert_eq!(
            error.cause,
            AssemblyCause::UnitGraphTopology {
                fault,
                declarations: declarations.into_iter().map(str::to_owned).collect(),
            },
            "{source}"
        );
    }
}

#[trace("FR-091-AC-21", "TC-406")]
#[test]
fn the_dimension_and_unit_causes_have_their_catalog_codes() {
    let span = Span { start: 0, end: 0 };
    let cases = [
        (
            AssemblyCause::DuplicateQuantityName {
                name: "m".into(),
                spans: vec![span],
            },
            "ambiguous_declaration/ambiguous-name",
        ),
        (
            AssemblyCause::UnresolvedQuantityName { name: "m".into() },
            "missing_declaration/missing-name",
        ),
        (
            AssemblyCause::QuantityCycle { edges: Vec::new() },
            "invalid_package/definition-cycle",
        ),
        (
            AssemblyCause::ZeroDenominator,
            "undefined_expression/unproved-nonzero",
        ),
        (
            AssemblyCause::UnitGraphTopology {
                fault: TopologyFault::TwoRoots,
                declarations: Vec::new(),
            },
            "invalid_package/unit-graph-topology",
        ),
        (
            AssemblyCause::DecimalScaleLimit(quire_exact_limit()),
            "stage_limit_exceeded/work-budget-exceeded",
        ),
    ];
    for (cause, code) in cases {
        assert_eq!(cause.catalog_code().to_string(), code, "{cause:?}");
    }
}

fn quire_exact_limit() -> qsl_foundation::diagnostic::LimitExceeded {
    qsl_foundation::diagnostic::LimitExceeded::new(
        qsl_foundation::diagnostic::LimitKind::WorkBudget,
        4,
        5,
    )
}
