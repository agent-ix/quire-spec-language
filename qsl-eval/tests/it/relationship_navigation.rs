// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1034 / TC-1041 (FR-364, FR-152-AC-7): `r.role` over a relationship end
//! evaluated through the checked-expression entry against an observation's
//! relationship extents and population manifest, with `resolve` deciding
//! each missing target as present, dangling or undecidable.
//!
//! Plan (QSL-795): the checker already types `r.role` to a `Navigate` node,
//! but the evaluator refused it as `unknown_required_feature`. The evaluator
//! must instead charge `model.navigate`, read the extent (`complete`,
//! `targets`, `sources`), decide each target with `resolve` before charging
//! its `collection.visit` (so a missing target decides before the count
//! bound), and then bound, retain and return the multiplicity-determined
//! value. One test per TC-1034 row; each asserts the typed outcome, the
//! charged points in order, and the work and result units.

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, Evaluation};
use qsl_forms::Expression;
use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::family::{FamilyOutcome, FamilyResult};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, Multiplicity, ObjectTypeRecord,
    RelationshipDirection, RelationshipEnd as DeclaredEnd, RelationshipRecord,
};
use qsl_semantics::model::intake::SelectedModels;
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::model::observation::{PopulationEnumerationFailure, RelationshipExtents};
use quire_exact::{
    from_admitted, BoundViolation, ChargePoint, CollectionKind, CollectionType, EffectiveId,
    LimitKind, Meter, ObjectId, ObjectReference, Outcome, OptionValue, Refusal, ScalarLimits,
    UniverseId, Value, ValueType,
};
use quire_semantic_value::checking::{CheckMode, CheckingLimits};
use quire_semantic_value::object_closure::ObjectClosure;
use quire_semantic_value::relationship_extent::{
    NavigationRelationship, PopulationEntry, PopulationEnumerator, RelationshipEnd,
    RelationshipExtent,
};

use crate::support::admitted_charge_points;

fn key(node: &str) -> DeclarationKey {
    DeclarationKey::fixture(node)
}

fn composition() -> DeclarationKey {
    key("relationship/Sys-composition-Pump")
}

fn housing() -> DeclarationKey {
    key("relationship/Pump-housing-Tank")
}

const UNLIMITED: ScalarLimits = qsl_semantics::check::SCALAR_LIMITS_UNLIMITED;

/// The destination multiplicities of the two relationships: the variants of
/// fixture **orders** (`min`, `optional`, `bag`).
#[derive(Clone, Copy)]
struct Shape {
    /// The composition's Pump end: `{0,3}` by default.
    parts: (u64, u64),
    /// Whether the composition's two ends are `unique`.
    parts_unique: bool,
    /// The housing's Tank end: `{1,1}` by default.
    home: (u64, u64),
}

impl Shape {
    const BASE: Self = Self {
        parts: (0, 3),
        parts_unique: true,
        home: (1, 1),
    };
}

fn mult(bounds: (u64, u64), unique: bool) -> Option<Multiplicity> {
    Some(Multiplicity {
        lower: bounds.0,
        upper: Some(bounds.1),
        ordered: false,
        unique,
    })
}

fn object_type(name: &str, supertypes: &[&str]) -> DomainPackageRecord {
    DomainPackageRecord::ObjectType(ObjectTypeRecord {
        key: key(name),
        interface_features: None,
        abstract_type: false,
        supertypes: supertypes.iter().copied().map(key).collect(),
    })
}

fn declared_end(
    object: &str,
    role: Option<&str>,
    multiplicity: Option<Multiplicity>,
) -> DeclaredEnd {
    DeclaredEnd {
        type_identity: key(object),
        role: role.map(str::to_owned),
        multiplicity,
    }
}

