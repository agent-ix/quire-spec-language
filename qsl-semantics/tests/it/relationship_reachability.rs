// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-365 / TC-1046: `reaches(a, b, name)` over a relationship end types as
//! `Boolean`, and every inadmissible form refuses with its code, cause and
//! family. One test per vector C01-C16 of fixture R; every expected value is
//! the vector's.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::{
    CheckCause, CheckRefusal, CheckedGraph, EndSide, MemberCandidate, NodeKind,
};
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, Extent, FieldMemberRecord, Multiplicity,
    OperationEffect, OperationMemberRecord, OperationParameterRecord, PopulationRecord,
    RelationshipDirection, RelationshipEnd, RelationshipRecord, ScalarTypeRecord,
    ScalarValueDomain, ValueTypeRef,
};
use qsl_semantics::model::key::DeclarationKey;
use quire_exact::{Presence, ValueType};
use quire_semantic_value::declaration::FieldRef;

use RelationshipDirection::{Bidirectional, SourceToTarget, TargetToSource, Undirected};

use crate::relationship_navigation::{
    assert_refusal, candidates, descendants, key, mult, object, object_type,
};

/// One end of a fixture-R relationship.
#[derive(Clone)]
struct End {
    object: &'static str,
    role: Option<&'static str>,
    multiplicity: Option<Multiplicity>,
}

fn end(object: &'static str, role: Option<&'static str>, m: Option<Multiplicity>) -> End {
    End {
        object,
        role,
        multiplicity: m,
    }
}

fn one() -> Option<Multiplicity> {
    Some(mult(0, Some(1)))
}

/// One relationship of fixture R, named `Order/<id>` as TC-1046 names it.
struct Link {
    id: &'static str,
    direction: RelationshipDirection,
    source: End,
    target: End,
}

fn link(id: &'static str, direction: RelationshipDirection, source: End, target: End) -> Link {
    Link {
        id,
        direction,
        source,
        target,
    }
}

/// The relationship table of TC-1046's fixture R.
fn links() -> Vec<Link> {
    let unbounded = Some(mult(0, None));
    vec![
        link(
            "next",
            SourceToTarget,
            end("Order", Some("next"), one()),
            end("Order", Some("previous"), one()),
        ),
        link(
            "link",
            Bidirectional,
            end("Order", Some("outgoing"), one()),
            end("Order", Some("incoming"), one()),
        ),
        link(
            "mate",
            Undirected,
            end("Order", Some("mate"), one()),
            end("Order", Some("mateOf"), one()),
        ),
        link(
            "back",
            TargetToSource,
            end("Order", Some("back"), unbounded),
            end("Order", Some("backOf"), unbounded),
        ),
        link(
            "kids",
            SourceToTarget,
            end("Order", Some("kids"), one()),
            end("Order", None, Some(mult(0, Some(3)))),
        ),
        link(
            "litter",
            SourceToTarget,
            end("Order", Some("litter"), one()),
            end(
                "Order",
                None,
                Some(Multiplicity {
                    unique: false,
                    ..mult(0, Some(3))
                }),
            ),
        ),
        link(
            "crowd",
            SourceToTarget,
            end("Order", Some("crowd"), one()),
            end("Order", None, unbounded),
        ),
        link(
            "ranked",
            SourceToTarget,
            end("Order", Some("ranked"), one()),
            end(
                "Order",
                None,
                Some(Multiplicity {
                    ordered: true,
                    ..mult(0, Some(1))
                }),
            ),
        ),
        link(
            "bare",
            SourceToTarget,
            end("Order", Some("bare"), one()),
            end("Order", None, None),
        ),
        link(
            "owner",
            SourceToTarget,
            end("Order", Some("owner"), one()),
            end("Person", None, one()),
        ),
        link(
            "parent",
            SourceToTarget,
            end("Order", Some("parent"), one()),
            end("Order", None, one()),
        ),
        link(
            "twinA",
            SourceToTarget,
            end("Order", Some("twin"), one()),
            end("Order", None, one()),
        ),
        link(
            "twinB",
            SourceToTarget,
            end("Order", Some("twin"), one()),
            end("Order", None, one()),
        ),
        link(
            "manyOwners",
            SourceToTarget,
            end("Order", Some("manyOwners"), one()),
            end("Person", None, Some(mult(0, Some(3)))),
        ),
        link(
            "loose",
            SourceToTarget,
            end("Order", Some("loose"), one()),
            end("Person", None, unbounded),
        ),
    ]
}

