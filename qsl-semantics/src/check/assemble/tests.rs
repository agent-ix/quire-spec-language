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

use super::{model_field, AssemblyCause, AssemblyError, AssemblyRefusal, Unmapped};
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