/// Fixture **orders**: `Sys` holds Pumps (`parts`), a Pump sits in a Tank
/// (`home`; `pump` from the Tank), `Part` is the supertype of Pump and Tank.
fn orders_package(shape: Shape) -> CheckedPackage {
    let relationship =
        |relationship_key: DeclarationKey, source: DeclaredEnd, target: DeclaredEnd| {
            DomainPackageRecord::Relationship(RelationshipRecord {
                key: relationship_key,
                declared_at: None,
                source,
                target,
                direction: RelationshipDirection::Undirected,
            })
        };
    let records = vec![
        object_type("Sys", &[]),
        object_type("Part", &[]),
        object_type("Pump", &["Part"]),
        object_type("Tank", &["Part"]),
        object_type("Valve", &[]),
        relationship(
            composition(),
            declared_end("Sys", Some("parts"), mult((0, 99), shape.parts_unique)),
            declared_end("Pump", None, mult(shape.parts, shape.parts_unique)),
        ),
        relationship(
            housing(),
            declared_end("Pump", Some("home"), mult((1, 1), true)),
            declared_end("Tank", Some("pump"), mult(shape.home, true)),
        ),
    ];
    let package = DomainPackage::new(DomainPackageRef::fixture("bundle.orders"), records);
    let digest = qsl_semantics::model::key::hex(&package.model_selection.digest);
    let identity = package.model_selection.identity.clone();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model M = {identity:?} version \"1\" digest \"sha256-jcs:{digest}\";\n"
    );
    let selected = SelectedModels::fixture(
        "M",
        qsl_foundation::Span { start: 0, end: 1 },
        package,
        ModelNormalizationLimits::UNLIMITED,
    )
    .expect("the model admits");
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "tc-1034", "fixture", "1"),
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
            .expect("the orders model checks"),
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

/// One scenario: a checked orders package and the observation it navigates.
struct Orders {
    package: CheckedPackage,
    shape: Shape,
}

impl Orders {
    fn new(shape: Shape) -> Self {
        Self {
            package: orders_package(shape),
            shape,
        }
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
        self.row("Q", &["Sys", "Pump", "Tank"], true)
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

    /// The model-layer observation: `held` objects (no fields), `extents`
    /// admitted without a closure check, and the populations `rows`.
    fn observe(
        &self,
        held: &[ObjectReference],
        extents: Vec<RelationshipExtent<DeclarationKey>>,
        rows: Vec<PopulationEntry<DeclarationKey>>,
    ) -> ObjectEnvironment {
        let types = self.package.graph().scope().types();
        let navigation = vec![
            NavigationRelationship {
                key: composition(),
                source: RelationshipEnd {
                    object_type: self.id("Sys"),
                    unique: self.shape.parts_unique,
                },
                target: RelationshipEnd {
                    object_type: self.id("Pump"),
                    unique: self.shape.parts_unique,
                },
            },
            NavigationRelationship {
                key: housing(),
                source: RelationshipEnd {
                    object_type: self.id("Pump"),
                    unique: true,
                },
                target: RelationshipEnd {
                    object_type: self.id("Tank"),
                    unique: true,
                },
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
        let closure = ObjectClosure::new(
            types,
            held.iter().cloned().map(|object| (object, vec![])),
            &[],
        )
        .expect("a closure of fieldless objects");
        ObjectEnvironment::new(closure).with_extents(extents)
    }

    /// The default observation: `s1`, `p1` to `p3` and `t1` held.
    fn held(&self) -> Vec<ObjectReference> {
        vec![
            self.at("Sys", "s1"),
            self.at("Sys", "s2"),
            self.at("Pump", "p1"),
            self.at("Pump", "p2"),
            self.at("Pump", "p3"),
            self.at("Tank", "t1"),
        ]
    }

    /// `receiver.role`, checked and evaluated under `limits`.
    fn navigate(
        &self,
        role: &str,
        receiver: &ObjectReference,
        objects: &ObjectEnvironment,
        limits: ScalarLimits,
    ) -> (Evaluation, Meter) {
        let checked = self
            .package
            .graph()
            .check_expression(
                vec![("r".to_owned(), ValueType::Reference(receiver.object_type()))],
                &Expression::field(Expression::name("r".to_owned()), role),
                None,
                CheckMode::Kernel,
                CheckingLimits::default(),
            )
            .unwrap_or_else(|refusal| panic!("r.{role} checks: {refusal:?}"));
        let mut meter = Meter::new(limits);
        let evaluation = self
            .package
            .evaluate(
                &checked,
                vec![Value::Reference(receiver.clone())],
                objects,
                &mut meter,
            )
            .expect("the evaluation does not fault");
        (evaluation, meter)
    }

    fn set_of_pumps(&self, bounds: (u64, u64), kind: CollectionKind, pumps: &[&str]) -> Value {
        let element = ValueType::Reference(self.id("Pump"));
        let bound = quire_exact::CardinalityBound::new(bounds.0, bounds.1).expect("a bound");
        from_admitted(
            CollectionType::new(kind, element, Some(bound)),
            pumps
                .iter()
                .map(|pump| Value::Reference(self.at("Pump", pump)))
                .collect(),
        )
    }

    fn option_of_tank(&self, tank: Option<&str>) -> Value {
        OptionValue::from_admitted(
            ValueType::Reference(self.id("Tank")),
            tank.map(|tank| Value::Reference(self.at("Tank", tank))),
        )
    }
}

fn completed(evaluation: Evaluation) -> Value {
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(value)) => value,
        other => panic!("expected a completed value, got {other:?}"),
    }
}

/// A family refusal: its catalog code, cause and fields.
fn refusal(
    evaluation: Evaluation,
) -> (
    String,
    String,
    std::collections::BTreeMap<&'static str, String>,
) {
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

/// A kernel cardinality refusal: violation, collection kind, bound, count.
fn cardinality(evaluation: Evaluation) -> (BoundViolation, CollectionKind, (u64, u64), u64) {
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Refused(Refusal::CardinalityOutOfBound {
            violation,
            kind,
            bound,
            count,
        })) => (violation, kind, (bound.minimum(), bound.maximum()), count),
        other => panic!("expected a cardinality refusal, got {other:?}"),
    }
}