fn relationship_key(id: &str) -> DeclarationKey {
    key(&format!("relationship/Order-{id}"))
}

fn relationship(link: &Link) -> DomainPackageRecord {
    let end = |end: &End| RelationshipEnd {
        type_identity: key(end.object),
        role: end.role.map(str::to_owned),
        multiplicity: end.multiplicity,
    };
    DomainPackageRecord::Relationship(RelationshipRecord {
        key: relationship_key(link.id),
        declared_at: None,
        source: end(&link.source),
        target: end(&link.target),
        direction: link.direction,
    })
}

fn optional_reference_field(owner: &str, name: &str, target: &str) -> DomainPackageRecord {
    DomainPackageRecord::FieldMember(FieldMemberRecord {
        key: key(&format!("{owner}/{name}")),
        owner: key(owner),
        value_type: ValueTypeRef::Package(key(target)),
        multiplicity: mult(1, Some(1)),
        presence: Presence::Optional,
        subsets: Vec::new(),
        redefines: None,
    })
}

fn population(name: &str, member: &str) -> DomainPackageRecord {
    DomainPackageRecord::Population(PopulationRecord {
        key: key(name),
        member_types: vec![key(member)],
        extent: Extent::Closed,
    })
}

/// `owner::name(parameter: parameter_type)`.
fn operation(
    owner: &str,
    name: &str,
    parameter: &str,
    parameter_type: &str,
) -> DomainPackageRecord {
    DomainPackageRecord::OperationMember(OperationMemberRecord {
        key: key(&format!("{owner}/{name}")),
        owner: key(owner),
        parameters: vec![OperationParameterRecord {
            key: key(&format!("{owner}/{name}/{parameter}")),
            value_type: ValueTypeRef::Package(key(parameter_type)),
            multiplicity: mult(1, Some(1)),
        }],
        result: None,
        effect: OperationEffect::default(),
        requires: Vec::new(),
        ensures: Vec::new(),
        has_body: false,
        redefines: None,
    })
}

/// Fixture R, with the extra populations `extra` (name, member type).
fn package(extra: &[(&str, &str)]) -> DomainPackage {
    let mut records = vec![
        object_type("Order", &[]),
        object_type("Rush", &["Order"]),
        object_type("Person", &[]),
        DomainPackageRecord::ScalarType(ScalarTypeRecord {
            key: key("Code"),
            domain: ScalarValueDomain::Integer { lower: 0, upper: 9 },
        }),
        optional_reference_field("Order", "trail", "Order"),
        optional_reference_field("Order", "parent", "Order"),
        population("orders", "Order"),
        population("people", "Person"),
        operation("Order", "reroute", "target", "Order"),
        operation("Order", "toPerson", "target", "Person"),
        operation("Order", "toCode", "target", "Code"),
        operation("Rush", "hop", "target", "Order"),
        operation("Rush", "hopRush", "target", "Rush"),
        operation("Person", "watch", "o", "Order"),
    ];
    records.extend(extra.iter().map(|(name, member)| population(name, member)));
    records.extend(links().iter().map(relationship));
    DomainPackage::new(DomainPackageRef::fixture("bundle.orders"), records)
}

/// Check `declarations` over fixture R with the extra populations `extra`.
fn check_with(
    extra: &[(&str, &str)],
    declarations: &str,
) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    crate::relationship_navigation::check_declarations(package(extra), declarations)
}

fn check(declarations: &str) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    check_with(&[], declarations)
}

fn invariant(owner: &str, body: &str) -> String {
    format!("invariant I using v on M::{owner} at current {{ {body} }}")
}

fn clause(kind: &str, operation: &str, body: &str) -> String {
    format!("{kind} I using v on M::{operation} {{ {body} }}")
}

/// Each end-reachability node of clause `I`: its static type, relationship and
/// role.
fn reachabilities(graph: &CheckedGraph) -> Vec<(ValueType, DeclarationKey, String)> {
    let body = graph.state_clause("I").expect("I is checked").body();
    descendants(body)
        .iter()
        .filter_map(|node| match node.kind() {
            NodeKind::ReachesEnd {
                relationship, role, ..
            } => Some((
                node.value_type().clone(),
                relationship.clone(),
                role.clone(),
            )),
            _ => None,
        })
        .collect()
}

