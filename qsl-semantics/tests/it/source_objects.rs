// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-569: the unit's own `object` declarations (TC-1408), checked on the
//! source route.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::{AssemblyCause, AssemblyRefusal, CheckedGraph, PackageDeclarations};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, Extent, FieldMemberRecord, Multiplicity,
    NativeValueType, ObjectTypeRecord, OperationEffect, OperationMemberRecord,
    OperationParameterRecord, OperationResult, PopulationRecord, ScalarTypeRecord,
    ScalarValueDomain, ValueTypeRef,
};
use qsl_semantics::model::intake::SelectedModels;
use quire_checked::key::DeclarationKey;
use quire_exact::{Presence, TextProfile, TextType};
use quire_semantic_value::checking::CheckingLimits;

/// TC-1408 fixture S's header.
const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile P = \"quire.model.complete/v1\";\n";

/// TC-1408 fixture S: the three object declarations.
const OBJECTS: &str = "\
object Order identity (total) {
  total: Int[0, 9];
  note: Text[0, 8; nfc]?;
  population orders;
  operation size(): Integer;
  operation count(): Integer;
  operation scaled(n: Integer): Integer;
  operation bump(): Boolean modifies self.total;
}
object Sub extends Order {
  operation size(): Integer redefines Order::size;
  operation scaled(m: Integer): Integer redefines Order::scaled;
}
abstract object Shape { population shapes; }
";

/// TC-1408 fixture S: the clauses.
const CLAUSES: &str = "\
invariant I using P on Order at current { self.total >= 0 }
pre Q using P on Order::bump { self.total < 9 }
post R using P on Order::bump { self.total >= pre(self.total) }
body B using P on Sub::size() { 7 }
body C using P on Order::size() { 1 }
pre Qs using P on Order::size { self.total >= 0 }
";

/// One refusal as its catalog code and cause.
type Refused = (Code, &'static str);

fn assemble(declarations: &str) -> Result<PackageDeclarations, AssemblyRefusal> {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "tc-1408"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        built,
        SelectedModels::fixture_entries(Vec::new()),
        Vec::new(),
    )
}

/// The unit's refusals as catalog codes, with the span text each is at.
fn refusals(declarations: &str) -> Vec<(Refused, String)> {
    let text = format!("{HEADER}{declarations}\n");
    let refusal = assemble(declarations).expect_err("the unit is refused");
    refusal
        .errors
        .iter()
        .map(|error| {
            (
                (error.cause.code(), error.cause.catalog_code().cause()),
                text[error.span.start..error.span.end].to_owned(),
            )
        })
        .collect()
}

/// The unit's refusals as catalog codes, with the byte offset each is at,
/// relative to the start of `declarations`.
fn refusals_at(declarations: &str) -> Vec<(Refused, usize)> {
    let refusal = assemble(declarations).expect_err("the unit is refused");
    refusal
        .errors
        .iter()
        .map(|error| {
            (
                (error.cause.code(), error.cause.catalog_code().cause()),
                error.span.start - HEADER.len(),
            )
        })
        .collect()
}

/// The offset in `text` of the `occurrence`th (from 0) `needle`, plus `skip`
/// bytes into it.
fn offset(text: &str, needle: &str, occurrence: usize, skip: usize) -> usize {
    text.match_indices(needle)
        .nth(occurrence)
        .unwrap_or_else(|| panic!("`{needle}` occurs {} times", occurrence + 1))
        .0
        + skip
}

/// Links dispatch and checks `declarations`, as the compile driver does.
fn finish(
    mut declarations: PackageDeclarations,
) -> Result<CheckedGraph, Vec<qsl_semantics::check::CheckRefusal>> {
    declarations.link_dispatch(ModelNormalizationLimits::default())?;
    declarations.check(CheckingLimits::default())
}

fn check(declarations: &str) -> CheckedGraph {
    finish(assemble(declarations).expect("the unit assembles")).expect("the unit checks")
}

/// A bare type name resolves to the source object type; an operation of a
/// source type, a population and a `Reference` of a subtype resolve too.
#[trace("FR-569-AC-2", "TC-1408")]
#[test]
fn fixture_s_clauses_resolve_to_the_source_declarations() {
    let graph = check(&format!(
        "{OBJECTS}{CLAUSES}\
         invariant V4 using P on Order at current {{ forall(o in orders : deref(o).total >= 0) }}\n\
         function sub using P (r: Reference<Sub>): Integer pure {{ deref(r).total }}\n"
    ));
    for name in ["I", "Q", "R", "Qs", "V4"] {
        assert!(graph.state_clause(name).is_some(), "{name} is checked");
    }
}