fn assert_charges(meter: &Meter, points: &[ChargePoint], work: u64, results: u64) {
    assert_eq!(admitted_charge_points(meter), points);
    assert_eq!(meter.consumed(LimitKind::WorkUnits), work);
    assert_eq!(meter.consumed(LimitKind::ResultUnits), results);
}

const NAVIGATE: ChargePoint = ChargePoint::ModelNavigate;
const VISIT: ChargePoint = ChargePoint::CollectionVisit;
const BOUND: ChargePoint = ChargePoint::CollectionBound;
const RETAIN: ChargePoint = ChargePoint::CollectionResultRetain;

fn composition_links(orders: &Orders, pumps: &[&str]) -> Vec<Link> {
    pumps
        .iter()
        .map(|pump| (orders.at("Sys", "s1"), orders.at("Pump", pump)))
        .collect()
}

/// TC-1034 row 1: the set in reference-key order, the exact schedule, and a
/// work limit equal to the schedule completes while one fewer is incomplete
/// at `collection.result-retain`.
#[trace("TC-1034", "FR-364-AC-5", "FR-152-AC-7")]
#[test]
fn a_multi_valued_end_returns_the_set_after_the_navigate_visit_bound_retain_schedule() {
    let orders = Orders::new(Shape::BASE);
    let objects = orders.observe(
        &orders.held(),
        vec![Orders::extent(
            composition(),
            true,
            composition_links(&orders, &["p2", "p1"]),
        )],
        vec![orders.q()],
    );
    let s1 = orders.at("Sys", "s1");
    let expected = orders.set_of_pumps((0, 3), CollectionKind::Set, &["p1", "p2"]);

    let (evaluation, meter) = orders.navigate("parts", &s1, &objects, UNLIMITED);
    assert_eq!(completed(evaluation), expected);
    assert_charges(&meter, &[NAVIGATE, VISIT, VISIT, BOUND, RETAIN], 5, 3);
    assert_eq!(meter.consumed(LimitKind::ValueOccurrences), 3);

    let exact = ScalarLimits {
        work_units: 5,
        ..UNLIMITED
    };
    let (evaluation, _) = orders.navigate("parts", &s1, &objects, exact);
    assert_eq!(completed(evaluation), expected);

    let short = ScalarLimits {
        work_units: 4,
        ..UNLIMITED
    };
    let (evaluation, meter) = orders.navigate("parts", &s1, &objects, short);
    let FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)) = evaluation.outcome else {
        panic!("one unit fewer is incomplete");
    };
    assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
    assert_eq!(incomplete.limit, 4);
    assert_eq!(incomplete.consumed, 4);
    assert_eq!(incomplete.charge_point, RETAIN);
    assert_charges(&meter, &[NAVIGATE, VISIT, VISIT, BOUND], 4, 0);
}

