// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-569: the unit's own `object` declarations (TC-1408), checked on the
//! source route.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::{
    AssemblyCause, AssemblyRefusal, CheckCause, CheckedGraph, PackageDeclarations,
    SourceObjectFault,
};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, Extent, FieldMemberRecord, Multiplicity,
    NativeValueType, ObjectTypeRecord, OperationEffect, OperationMemberRecord,
    OperationParameterRecord, OperationResult, PopulationRecord, ScalarTypeRecord,
    ScalarValueDomain, ValueTypeRef,
};
use qsl_semantics::model::intake::SelectedModels;
use qsl_semantics::model::normalize::ModelRefusalCause;
use quire_checked::key::DeclarationKey;
use quire_exact::{Presence, TextProfile, TextType};
use quire_semantic_value::checking::CheckingLimits;

/// TC-1408 fixture S's header.
const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile P = \"quire.model.complete/v1\";\n\
    profile Pp = \"quire.protocol.finite-global/v1\";\n";

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
post R using P on Order::bump { self.total > pre(self.total) }
body B using P on Sub::size() { 7 }
body C using P on Order::size() { 1 }
pre Qs using P on Order::size { self.total >= 0 }
invariant Sized using P on Order at current { self.size() >= 0 }
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
    let pending = declarations.link_dispatch(ModelNormalizationLimits::default())?;
    declarations
        .check(CheckingLimits::default())
        .map_err(|refusals| pending.report(refusals))
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

/// Fixture S's clauses written against the model: `on M::Order`.
const MODEL_CLAUSES: &str = "\
invariant I using P on M::Order at current { self.total >= 0 }
pre Q using P on M::Order::bump { self.total < 9 }
post R using P on M::Order::bump { self.total > pre(self.total) }
body B using P on M::Sub::size() { 7 }
body C using P on M::Order::size() { 1 }
pre Qs using P on M::Order::size { self.total >= 0 }
invariant Sized using P on M::Order at current { self.size() >= 0 }
";

const AMBIGUOUS: Refused = (Code::AmbiguousDeclaration, "ambiguous-name");
const MISSING: Refused = (Code::MissingDeclaration, "missing-name");
const MALFORMED: Refused = (Code::InvalidModelBinding, "malformed-declaration");

/// The text of `declarations` as the model unit `assemble_model` assembles.
fn model_text(declarations: &str) -> String {
    format!("{HEADER}model M = \"test/orders\";\n{declarations}\n")
}

/// The refusals of the model unit with `declarations`, as catalog codes with
/// the offset each is at in [`model_text`].
fn model_refusals_at(declarations: &str) -> Vec<(Refused, usize)> {
    assemble_model(declarations, model_package(false))
        .expect_err("the unit is refused")
        .errors
        .iter()
        .map(|error| {
            (
                (error.cause.code(), error.cause.catalog_code().cause()),
                error.span.start,
            )
        })
        .collect()
}

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
    // Population `orders` beside a function `orders`: the population's name
    // and the function's declaration.
    let text = format!("{OBJECTS}function orders using P (): Integer pure {{ 1 }}\n");
    assert_eq!(
        refusals_at(&text),
        [
            (AMBIGUOUS, offset(&text, "population orders", 0, 11)),
            (AMBIGUOUS, offset(&text, "function orders", 0, 0)),
        ]
    );
}

/// `object M` in a unit with `model M`: both declarations.
#[trace("FR-569-AC-3", "TC-1408")]
#[test]
fn an_object_named_like_a_model_alias_is_refused_at_both() {
    let refused = model_refusals_at("object M { }");
    let text = model_text("object M { }");
    assert_eq!(
        refused,
        [
            (AMBIGUOUS, offset(&text, "model M", 0, 0)),
            (AMBIGUOUS, offset(&text, "object M", 0, 7)),
        ]
    );
}

