// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1048 (FR-365): `reaches(a, b, name)` over a relationship end evaluated
//! through a state clause against an observation's relationship extents and
//! population manifest, reading only `complete`, `targets`, `sources` and
//! `resolve`. One test per TC-1048 row; each asserts the Boolean or the typed
//! outcome, the charged points in order, and the work units. Every expected
//! value is the row's.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, Evaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{PackageDeclarations, SCALAR_LIMITS_UNLIMITED};
use qsl_semantics::family::{FamilyOutcome, FamilyResult};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, Extent, FieldMemberRecord, Multiplicity,
    ObjectTypeRecord, OperationEffect, OperationMemberRecord, OperationParameterRecord,
    PopulationRecord, RelationshipDirection, RelationshipEnd as DeclaredEnd, RelationshipRecord,
    ValueTypeRef,
};
use qsl_semantics::model::intake::SelectedModels;
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::model::observation::{
    AdmissionUsage, AdmittedObservations, DocumentRef, Observation, PopulationEnumerationFailure,
    RelationshipExtents, SelfKind,
};
use quire_exact::{
    ChargePoint, EffectiveId, FieldValue, LimitKind, Meter, ObjectId, ObjectReference, Outcome,
    Presence, ScalarLimits, UniverseId, Value,
};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::object_closure::ObjectClosure;
use quire_semantic_value::relationship_extent::{
    NavigationRelationship, PopulationEntry, PopulationEnumerator, RelationshipEnd,
    RelationshipExtent,
};

use crate::support::admitted_charge_points;

const UNLIMITED: ScalarLimits = SCALAR_LIMITS_UNLIMITED;
const EXPAND: ChargePoint = ChargePoint::GraphExpand;
const EDGE: ChargePoint = ChargePoint::GraphEdge;
const RETAIN: ChargePoint = ChargePoint::GraphResultRetain;

fn key(node: &str) -> DeclarationKey {
    DeclarationKey::fixture(format!("ix://test/orders/{node}"))
}

fn next() -> DeclarationKey {
    key("relationship/Order-next")
}

fn link() -> DeclarationKey {
    key("relationship/Order-link")
}

fn mult(bounds: (u64, u64), unique: bool) -> Option<Multiplicity> {
    Some(Multiplicity {
        lower: bounds.0,
        upper: Some(bounds.1),
        ordered: false,
        unique,
    })
}

/// The variants of fixture R the rows use: `Order/next`'s ends are `unique`
/// (**bag** sets both `false`) and its target end has multiplicity `target`.
#[derive(Clone, Copy)]
struct Shape {
    unique: bool,
    target: (u64, u64),
}

impl Shape {
    const BASE: Self = Self {
        unique: true,
        target: (0, 1),
    };
}

/// The state clauses every row evaluates, all on `Order::reroute(target)`.
const CLAUSES: &str = "\
pre PreNext using v on M::Order::reroute { reaches(self, target, next) }
post PostNext using v on M::Order::reroute { reaches(self, target, next) }
pre PreIncoming using v on M::Order::reroute { reaches(self, target, incoming) }
pre PreOutgoing using v on M::Order::reroute { reaches(self, target, outgoing) }
";

fn object_type(name: &str, supertypes: &[&str]) -> DomainPackageRecord {
    DomainPackageRecord::ObjectType(ObjectTypeRecord {
        key: key(name),
        interface_features: None,
        abstract_type: false,
        supertypes: supertypes.iter().copied().map(key).collect(),
    })
}

fn declared_end(role: &str, multiplicity: Option<Multiplicity>) -> DeclaredEnd {
    DeclaredEnd {
        type_identity: key("Order"),
        role: Some(role.to_owned()),
        multiplicity,
    }
}

/// Fixture R's `Order`, `Rush`, the optional field `trail`, the population
/// `orders`, the operation `reroute(target)` and `Order/next` and
/// `Order/link`, checked with the four clauses of [`CLAUSES`].
fn orders_package(shape: Shape, declarations: &str) -> CheckedPackage {
    let relationship = |relationship_key, direction, source: DeclaredEnd, target: DeclaredEnd| {
        DomainPackageRecord::Relationship(RelationshipRecord {
            key: relationship_key,
            declared_at: None,
            source,
            target,
            direction,
        })
    };
    let records = vec![
        object_type("Order", &[]),
        object_type("Rush", &["Order"]),
        DomainPackageRecord::FieldMember(FieldMemberRecord {
            key: key("Order/trail"),
            owner: key("Order"),
            value_type: ValueTypeRef::Package(key("Order")),
            multiplicity: Multiplicity {
                lower: 1,
                upper: Some(1),
                ordered: false,
                unique: true,
            },
            presence: Presence::Optional,
            subsets: Vec::new(),
            redefines: None,
        }),
        DomainPackageRecord::Population(PopulationRecord {
            key: key("orders"),
            member_types: vec![key("Order")],
            extent: Extent::Closed,
            symmetric: false,
        }),
        DomainPackageRecord::OperationMember(OperationMemberRecord {
            key: key("Order/reroute"),
            owner: key("Order"),
            parameters: vec![OperationParameterRecord {
                key: key("Order/reroute/target"),
                value_type: ValueTypeRef::Package(key("Order")),
                multiplicity: Multiplicity {
                    lower: 1,
                    upper: Some(1),
                    ordered: false,
                    unique: true,
                },
            }],
            result: None,
            effect: OperationEffect::default(),
            requires: Vec::new(),
            ensures: Vec::new(),
            has_body: false,
            redefines: None,
        }),
        relationship(
            next(),
            RelationshipDirection::SourceToTarget,
            declared_end("next", mult((0, 1), shape.unique)),
            declared_end("previous", mult(shape.target, shape.unique)),
        ),
        relationship(
            link(),
            RelationshipDirection::Bidirectional,
            declared_end("outgoing", mult((0, 1), true)),
            declared_end("incoming", mult((0, 1), true)),
        ),
    ];
    let package = DomainPackage::new(DomainPackageRef::fixture("bundle.orders"), records);
    let digest = qsl_semantics::model::key::hex(&package.model_selection.digest);
    let identity = package.model_selection.identity.clone();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         model M = {identity:?} version \"1\" digest \"sha256-jcs:{digest}\";\n\
         {CLAUSES}{declarations}"
    );
    let selected = SelectedModels::fixture(
        "M",
        qsl_foundation::Span { start: 0, end: 1 },
        package,
        ModelNormalizationLimits::UNLIMITED,
    )
    .expect("the model admits");
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "tc-1048", "fixture", "1"),
        "unit.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit");
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let declarations =
        PackageDeclarations::assemble(parsed.source().reference().clone(), built, selected, vec![])
            .expect("the model assembles");
    CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .expect("the clauses check"),
    )
}

