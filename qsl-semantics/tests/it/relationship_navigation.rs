// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-363 / TC-1031: `r.name` over a relationship end is typed by the
//! destination end's multiplicity, and every inadmissible navigation refuses
//! with its `StateModel` cause. One test per vector N01-N12 of fixture N.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::{
    AmbiguousMember, CheckCause, CheckRefusal, CheckedGraph, CheckedNode, EndSide, MemberCandidate,
    NavigationBound, NodeKind, PackageDeclarations,
};
use qsl_semantics::family::FamilyKind;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, FieldMemberRecord, Multiplicity,
    ObjectTypeRecord, RelationshipDirection, RelationshipEnd, RelationshipRecord, ScalarTypeRecord,
    ScalarValueDomain, ValueTypeRef,
};
use qsl_semantics::model::intake::SelectedModels;
use qsl_semantics::model::key::DeclarationKey;
use quire_exact::{
    CardinalityBound, CollectionKind, CollectionType, EffectiveId, IllTypedCause, Presence,
    ValueType,
};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::declaration::FieldRef;

use RelationshipDirection::{Bidirectional, SourceToTarget, TargetToSource, Undirected};

fn key(node: &str) -> DeclarationKey {
    DeclarationKey::fixture(format!("ix://test/orders/{node}"))
}

/// A multiplicity `{lower,upper}` with `unique: true` and `ordered: false`.
fn mult(lower: u64, upper: Option<u64>) -> Multiplicity {
    Multiplicity {
        lower,
        upper,
        ordered: false,
        unique: true,
    }
}

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

/// One relationship of fixture N, named `Source/role` as TC-1031 names it.
#[derive(Clone)]
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

/// The relationship table of TC-1031's fixture N.
fn links() -> Vec<Link> {
    let unbounded = Some(mult(0, None));
    vec![
        link(
            "Fulfilment/order",
            SourceToTarget,
            end("Fulfilment", Some("order"), unbounded),
            end("Order", None, Some(mult(0, Some(1)))),
        ),
        link(
            "Fulfilment/carrier",
            SourceToTarget,
            end("Fulfilment", Some("carrier"), unbounded),
            end("Carrier", None, Some(mult(1, Some(1)))),
        ),
        link(
            "Order/lines",
            Bidirectional,
            end("Order", Some("lines"), Some(mult(1, Some(1)))),
            end("Line", Some("owner"), Some(mult(0, Some(3)))),
        ),
        link(
            "Order/tags",
            SourceToTarget,
            end("Order", Some("tags"), unbounded),
            end(
                "Tag",
                None,
                Some(Multiplicity {
                    unique: false,
                    ..mult(1, Some(2))
                }),
            ),
        ),
        link(
            "Order/watchers",
            SourceToTarget,
            end("Order", Some("watchers"), unbounded),
            end("Person", None, unbounded),
        ),
        link(
            "Order/steps",
            SourceToTarget,
            end("Order", Some("steps"), unbounded),
            end(
                "Step",
                None,
                Some(Multiplicity {
                    ordered: true,
                    ..mult(0, Some(3))
                }),
            ),
        ),
        link(
            "Order/notes",
            SourceToTarget,
            end("Order", Some("notes"), unbounded),
            end("Note", None, None),
        ),
        link(
            "Order/twin",
            Bidirectional,
            end("Order", Some("twin"), Some(mult(0, Some(1)))),
            end("Order", Some("twin"), Some(mult(0, Some(1)))),
        ),
        link(
            "Order/rival",
            TargetToSource,
            end("Order", Some("rival"), Some(mult(0, Some(1)))),
            end("Rival", Some("rivalOf"), Some(mult(0, Some(1)))),
        ),
        link(
            "Order/peer",
            Undirected,
            end("Order", Some("peer"), Some(mult(0, Some(1)))),
            end("Person", Some("peerOf"), Some(mult(0, Some(1)))),
        ),
        link(
            "Dual/link",
            SourceToTarget,
            end("Dual", Some("link"), unbounded),
            end("Order", None, Some(mult(1, Some(1)))),
        ),
    ]
}