/// A quantifier binder named like a source population is ambiguous at the
/// use, naming both.
#[trace("FR-569-AC-3", "TC-1408")]
#[test]
fn a_binder_named_like_a_population_is_ambiguous() {
    let declarations = assemble(&format!(
        "{OBJECTS}invariant Bad using P on Order at current {{ forall(orders in orders : true) }}\n"
    ))
    .expect("the unit assembles");
    let refusals = finish(declarations).expect_err("the binder is ambiguous");
    let [refusal] = &refusals[..] else {
        panic!("one refusal: {refusals:?}");
    };
    let CheckCause::AmbiguousName { name, loci } = &refusal.cause else {
        panic!("an ambiguous name: {refusal:?}");
    };
    assert_eq!(name, "orders");
    assert_eq!(loci.len(), 2, "the use and the binder: {loci:?}");
    // A binder of another name over the same population checks.
    check(&format!(
        "{OBJECTS}invariant Good using P on Order at current {{ forall(o in orders : true) }}\n"
    ));
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

/// FR-569-AC-4: each declaration check refuses once, with its code and cause,
/// at the offending name.
#[trace("FR-569-AC-4", "TC-1408")]
#[test]
fn a_declaration_check_refuses_with_its_code_at_the_offending_name() {
    let cycle = (Code::InvalidPackage, "definition-cycle");
    // (declarations, the text the refusal is at, its code and cause)
    let cases: [(&str, &str, Refused); 8] = [
        (
            "object X extends Receipt { }\nrecord Receipt { total: Integer; }",
            "Receipt",
            MALFORMED,
        ),
        ("object X extends Ghost { }", "Ghost", MISSING),
        (
            "object X identity (ghost) { total: Integer; }",
            "ghost",
            MISSING,
        ),
        (
            "object X { total: Integer; operation f() modifies self.ghost; }",
            "ghost",
            MISSING,
        ),
        (
            "object X { operation f() creates Ghost; }",
            "Ghost",
            MISSING,
        ),
        (
            "object X { operation f(n: Integer) deletes n; }",
            "n;",
            MALFORMED,
        ),
        (
            "object X { operation f() redefines Order::reset; }",
            "Order::reset",
            MISSING,
        ),
        (
            "object A extends B { }\nobject B extends A { }",
            "B {",
            cycle,
        ),
    ];
    for (declarations, at, expected) in cases {
        let text = format!("{OBJECTS}{declarations}");
        // The needle's first occurrence inside the case's own declarations.
        let start = text.find(declarations).unwrap()
            + declarations
                .find(at)
                .unwrap_or_else(|| panic!("`{at}` in {declarations}"));
        let at_extends = if at == "B {" {
            // The cycle is reported at the first `extends` entry.
            text.find(declarations).unwrap() + declarations.find("B").unwrap()
        } else {
            start
        };
        assert_eq!(
            refusals_at(&text),
            [(expected, at_extends)],
            "{declarations}"
        );
    }
}

/// `extends M::Order` names a model type: `malformed-declaration` at the
/// entry.
#[trace("FR-569-AC-4", "TC-1408")]
#[test]
fn extending_a_model_type_is_refused_at_the_entry() {
    let declarations = "object X extends M::Order { }";
    let text = model_text(declarations);
    assert_eq!(
        model_refusals_at(declarations),
        [(MALFORMED, offset(&text, "M::Order", 0, 0))]
    );
}

/// A redefinition whose parameter type is not contravariant refuses
/// `variance-parameter` once, naming the redefining member.
#[trace("FR-569-AC-4", "TC-1408")]
#[test]
fn a_redefinition_with_a_narrower_parameter_refuses_variance_parameter() {
    let text = OBJECTS.replace(
        "operation scaled(m: Integer): Integer redefines",
        "operation scaled(m: Text[0, 8; nfc]): Integer redefines",
    );
    let declarations = assemble(&format!("{text}{CLAUSES}")).expect("the unit assembles");
    let refusals = finish(declarations).expect_err("the parameter is not contravariant");
    let [refusal] = &refusals[..] else {
        panic!("one refusal: {refusals:?}");
    };
    let CheckCause::ModelConformance(failure) = &refusal.cause else {
        panic!("a conformance failure: {refusal:?}");
    };
    assert!(failure.member.node.ends_with("/Sub/scaled"), "{failure:?}");
    assert!(
        failure.parent.node.ends_with("/Order/scaled"),
        "{failure:?}"
    );
    assert!(
        matches!(&failure.cause, ModelRefusalCause::VarianceParameter { .. }),
        "{failure:?}"
    );
}

/// `redefines Order::reset`: `Order` is a proper supertype that exposes no
/// `reset`, so the refusal is at the member.
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
    let AssemblyCause::SourceObject(SourceObjectFault::SupertypeCycle { edges }) = &error.cause
    else {
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

/// Every checked node type under `node`, with the package identity written
/// `P`, so that the source and model routes compare equal.
fn type_names(node: qsl_semantics::check::CheckedNode<'_>, out: &mut Vec<String>) {
    out.push(
        format!("{:?}", node.value_type())
            .replace("source/qsl-semantics-integration-tests", "P")
            .replace("test/orders", "P"),
    );
    for child in node.children() {
        type_names(child, out);
    }
}

/// The checked types of the clause `name`'s expression.
fn clause_types(graph: &CheckedGraph, name: &str) -> Vec<String> {
    let mut out = Vec::new();
    type_names(
        graph.state_clause(name).expect("the clause").body(),
        &mut out,
    );
    out
}

/// The checked types of the function `name`'s body.
fn function_types(graph: &CheckedGraph, name: &str) -> Vec<String> {
    let mut out = Vec::new();
    let body = (0..)
        .map_while(|index| graph.function_state(index))
        .find(|function| function.name == name)
        .expect("the function")
        .body;
    type_names(body, &mut out);
    out
}

/// The receiver type names of a dispatch table's entries.
fn tables_receivers(table: &qsl_semantics::check::DispatchTable) -> Vec<String> {
    table
        .entries()
        .iter()
        .map(|(receiver, _)| {
            format!("{receiver:?}")
                .rsplit('/')
                .next()
                .unwrap_or_default()
                .trim_end_matches("\" }])")
                .to_owned()
        })
        .collect()
}

/// A checked expression's node kinds, package identities written `P`.
fn shape_of(node: qsl_semantics::check::CheckedNode<'_>, out: &mut String) {
    out.push_str(
        &format!("{:?};", node.kind())
            .replace("source/qsl-semantics-integration-tests", "P")
            .replace("test/orders", "P"),
    );
    for child in node.children() {
        shape_of(child, out);
    }
}

/// One dispatch entry without its receiver's package identity: the receiver
/// type's name, body, precondition and precondition clauses.
type CandidateShape = (String, usize, Option<usize>, Vec<usize>);

/// The dispatch tables' candidates, by receiver type name.
fn tables(graph: &CheckedGraph) -> Vec<Vec<CandidateShape>> {
    graph
        .dispatch_tables()
        .iter()
        .map(|table| {
            table
                .entries()
                .iter()
                .map(|(receiver, candidate)| {
                    let receiver = format!("{receiver:?}");
                    let name = receiver
                        .rsplit("/")
                        .next()
                        .unwrap_or_default()
                        .trim_end_matches("\" }])")
                        .to_owned();
                    (
                        name,
                        candidate.body,
                        candidate.precondition,
                        candidate.precondition_clauses.clone(),
                    )
                })
                .collect()
        })
        .collect()
}

/// The source `Sub`-typed and `Order`-typed expressions of AC-5, with the
/// names `object` qualifies by `qualifier`.
fn ac5_extras(qualifier: &str) -> String {
    format!(
        "bound B on {q}orders [3]\n\
         invariant Cap using P on {q}Order at current {{ count(o in {q}orders where o.total = 0) < 4 }}\n\
         invariant Every using P on {q}Order at current {{ forall(o in allInstances<{q}Order>({q}orders) : deref(o).total >= 0) }}\n\
         invariant SizedSub using P on {q}Sub at current {{ self.size() >= 0 }}\n\
         function totalOf using P (s: Reference<{q}Sub>): Int[0, 9] pure {{ deref(s).total }}\n",
        q = qualifier
    )
}

/// FR-569-AC-5 (S-R): a clause on `Sub::size` joins `Sub.size`'s effective
/// precondition as `Qs2 or Qs` with the own clause first, and S and D link
/// the same dispatch tables and check the same types.
#[trace("FR-569-AC-5", "TC-1408")]
#[test]
fn the_source_and_model_twins_link_equal_tables_and_check_equal_types() {
    let source = check(&format!(
        "{OBJECTS}{CLAUSES}pre Qs2 using P on Sub::size {{ self.total >= 1 }}\n{}",
        ac5_extras("")
    ));
    let model = finish(
        assemble_model(
            &format!(
                "{MODEL_CLAUSES}pre Qs2 using P on M::Sub::size {{ self.total >= 1 }}\n{}",
                ac5_extras("M::")
            ),
            model_package(false),
        )
        .expect("the model twin assembles"),
    )
    .expect("the model twin checks");

    // Dispatch tables: equal, with `Sub.size` carrying its own clause first.
    let (source_tables, model_tables) = (tables(&source), tables(&model));
    assert_eq!(source_tables, model_tables);
    assert!(source_tables.iter().any(|table| {
        table
            .iter()
            .all(|entry| entry.0 == "Order" || entry.0 == "Sub")
    }));
    let precondition_of = |graph: &CheckedGraph, name: &str| -> String {
        let table = graph
            .dispatch_tables()
            .iter()
            .find(|table| tables_receivers(table).iter().any(|r| r == "Sub"))
            .expect("the size table");
        let (_, candidate) = table
            .entries()
            .iter()
            .find(|(receiver, _)| format!("{receiver:?}").contains(&format!("/{name}\"")))
            .unwrap_or_else(|| panic!("{name} has an entry"));
        let function = graph
            .function_state(candidate.precondition.expect("an effective precondition"))
            .expect("the precondition function");
        let mut shape = String::new();
        shape_of(function.body, &mut shape);
        shape
    };
    // `Sub.size`'s effective precondition is `Qs2 or Qs`, the own clause
    // first (`>= 1` is Qs2, `>= 0` is Qs); `Order.size`'s is `Qs` alone.
    let sub = precondition_of(&source, "Sub");
    assert!(sub.starts_with("Connective(Or"), "{sub}");
    assert!(
        sub.find("Integer(1)").unwrap() < sub.find("Integer(0)").unwrap(),
        "Qs2 first: {sub}"
    );
    assert!(!precondition_of(&source, "Order").contains("Connective"));
    assert_eq!(sub, precondition_of(&model, "Sub"));
    assert_eq!(
        precondition_of(&source, "Order"),
        precondition_of(&model, "Order")
    );

    // Checked types: equal on the two routes, and the expected ones.
    for clause in ["Cap", "Every", "SizedSub"] {
        assert_eq!(clause_types(&source, clause), clause_types(&model, clause));
    }
    assert_eq!(
        function_types(&source, "totalOf"),
        function_types(&model, "totalOf")
    );
    let cap = clause_types(&source, "Cap");
    assert!(
        cap.contains(&"Int(IntegerInterval { lower: Integer(0), upper: Integer(3) })".to_owned())
    );
    assert!(cap.iter().any(|t| t.contains("kind: Set")
        && t.contains("bound: Some(CardinalityBound { minimum: 0, maximum: 3 })")
        && t.contains("ix://P/Order")));
    assert!(cap.contains(&"Population(Some(3))".to_owned()));
    let every = clause_types(&source, "Every");
    assert!(every
        .iter()
        .any(|t| t.contains("kind: Set") && t.contains("ix://P/Order")));
    let total_of = function_types(&source, "totalOf");
    assert_eq!(
        total_of[0], "Int(IntegerInterval { lower: Integer(0), upper: Integer(9) })",
        "deref(s).total is the field's Int[0, 9]"
    );
    assert!(total_of[1].contains("ix://P/Sub"), "s is a Reference<Sub>");
}

/// FR-569-AC-5 (S-R'): `Sub` redefines `bump` and also modifies `note`, which
/// `Order.bump` does not; S and D each refuse one `effect-escape` on the
/// `note` field of the redefining member.
#[trace("FR-569-AC-5", "TC-1408")]
#[test]
fn a_redefinition_that_widens_the_frame_refuses_effect_escape_on_both_routes() {
    let text = OBJECTS.replace(
        "operation scaled(m: Integer): Integer redefines Order::scaled;",
        "operation scaled(m: Integer): Integer redefines Order::scaled;\n  \
         operation bump(): Boolean redefines Order::bump modifies self.total, self.note;",
    );
    let source = finish(assemble(&format!("{text}{CLAUSES}")).expect("S-R' assembles"))
        .expect_err("the frame escapes");
    let model = finish(assemble_model(MODEL_CLAUSES, model_package(true)).expect("D-R' assembles"))
        .expect_err("the frame escapes");
    for (route, refusals) in [("S-R'", source), ("D-R'", model)] {
        let [refusal] = &refusals[..] else {
            panic!("{route}: one refusal: {refusals:?}");
        };
        let CheckCause::ModelConformance(failure) = &refusal.cause else {
            panic!("{route}: a conformance failure: {refusal:?}");
        };
        assert_eq!(
            (failure.code, failure.axis),
            (Code::IllTyped, "effect"),
            "{route}"
        );
        assert!(
            failure.member.node.ends_with("/Sub/bump"),
            "{route}: {failure:?}"
        );
        assert!(
            failure.parent.node.ends_with("/Order/bump"),
            "{route}: {failure:?}"
        );
        assert!(
            matches!(&failure.cause, ModelRefusalCause::EffectEscape { field } if field.node.ends_with("/Order/note")),
            "{route}: {failure:?}"
        );
    }
}

/// FR-569-AC-2: `on Order` and `on M::Order` check against different field
/// sets when the source `Order` declares a field the model's lacks.
#[trace("FR-569-AC-2", "TC-1408")]
#[test]
fn a_bare_and_a_qualified_context_see_different_field_sets() {
    let objects = OBJECTS.replace(
        "  total: Int[0, 9];",
        "  total: Int[0, 9];\n  extra: Integer;",
    );
    // `extra` is a field of the source `Order` only.
    let both = |clause: &str| {
        finish(
            assemble_model(&format!("{objects}{clause}"), model_package(false))
                .expect("the unit assembles"),
        )
    };
    both("invariant OnSource using P on Order at current { self.extra >= 0 }\n")
        .expect("`on Order` sees `extra`");
    let refusals = both("invariant OnModel using P on M::Order at current { self.extra >= 0 }\n")
        .expect_err("`on M::Order` does not");
    assert!(
        matches!(&refusals[..], [refusal] if matches!(&refusal.cause, CheckCause::StateModel(_) | CheckCause::MissingName(_))),
        "{refusals:?}"
    );
    // The model's `note` is seen by both.
    both(
        "invariant A using P on Order at current { present(self.note) or true }\n\
         invariant B using P on M::Order at current { present(self.note) or true }\n",
    )
    .expect("both see `note`");
}

/// A protocol `Sell` over `over_type`, with `w` the role on the whole model
/// `M` and `Seller` a static role on the source `Order`; its `finish` reads
/// `view.extra`, a field of the source `Order` only.
fn sell(over_type: &str) -> String {
    format!(
        "protocol Sell using Pp over (view: {over_type}) on origin {{\n\
           role w on M;\n\
           role Seller on Order;\n\
           run sequence Main {{\n\
             attempt A by Seller on Order::bump contracts [] as (b: Boolean) {{ true }};\n\
           }}\n\
           finish Done as (d: Boolean) {{ view.extra >= 0 }};\n\
         }}\n"
    )
}

/// FR-218-AC-39 (S-M): `w` is the role on the whole model `M` and `Seller` a
/// static role on the source `Order`. `over (view: Order)` has the source
/// `Order` as its model context, so it reads the source-only field `extra`;
/// `over (view: M::Order)` has the model's, which has none.
#[trace("FR-218-AC-39", "FR-569-AC-2", "TC-1408")]
#[test]
fn a_protocol_roles_and_over_resolve_to_the_source_or_the_model_by_spelling() {
    let objects = OBJECTS.replace(
        "  total: Int[0, 9];",
        "  total: Int[0, 9];\n  extra: Integer;",
    );
    let on_source = finish(
        assemble_model(&format!("{objects}{}", sell("Order")), model_package(false))
            .expect("the S-M unit assembles"),
    )
    .unwrap_or_else(|refusals| panic!("over Order: {refusals:?}"));
    let [protocol] = on_source.protocols() else {
        panic!("one protocol");
    };
    let role = |name: &str| {
        protocol
            .roles
            .iter()
            .find(|role| role.name == name)
            .unwrap_or_else(|| panic!("role {name}"))
    };
    // `w` is on the whole model `M`: it names the model's package and no
    // object type. `Seller` is on the source `Order`: an object type of the
    // source package.
    assert_eq!(role("w").model_selection.as_deref(), Some("test/orders"));
    assert_eq!(role("w").context, None);
    assert!(role("Seller").context.is_some());
    assert_eq!(role("Seller").model_selection, None);

    // The same protocol over the model's `Order` cannot read `extra`.
    let refusals = finish(
        assemble_model(
            &format!("{objects}{}", sell("M::Order")),
            model_package(false),
        )
        .expect("the S-M unit assembles"),
    )
    .map(|_| ())
    .expect_err("M::Order has no `extra`");
    assert!(!refusals.is_empty());
    // And over the model's `Order` without reading `extra`, it checks: the
    // two contexts differ only in what they see.
    let without = sell("M::Order").replace("view.extra >= 0", "view.total >= 0");
    finish(
        assemble_model(&format!("{objects}{without}"), model_package(false))
            .expect("the S-M unit assembles"),
    )
    .unwrap_or_else(|refusals| panic!("over M::Order: {refusals:?}"));
}

/// Source objects are unit-scoped: a clause in another unit of the package
/// that names a source type refuses `missing-name`, whichever unit holds the
/// declaration.
#[trace("FR-569-AC-2", "TC-1408")]
#[test]
fn a_source_type_is_visible_only_in_the_unit_that_declares_it() {
    let header = "profile P = \"quire.model.complete/v1\";\n";
    let objects = format!("{header}{OBJECTS}");
    let clause = format!("{header}invariant X using P on Order at current {{ self.total >= 0 }}\n");
    // The declaring unit's own clause checks.
    let alone = crate::model_operations::try_assemble_units(
        &[(
            "a",
            &format!(
                "{objects}{clause_body}",
                clause_body = "invariant X using P on Order at current { self.total >= 0 }\n"
            ),
        )],
        &[],
    );
    assert!(alone.is_ok(), "{:?}", alone.err());
    // A clause in another unit, whichever order the units sort in.
    for units in [
        [("a", &objects), ("b", &clause)],
        [("a", &clause), ("b", &objects)],
    ] {
        let refusal = crate::model_operations::try_assemble_units(
            &[
                (units[0].0, units[0].1.as_str()),
                (units[1].0, units[1].1.as_str()),
            ],
            &[],
        )
        .expect_err("the other unit does not see the type");
        let [error] = &refusal.errors[..] else {
            panic!("one refusal: {refusal:?}");
        };
        assert!(
            matches!(&error.cause, AssemblyCause::UnresolvedTypeName { name } if name == "Order"),
            "{error:?}"
        );
        assert_eq!(
            (error.cause.code(), error.cause.catalog_code().cause()),
            MISSING
        );
    }
}
