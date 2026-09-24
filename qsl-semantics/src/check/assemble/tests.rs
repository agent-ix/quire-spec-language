// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-091's assembler over real complete-V1 source: S1 (`qsl_cst::parse`),
//! S2 (`qsl_forms::build_unit`), then [`PackageDeclarations::assemble`].

use ix_trace_rs::trace;
use qsl_forms::{build_unit, FormsLimits};
use qsl_foundation::{SourceIdentity, Span};
use quire_exact::{IeeeWidth, IntegerInterval, RoundingMode, ValueType};

use super::{AssemblyCause, AssemblyError, AssemblyRefusal};
use crate::check::{
    CheckCause, CheckingLimits, Location, Origin, PackageDeclarations, TypeFormFault,
};

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
    PackageDeclarations::assemble(parsed.source().reference().clone(), unit)
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
    assert_eq!(
        found,
        [AssemblyError {
            cause: AssemblyCause::AmbiguousTypeName {
                name: "A".into(),
                candidates: vec![
                    after(&text, "type A = Int[0, 1]", "A"),
                    after(&text, "type A = Int[0, 2]", "A"),
                ],
            },
            span: after(&text, "(x: A)", "A"),
        }]
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

#[trace("FR-091-AC-19", "TC-405")]
#[test]
fn floating_and_reference_types_are_refused() {
    let (text, found) =
        errors("function f using v(x: Float64[nearest-even]): Boolean pure { true }");
    assert_eq!(
        found,
        [AssemblyError {
            cause: AssemblyCause::FloatingType {
                width: IeeeWidth::Binary64,
                rounding: RoundingMode::NearestEven,
                profile: Some("v".into()),
            },
            span: last(&text, "Float64[nearest-even]"),
        }]
    );
    assert_eq!(
        found[0].cause.catalog_code().to_string(),
        "unknown_required_feature/unsupported-feature"
    );
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
        assembled
            .expect("the unit assembles")
            .check(CheckingLimits::default())
            .expect_err("check refuses")
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
    ];
    for (cause, code) in cases {
        assert_eq!(cause.catalog_code().to_string(), code, "{cause:?}");
    }
}

#[trace("FR-091-AC-22", "TC-412")]
#[test]
fn using_aliases_resolve_to_the_units_profile_selections() {
    let (_, assembled) = assemble("function f using v(): Boolean pure { true }");
    let package = assembled.expect("f's alias names the profile selection v");
    assert_eq!(
        package.functions[0]
            .using()
            .map(|using| using.alias.as_str()),
        Some("v")
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
            assert_eq!(spans.len(), 2);
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
    let file = syn::parse_file(include_str!("../assemble.rs")).expect("assemble.rs parses");
    let mut edges = CstEdges::default();
    edges.visit_file(&file);
    assert_eq!(edges.0, 0, "the assembler's shipped code names qsl_cst");
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