/// `links` with the relationship `id` set to `direction`.
fn redirected(id: &str, direction: RelationshipDirection) -> Vec<Link> {
    links()
        .into_iter()
        .map(|link| {
            if link.id == id {
                Link { direction, ..link }
            } else {
                link
            }
        })
        .collect()
}

fn relationship_node(id: &str) -> String {
    format!("relationship/{}", id.replace('/', "-"))
}

fn relationship_key(id: &str) -> DeclarationKey {
    key(&relationship_node(id))
}

fn object_type(name: &str, supertypes: &[&str]) -> DomainPackageRecord {
    DomainPackageRecord::ObjectType(ObjectTypeRecord {
        key: key(name),
        interface_features: None,
        abstract_type: false,
        supertypes: supertypes.iter().copied().map(key).collect(),
    })
}

fn field(owner: &str, name: &str, value_type: &str) -> DomainPackageRecord {
    DomainPackageRecord::FieldMember(FieldMemberRecord {
        key: key(&format!("{owner}/{name}")),
        owner: key(owner),
        value_type: ValueTypeRef::Package(key(value_type)),
        multiplicity: mult(1, Some(1)),
        presence: Presence::Required,
        subsets: Vec::new(),
        redefines: None,
    })
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

/// Fixture N: its object types, `Order/code` and `Dual/link` fields, and the
/// given relationships.
fn package(links: &[Link]) -> DomainPackage {
    let mut records = vec![
        object_type("Order", &[]),
        object_type("Fulfilment", &[]),
        object_type("Rush", &["Fulfilment"]),
        object_type("Line", &[]),
        object_type("Tag", &[]),
        object_type("Person", &[]),
        object_type("Step", &[]),
        object_type("Note", &[]),
        object_type("Carrier", &[]),
        object_type("Dual", &[]),
        object_type("Rival", &[]),
        DomainPackageRecord::ScalarType(ScalarTypeRecord {
            key: key("Code"),
            domain: ScalarValueDomain::Integer { lower: 0, upper: 9 },
        }),
        field("Order", "code", "Code"),
        field("Dual", "link", "Order"),
    ];
    records.extend(links.iter().map(relationship));
    DomainPackage::new(DomainPackageRef::fixture("bundle.orders"), records)
}

/// Compile fixture N with `links` and check
/// `invariant I using v on M::<owner> at current { <body> }`.
fn compile(links: &[Link], owner: &str, body: &str) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    check_invariant(package(links), owner, body)
}

/// Check `invariant I using v on M::<owner> at current { <body> }` over the
/// admitted model `package`, selected as `M`.
pub(crate) fn check_invariant(
    package: DomainPackage,
    owner: &str,
    body: &str,
) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    check_declarations(
        package,
        &format!("invariant I using v on M::{owner} at current {{ {body} }}"),
    )
}

/// Check `declarations` over the admitted model `package`, selected as `M`.
fn check_declarations(
    package: DomainPackage,
    declarations: &str,
) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    let digest = qsl_semantics::model::key::hex(&package.model_selection.digest);
    let identity = package.model_selection.identity.clone();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model M = {identity:?} version \"1\" digest \"sha256-jcs:{digest}\";\n\
         {declarations}\n"
    );
    let selected = SelectedModels::fixture(
        "M",
        qsl_foundation::Span { start: 0, end: 1 },
        package,
        ModelNormalizationLimits::UNLIMITED,
    )
    .expect("the model admits");
    let built = crate::model_operations::parse_and_build(&unit);
    let raw = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "tc-1031", "fixture", "1"),
        "unit.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit");
    let declarations =
        PackageDeclarations::assemble(raw.source().reference().clone(), built, selected, vec![])
            .expect("the model assembles");
    declarations.check(CheckingLimits::default())
}