fn assert_admitted(declarations: &str, relationship: &str, role: &str) {
    let graph =
        check(declarations).unwrap_or_else(|refusals| panic!("{declarations}: {refusals:?}"));
    assert_eq!(
        reachabilities(&graph),
        vec![(
            ValueType::Boolean,
            relationship_key(relationship),
            role.to_owned()
        )],
        "{declarations}"
    );
}

fn assert_refused(declarations: &str, code: Code, cause: &str, path: Vec<usize>) -> CheckRefusal {
    assert_refusal(check(declarations), declarations, code, cause, path)
}

/// FR-365-AC-1 (C01): `not reaches(self, self, next)` as an invariant checks
/// `Boolean`; the node holds relationship `Order/next` and role `next`.
#[trace("TC-1046", "FR-365-AC-1")]
#[test]
fn c01_a_reachability_over_an_end_checks_boolean_with_its_relationship_and_role() {
    assert_admitted(
        &invariant("Order", "not reaches(self, self, next)"),
        "next",
        "next",
    );
}

/// FR-365-AC-1 (C02): `pre` and `post` clauses of `reroute` check `Boolean`.
#[trace("TC-1046", "FR-365-AC-1")]
#[test]
fn c02_a_precondition_and_a_postcondition_check_boolean() {
    for kind in ["pre", "post"] {
        assert_admitted(
            &clause(kind, "Order::reroute", "reaches(self, target, next)"),
            "next",
            "next",
        );
    }
}

/// FR-365-AC-1 (C03): a `Rush` receiver and a base-typed or subtype operand
/// each conform to the near end's type `Order`.
#[trace("TC-1046", "FR-365-AC-1")]
#[test]
fn c03_a_subtype_receiver_and_subtype_or_base_operands_check() {
    assert_admitted(
        &invariant("Rush", "reaches(self, self, next)"),
        "next",
        "next",
    );
    for operation in ["hop", "hopRush"] {
        assert_admitted(
            &clause(
                "pre",
                &format!("Rush::{operation}"),
                "reaches(self, target, next)",
            ),
            "next",
            "next",
        );
    }
}

/// FR-365-AC-1 (C04): the inverse end under `bidirectional`, the inverse end
/// under `undirected`, and the forward end under `bidirectional` check.
#[trace("TC-1046", "FR-365-AC-1")]
#[test]
fn c04_inverse_and_forward_ends_under_bidirectional_and_undirected_check() {
    for (role, relationship) in [
        ("incoming", "link"),
        ("mateOf", "mate"),
        ("outgoing", "link"),
    ] {
        assert_admitted(
            &invariant("Order", &format!("reaches(self, self, {role})")),
            relationship,
            role,
        );
    }
}

/// FR-365-AC-1 (C05): the field-edge form checks as before: a field-edge
/// `Reaches` node over the field `trail`.
#[trace("TC-1046", "FR-365-AC-1")]
#[test]
fn c05_the_field_edge_form_still_checks() {
    let declarations = invariant("Order", "reaches(self, self, trail)");
    let graph = check(&declarations).expect("the field-edge form checks");
    let body = graph.state_clause("I").expect("I is checked").body();
    assert_eq!(body.value_type(), &ValueType::Boolean);
    assert!(
        matches!(body.kind(), NodeKind::Reaches { edge, .. }
            if edge == &FieldRef::new(object(&graph, "Order"), "trail")),
        "a field edge, not an end"
    );
}