/// The enumerator the model layer passes: the rows in order.
struct Rows(std::vec::IntoIter<PopulationEntry<DeclarationKey>>);

impl PopulationEnumerator<DeclarationKey> for Rows {
    type Error = PopulationEnumerationFailure;

    fn next_row(
        &mut self,
    ) -> Result<Option<PopulationEntry<DeclarationKey>>, PopulationEnumerationFailure> {
        Ok(self.0.next())
    }
}

type Link = (ObjectReference, ObjectReference);

/// One scenario: a checked package and the observations its clauses read.
struct Orders {
    package: CheckedPackage,
    shape: Shape,
}

impl Orders {
    fn new(shape: Shape) -> Self {
        Self::with_declarations(shape, "")
    }

    /// The scenario whose unit also holds `declarations`.
    fn with_declarations(shape: Shape, declarations: &str) -> Self {
        Self {
            package: orders_package(shape, declarations),
            shape,
        }
    }

    fn base() -> Self {
        Self::new(Shape::BASE)
    }

    fn id(&self, name: &str) -> EffectiveId {
        self.package
            .graph()
            .scope()
            .types()
            .object_types()
            .find(|declared| declared.name() == format!("M::{name}"))
            .unwrap_or_else(|| panic!("M::{name} is declared"))
            .key()
    }

    fn at(&self, type_name: &str, object: &str) -> ObjectReference {
        ObjectReference::new(
            UniverseId::from_digest([9; 32]),
            self.id(type_name),
            ObjectId::new(object.to_owned()).expect("a nonempty object id"),
        )
    }

    fn order(&self, object: &str) -> ObjectReference {
        self.at("Order", object)
    }

    fn row(
        &self,
        population: &str,
        covered: &[&str],
        complete: bool,
    ) -> PopulationEntry<DeclarationKey> {
        PopulationEntry::try_new(
            key(population),
            covered.iter().map(|name| self.id(name)),
            complete,
        )
        .expect("a nonempty population row")
    }

    /// The population `Q` holding every type, complete.
    fn q(&self) -> PopulationEntry<DeclarationKey> {
        self.row("Q", &["Order", "Rush"], true)
    }

    /// Variant **spare**: `Q` and a population `fast` over `Rush` that is
    /// not held complete.
    fn spare_rows(&self) -> Vec<PopulationEntry<DeclarationKey>> {
        vec![self.q(), self.row("fast", &["Rush"], false)]
    }

    fn extent(
        relationship: DeclarationKey,
        complete: bool,
        instances: Vec<Link>,
    ) -> RelationshipExtent<DeclarationKey> {
        RelationshipExtent {
            relationship,
            complete,
            instances,
        }
    }

    /// The model-layer observation: `held` objects, `extents` admitted
    /// without a closure check, and the populations `rows`.
    fn observe(
        &self,
        held: &[ObjectReference],
        extents: Vec<RelationshipExtent<DeclarationKey>>,
        rows: Vec<PopulationEntry<DeclarationKey>>,
    ) -> ObjectEnvironment {
        self.observe_with(
            held.iter().cloned().map(|object| (object, vec![])),
            extents,
            rows,
        )
    }