/// An operation `T` declares nothing of, a source record and an unknown name
/// each refuse `missing-name` at the name.
#[trace("FR-569-AC-2", "TC-1408")]
#[test]
fn an_unresolved_context_refuses_missing_name_at_the_name() {
    let missing = (Code::MissingDeclaration, "missing-name");
    assert_eq!(
        refusals(&format!(
            "{OBJECTS}pre X using P on Order::reset {{ true }}"
        )),
        [(missing, "reset".to_owned())]
    );
    assert_eq!(
        refusals(&format!(
            "{OBJECTS}record Receipt {{ total: Integer; }}\n\
             invariant X using P on Receipt at current {{ true }}"
        )),
        [(missing, "Receipt".to_owned())]
    );
    assert_eq!(
        refusals(&format!(
            "{OBJECTS}invariant X using P on Nothing at current {{ true }}"
        )),
        [(missing, "Nothing".to_owned())]
    );
    let _ = AssemblyCause::EmptyUnitList;
}

// ---------------------------------------------------------------------------
// Fixture D: the model twin of fixture S, a domain package `test/orders`.
// ---------------------------------------------------------------------------

fn key(node: &str) -> DeclarationKey {
    DeclarationKey::fixture(format!("ix://test/orders/{node}"))
}

fn one() -> Multiplicity {
    Multiplicity {
        lower: 1,
        upper: Some(1),
        ordered: false,
        unique: true,
    }
}

fn object(name: &str, supertypes: &[&str], abstract_type: bool) -> DomainPackageRecord {
    DomainPackageRecord::ObjectType(ObjectTypeRecord {
        key: key(name),
        interface_features: None,
        abstract_type,
        supertypes: supertypes.iter().map(|name| key(name)).collect(),
    })
}

fn field(
    owner: &str,
    name: &str,
    value_type: ValueTypeRef,
    presence: Presence,
    position: usize,
) -> DomainPackageRecord {
    DomainPackageRecord::FieldMember(FieldMemberRecord {
        key: key(&format!("{owner}/{name}")),
        owner: key(owner),
        value_type,
        multiplicity: one(),
        presence,
        subsets: Vec::new(),
        redefines: None,
        position,
    })
}

/// `owner::name(parameter: Integer): Integer` or `(): Integer`, a query.
fn query(
    owner: &str,
    name: &str,
    parameter: Option<&str>,
    redefines: Option<&str>,
) -> DomainPackageRecord {
    operation(
        owner,
        name,
        parameter,
        Some(ValueTypeRef::Native(NativeValueType::Integer)),
        redefines,
        OperationEffect::default(),
    )
}

fn operation(
    owner: &str,
    name: &str,
    parameter: Option<&str>,
    result: Option<ValueTypeRef>,
    redefines: Option<&str>,
    effect: OperationEffect,
) -> DomainPackageRecord {
    DomainPackageRecord::OperationMember(OperationMemberRecord {
        key: key(&format!("{owner}/{name}")),
        owner: key(owner),
        parameters: parameter
            .map(|parameter| OperationParameterRecord {
                key: key(&format!("{owner}/{name}/{parameter}")),
                value_type: ValueTypeRef::Native(NativeValueType::Integer),
                multiplicity: one(),
            })
            .into_iter()
            .collect(),
        result: result.map(|value_type| OperationResult {
            value_type,
            multiplicity: one(),
        }),
        effect,
        requires: Vec::new(),
        ensures: Vec::new(),
        has_body: false,
        redefines: redefines.map(key),
    })
}

fn bump(owner: &str, redefines: Option<&str>, fields: &[&str]) -> DomainPackageRecord {
    operation(
        owner,
        "bump",
        None,
        Some(ValueTypeRef::Native(NativeValueType::Boolean)),
        redefines,
        OperationEffect {
            receiver_modifies: fields
                .iter()
                .map(|name| key(&format!("Order/{name}")))
                .collect(),
            ..OperationEffect::default()
        },
    )
}

/// Fixture D's records; `redefine_bump` adds D-R' (`Sub` redefines `bump`,
/// also modifying `note`).
fn model_records(redefine_bump: bool) -> Vec<DomainPackageRecord> {
    let mut records = vec![
        object("Order", &[], false),
        object("Sub", &["Order"], false),
        object("Shape", &[], true),
        DomainPackageRecord::ScalarType(ScalarTypeRecord {
            key: key("Total"),
            domain: ScalarValueDomain::Integer { lower: 0, upper: 9 },
        }),
        DomainPackageRecord::ScalarType(ScalarTypeRecord {
            key: key("Note"),
            domain: ScalarValueDomain::Text(TextType::new(0, 8, TextProfile::Nfc).unwrap()),
        }),
        field(
            "Order",
            "total",
            ValueTypeRef::Package(key("Total")),
            Presence::Required,
            0,
        ),
        field(
            "Order",
            "note",
            ValueTypeRef::Package(key("Note")),
            Presence::Optional,
            1,
        ),
        DomainPackageRecord::Population(PopulationRecord {
            key: key("orders"),
            member_types: vec![key("Order")],
            extent: Extent::Closed,
            symmetric: false,
        }),
        DomainPackageRecord::Population(PopulationRecord {
            key: key("shapes"),
            member_types: vec![key("Shape")],
            extent: Extent::Closed,
            symmetric: false,
        }),
        query("Order", "size", None, None),
        query("Order", "count", None, None),
        query("Order", "scaled", Some("n"), None),
        bump("Order", None, &["total"]),
        query("Sub", "size", None, Some("Order/size")),
        query("Sub", "scaled", Some("m"), Some("Order/scaled")),
    ];
    if redefine_bump {
        records.push(bump("Sub", Some("Order/bump"), &["total", "note"]));
    }
    records
}