/// TC-1034 row 3: above the maximum, after the bound and no retain.
#[trace("TC-1034", "FR-364-AC-6")]
#[test]
fn a_count_above_the_upper_bound_refuses_after_the_bound_charge_with_no_retain() {
    let orders = Orders::new(Shape::BASE);
    let mut held = orders.held();
    held.push(orders.at("Pump", "p4"));
    let objects = orders.observe(
        &held,
        vec![Orders::extent(
            composition(),
            true,
            composition_links(&orders, &["p1", "p2", "p3", "p4"]),
        )],
        vec![orders.q()],
    );
    let (evaluation, meter) = orders.navigate("parts", &orders.at("Sys", "s1"), &objects, UNLIMITED);
    assert_eq!(
        cardinality(evaluation),
        (
            BoundViolation::AboveMaximum,
            CollectionKind::Set,
            (0, 3),
            4
        )
    );
    assert_charges(&meter, &[NAVIGATE, VISIT, VISIT, VISIT, VISIT, BOUND], 6, 0);
}

/// TC-1034 row 4 (model-layer extent): the dangling `p4` refuses before the
/// bound, so no cardinality refusal, after three visits.
#[trace("TC-1034", "FR-364-AC-8")]
#[test]
fn a_dangling_target_refuses_before_the_count_bound_after_the_visits_before_it() {
    let orders = Orders::new(Shape::BASE);
    let objects = orders.observe(
        &orders.held(),
        vec![Orders::extent(
            composition(),
            true,
            composition_links(&orders, &["p1", "p2", "p3", "p4"]),
        )],
        vec![orders.q()],
    );
    let (evaluation, meter) = orders.navigate("parts", &orders.at("Sys", "s1"), &objects, UNLIMITED);
    let (code, cause, fields) = refusal(evaluation);
    assert_eq!(
        (code.as_str(), cause.as_str()),
        ("dangling_reference", "absent-target-in-complete-population")
    );
    assert_eq!(
        fields,
        std::collections::BTreeMap::from([
            ("reference", "p4".to_owned()),
            ("relationship", "relationship/Sys-composition-Pump".to_owned()),
        ])
    );
    assert_charges(&meter, &[NAVIGATE, VISIT, VISIT, VISIT], 4, 0);
}

/// TC-1034 row 5: a single-valued end is the reference after one navigate.
#[trace("TC-1034", "FR-364-AC-7")]
#[test]
fn a_single_valued_end_returns_its_reference_after_model_navigate_alone() {
    let orders = Orders::new(Shape::BASE);
    let objects = orders.observe(
        &orders.held(),
        vec![Orders::extent(
            housing(),
            true,
            vec![
                (orders.at("Pump", "p1"), orders.at("Tank", "t1")),
                (orders.at("Pump", "p2"), orders.at("Tank", "t1")),
            ],
        )],
        vec![orders.q()],
    );
    for pump in ["p1", "p2"] {
        let (evaluation, meter) =
            orders.navigate("home", &orders.at("Pump", pump), &objects, UNLIMITED);
        assert_eq!(
            completed(evaluation),
            Value::Reference(orders.at("Tank", "t1"))
        );
        assert_charges(&meter, &[NAVIGATE], 1, 0);
    }
}

/// TC-1034 row 6: the traversal from the target end reads `sources`; two
/// sources through `[1,1]` is above the maximum after one navigate.
#[trace("TC-1034", "FR-364-AC-6", "FR-364-AC-7")]
#[test]
fn a_traversal_from_the_target_end_reads_the_sources_of_the_extent() {
    let orders = Orders::new(Shape::BASE);
    let t1 = orders.at("Tank", "t1");
    let both = orders.observe(
        &orders.held(),
        vec![Orders::extent(
            housing(),
            true,
            vec![
                (orders.at("Pump", "p1"), t1.clone()),
                (orders.at("Pump", "p2"), t1.clone()),
            ],
        )],
        vec![orders.q()],
    );
    let (evaluation, meter) = orders.navigate("pump", &t1, &both, UNLIMITED);
    assert_eq!(
        cardinality(evaluation),
        (
            BoundViolation::AboveMaximum,
            CollectionKind::Set,
            (1, 1),
            2
        )
    );
    assert_charges(&meter, &[NAVIGATE], 1, 0);

    let one = orders.observe(
        &orders.held(),
        vec![Orders::extent(
            housing(),
            true,
            vec![(orders.at("Pump", "p1"), t1.clone())],
        )],
        vec![orders.q()],
    );
    let (evaluation, meter) = orders.navigate("pump", &t1, &one, UNLIMITED);
    assert_eq!(
        completed(evaluation),
        Value::Reference(orders.at("Pump", "p1"))
    );
    assert_charges(&meter, &[NAVIGATE], 1, 0);
}