    fn observe_with<'n>(
        &self,
        held: impl IntoIterator<Item = (ObjectReference, Vec<(&'n str, FieldValue)>)>,
        extents: Vec<RelationshipExtent<DeclarationKey>>,
        rows: Vec<PopulationEntry<DeclarationKey>>,
    ) -> ObjectEnvironment {
        let types = self.package.graph().scope().types();
        let end = |unique| RelationshipEnd {
            object_type: self.id("Order"),
            unique,
        };
        let navigation = vec![
            NavigationRelationship {
                key: next(),
                source: end(self.shape.unique),
                target: end(self.shape.unique),
            },
            NavigationRelationship {
                key: link(),
                source: end(true),
                target: end(true),
            },
        ];
        let limit = rows.len();
        let extents = RelationshipExtents::admit(
            types,
            navigation,
            extents,
            &mut Rows(rows.into_iter()),
            limit,
        )
        .expect("the extents admit");
        let closure = ObjectClosure::new(types, held, &[]).expect("a closure of the held objects");
        ObjectEnvironment::new(closure).with_extents(extents)
    }

    /// Objects `o1` to `o5` held, `Order`s.
    fn held(&self) -> Vec<ObjectReference> {
        ["o1", "o2", "o3", "o4", "o5"]
            .iter()
            .map(|name| self.order(name))
            .collect()
    }

    fn observations(
        &self,
        clause: &str,
        start: &ObjectReference,
        goal: &ObjectReference,
        pre: Option<&ObjectEnvironment>,
        post: Option<&ObjectEnvironment>,
    ) -> AdmittedObservations {
        let instant = |label: &str, environment: &ObjectEnvironment| Observation {
            identity: DocumentRef {
                authority: "test".to_owned(),
                identity: label.to_owned(),
                revision_namespace: "fixture".to_owned(),
                revision: "1".to_owned(),
                digest: [3; 32],
            },
            environment: environment.clone(),
            populations: BTreeMap::new(),
            out_of_range: Vec::new(),
        };
        AdmittedObservations {
            usage: AdmissionUsage::default(),
            clause: self
                .package
                .graph()
                .state_clause(clause)
                .expect("the clause is declared")
                .identity(),
            current: None,
            pre: pre.map(|environment| instant("pre", environment)),
            post: post.map(|environment| instant("post", environment)),
            self_object: start.clone(),
            invocation: None,
            self_kind: SelfKind::Object,
            receiver_pre: None,
            receiver_post: None,
            parameters: vec![("target".to_owned(), Value::Reference(goal.clone()))],
            result: None,
            created: Vec::new(),
            deleted: Vec::new(),
        }
    }

    /// `reaches(start, goal, role)` through `clause` over `environment` (the
    /// clause's own observation) under `limits`.
    fn run(
        &self,
        clause: &str,
        start: &ObjectReference,
        goal: &ObjectReference,
        environment: &ObjectEnvironment,
        limits: ScalarLimits,
    ) -> (Evaluation, Meter) {
        let observations = if clause.starts_with("Post") {
            self.observations(clause, start, goal, Some(environment), Some(environment))
        } else {
            self.observations(clause, start, goal, Some(environment), None)
        };
        let mut meter = Meter::new(limits);
        let report = self
            .package
            .evaluate_clause(
                &QualifiedName::unqualified(clause).unwrap(),
                &observations,
                &mut meter,
            )
            .expect("the evaluation does not fault");
        (report.evaluation, meter)
    }

    fn reaches(
        &self,
        start: &ObjectReference,
        goal: &ObjectReference,
        environment: &ObjectEnvironment,
        limits: ScalarLimits,
    ) -> (Evaluation, Meter) {
        self.run("PreNext", start, goal, environment, limits)
    }
}

fn boolean(evaluation: Evaluation) -> bool {
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(holds))) => holds,
        other => panic!("expected a Boolean, got {other:?}"),
    }
}

/// A family refusal or incomplete outcome: code, cause and fields.
fn refusal(evaluation: Evaluation) -> (String, String, BTreeMap<&'static str, String>) {
    match evaluation.outcome {
        FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause)) => {
            let code = cause.catalog_code();
            (
                code.code().to_owned(),
                code.cause().to_owned(),
                cause.catalog_fields().unwrap_or_default(),
            )
        }
        other => panic!("expected a family refusal, got {other:?}"),
    }
}

fn assert_charges(meter: &Meter, points: &[ChargePoint], work: u64, results: u64) {
    assert_eq!(admitted_charge_points(meter), points);
    assert_eq!(meter.consumed(LimitKind::WorkUnits), work);
    assert_eq!(meter.consumed(LimitKind::ResultUnits), results);
}

/// `Order/next` links `(source, target)` by object name.
fn links(
    orders: &Orders,
    relationship: DeclarationKey,
    pairs: &[(&str, &str)],
) -> RelationshipExtent<DeclarationKey> {
    Orders::extent(
        relationship,
        true,
        pairs
            .iter()
            .map(|(source, target)| (orders.order(source), orders.order(target)))
            .collect(),
    )
}

fn chain(orders: &Orders) -> ObjectEnvironment {
    orders.observe(
        &orders.held(),
        vec![links(orders, next(), &[("o1", "o2"), ("o2", "o3")])],
        vec![orders.q()],
    )
}

/// One row of 'a true or false after a charge schedule' over `environment`.
fn assert_decides(
    orders: &Orders,
    environment: &ObjectEnvironment,
    start: &str,
    goal: &str,
    expected: bool,
    points: &[ChargePoint],
    work: u64,
) {
    let (evaluation, meter) = orders.reaches(
        &orders.order(start),
        &orders.order(goal),
        environment,
        UNLIMITED,
    );
    assert_eq!(boolean(evaluation), expected, "reaches({start}, {goal})");
    assert_charges(&meter, points, work, 1);
}

/// TC-1048 row 1: a two-link chain reaches its far end.
#[trace("TC-1048", "FR-365-AC-7", "FR-365-AC-9")]
#[test]
fn row_1_a_chain_reaches_its_far_end_after_five_work_units() {
    let orders = Orders::base();
    assert_decides(
        &orders,
        &chain(&orders),
        "o1",
        "o3",
        true,
        &[EXPAND, EDGE, EXPAND, EDGE, RETAIN],
        5,
    );
}

/// TC-1048 row 2: a chain does not reach its start; `reaches(o1, o1)` is
/// `false`.
#[trace("TC-1048", "FR-365-AC-7", "FR-365-AC-9")]
#[test]
fn row_2_a_chain_does_not_reach_its_start() {
    let orders = Orders::base();
    let environment = chain(&orders);
    assert_decides(
        &orders,
        &environment,
        "o3",
        "o1",
        false,
        &[EXPAND, RETAIN],
        2,
    );
    assert_decides(
        &orders,
        &environment,
        "o1",
        "o1",
        false,
        &[EXPAND, EDGE, EXPAND, EDGE, EXPAND, RETAIN],
        6,
    );
}