/// FR-365-AC-2 (C06): a name matching a field and an end, and a name matching
/// two ends, refuse `ambiguous_declaration`/`ambiguous-name` naming both
/// candidates.
#[trace("TC-1046", "FR-365-AC-2")]
#[test]
fn c06_an_ambiguous_name_refuses_naming_both_candidates() {
    let graph = check(&invariant("Order", "true")).expect("fixture R checks");
    let parent = assert_refused(
        &invariant("Order", "reaches(self, self, parent)"),
        Code::AmbiguousDeclaration,
        "ambiguous-name",
        vec![],
    );
    assert_eq!(
        candidates(&parent),
        (
            "parent",
            &[
                MemberCandidate::Field(FieldRef::new(object(&graph, "Order"), "parent")),
                MemberCandidate::RelationshipEnd {
                    relationship: relationship_key("parent"),
                    side: EndSide::Source,
                },
            ][..]
        )
    );
    let twin = assert_refused(
        &invariant("Order", "reaches(self, self, twin)"),
        Code::AmbiguousDeclaration,
        "ambiguous-name",
        vec![],
    );
    assert_eq!(
        candidates(&twin),
        (
            "twin",
            &[
                MemberCandidate::RelationshipEnd {
                    relationship: relationship_key("twinA"),
                    side: EndSide::Source,
                },
                MemberCandidate::RelationshipEnd {
                    relationship: relationship_key("twinB"),
                    side: EndSide::Source,
                },
            ][..]
        )
    );
}

/// FR-365-AC-2 (C07): a name matching neither refuses
/// `missing_declaration`/`missing-name`.
#[trace("TC-1046", "FR-365-AC-2")]
#[test]
fn c07_a_name_matching_neither_is_missing() {
    assert_refused(
        &invariant("Order", "reaches(self, self, nothing)"),
        Code::MissingDeclaration,
        "missing-name",
        vec![],
    );
}

/// FR-365-AC-3 (C08): an operand of another object type, and a scalar
/// operand, refuse `ill_typed`/`operator-ineligible` at the form.
#[trace("TC-1046", "FR-365-AC-3")]
#[test]
fn c08_an_operand_that_does_not_conform_to_the_near_end_is_ineligible() {
    for operation in ["toPerson", "toCode"] {
        assert_refused(
            &clause(
                "pre",
                &format!("Order::{operation}"),
                "reaches(self, target, next)",
            ),
            Code::IllTyped,
            "operator-ineligible",
            vec![],
        );
    }
}

/// FR-365-AC-3 (C09): the inverse of a `source-to-target` relationship is
/// refused for its direction.
#[trace("TC-1046", "FR-365-AC-3")]
#[test]
fn c09_the_inverse_end_of_a_source_to_target_relationship_is_ineligible() {
    assert_refused(
        &invariant("Order", "reaches(self, self, previous)"),
        Code::IllTyped,
        "operator-ineligible",
        vec![],
    );
}

/// FR-365-AC-3 (C10): a destination of another type, single-valued or finite
/// multi-valued, refuses `ill_typed`/`operator-ineligible`.
#[trace("TC-1046", "FR-365-AC-3")]
#[test]
fn c10_a_foreign_destination_is_ineligible() {
    for role in ["owner", "manyOwners"] {
        assert_refused(
            &invariant("Order", &format!("reaches(self, self, {role})")),
            Code::IllTyped,
            "operator-ineligible",
            vec![],
        );
    }
}

/// FR-365-AC-1 (C17): a finite multi-valued destination of the near end's own
/// type, `Set`-shaped (`kids`) or `Bag`-shaped (`litter`), checks `Boolean`
/// with the same node shape as a single-valued end.
#[trace("TC-1046", "FR-365-AC-1")]
#[test]
fn c17_a_finite_multi_valued_destination_checks_boolean() {
    for role in ["kids", "litter"] {
        assert_admitted(
            &invariant("Order", &format!("reaches(self, self, {role})")),
            role,
            role,
        );
    }
    assert_admitted(
        &clause("pre", "Rush::hop", "reaches(self, target, kids)"),
        "kids",
        "kids",
    );
}

/// FR-365-AC-3 (C18): the operand check runs first, so a second operand that
/// does not conform to the near end's type refuses `operator-ineligible` even
/// when the destination is also unbounded (`expression-form` otherwise).
#[trace("TC-1046", "FR-365-AC-3")]
#[test]
fn c18_the_operand_check_is_reported_before_direction_and_multiplicity() {
    assert_refused(
        &clause("pre", "Order::toPerson", "reaches(self, target, crowd)"),
        Code::IllTyped,
        "operator-ineligible",
        vec![],
    );
}