/// TC-1034 row 7: below the minimum, single-valued and multi-valued.
#[trace("TC-1034", "FR-364-AC-6")]
#[test]
fn a_count_below_the_lower_bound_refuses_below_minimum_at_the_same_point() {
    let orders = Orders::new(Shape::BASE);
    // `p3` has no housing instance: an empty complete extent.
    let objects = orders.observe(
        &orders.held(),
        vec![Orders::extent(housing(), true, vec![])],
        vec![orders.q()],
    );
    let (evaluation, meter) = orders.navigate("home", &orders.at("Pump", "p3"), &objects, UNLIMITED);
    assert_eq!(
        cardinality(evaluation),
        (
            BoundViolation::BelowMinimum,
            CollectionKind::Set,
            (1, 1),
            0
        )
    );
    assert_charges(&meter, &[NAVIGATE], 1, 0);

    let min = Orders::new(Shape {
        parts: (2, 3),
        ..Shape::BASE
    });
    let objects = min.observe(
        &min.held(),
        vec![Orders::extent(
            composition(),
            true,
            composition_links(&min, &["p1"]),
        )],
        vec![min.q()],
    );
    let (evaluation, meter) = min.navigate("parts", &min.at("Sys", "s1"), &objects, UNLIMITED);
    assert_eq!(
        cardinality(evaluation),
        (
            BoundViolation::BelowMinimum,
            CollectionKind::Set,
            (2, 3),
            1
        )
    );
    assert_charges(&meter, &[NAVIGATE, VISIT, BOUND], 3, 0);
}

/// TC-1034 row 8: an optional end is `none` for no instance, the present
/// option for one.
#[trace("TC-1034", "FR-364-AC-7")]
#[test]
fn an_optional_end_is_none_for_no_instance_and_the_present_option_for_one() {
    let orders = Orders::new(Shape {
        home: (0, 1),
        ..Shape::BASE
    });
    let objects = orders.observe(
        &orders.held(),
        vec![Orders::extent(
            housing(),
            true,
            vec![(orders.at("Pump", "p1"), orders.at("Tank", "t1"))],
        )],
        vec![orders.q()],
    );
    let (evaluation, meter) = orders.navigate("home", &orders.at("Pump", "p3"), &objects, UNLIMITED);
    assert_eq!(completed(evaluation), orders.option_of_tank(None));
    assert_charges(&meter, &[NAVIGATE], 1, 0);

    let (evaluation, meter) = orders.navigate("home", &orders.at("Pump", "p1"), &objects, UNLIMITED);
    assert_eq!(completed(evaluation), orders.option_of_tank(Some("t1")));
    assert_charges(&meter, &[NAVIGATE], 1, 0);
}

/// TC-1034 row 9: an incomplete and an unlisted extent are incomplete after
/// one navigate; a complete empty extent is the empty collection and `none`.
#[trace("TC-1034", "FR-364-AC-9")]
#[test]
fn an_incomplete_or_unlisted_extent_is_incomplete_scope_and_an_empty_one_is_empty() {
    let orders = Orders::new(Shape::BASE);
    let s1 = orders.at("Sys", "s1");
    let expected_fields = std::collections::BTreeMap::from([
        ("receiver", "s1".to_owned()),
        ("relationship", "relationship/Sys-composition-Pump".to_owned()),
    ]);

    let incomplete = orders.observe(
        &orders.held(),
        vec![Orders::extent(
            composition(),
            false,
            composition_links(&orders, &["p1"]),
        )],
        vec![orders.q()],
    );
    let unlisted = orders.observe(&orders.held(), vec![], vec![orders.q()]);
    for objects in [&incomplete, &unlisted] {
        let (evaluation, meter) = orders.navigate("parts", &s1, objects, UNLIMITED);
        let (code, cause, fields) = refusal(evaluation);
        assert_eq!(
            (code.as_str(), cause.as_str()),
            ("incomplete_population", "incomplete-scope")
        );
        assert_eq!(fields, expected_fields);
        assert_charges(&meter, &[NAVIGATE], 1, 0);
    }

    let empty = orders.observe(
        &orders.held(),
        vec![Orders::extent(composition(), true, vec![])],
        vec![orders.q()],
    );
    let (evaluation, meter) = orders.navigate("parts", &orders.at("Sys", "s2"), &empty, UNLIMITED);
    assert_eq!(
        completed(evaluation),
        orders.set_of_pumps((0, 3), CollectionKind::Set, &[])
    );
    assert_charges(&meter, &[NAVIGATE, BOUND, RETAIN], 3, 1);

    let optional = Orders::new(Shape {
        home: (0, 1),
        ..Shape::BASE
    });
    let empty = optional.observe(
        &optional.held(),
        vec![Orders::extent(housing(), true, vec![])],
        vec![optional.q()],
    );
    let (evaluation, meter) =
        optional.navigate("home", &optional.at("Pump", "p3"), &empty, UNLIMITED);
    assert_eq!(completed(evaluation), optional.option_of_tank(None));
    assert_charges(&meter, &[NAVIGATE], 1, 0);
}