/// TC-1048 row 3: an isolated object does not reach itself, and equal-valued
/// objects are not the same target.
#[trace("TC-1048", "FR-365-AC-7")]
#[test]
fn row_3_an_isolated_object_reaches_neither_itself_nor_an_equal_valued_one() {
    let orders = Orders::base();
    let environment = orders.observe(
        &orders.held(),
        vec![links(&orders, next(), &[])],
        vec![orders.q()],
    );
    assert_decides(
        &orders,
        &environment,
        "o4",
        "o4",
        false,
        &[EXPAND, RETAIN],
        2,
    );
    assert_decides(
        &orders,
        &environment,
        "o4",
        "o5",
        false,
        &[EXPAND, RETAIN],
        2,
    );
}

/// TC-1048 row 4: a self-loop returns to its start.
#[trace("TC-1048", "FR-365-AC-7")]
#[test]
fn row_4_a_self_loop_reaches_itself() {
    let orders = Orders::base();
    let environment = orders.observe(
        &orders.held(),
        vec![links(&orders, next(), &[("o4", "o4")])],
        vec![orders.q()],
    );
    assert_decides(
        &orders,
        &environment,
        "o4",
        "o4",
        true,
        &[EXPAND, EDGE, RETAIN],
        3,
    );
}

/// TC-1048 row 5: a cycle returns to its start.
#[trace("TC-1048", "FR-365-AC-7")]
#[test]
fn row_5_a_cycle_reaches_its_start() {
    let orders = Orders::base();
    let environment = orders.observe(
        &orders.held(),
        vec![links(&orders, next(), &[("o1", "o2"), ("o2", "o1")])],
        vec![orders.q()],
    );
    assert_decides(
        &orders,
        &environment,
        "o1",
        "o1",
        true,
        &[EXPAND, EDGE, EXPAND, EDGE, RETAIN],
        5,
    );
}

/// TC-1048 row 6: a cycle that holds no target is `false`, and the repeated
/// neighbour is discovered once.
#[trace("TC-1048", "FR-365-AC-7", "FR-365-AC-9")]
#[test]
fn row_6_a_cycle_without_the_target_is_false() {
    let orders = Orders::base();
    let environment = orders.observe(
        &orders.held(),
        vec![links(&orders, next(), &[("o1", "o2"), ("o2", "o1")])],
        vec![orders.q()],
    );
    assert_decides(
        &orders,
        &environment,
        "o1",
        "o3",
        false,
        &[EXPAND, EDGE, EXPAND, EDGE, RETAIN],
        5,
    );
}

/// TC-1048 row 7: a complete extent with no link is `false`.
#[trace("TC-1048", "FR-365-AC-7", "FR-365-AC-10")]
#[test]
fn row_7_an_empty_complete_extent_is_false() {
    let orders = Orders::base();
    let environment = orders.observe(
        &orders.held(),
        vec![links(&orders, next(), &[])],
        vec![orders.q()],
    );
    assert_decides(
        &orders,
        &environment,
        "o1",
        "o2",
        false,
        &[EXPAND, RETAIN],
        2,
    );
}

/// TC-1048 row 8: a `Rush` start navigates the base-typed relationship.
#[trace("TC-1048", "FR-365-AC-1", "FR-365-AC-7")]
#[test]
fn row_8_a_subtype_start_reaches_through_the_base_typed_relationship() {
    let orders = Orders::base();
    let r1 = orders.at("Rush", "r1");
    let mut held = orders.held();
    held.push(r1.clone());
    let environment = orders.observe(
        &held,
        vec![Orders::extent(
            next(),
            true,
            vec![
                (r1.clone(), orders.order("o2")),
                (orders.order("o2"), orders.order("o3")),
            ],
        )],
        vec![orders.q()],
    );
    let (evaluation, _) = orders.reaches(&r1, &orders.order("o2"), &environment, UNLIMITED);
    assert!(boolean(evaluation));
}

/// TC-1048 row 9: a repeated link charges one edge per occurrence and
/// discovers its target once.
#[trace("TC-1048", "FR-365-AC-8", "FR-365-AC-9")]
#[test]
fn row_9_a_repeated_link_charges_each_occurrence_and_is_discovered_once() {
    let orders = Orders::new(Shape {
        unique: false,
        ..Shape::BASE
    });
    let environment = orders.observe(
        &orders.held(),
        vec![links(
            &orders,
            next(),
            &[("o1", "o2"), ("o1", "o2"), ("o2", "o3")],
        )],
        vec![orders.q()],
    );
    assert_decides(
        &orders,
        &environment,
        "o1",
        "o3",
        true,
        &[EXPAND, EDGE, EXPAND, EDGE, EDGE, RETAIN],
        6,
    );
    let environment = orders.observe(
        &orders.held(),
        vec![links(&orders, next(), &[("o1", "o2"), ("o1", "o2")])],
        vec![orders.q()],
    );
    assert_decides(
        &orders,
        &environment,
        "o1",
        "o5",
        false,
        &[EXPAND, EDGE, EXPAND, EDGE, RETAIN],
        5,
    );
}

/// TC-1048 row 10: the inverse role reads `sources`.
#[trace("TC-1048", "FR-365-AC-8")]
#[test]
fn row_10_the_inverse_role_walks_the_sources() {
    let orders = Orders::base();
    let environment = orders.observe(
        &orders.held(),
        vec![links(&orders, link(), &[("o1", "o2"), ("o2", "o3")])],
        vec![orders.q()],
    );
    let (evaluation, _) = orders.run(
        "PreIncoming",
        &orders.order("o3"),
        &orders.order("o1"),
        &environment,
        UNLIMITED,
    );
    assert!(boolean(evaluation));
    let (evaluation, _) = orders.run(
        "PreOutgoing",
        &orders.order("o3"),
        &orders.order("o1"),
        &environment,
        UNLIMITED,
    );
    assert!(!boolean(evaluation));
}