pub(crate) fn descendants(root: CheckedNode<'_>) -> Vec<CheckedNode<'_>> {
    let mut all = vec![root];
    let mut pending = root.children();
    while let Some(node) = pending.pop() {
        all.push(node);
        pending.extend(node.children());
    }
    all
}

/// The object type `M::<name>`.
pub(crate) fn object(graph: &CheckedGraph, name: &str) -> EffectiveId {
    graph
        .scope()
        .types()
        .object_types()
        .find(|declared| declared.name() == format!("M::{name}"))
        .unwrap_or_else(|| panic!("M::{name} is declared"))
        .key()
}

fn reference(graph: &CheckedGraph, name: &str) -> ValueType {
    ValueType::Reference(object(graph, name))
}

fn collection(kind: CollectionKind, element: ValueType, lower: u64, upper: u64) -> ValueType {
    ValueType::collection(CollectionType::new(
        kind,
        element,
        Some(CardinalityBound::new(lower, upper).expect("a nonempty bound")),
    ))
}

/// Each `Navigate` node of invariant `I`: its static type, relationship and
/// role. The body holds no `Attribute` node, so no `deref` read stands in for
/// a navigation.
pub(crate) fn navigations(graph: &CheckedGraph) -> Vec<(ValueType, DeclarationKey, String)> {
    let body = graph.state_clause("I").expect("I is checked").body();
    let nodes = descendants(body);
    assert!(
        nodes
            .iter()
            .all(|node| !matches!(node.kind(), NodeKind::Attribute { .. })),
        "a navigation is not a field read"
    );
    nodes
        .iter()
        .filter_map(|node| match node.kind() {
            NodeKind::Navigate {
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

/// `body` over `owner` checks to navigations of exactly
/// `(type, relationship, role)`, one per occurrence of the name.
fn assert_navigates(
    links: &[Link],
    owner: &str,
    body: &str,
    expected: impl FnOnce(&CheckedGraph) -> (ValueType, DeclarationKey, &'static str),
    occurrences: usize,
) {
    let graph = compile(links, owner, body)
        .unwrap_or_else(|refusals| panic!("{body} on M::{owner} checks: {refusals:?}"));
    let (value_type, relationship, role) = expected(&graph);
    assert_eq!(
        navigations(&graph),
        vec![(value_type, relationship, role.to_owned()); occurrences],
        "{body} on M::{owner}"
    );
}

/// `body` over `owner` is refused once, with `code`, `cause` and the located
/// `path`, attributed to the `StateModel` family and its catalog prefix.
fn assert_refused(
    links: &[Link],
    owner: &str,
    body: &str,
    code: Code,
    cause: &str,
    path: Vec<usize>,
) -> CheckRefusal {
    assert_refusal(
        compile(links, owner, body),
        &format!("{body} on M::{owner}"),
        code,
        cause,
        path,
    )
}

/// `result` is one `StateModel` refusal with `code`, `cause` and `path`.
fn assert_refusal(
    result: Result<CheckedGraph, Vec<CheckRefusal>>,
    body: &str,
    code: Code,
    cause: &str,
    path: Vec<usize>,
) -> CheckRefusal {
    let Err(mut refusals) = result else {
        panic!("{body} is refused")
    };
    assert_eq!(refusals.len(), 1, "{body}: {refusals:?}");
    let refusal = refusals.remove(0);
    assert_eq!(refusal.cause.code(), code, "{body}: {refusal:?}");
    assert_eq!(refusal.cause.cause(), Some(cause), "{body}: {refusal:?}");
    assert_eq!(refusal.location.path(), path, "{body}: {refusal:?}");
    let CheckCause::StateModel(model) = &refusal.cause else {
        panic!("{body}: a StateModel refusal, not {refusal:?}");
    };
    assert_eq!(model.family(), FamilyKind::StateModel);
    assert_eq!(
        refusal
            .cause
            .catalog_code()
            .and_then(|catalog| catalog.prefix()),
        Some(FamilyKind::StateModel.catalog_code_prefix()),
        "{body}: the catalog code carries the state-model prefix"
    );
    refusal
}

fn candidates(refusal: &CheckRefusal) -> (&str, &[MemberCandidate]) {
    let CheckCause::StateModel(model) = &refusal.cause else {
        panic!("a StateModel refusal: {refusal:?}");
    };
    let CheckCause::AmbiguousMember(ambiguous) = &model.cause else {
        panic!("an ambiguous member: {refusal:?}");
    };
    let AmbiguousMember { name, candidates } = &**ambiguous;
    (name, candidates)
}

/// FR-363-AC-1 (N01): `present(self.order)` over the `[0,1]` destination
/// checks, and `self.order` is an `Option<Reference<M::Order>>` navigation.
#[trace("TC-1031", "FR-363-AC-1")]
#[test]
fn n01_present_of_a_zero_or_one_end_checks_as_an_option_reference() {
    assert_navigates(
        &links(),
        "Fulfilment",
        "present(self.order)",
        |graph| {
            (
                ValueType::option(reference(graph, "Order")),
                relationship_key("Fulfilment/order"),
                "order",
            )
        },
        1,
    );
}

/// FR-363-AC-1 (N02): a `[1,1]` destination types as `Reference<U>`.
#[trace("TC-1031", "FR-363-AC-1")]
#[test]
fn n02_a_one_to_one_end_is_a_reference() {
    assert_navigates(
        &links(),
        "Fulfilment",
        "self.carrier = self.carrier",
        |graph| {
            (
                reference(graph, "Carrier"),
                relationship_key("Fulfilment/carrier"),
                "carrier",
            )
        },
        2,
    );
}

/// FR-363-AC-1 (N03): a receiver whose type is a subtype of the source end's
/// type navigates.
#[trace("TC-1031", "FR-363-AC-1")]
#[test]
fn n03_a_subtype_receiver_navigates() {
    assert_navigates(
        &links(),
        "Rush",
        "present(self.order)",
        |graph| {
            (
                ValueType::option(reference(graph, "Order")),
                relationship_key("Fulfilment/order"),
                "order",
            )
        },
        1,
    );
}

/// FR-363-AC-1 (N04): the source role navigates forward to the target end's
/// `[0,3]` set; the target end's role names the inverse end under
/// `bidirectional`, whose destination is the source end `{1,1}`.
#[trace("TC-1031", "FR-363-AC-1")]
#[test]
fn n04_the_source_role_and_the_target_role_name_the_forward_and_inverse_ends() {
    assert_navigates(
        &links(),
        "Order",
        "size(self.lines) >= 0",
        |graph| {
            (
                collection(CollectionKind::Set, reference(graph, "Line"), 0, 3),
                relationship_key("Order/lines"),
                "lines",
            )
        },
        1,
    );
    assert_navigates(
        &links(),
        "Line",
        "self.owner = self.owner",
        |graph| {
            (
                reference(graph, "Order"),
                relationship_key("Order/lines"),
                "owner",
            )
        },
        2,
    );
}

/// FR-363-AC-1 (N05): another finite non-unique `[l,u]` types as a bag.
#[trace("TC-1031", "FR-363-AC-1")]
#[test]
fn n05_a_finite_non_unique_end_is_a_bag() {
    assert_navigates(
        &links(),
        "Order",
        "size(self.tags) >= 0",
        |graph| {
            (
                collection(CollectionKind::Bag, reference(graph, "Tag"), 1, 2),
                relationship_key("Order/tags"),
                "tags",
            )
        },
        1,
    );
}

/// FR-363-AC-3 (N06): `undirected` admits both traversals.
#[trace("TC-1031", "FR-363-AC-1", "FR-363-AC-3")]
#[test]
fn n06_an_undirected_relationship_admits_both_traversals() {
    assert_navigates(
        &links(),
        "Order",
        "present(self.peer)",
        |graph| {
            (
                ValueType::option(reference(graph, "Person")),
                relationship_key("Order/peer"),
                "peer",
            )
        },
        1,
    );
    assert_navigates(
        &links(),
        "Person",
        "present(self.peerOf)",
        |graph| {
            (
                ValueType::option(reference(graph, "Order")),
                relationship_key("Order/peer"),
                "peerOf",
            )
        },
        1,
    );
}

/// FR-363-AC-2 (N07): a name matching the source and the target end of one
/// relationship, and a name matching a field and an end, refuse
/// `ambiguous_declaration`/`ambiguous-name` at the form, naming every
/// candidate.
#[trace("TC-1031", "FR-363-AC-2")]
#[test]
fn n07_an_ambiguous_name_refuses_naming_every_candidate() {
    let twin = assert_refused(
        &links(),
        "Order",
        "present(self.twin)",
        Code::AmbiguousDeclaration,
        "ambiguous-name",
        vec![0],
    );
    assert_eq!(
        candidates(&twin),
        (
            "twin",
            &[
                MemberCandidate::RelationshipEnd {
                    relationship: relationship_key("Order/twin"),
                    side: EndSide::Source,
                },
                MemberCandidate::RelationshipEnd {
                    relationship: relationship_key("Order/twin"),
                    side: EndSide::Target,
                },
            ][..]
        )
    );

    let dual = assert_refused(
        &links(),
        "Dual",
        "present(self.link)",
        Code::AmbiguousDeclaration,
        "ambiguous-name",
        vec![0],
    );
    let graph = compile(&links(), "Dual", "true").expect("Dual checks");
    assert_eq!(
        candidates(&dual),
        (
            "link",
            &[
                MemberCandidate::Field(FieldRef::new(object(&graph, "Dual"), "link")),
                MemberCandidate::RelationshipEnd {
                    relationship: relationship_key("Dual/link"),
                    side: EndSide::Source,
                },
            ][..]
        )
    );
}

/// FR-363-AC-2 (N08): a name matching no field and no end refuses
/// `ill_typed`/`type-mismatch` at the form.
#[trace("TC-1031", "FR-363-AC-2")]
#[test]
fn n08_an_unknown_name_is_a_type_mismatch() {
    assert_refused(
        &links(),
        "Order",
        "present(self.nope)",
        Code::IllTyped,
        "type-mismatch",
        vec![0],
    );
}

/// FR-363-AC-3 (N09): a forward traversal against `target-to-source`, and an
/// inverse traversal against `source-to-target`, refuse
/// `ill_typed`/`operator-ineligible`; the inverse of a `target-to-source`
/// relationship navigates.
#[trace("TC-1031", "FR-363-AC-3")]
#[test]
fn n09_a_traversal_against_the_direction_is_operator_ineligible() {
    assert_refused(
        &links(),
        "Order",
        "present(self.rival)",
        Code::IllTyped,
        "operator-ineligible",
        vec![0],
    );
    assert_navigates(
        &links(),
        "Rival",
        "present(self.rivalOf)",
        |graph| {
            (
                ValueType::option(reference(graph, "Order")),
                relationship_key("Order/rival"),
                "rivalOf",
            )
        },
        1,
    );
    assert_refused(
        &redirected("Order/lines", SourceToTarget),
        "Line",
        "self.owner = self.owner",
        Code::IllTyped,
        "operator-ineligible",
        vec![0],
    );
}

/// FR-363-AC-4 (N10): an unbounded, an ordered and a multiplicity-less
/// destination end each refuse `unsupported_construct`/`expression-form`.
#[trace("TC-1031", "FR-363-AC-4")]
#[test]
fn n10_an_unbounded_ordered_or_undeclared_end_is_unsupported() {
    for (name, bound) in [
        ("watchers", NavigationBound::Unbounded),
        ("steps", NavigationBound::Ordered),
        ("notes", NavigationBound::Undeclared),
    ] {
        let refusal = assert_refused(
            &links(),
            "Order",
            &format!("size(self.{name}) >= 0"),
            Code::UnsupportedConstruct,
            "expression-form",
            vec![0, 0],
        );
        let CheckCause::StateModel(model) = &refusal.cause else {
            panic!("{name}: a StateModel refusal");
        };
        assert_eq!(
            model.cause,
            CheckCause::UnsupportedNavigation(bound),
            "{name}"
        );
    }
}

/// FR-363-AC-4 (N11): the direction refusal precedes the bound refusal.
#[trace("TC-1031", "FR-363-AC-4")]
#[test]
fn n11_the_direction_refusal_precedes_the_bound_refusal() {
    assert_refused(
        &redirected("Order/watchers", TargetToSource),
        "Order",
        "size(self.watchers) >= 0",
        Code::IllTyped,
        "operator-ineligible",
        vec![0, 0],
    );
}

/// A read `x.f` whose receiver cannot be a model receiver is still the record
/// field read it was before FR-363: a `Value`-family `ill_typed`/`type-mismatch`
/// at the form, not a `StateModel` refusal at the operand.
fn assert_old_field_refusal(declarations: &str) {
    let Err(refusals) = check_declarations(package(&links()), declarations) else {
        panic!("{declarations} is refused");
    };
    assert_eq!(refusals.len(), 1, "{declarations}: {refusals:?}");
    assert_eq!(
        refusals[0].cause,
        CheckCause::IllTyped(IllTypedCause::TypeMismatch),
        "{declarations}"
    );
    assert_eq!(refusals[0].location.path(), vec![0], "{declarations}");
}

/// `x.f` over an `Integer`, a `Set<Integer>` and a tuple keeps its
/// `Value`-family refusal at the form, in a function body and in a state clause.
#[test]
fn a_non_model_receiver_keeps_its_field_read_refusal() {
    for declarations in [
        "function f using v(n: Integer): Boolean pure { n.x = n.x }",
        "function f using v(s: Set<Integer>[0, 2]): Boolean pure { s.x = s.x }",
        "tuple Pair(Integer, Integer);\n\
         function f using v(p: Pair): Boolean pure { p.x = p.x }",
        "invariant I using v on M::Order at current { self.code.x = self.code.x }",
    ] {
        assert_old_field_refusal(declarations);
    }
}

/// A bare `r.f` over a `Reference` in a function body is not a field read of
/// the object, and keeps its `Value`-family refusal at the form.
#[test]
fn a_bare_field_read_in_a_function_body_keeps_its_refusal() {
    assert_old_field_refusal(
        "function f using v(o: Reference<M::Order>): Boolean pure { o.code = o.code }",
    );
}

/// FR-363-AC-5 (N12): `deref(self).order` refuses `type-mismatch` at the
/// form, and `self.order.order` over an `Option<Reference<M::Order>>` refuses
/// it at the receiver operand `self.order`, not the whole expression.
#[trace("TC-1031", "FR-363-AC-5")]
#[test]
fn n12_a_deref_or_a_non_reference_receiver_is_a_type_mismatch() {
    let deref = assert_refused(
        &links(),
        "Fulfilment",
        "present(deref(self).order)",
        Code::IllTyped,
        "type-mismatch",
        vec![0],
    );
    assert_eq!(deref.location.path(), vec![0], "the form deref(self).order");

    let option = assert_refused(
        &links(),
        "Fulfilment",
        "present(self.order.order)",
        Code::IllTyped,
        "type-mismatch",
        vec![0, 0],
    );
    assert_eq!(option.location.path(), vec![0, 0], "the operand self.order");

    // `lines` is navigable from `Order`, so only the receiver's type refuses.
    assert_refused(
        &links(),
        "Fulfilment",
        "present(self.order.lines)",
        Code::IllTyped,
        "type-mismatch",
        vec![0, 0],
    );
}

/// FR-363-AC-5 (N13): in a function body, `o.lines` over an `Option` or a
/// collection of references refuses `type-mismatch` at the receiver operand
/// `o`, as it does in an invariant; the receiver's type, not the context,
/// decides.
#[trace("TC-1031", "FR-363-AC-5")]
#[test]
fn n13_a_non_reference_receiver_in_a_function_body_is_a_type_mismatch() {
    for parameter in [
        "Option<Reference<M::Order>>",
        "Set<Reference<M::Order>>[0, 2]",
    ] {
        let declarations =
            format!("function f using v(o: {parameter}): Boolean pure {{ o.lines = o.lines }}");
        assert_refusal(
            check_declarations(package(&links()), &declarations),
            &declarations,
            Code::IllTyped,
            "type-mismatch",
            vec![0, 0],
        );
    }
}

/// FR-363-AC-1 (N14): a `Reference` receiver navigates in a function body.
#[trace("TC-1031", "FR-363-AC-1")]
#[test]
fn n14_a_reference_receiver_navigates_in_a_function_body() {
    let declarations =
        "function f using v(o: Reference<M::Order>): Boolean pure { size(o.lines) >= 0 }";
    check_declarations(package(&links()), declarations)
        .unwrap_or_else(|refusals| panic!("{declarations} checks: {refusals:?}"));
}