/// TC-1034 row 10 / TC-1041 variant **spare**: a missing target a declared
/// population covering Pump does not hold complete is undecidable, so the
/// outcome is incomplete, not dangling, and never the cardinality refusal
/// the count (4 above 3) would give.
#[trace("TC-1034", "FR-364-AC-8", "FR-364-AC-12")]
#[test]
fn an_undecidable_target_is_incomplete_scope_before_the_count_bound() {
    let orders = Orders::new(Shape::BASE);
    let objects = orders.observe(
        &orders.held(),
        vec![Orders::extent(
            composition(),
            true,
            composition_links(&orders, &["p1", "p2", "p3", "p9"]),
        )],
        vec![orders.q(), orders.row("Spare", &["Pump"], false)],
    );
    let (evaluation, meter) = orders.navigate("parts", &orders.at("Sys", "s1"), &objects, UNLIMITED);
    let (code, cause, fields) = refusal(evaluation);
    assert_eq!(
        (code.as_str(), cause.as_str()),
        ("incomplete_population", "incomplete-scope")
    );
    assert_eq!(
        fields,
        std::collections::BTreeMap::from([
            ("reference", "p9".to_owned()),
            ("relationship", "relationship/Sys-composition-Pump".to_owned()),
        ])
    );
    assert_charges(&meter, &[NAVIGATE, VISIT, VISIT, VISIT], 4, 0);
}

/// TC-1034 row 11: a non-unique end holding a target twice is the bag with
/// that target twice.
#[trace("TC-1034", "FR-364-AC-5")]
#[test]
fn a_non_unique_end_returns_the_bag_with_a_repeated_target_twice() {
    let orders = Orders::new(Shape {
        parts_unique: false,
        ..Shape::BASE
    });
    let objects = orders.observe(
        &orders.held(),
        vec![Orders::extent(
            composition(),
            true,
            composition_links(&orders, &["p1", "p1"]),
        )],
        vec![orders.q()],
    );
    let (evaluation, meter) = orders.navigate("parts", &orders.at("Sys", "s1"), &objects, UNLIMITED);
    assert_eq!(
        completed(evaluation),
        orders.set_of_pumps((0, 3), CollectionKind::Bag, &["p1", "p1"])
    );
    assert_charges(&meter, &[NAVIGATE, VISIT, VISIT, BOUND, RETAIN], 5, 3);
}

/// TC-1034 row 12: two evaluations of one navigation give equal results and
/// charges, and leave the observation equal to a clone taken before.
#[trace("TC-1034", "FR-364-AC-4")]
#[test]
fn navigating_twice_gives_equal_results_and_leaves_the_observation_unchanged() {
    let orders = Orders::new(Shape::BASE);
    let objects = orders.observe(
        &orders.held(),
        vec![Orders::extent(
            composition(),
            true,
            composition_links(&orders, &["p1", "p2"]),
        )],
        vec![orders.q()],
    );
    let before = objects.clone();
    let s1 = orders.at("Sys", "s1");
    let (first, first_meter) = orders.navigate("parts", &s1, &objects, UNLIMITED);
    let (second, second_meter) = orders.navigate("parts", &s1, &objects, UNLIMITED);
    assert_eq!(completed(first), completed(second));
    assert_eq!(
        admitted_charge_points(&first_meter),
        admitted_charge_points(&second_meter)
    );
    let types = orders.package.graph().scope().types();
    let extents = |environment: &ObjectEnvironment| {
        let held = environment.extents().expect("extents");
        (
            held.state(&composition()),
            held.targets(&composition(), &s1).cloned().collect::<Vec<_>>(),
            held.manifest(types).expect("a manifest").rows().count(),
        )
    };
    assert_eq!(extents(&objects), extents(&before));
}