/// `evaluation` is the incomplete outcome with these fields.
fn assert_incomplete(
    evaluation: Evaluation,
    kind: LimitKind,
    limit: u64,
    consumed: u64,
    next_charge: u64,
    point: ChargePoint,
) {
    let FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)) = evaluation.outcome else {
        panic!(
            "expected an incomplete outcome, got {:?}",
            evaluation.outcome
        );
    };
    assert_eq!(incomplete.limit_kind, kind);
    assert_eq!(incomplete.limit, limit);
    assert_eq!(incomplete.consumed, consumed);
    assert_eq!(
        incomplete.next_charge,
        quire_exact::Integer::from(next_charge)
    );
    assert_eq!(incomplete.charge_point, point);
}

fn row_1(
    orders: &Orders,
    environment: &ObjectEnvironment,
    limits: ScalarLimits,
) -> (Evaluation, Meter) {
    orders.reaches(
        &orders.order("o1"),
        &orders.order("o3"),
        environment,
        limits,
    )
}

/// TC-1048 row 11: work-unit budgets 5, 4 and 3 over row 1.
#[trace("TC-1048", "FR-365-AC-9")]
#[test]
fn row_11_work_units_complete_at_five_and_stop_at_the_next_charge_below() {
    let orders = Orders::base();
    let environment = chain(&orders);
    let (evaluation, meter) = row_1(
        &orders,
        &environment,
        ScalarLimits {
            work_units: 5,
            ..UNLIMITED
        },
    );
    assert!(boolean(evaluation));
    assert_charges(&meter, &[EXPAND, EDGE, EXPAND, EDGE, RETAIN], 5, 1);
    let (evaluation, _) = row_1(
        &orders,
        &environment,
        ScalarLimits {
            work_units: 4,
            ..UNLIMITED
        },
    );
    assert_incomplete(evaluation, LimitKind::WorkUnits, 4, 4, 1, RETAIN);
    let (evaluation, _) = row_1(
        &orders,
        &environment,
        ScalarLimits {
            work_units: 3,
            ..UNLIMITED
        },
    );
    assert_incomplete(evaluation, LimitKind::WorkUnits, 3, 3, 1, EDGE);
}

/// TC-1048 row 12: value-occurrence budgets 2 and 1 over row 1.
#[trace("TC-1048", "FR-365-AC-9")]
#[test]
fn row_12_value_occurrences_complete_at_two_and_stop_at_the_second_expand_below() {
    let orders = Orders::base();
    let environment = chain(&orders);
    let (evaluation, _) = row_1(
        &orders,
        &environment,
        ScalarLimits {
            value_occurrences: 2,
            ..UNLIMITED
        },
    );
    assert!(boolean(evaluation));
    let (evaluation, _) = row_1(
        &orders,
        &environment,
        ScalarLimits {
            value_occurrences: 1,
            ..UNLIMITED
        },
    );
    assert_incomplete(evaluation, LimitKind::ValueOccurrences, 1, 1, 2, EXPAND);
}

/// TC-1048 row 13: result-unit budgets 1 and 0 over row 1.
#[trace("TC-1048", "FR-365-AC-9")]
#[test]
fn row_13_result_units_complete_at_one_and_stop_at_retain_at_zero() {
    let orders = Orders::base();
    let environment = chain(&orders);
    let (evaluation, _) = row_1(
        &orders,
        &environment,
        ScalarLimits {
            result_units: 1,
            ..UNLIMITED
        },
    );
    assert!(boolean(evaluation));
    let (evaluation, _) = row_1(
        &orders,
        &environment,
        ScalarLimits {
            result_units: 0,
            ..UNLIMITED
        },
    );
    assert_incomplete(evaluation, LimitKind::ResultUnits, 0, 0, 1, RETAIN);
}

/// TC-1048 row 14: a zero work-unit limit is real and stops at the start's
/// expand.
#[trace("TC-1048", "FR-365-AC-9")]
#[test]
fn row_14_a_zero_work_limit_stops_before_the_start_is_discovered() {
    let orders = Orders::base();
    let environment = chain(&orders);
    let (evaluation, meter) = row_1(
        &orders,
        &environment,
        ScalarLimits {
            work_units: 0,
            ..UNLIMITED
        },
    );
    assert_incomplete(evaluation, LimitKind::WorkUnits, 0, 0, 1, EXPAND);
    assert_eq!(admitted_charge_points(&meter), []);
}

/// TC-1048 row 15: repeated runs start fresh and change nothing.
#[trace("TC-1048", "FR-365-AC-12")]
#[test]
fn row_15_repeated_runs_are_equal_and_leave_the_observation_unchanged() {
    let orders = Orders::base();
    let environment = chain(&orders);
    let before = format!("{environment:?}");
    let mut seen = Vec::new();
    for work_units in [
        UNLIMITED.work_units,
        UNLIMITED.work_units,
        4,
        UNLIMITED.work_units,
    ] {
        let (evaluation, meter) = row_1(
            &orders,
            &environment,
            ScalarLimits {
                work_units,
                ..UNLIMITED
            },
        );
        seen.push((
            work_units,
            format!("{:?}", evaluation.outcome),
            admitted_charge_points(&meter),
        ));
    }
    assert_eq!(seen[0].1, seen[1].1);
    assert_eq!(seen[0].2, seen[1].2);
    assert_eq!(
        seen[0].1, seen[3].1,
        "the incomplete run changes no later result"
    );
    assert_eq!(seen[0].2, seen[3].2);
    assert_eq!(
        seen[0].2,
        [EXPAND, EDGE, EXPAND, EDGE, RETAIN],
        "the complete runs' charges"
    );
    assert_ne!(seen[2].1, seen[0].1, "the 4 run is incomplete");
    assert_eq!(
        format!("{environment:?}"),
        before,
        "the observation equals its clone"
    );
}