/// FR-365-AC-3 (C11): an unbounded, an ordered and a multiplicity-less
/// destination each refuse `unsupported_construct`/`expression-form`.
#[trace("TC-1046", "FR-365-AC-3")]
#[test]
fn c11_an_unbounded_ordered_or_undeclared_destination_is_unsupported() {
    for role in ["crowd", "ranked", "bare"] {
        assert_refused(
            &invariant("Order", &format!("reaches(self, self, {role})")),
            Code::UnsupportedConstruct,
            "expression-form",
            vec![],
        );
    }
}

/// FR-365-AC-3 (C12): `back`, a forward traversal of a `target-to-source`
/// relationship with an unbounded destination, is a direction refusal, not an
/// `expression-form`.
#[trace("TC-1046", "FR-365-AC-3")]
#[test]
fn c12_direction_is_reported_before_multiplicity() {
    assert_refused(
        &invariant("Order", "reaches(self, self, back)"),
        Code::IllTyped,
        "operator-ineligible",
        vec![],
    );
}

/// FR-365-AC-3 (C13): `reaches` in a `Value` function body refuses
/// `ill_typed`/`operator-ineligible` at the form.
#[trace("TC-1046", "FR-365-AC-3")]
#[test]
fn c13_a_function_body_has_no_observation_to_traverse() {
    assert_refused(
        "function f using v(o: Reference<M::Order>): Boolean pure { reaches(self, self, next) }",
        Code::IllTyped,
        "operator-ineligible",
        vec![],
    );
}

/// FR-365-AC-4 (C15): `Person::watch(o: Reference<M::Order>)` with
/// `reaches(o, o, next)` checks and adds the `orders` domain once beside the
/// context population `people`; with a second no-maximum population `archive`
/// over `Order` it refuses `ambiguous_declaration`/`ambiguous-name` naming
/// `archive` and `orders` in ascending key order.
#[trace("TC-1046", "FR-365-AC-4")]
#[test]
fn c15_the_walked_population_is_added_once_and_two_cover_ambiguously() {
    let declarations = clause("pre", "Person::watch", "reaches(o, o, next)");
    let graph = check(&declarations).expect("the clause checks");
    assert_eq!(
        reachabilities(&graph),
        vec![(
            ValueType::Boolean,
            relationship_key("next"),
            "next".to_owned()
        )]
    );
    assert_eq!(claim_domains(&graph), 2, "people and orders, once each");

    let refusal = assert_refusal(
        check_with(&[("archive", "Order")], &declarations),
        &declarations,
        Code::AmbiguousDeclaration,
        "ambiguous-name",
        vec![],
    );
    let CheckCause::StateModel(model) = &refusal.cause else {
        panic!("a StateModel refusal: {refusal:?}");
    };
    let CheckCause::AmbiguousName { name, .. } = &model.cause else {
        panic!("an ambiguous population name: {refusal:?}");
    };
    assert_eq!(
        name, "M::Order (ix://test/orders/archive, ix://test/orders/orders)",
        "archive and orders, in ascending DeclarationKey order"
    );
}

/// The population domains the `claim` occurrence of the checked clause names.
fn claim_domains(graph: &CheckedGraph) -> usize {
    use qsl_semantics::family::ClaimExtent;
    let counts: Vec<usize> = graph
        .requirements()
        .iter()
        .filter(|(key, _)| key.origin().role().as_str() == "claim")
        .map(|(_, record)| match record.requirements().extent() {
            ClaimExtent::Unbounded(domains) => domains.len(),
            other => panic!("an unbounded extent, not {other:?}"),
        })
        .collect();
    let [count] = counts[..] else {
        panic!("one claim occurrence: {counts:?}");
    };
    count
}

/// FR-365-AC-4: a reachability over the context population adds no second
/// domain.
#[trace("TC-1046", "FR-365-AC-4")]
#[test]
fn the_context_population_is_not_added_twice() {
    let graph = check(&invariant("Order", "reaches(self, self, next)")).expect("checks");
    assert_eq!(claim_domains(&graph), 1, "orders alone");
}

/// FR-365-AC-3 (C16): `loose`'s destination fails both the multiplicity and
/// the type check, and the multiplicity is reported first.
#[trace("TC-1046", "FR-365-AC-3")]
#[test]
fn c16_multiplicity_is_reported_before_destination_type() {
    assert_refused(
        &invariant("Order", "reaches(self, self, loose)"),
        Code::UnsupportedConstruct,
        "expression-form",
        vec![],
    );
}