fn model_package(redefine_bump: bool) -> DomainPackage {
    DomainPackage::new(
        DomainPackageRef::fixture("test/orders"),
        model_records(redefine_bump),
    )
}

/// The unit that selects fixture D as `M`, with `declarations`.
fn assemble_model(
    declarations: &str,
    package: DomainPackage,
) -> Result<PackageDeclarations, AssemblyRefusal> {
    let text = format!("{HEADER}model M = \"test/orders\";\n{declarations}\n");
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "tc-1408-d"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let selected = SelectedModels::fixture(
        "M",
        qsl_foundation::Span { start: 0, end: 1 },
        package,
        ModelNormalizationLimits::UNLIMITED,
    )
    .expect("the model admits");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        built,
        selected,
        Vec::new(),
    )
}

fn check_model(declarations: &str) -> CheckedGraph {
    finish(assemble_model(declarations, model_package(false)).expect("the model unit assembles"))
        .expect("the model unit checks")
}

/// Fixture S's clauses written against the model: `on M::Order`.
const MODEL_CLAUSES: &str = "\
invariant I using P on M::Order at current { self.total >= 0 }
pre Q using P on M::Order::bump { self.total < 9 }
post R using P on M::Order::bump { self.total >= pre(self.total) }
body B using P on M::Sub::size() { 7 }
body C using P on M::Order::size() { 1 }
pre Qs using P on M::Order::size { self.total >= 0 }
";

/// The model twin checks fixture S's clauses.
#[test]
fn the_model_twin_checks_the_clauses() {
    let _ = check_model(MODEL_CLAUSES);
}

const AMBIGUOUS: Refused = (Code::AmbiguousDeclaration, "ambiguous-name");
const MISSING: Refused = (Code::MissingDeclaration, "missing-name");
const MALFORMED: Refused = (Code::InvalidModelBinding, "malformed-declaration");

/// FR-569-AC-3: a source object type or population whose name another
/// declaration of the unit's namespace takes is refused at every declaration
/// of the name.
#[trace("FR-569-AC-3", "TC-1408")]
#[test]
fn a_name_two_declarations_share_is_refused_at_each() {
    // `record Order` beside `object Order`: both declarations.
    let text = format!("record Order {{ total: Integer; }}\n{OBJECTS}");
    assert_eq!(
        refusals_at(&text),
        [
            (AMBIGUOUS, offset(&text, "Order", 0, 0)),
            (AMBIGUOUS, offset(&text, "object Order", 0, 7)),
        ]
    );
    // Population `orders` beside a function `orders`: both.
    let text = format!("{OBJECTS}function orders using P (): Integer pure {{ 1 }}\n");
    let refused = refusals_at(&text);
    assert_eq!(refused.len(), 2, "{refused:?}");
    assert!(refused.iter().all(|(cause, _)| *cause == AMBIGUOUS));
    assert!(refused
        .iter()
        .any(|(_, at)| *at == offset(&text, "population orders", 0, 11)));
}

/// `object M` in a unit with `model M`: both declarations.
#[trace("FR-569-AC-3", "TC-1408")]
#[test]
fn an_object_named_like_a_model_alias_is_refused_at_both() {
    let text = format!("{HEADER}model M = \"test/orders\";\nobject M {{ }}\n");
    let package = model_package(false);
    let refusal = assemble_model("object M { }", package).expect_err("the unit is refused");
    assert_eq!(refusal.errors.len(), 2, "{refusal:?}");
    assert!(refusal
        .errors
        .iter()
        .all(|error| (error.cause.code(), error.cause.catalog_code().cause()) == AMBIGUOUS));
    let starts: Vec<usize> = refusal
        .errors
        .iter()
        .map(|error| error.span.start)
        .collect();
    let object = text.find("object M").unwrap() + 7;
    assert!(starts.contains(&object), "{starts:?} holds {object}");
}