/// TC-1048 row 16: an incomplete or unlisted extent is incomplete after the
/// start's expand, never `true` from a known link.
#[trace("TC-1048", "FR-365-AC-10")]
#[test]
fn row_16_an_unlisted_or_incomplete_extent_is_incomplete_after_the_start_expand() {
    let orders = Orders::base();
    let o1 = orders.order("o1");
    let incomplete_chain = orders.observe(
        &orders.held(),
        vec![Orders::extent(
            next(),
            false,
            vec![
                (o1.clone(), orders.order("o2")),
                (orders.order("o2"), orders.order("o3")),
            ],
        )],
        vec![orders.q()],
    );
    let unlisted = orders.observe(&orders.held(), vec![], vec![orders.q()]);
    let partial = orders.observe(
        &orders.held(),
        vec![Orders::extent(
            next(),
            false,
            vec![(o1.clone(), orders.order("o2"))],
        )],
        vec![orders.q()],
    );
    for (environment, goal) in [
        (&incomplete_chain, "o3"),
        (&unlisted, "o3"),
        (&partial, "o2"),
    ] {
        let (evaluation, meter) = orders.reaches(&o1, &orders.order(goal), environment, UNLIMITED);
        let (code, cause, fields) = refusal(evaluation);
        assert_eq!(
            (code.as_str(), cause.as_str()),
            ("incomplete_population", "incomplete-scope")
        );
        assert_eq!(
            fields,
            BTreeMap::from([("relationship", next().node), ("receiver", "o1".to_owned()),])
        );
        assert_charges(&meter, &[EXPAND], 1, 0);
    }
}

/// TC-1048 row 17: no neighbour count is compared with a bound.
#[trace("TC-1048", "FR-365-AC-12")]
#[test]
fn row_17_a_neighbour_count_outside_the_end_bound_refuses_nothing() {
    for target in [(0, 1), (1, 1)] {
        let orders = Orders::new(Shape {
            target,
            ..Shape::BASE
        });
        let environment = orders.observe(
            &orders.held(),
            vec![links(
                &orders,
                next(),
                &[("o1", "o2"), ("o1", "o3"), ("o1", "o4")],
            )],
            vec![orders.q()],
        );
        assert_decides(
            &orders,
            &environment,
            "o1",
            "o5",
            false,
            &[EXPAND, EDGE, EXPAND, EDGE, EXPAND, EDGE, EXPAND, RETAIN],
            8,
        );
    }
}

/// TC-1048 row 18: a precondition reads the pre extent and a postcondition
/// the post extent.
#[trace("TC-1048", "FR-365-AC-12")]
#[test]
fn row_18_each_clause_reads_its_own_observations_extent() {
    let orders = Orders::base();
    let (o1, o2) = (orders.order("o1"), orders.order("o2"));
    let pre = orders.observe(
        &orders.held(),
        vec![links(&orders, next(), &[("o1", "o2")])],
        vec![orders.q()],
    );
    let post = orders.observe(
        &orders.held(),
        vec![links(&orders, next(), &[])],
        vec![orders.q()],
    );
    for (clause, expected) in [("PreNext", true), ("PostNext", false)] {
        let observations = orders.observations(clause, &o1, &o2, Some(&pre), Some(&post));
        let mut meter = Meter::new(UNLIMITED);
        let report = orders
            .package
            .evaluate_clause(
                &QualifiedName::unqualified(clause).unwrap(),
                &observations,
                &mut meter,
            )
            .expect("the evaluation does not fault");
        assert_eq!(boolean(report.evaluation), expected, "{clause}");
    }
}

/// TC-1048 row 19: only the named relationship's extent is read.
#[trace("TC-1048", "FR-365-AC-7")]
#[test]
fn row_19_another_relationship_or_a_field_is_never_read() {
    let orders = Orders::base();
    let objects = orders.held().into_iter().map(|object| {
        if object == orders.order("o1") {
            (
                object,
                vec![(
                    "trail",
                    FieldValue::Present(Value::Reference(orders.order("o3"))),
                )],
            )
        } else {
            (object, vec![])
        }
    });
    let environment = orders.observe_with(
        objects,
        vec![
            links(&orders, next(), &[]),
            links(&orders, link(), &[("o1", "o3")]),
        ],
        vec![orders.q()],
    );
    assert_decides(
        &orders,
        &environment,
        "o1",
        "o3",
        false,
        &[EXPAND, RETAIN],
        2,
    );
}

fn assert_neighbour_refusal(evaluation: Evaluation, cause: (&str, &str), reference: &str) {
    let (code, found, fields) = refusal(evaluation);
    assert_eq!((code.as_str(), found.as_str()), cause);
    assert_eq!(
        fields,
        BTreeMap::from([
            ("reference", reference.to_owned()),
            ("relationship", next().node)
        ])
    );
}

const DANGLING: (&str, &str) = ("dangling_reference", "absent-target-in-complete-population");
const UNDECIDABLE: (&str, &str) = ("incomplete_population", "incomplete-scope");

/// TC-1048 row 20: a dangling neighbour refuses before its edge charge,
/// including when it is the target.
#[trace("TC-1048", "FR-365-AC-11")]
#[test]
fn row_20_a_dangling_neighbour_refuses_before_its_edge_charge_even_as_the_target() {
    let orders = Orders::base();
    let environment = orders.observe(
        &orders.held(),
        vec![links(&orders, next(), &[("o1", "o2"), ("o2", "d9")])],
        vec![orders.q()],
    );
    for goal in ["o3", "d9"] {
        let (evaluation, meter) = orders.reaches(
            &orders.order("o1"),
            &orders.order(goal),
            &environment,
            UNLIMITED,
        );
        assert_neighbour_refusal(evaluation, DANGLING, "d9");
        assert_charges(&meter, &[EXPAND, EDGE, EXPAND], 3, 0);
    }
}

/// TC-1048 row 21: the first neighbour in order that is not present decides.
#[trace("TC-1048", "FR-365-AC-11")]
#[test]
fn row_21_the_first_neighbour_in_order_that_is_not_present_decides() {
    let orders = Orders::base();
    let held = orders.held();
    let environment = |extents: &[(ObjectReference, ObjectReference)]| {
        orders.observe(
            &held,
            vec![Orders::extent(next(), true, extents.to_vec())],
            orders.spare_rows(),
        )
    };
    let (o1, o2, o3) = (orders.order("o1"), orders.order("o2"), orders.order("o3"));
    let s9 = orders.at("Rush", "s9");
    let (evaluation, meter) = orders.reaches(
        &o1,
        &o3,
        &environment(&[(o1.clone(), o2.clone()), (o2.clone(), s9)]),
        UNLIMITED,
    );
    assert_neighbour_refusal(evaluation, UNDECIDABLE, "s9");
    assert_charges(&meter, &[EXPAND, EDGE, EXPAND], 3, 0);

    let (s1, d2) = (orders.at("Rush", "s1"), orders.at("Order", "d2"));
    let (evaluation, meter) = orders.reaches(
        &o1,
        &o3,
        &environment(&[
            (o1.clone(), o2.clone()),
            (o2.clone(), s1.clone()),
            (o2.clone(), d2.clone()),
        ]),
        UNLIMITED,
    );
    // Reference-key order is (universe, type, id), so the type identities
    // decide which of the two sorts first. The expected outcome is derived
    // from that production order; the other order is not producible with
    // these two types.
    let (cause, first) = if s1 < d2 {
        (UNDECIDABLE, "s1")
    } else {
        (DANGLING, "d2")
    };
    assert_eq!(
        first, "d2",
        "the Order neighbour sorts first in the fixture"
    );
    assert_neighbour_refusal(evaluation, cause, first);
    assert_charges(&meter, &[EXPAND, EDGE, EXPAND], 3, 0);
}

/// TC-1048 row 22: a dangling or undecidable start decides before any charge.
#[trace("TC-1048", "FR-365-AC-11")]
#[test]
fn row_22_a_missing_start_decides_before_any_charge() {
    let orders = Orders::base();
    let closed = orders.observe(
        &orders.held(),
        vec![links(&orders, next(), &[])],
        vec![orders.q()],
    );
    let (evaluation, meter) =
        orders.reaches(&orders.order("d9"), &orders.order("o1"), &closed, UNLIMITED);
    assert_neighbour_refusal(evaluation, DANGLING, "d9");
    assert_charges(&meter, &[], 0, 0);

    let spare = orders.observe(
        &orders.held(),
        vec![links(&orders, next(), &[])],
        orders.spare_rows(),
    );
    let (evaluation, meter) = orders.reaches(
        &orders.at("Rush", "s9"),
        &orders.order("o1"),
        &spare,
        UNLIMITED,
    );
    assert_neighbour_refusal(evaluation, UNDECIDABLE, "s9");
    assert_charges(&meter, &[], 0, 0);
}

/// The one-or-more-edge closure of `edges` (a function from node to at most
/// one successor) by Warshall's Boolean adjacency-matrix closure, authored
/// without a search.
fn closure(nodes: usize, edges: &[Option<usize>]) -> Vec<Vec<bool>> {
    let mut reach = vec![vec![false; nodes]; nodes];
    for (from, to) in edges.iter().enumerate() {
        if let Some(to) = to {
            reach[from][*to] = true;
        }
    }
    for via in 0..nodes {
        for from in 0..nodes {
            for to in 0..nodes {
                if reach[from][via] && reach[via][to] {
                    reach[from][to] = true;
                }
            }
        }
    }
    reach
}

/// Every functional graph on `nodes` objects: each object holds no link or
/// one link to any object.
fn functional_graphs(nodes: usize) -> Vec<Vec<Option<usize>>> {
    let mut graphs = vec![Vec::new()];
    for _ in 0..nodes {
        graphs = graphs
            .into_iter()
            .flat_map(|graph| {
                std::iter::once(None)
                    .chain((0..nodes).map(Some))
                    .map(move |successor| {
                        let mut extended = graph.clone();
                        extended.push(successor);
                        extended
                    })
            })
            .collect();
    }
    graphs
}