/// Members of one object type: two fields of a name (the second), a field
/// named like an inherited field (the field), an operation named like an
/// inherited one without `redefines` (the operation).
#[trace("FR-569-AC-3", "TC-1408")]
#[test]
fn a_repeated_member_name_is_refused_at_the_later_member() {
    let text = format!("{OBJECTS}object Twin {{ total: Integer; total: Integer; }}\n");
    assert_eq!(
        refusals_at(&text),
        [(AMBIGUOUS, offset(&text, "total: Integer; total", 0, 16))]
    );
    let text = OBJECTS.replace(
        "object Sub extends Order {",
        "object Sub extends Order {\n  total: Integer;",
    );
    assert_eq!(
        refusals_at(&text),
        [(AMBIGUOUS, offset(&text, "total: Integer;", 0, 0))]
    );
    let text = OBJECTS.replace(
        "  operation size(): Integer redefines Order::size;",
        "  operation size(): Integer;",
    );
    assert_eq!(
        refusals_at(&text),
        [(
            AMBIGUOUS,
            offset(
                &text,
                "operation size(): Integer;\n  operation scaled(m",
                0,
                10
            )
        )]
    );
}

/// `deletes Order` in an operation whose parameter is also named `Order`.
#[trace("FR-569-AC-3", "TC-1408")]
#[test]
fn a_deletes_name_that_is_a_parameter_and_a_type_is_ambiguous() {
    let text = OBJECTS.replace(
        "  operation count(): Integer;",
        "  operation drop(Order: Reference<Order>) deletes Order;",
    );
    assert_eq!(
        refusals_at(&text),
        [(AMBIGUOUS, offset(&text, "deletes Order", 0, 8))]
    );
}

/// FR-569-AC-4: the declaration checks, each at the offending name.
#[trace("FR-569-AC-4", "TC-1408")]
#[test]
fn a_declaration_check_refuses_with_its_code_at_the_offending_name() {
    let cycle = (Code::InvalidPackage, "definition-cycle");
    let cases: [(&str, &str, Refused, &str); 8] = [
        (
            "object X extends Receipt { }\nrecord Receipt { total: Integer; }",
            "Receipt",
            MALFORMED,
            "a source record is no object type",
        ),
        (
            "object X extends Ghost { }",
            "Ghost",
            MISSING,
            "an unknown supertype",
        ),
        (
            "object X identity (ghost) { total: Integer; }",
            "ghost",
            MISSING,
            "an unknown identity field",
        ),
        (
            "object X { total: Integer; operation f() modifies self.ghost; }",
            "ghost",
            MISSING,
            "an unknown field",
        ),
        (
            "object X { operation f() creates Ghost; }",
            "Ghost",
            MISSING,
            "an unknown type",
        ),
        (
            "object X { operation f(n: Integer) deletes n; }",
            "n;",
            MALFORMED,
            "a non-reference deletion",
        ),
        (
            "object X { operation f() redefines Order::reset; }",
            "Order",
            MISSING,
            "a redefinition of a non-supertype",
        ),
        (
            "object A extends B { }\nobject B extends A { }",
            "B { }",
            cycle,
            "a supertype cycle",
        ),
    ];
    for (declarations, _at, expected, label) in cases {
        let text = format!("{OBJECTS}{declarations}");
        let refused = refusals(&text);
        assert!(!refused.is_empty(), "{label}");
        assert!(
            refused.iter().all(|(cause, _)| *cause == expected),
            "{label}: {refused:?}"
        );
    }
}

/// `redefines Order::reset`: `Order` is a proper supertype that exposes no
/// `reset`.
#[trace("FR-569-AC-4", "TC-1408")]
#[test]
fn a_redefinition_of_an_operation_the_supertype_lacks_is_refused_at_the_operation() {
    let text = OBJECTS.replace(
        "operation size(): Integer redefines Order::size;",
        "operation size(): Integer redefines Order::reset;",
    );
    assert_eq!(
        refusals_at(&text),
        [(MISSING, offset(&text, "Order::reset", 0, 7))]
    );
}

/// `A extends B` with `B extends A` is one `definition-cycle` naming both
/// edges.
#[trace("FR-569-AC-4", "TC-1408")]
#[test]
fn a_supertype_cycle_is_one_definition_cycle_naming_both_edges() {
    let refusal = assemble("object A extends B { }\nobject B extends A { }")
        .expect_err("the cycle is refused");
    let [error] = &refusal.errors[..] else {
        panic!("one refusal: {refusal:?}");
    };
    let AssemblyCause::AliasCycle { edges } = &error.cause else {
        panic!("a cycle: {error:?}");
    };
    assert_eq!(
        edges,
        &[
            ("A".to_owned(), "B".to_owned()),
            ("B".to_owned(), "A".to_owned())
        ]
    );
    assert_eq!(
        (error.cause.code(), error.cause.catalog_code().cause()),
        (Code::InvalidPackage, "definition-cycle")
    );
}