/// TC-1048 row 23: for every ordered pair of every functional graph on one to
/// three objects, forward over `Order/next` and through `incoming` over
/// `Order/link`, the Boolean equals the independent closure; renaming the
/// identities changes no result.
#[trace("TC-1048", "FR-365-AC-8")]
#[test]
fn row_23_every_small_graph_agrees_with_the_independent_closure() {
    let orders = Orders::base();
    for names in [["a1", "a2", "a3"], ["z3", "z2", "z1"]] {
        let objects: Vec<ObjectReference> = names.iter().map(|name| orders.order(name)).collect();
        for count in 1..=3 {
            for edges in functional_graphs(count) {
                let pairs: Vec<(ObjectReference, ObjectReference)> = edges
                    .iter()
                    .enumerate()
                    .filter_map(|(from, to)| {
                        to.map(|to| (objects[from].clone(), objects[to].clone()))
                    })
                    .collect();
                let forward = closure(count, &edges);
                // The closure of the reversed relation is the transpose of
                // the forward closure.
                let transposed: Vec<Vec<bool>> = (0..count)
                    .map(|from| (0..count).map(|to| forward[to][from]).collect())
                    .collect();
                for (relationship, clause, expected) in [
                    (next(), "PreNext", &forward),
                    (link(), "PreIncoming", &transposed),
                ] {
                    let environment = orders.observe(
                        &objects,
                        vec![Orders::extent(relationship, true, pairs.clone())],
                        vec![orders.q()],
                    );
                    for from in 0..count {
                        for to in 0..count {
                            let (evaluation, _) = orders.run(
                                clause,
                                &objects[from],
                                &objects[to],
                                &environment,
                                UNLIMITED,
                            );
                            assert_eq!(
                                boolean(evaluation),
                                expected[from][to],
                                "{clause} {names:?} {edges:?} {from}->{to}"
                            );
                        }
                    }
                }
            }
        }
    }
}

/// TC-1054 row 13: with `bound OrdersBound on M::orders [2]`, an
/// observation whose `orders` population lists 2 objects evaluates the
/// reachability as for an unbounded population; one listing 3 refuses
/// `cardinality_out_of_bound`/`above-maximum` naming 2 and 3, before any
/// charge and as no resource outcome.
#[trace("TC-1054", "FR-380-AC-7")]
#[test]
fn an_observation_above_the_authored_bound_refuses_above_maximum() {
    let orders = Orders::with_declarations(Shape::BASE, "bound OrdersBound on M::orders [2]\n");
    let population = key("orders").node;
    let observed = |count: u64| {
        chain(&orders).with_population_sizes(BTreeMap::from([(population.clone(), count)]))
    };

    let (at_the_bound, meter) = orders.reaches(
        &orders.order("o1"),
        &orders.order("o3"),
        &observed(2),
        UNLIMITED,
    );
    assert!(
        boolean(at_the_bound),
        "decided as for an unbounded population"
    );
    assert_charges(&meter, &[EXPAND, EDGE, EXPAND, EDGE, RETAIN], 5, 1);

    let (above, meter) = orders.reaches(
        &orders.order("o1"),
        &orders.order("o3"),
        &observed(3),
        UNLIMITED,
    );
    let FamilyOutcome::Evaluated(Outcome::Refused(refusal)) = &above.outcome else {
        panic!("expected a kernel refusal, got {:?}", above.outcome);
    };
    assert!(matches!(
        refusal,
        quire_exact::Refusal::CardinalityOutOfBound {
            violation: quire_exact::BoundViolation::AboveMaximum,
            count: 3,
            ..
        }
    ));
    let record = above
        .refusal_record(orders.package.graph())
        .expect("cardinality_out_of_bound has a catalog code");
    assert_eq!(
        record.code(),
        qsl_foundation::diagnostic::CatalogCode::new("cardinality_out_of_bound", "above-maximum")
    );
    let fields: Vec<(&str, &str)> = record
        .fields()
        .iter()
        .map(|(name, value)| (*name, value.as_str()))
        .collect();
    assert_eq!(
        fields,
        [("bound", "[0, 2]"), ("collection", "set"), ("count", "3")],
        "the maximum 2 and the count 3"
    );
    assert_charges(&meter, &[], 0, 0);

    // Without the bound the same observation is decided.
    let unbounded = Orders::base();
    let (decided, _) = unbounded.reaches(
        &unbounded.order("o1"),
        &unbounded.order("o3"),
        &chain(&unbounded).with_population_sizes(BTreeMap::from([(population, 3)])),
        UNLIMITED,
    );
    assert!(boolean(decided));
}

/// TC-1054 row 13: the bound refuses for every population the
/// clause's record holds as bounded, not only when a `reaches` node runs: a
/// clause over the bounded context population alone, and one whose
/// `reaches` is skipped by a short-circuit, both refuse above the maximum.
#[trace("TC-1054", "FR-380-AC-7")]
#[test]
fn a_bounded_population_refuses_for_a_clause_that_never_evaluates_a_reaches() {
    let orders = Orders::with_declarations(
        Shape::BASE,
        "bound OrdersBound on M::orders [2]\n\
         pre PreContext using v on M::Order::reroute { true }\n\
         pre PreShort using v on M::Order::reroute { true or reaches(self, target, next) }\n",
    );
    let population = key("orders").node;
    for clause in ["PreContext", "PreShort"] {
        for (count, refused) in [(2_u64, false), (3, true)] {
            let environment =
                chain(&orders).with_population_sizes(BTreeMap::from([(population.clone(), count)]));
            let (evaluation, meter) = orders.run(
                clause,
                &orders.order("o1"),
                &orders.order("o3"),
                &environment,
                UNLIMITED,
            );
            match (&evaluation.outcome, refused) {
                (FamilyOutcome::Evaluated(Outcome::Refused(refusal)), true) => {
                    assert!(
                        matches!(
                            refusal,
                            quire_exact::Refusal::CardinalityOutOfBound {
                                violation: quire_exact::BoundViolation::AboveMaximum,
                                count: 3,
                                ..
                            }
                        ),
                        "{clause}: {refusal:?}"
                    );
                    assert_charges(&meter, &[], 0, 0);
                }
                (FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true))), false) => {}
                (other, _) => panic!("{clause} over {count} objects: {other:?}"),
            }
        }
    }
}
