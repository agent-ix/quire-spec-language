// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-198 continued: `allInstances<T>(p)` and `lookup<T>(p, r) absent m`
//! (FR-153) as complete-V1 *source forms*, exercised through the real
//! checker/evaluator boundary (`CheckedPackage::check_expression` and
//! `CheckedPackage::evaluate`), never by calling
//! `crate::model::population::all_instances`/`lookup` directly.
//!
//! `tests/model_population.rs`'s L01-L08 already back FR-153 at the
//! `crate::model::population` layer; those calls never pass through
//! `Expression::AllInstances`/`Expression::Lookup`, the `Typer`, `NodeKind`,
//! `Machine` or `crate::value::model_query`'s own identity bridge — the
//! layer QSL #120 slice 1a actually adds. This file is L12 onward: the same
//! TC-195 F1 model (`model.A`, `model.B`, generalization `B -> A`) and TC-198
//! P1 population document, run as real source expressions with a
//! `Value::Population` parameter and, for `lookup`, a `Value::Reference`
//! argument bridged through `ObjectReference`/`UniverseIdentity`/`NodeKey`
//! exactly as `crate::value::model_query` does.

use std::sync::Arc;

use ix_trace_rs::trace;
use quire_spec_language::model::accounting::ModelNormalizationLimits;
use quire_spec_language::model::bundle::{
    Bundle, BundleRecord, FieldMemberRecord, GeneralizationRecord, ModelSelection, Multiplicity,
    ObjectTypeRecord,
};
use quire_spec_language::model::dispatch::GeneralizationClosure;
use quire_spec_language::model::key::{EffectiveId, ProducerKey};
use quire_spec_language::model::normalize::{
    normalize, object_universe, EffectiveView, NormalizeOutcome,
};
use quire_spec_language::model::population::{
    admit_binding, AbsenceMode, AdmissionMeter, AdmissionOutcome, PopulationAdmissionLimits,
    PopulationBinding, PopulationDocument, PopulationMember,
};
use quire_spec_language::value::{
    BinaryOperator, CardinalityBound, ChargePoint, CheckCause, CheckMode, CheckRefusal,
    CheckedExpression, CheckedPackage, CheckingLimits, CollectionKind, CollectionType,
    CompositeDeclaration, CompositeShape, DeclarationCause, Expression, FieldDeclaration,
    IllTypedCause, Integer, LimitKind, Meter, NodeKey, ObjectEnvironment, ObjectIdentity,
    ObjectReference, ObjectTypeDeclaration, Outcome, PackageDeclarations, Presence, ScalarLimits,
    TypeEnvironment, Undefined, UniverseIdentity, Value, ValueType,
};

const MULTIPLICITY_0_1: Multiplicity = Multiplicity {
    lower: 0,
    upper: Some(1),
    ordered: false,
    unique: true,
};

const SCALAR_UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

fn object_type(identity: &str) -> BundleRecord {
    BundleRecord::ObjectType(ObjectTypeRecord {
        key: ProducerKey::fixture(identity),
        interface_features: None,
    })
}

fn field_member(identity: &str, owner: &str, value_type: &str) -> BundleRecord {
    BundleRecord::FieldMember(FieldMemberRecord {
        key: ProducerKey::fixture(identity),
        owner: ProducerKey::fixture(owner),
        value_type: ProducerKey::fixture(value_type),
        multiplicity: MULTIPLICITY_0_1,
    })
}

fn generalization(identity: &str, specific: &str, general: &str) -> BundleRecord {
    BundleRecord::Generalization(GeneralizationRecord {
        key: ProducerKey::fixture(identity),
        specific: ProducerKey::fixture(specific),
        general: ProducerKey::fixture(general),
    })
}

/// TC-195 F1, imported as `M` by TC-198 (see `tests/model_population.rs`):
/// types `A`, `B`; field `A.x` of `A`; generalization `B -> A`.
fn fixture_f1() -> Bundle {
    Bundle::new(
        ModelSelection::fixture("bundle.n01"),
        vec![
            object_type("model.A"),
            object_type("model.B"),
            field_member("model.A.x", "model.A", "model.A"),
            generalization("model.gen.B-A", "model.B", "model.A"),
        ],
    )
}

fn view_of(bundle: &Bundle) -> EffectiveView {
    match normalize(bundle, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Completed(view) => view,
        other => panic!("expected a completed effective view, got {other:?}"),
    }
}

fn type_id(view: &EffectiveView, identity: &str) -> EffectiveId {
    let original = ProducerKey::fixture(identity);
    view.declarations
        .iter()
        .find(|entry| {
            entry.preimage.owner_effective_type.is_none() && entry.preimage.original == original
        })
        .map(|entry| entry.effective_id.clone())
        .unwrap_or_else(|| panic!("{identity} has no type-level effective declaration"))
}

fn member(object: &str, type_identity: &str) -> PopulationMember {
    PopulationMember {
        object: object.to_owned(),
        type_identity: ProducerKey::fixture(type_identity),
    }
}

/// FCD FR-121 document P1: members `a1`/`a2` of `model.A`, `b1` of `model.B`.
fn p1(model_identity: &str) -> PopulationDocument {
    PopulationDocument {
        closed_world: true,
        model_identity: model_identity.to_owned(),
        members: vec![
            member("a1", "model.A"),
            member("a2", "model.A"),
            member("b1", "model.B"),
        ],
    }
}

fn admitted_binding(
    bundle: &Bundle,
    view: &EffectiveView,
    document: &PopulationDocument,
) -> PopulationBinding {
    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    match admit_binding(
        bundle,
        view,
        document,
        GeneralizationClosure::Closed,
        Some(3),
        &mut admission,
    ) {
        AdmissionOutcome::Admitted(binding) => binding,
        other => panic!("expected an admitted binding, got {other:?}"),
    }
}

/// The direct byte transfer of a model type/universe/object identity
/// into a checked `crate::value::ObjectReference`, exactly as
/// `crate::value::model_query::to_object_reference` bridges a real
/// `all_instances`/`lookup` result. Test-side construction of an argument
/// `Value::Reference`, not a re-implementation of the module under test.
fn object_reference(
    universe: &EffectiveId,
    type_identity: &EffectiveId,
    object: &str,
) -> ObjectReference {
    let universe = UniverseIdentity::new(universe.as_bytes()).unwrap();
    let object_type = NodeKey::from_hex(&type_identity.hex()).unwrap();
    let identity = ObjectIdentity::new(object.as_bytes()).unwrap();
    ObjectReference::new(universe, object_type, identity)
}

fn node_key(identity: &EffectiveId) -> NodeKey {
    NodeKey::from_hex(&identity.hex()).unwrap()
}

struct Scenario {
    universe: EffectiveId,
    a: EffectiveId,
    b: EffectiveId,
    binding: PopulationBinding,
}

fn scenario() -> Scenario {
    let bundle = fixture_f1();
    let view = view_of(&bundle);
    let universe = object_universe(&bundle).unwrap().identity();
    let a = type_id(&view, "model.A");
    let b = type_id(&view, "model.B");
    let binding = admitted_binding(&bundle, &view, &p1("bundle.n01"));
    Scenario {
        universe,
        a,
        b,
        binding,
    }
}

/// A checked package whose `TypeEnvironment` declares `M::A`/`M::B` as
/// object types under their real checked `NodeKey`s (the same 32 bytes as
/// `scenario`'s own `EffectiveId`s) — required for `ValueType::Reference`
/// to check at all: `TypeEnvironment::type_refusal` refuses any
/// `Reference<T>` whose `T` is not a declared object type.
fn types(scenario: &Scenario) -> TypeEnvironment {
    TypeEnvironment::new(
        [],
        [
            ObjectTypeDeclaration::new(node_key(&scenario.a), "M::A", vec![]),
            ObjectTypeDeclaration::new(node_key(&scenario.b), "M::B", vec![]),
        ],
    )
    .unwrap()
}

fn package(scenario: &Scenario) -> CheckedPackage {
    PackageDeclarations {
        types: types(scenario),
        ..PackageDeclarations::default()
    }
    .check(CheckingLimits::default())
    .unwrap()
}

/// A real object world containing every object these tests pass as a
/// `lookup<T>(p, r)` argument, `b1` (a member of the closed population `p`)
/// and `c9` (not a member of `p`, but still a real object of the wider
/// world) alike: `Value::validate`'s parameter admission refuses a
/// dangling `Value::Reference` argument against this environment
/// regardless of the separate, narrower `PopulationBinding` `lookup`
/// itself checks membership against — the two are deliberately different
/// checks over different scopes.
fn objects(scenario: &Scenario) -> ObjectEnvironment {
    ObjectEnvironment::new(
        &types(scenario),
        [
            (
                object_reference(&scenario.universe, &scenario.b, "b1"),
                vec![],
            ),
            (
                object_reference(&scenario.universe, &scenario.a, "c9"),
                vec![],
            ),
        ],
    )
    .unwrap()
}

fn check(
    package: &CheckedPackage,
    parameters: &[(&str, ValueType)],
    expression: &Expression,
) -> CheckedExpression {
    let parameters = parameters
        .iter()
        .map(|(name, value_type)| ((*name).to_owned(), value_type.clone()))
        .collect();
    package
        .check_expression(
            parameters,
            expression,
            None,
            CheckMode::Kernel,
            CheckingLimits::default(),
        )
        .unwrap()
}

/// Like [`check`], for a checked program findings 1/5 expect the checker to
/// refuse outright -- `Population` named anywhere FR-153 does not give it
/// Outputs, or a non-`Population` operand where `allInstances`/`lookup`
/// require one.
fn check_refusal(
    package: &CheckedPackage,
    parameters: &[(&str, ValueType)],
    expression: &Expression,
) -> CheckRefusal {
    let parameters = parameters
        .iter()
        .map(|(name, value_type)| ((*name).to_owned(), value_type.clone()))
        .collect();
    match package.check_expression(
        parameters,
        expression,
        None,
        CheckMode::Kernel,
        CheckingLimits::default(),
    ) {
        Err(refusal) => refusal,
        Ok(checked) => panic!(
            "expected a check refusal, got a checked {:?}",
            checked.value_type()
        ),
    }
}

/// A stable `NodeKey` distinct from any `scenario()` type, for a declaration
/// this file constructs but never admits into `fixture_f1`'s model -- finding
/// 1's record-field/object-attribute cases (checked at
/// `TypeEnvironment::new` time, never through an expression) and finding 2's
/// package-declared-but-not-in-the-model case need one each.
fn fixed_key(byte: u8) -> NodeKey {
    NodeKey::from_hex(&format!("{byte:02x}").repeat(32)).unwrap()
}

fn run(
    package: &CheckedPackage,
    parameters: &[(&str, ValueType)],
    expression: &Expression,
    arguments: Vec<Value>,
    limits: ScalarLimits,
    objects: &ObjectEnvironment,
) -> (Outcome<Value>, Meter) {
    let checked = check(package, parameters, expression);
    let mut meter = Meter::new(limits);
    let evaluation = package
        .evaluate(&checked, arguments, objects, &mut meter)
        .unwrap();
    (evaluation.outcome, meter)
}

fn population_name() -> Expression {
    Expression::Name("p".to_owned())
}

fn all_instances(target: ValueType) -> Expression {
    Expression::AllInstances {
        target,
        population: Box::new(population_name()),
    }
}

fn lookup(target: ValueType, absence: AbsenceMode) -> Expression {
    Expression::Lookup {
        target,
        population: Box::new(population_name()),
        reference: Box::new(Expression::Name("r".to_owned())),
        absence,
    }
}

/// `Value` deliberately has no structural `PartialEq` (equality is the
/// FR-149 relation, not `==`), so tests compare the one field that matters
/// through its own comparable accessor instead: every element of an
/// `allInstances` result is a bare `Value::Reference`, and `ObjectReference`
/// does derive `PartialEq`.
fn reference_elements(value: &Value) -> Vec<ObjectReference> {
    match value {
        Value::Collection(collection) => collection
            .elements()
            .iter()
            .map(|element| match element {
                Value::Reference(reference) => reference.clone(),
                other => panic!("a reference element, not {other:?}"),
            })
            .collect(),
        other => panic!("a collection, not {other:?}"),
    }
}

/// TC-198 L12: `allInstances<M::A>(p)` as a real source expression selects
/// every current member whose most-specific type conforms to `M::A` —
/// `b1` (via the closed `B -> A` generalization) then `a1`, `a2` — in
/// canonical reference order, backing FR-153-AC-1 (closed population),
/// FR-153-AC-5 (bounded, typed `Set<Reference<T>>` result) directly through
/// the checker/evaluator, and FR-153-AC-6 (`b1`'s reference is identical whether
/// selected under `M::A` or queried directly under `M::B`).
#[test]
#[trace("TC-198", "FR-153-AC-1", "FR-153-AC-5", "FR-153-AC-6")]
fn l12_all_instances_expression_selects_subtype_population_once() {
    let scenario = scenario();
    let package = package(&scenario);
    let parameters = [("p", ValueType::Population(3))];
    let expression = all_instances(ValueType::Reference(node_key(&scenario.a)));
    let arguments = vec![Value::Population(Arc::new(scenario.binding.clone()))];

    let checked = check(&package, &parameters, &expression);
    let collection_type = match checked.value_type() {
        ValueType::Collection(collection_type) => collection_type.clone(),
        other => panic!("expected a checked collection result type, got {other:?}"),
    };
    assert_eq!(collection_type.bound().minimum(), 0);
    assert_eq!(collection_type.bound().maximum(), 3);
    assert_eq!(
        collection_type.element(),
        &ValueType::Reference(node_key(&scenario.a))
    );

    let (outcome, meter) = run(
        &package,
        &parameters,
        &expression,
        arguments,
        SCALAR_UNLIMITED,
        &ObjectEnvironment::default(),
    );
    let value = match outcome {
        Outcome::Completed(value) => value,
        other => panic!("expected a completed collection, got {other:?}"),
    };
    let expected = vec![
        object_reference(&scenario.universe, &scenario.b, "b1"),
        object_reference(&scenario.universe, &scenario.a, "a1"),
        object_reference(&scenario.universe, &scenario.a, "a2"),
    ];
    assert_eq!(reference_elements(&value), expected);
    assert!(meter.consumed(LimitKind::WorkUnits) > 0);

    // FR-153-AC-6: the same object, `b1`, selected under `M::B` directly, is the
    // identical reference the `M::A` selection above already carries.
    let arguments_b = vec![Value::Population(Arc::new(scenario.binding.clone()))];
    let expression_b = all_instances(ValueType::Reference(node_key(&scenario.b)));
    let (outcome_b, _) = run(
        &package,
        &parameters,
        &expression_b,
        arguments_b,
        SCALAR_UNLIMITED,
        &ObjectEnvironment::default(),
    );
    let value_b = match outcome_b {
        Outcome::Completed(value) => value,
        other => panic!("expected a completed collection, got {other:?}"),
    };
    let b1_via_b = object_reference(&scenario.universe, &scenario.b, "b1");
    assert_eq!(reference_elements(&value_b), vec![b1_via_b.clone()]);
    assert_eq!(reference_elements(&value)[0], b1_via_b);
}

/// TC-198 L13: `allInstances<M::A>(p)` still charges `population.visit` per
/// candidate and `collection.bound`/`collection.result-retain` for the
/// materialized result through the real evaluator, so a low `work_units`
/// ceiling denies exactly the same way the direct `crate::model::population`
/// call already does (`tests/model_population.rs`'s
/// `l01_all_instances_incomplete_at_result_retain`) — backing FR-153-AC-3
/// (accounting) at the expression layer, not only at the model layer.
///
/// Pins the exact totals `tests/model_population.rs`'s L01 already proves at
/// the model layer (3 `population.visit` + 1 `collection.bound` + 1
/// `collection.result-retain` charge = 5 work units; the `n + 1 = 4`
/// `result_units` the retain charge sizes for a 3-element selection), so a
/// regression that adds or drops a charge at the expression layer -- rather
/// than only at evaluation of the bridge around it -- fails here too, then
/// pins the low-limit denial to the exact same charge point L01 denies at.
#[test]
#[trace("TC-198", "FR-153-AC-3")]
fn l13_all_instances_expression_incomplete_under_a_low_work_limit() {
    let scenario = scenario();
    let package = package(&scenario);
    let parameters = [("p", ValueType::Population(3))];
    let expression = all_instances(ValueType::Reference(node_key(&scenario.a)));

    let (outcome, meter) = run(
        &package,
        &parameters,
        &expression,
        vec![Value::Population(Arc::new(scenario.binding.clone()))],
        SCALAR_UNLIMITED,
        &ObjectEnvironment::default(),
    );
    match outcome {
        Outcome::Completed(_) => {}
        other => panic!("expected a completed collection under no limit, got {other:?}"),
    }
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 5);
    assert_eq!(meter.consumed(LimitKind::ResultUnits), 4);

    let limited = ScalarLimits {
        work_units: 4,
        ..SCALAR_UNLIMITED
    };
    let (outcome, _) = run(
        &package,
        &parameters,
        &expression,
        vec![Value::Population(Arc::new(scenario.binding.clone()))],
        limited,
        &ObjectEnvironment::default(),
    );
    match outcome {
        Outcome::Incomplete(incomplete) => {
            assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
            assert_eq!(incomplete.limit, 4);
            assert_eq!(incomplete.consumed, 4);
            assert_eq!(incomplete.next_charge, Integer::from(1_u64));
            assert_eq!(incomplete.charge_point, ChargePoint::CollectionResultRetain);
        }
        other => panic!("expected Incomplete under a work_units:4 ceiling, got {other:?}"),
    }
}

/// TC-198 L14: `lookup<M::A>(p, r) absent undefined` through the real
/// checker/evaluator returns the present member's reference for a member
/// key and `Outcome::Undefined` for an absent one, backing FR-153-AC-2
/// (per-mode presence) and FR-153-AC-4 (typed present-result Outputs) at the
/// expression layer.
#[test]
#[trace("TC-198", "FR-153-AC-2", "FR-153-AC-4")]
fn l14_lookup_expression_undefined_mode() {
    let scenario = scenario();
    let package = package(&scenario);
    let parameters = [
        ("p", ValueType::Population(3)),
        ("r", ValueType::Reference(node_key(&scenario.b))),
    ];
    let expression = lookup(
        ValueType::Reference(node_key(&scenario.a)),
        AbsenceMode::Undefined,
    );

    let present_reference = object_reference(&scenario.universe, &scenario.b, "b1");
    let (present, _) = run(
        &package,
        &parameters,
        &expression,
        vec![
            Value::Population(Arc::new(scenario.binding.clone())),
            Value::Reference(present_reference.clone()),
        ],
        SCALAR_UNLIMITED,
        &objects(&scenario),
    );
    match present {
        Outcome::Completed(Value::Reference(reference)) => {
            assert_eq!(reference, present_reference);
        }
        other => panic!("expected a completed reference, got {other:?}"),
    }

    let absent_reference = object_reference(&scenario.universe, &scenario.a, "c9");
    let parameters_absent = [
        ("p", ValueType::Population(3)),
        ("r", ValueType::Reference(node_key(&scenario.a))),
    ];
    let (absent, _) = run(
        &package,
        &parameters_absent,
        &lookup(
            ValueType::Reference(node_key(&scenario.a)),
            AbsenceMode::Undefined,
        ),
        vec![
            Value::Population(Arc::new(scenario.binding.clone())),
            Value::Reference(absent_reference),
        ],
        SCALAR_UNLIMITED,
        &objects(&scenario),
    );
    match absent {
        Outcome::Undefined(reason) => assert_eq!(reason, Undefined::AbsentKey),
        other => panic!("expected Undefined(AbsentKey) for an absent key, got {other:?}"),
    }
}

/// TC-198 L15: `lookup<M::A>(p, r) absent empty` wraps a present member's
/// reference as `Option::present` and an absent key as `Option::none`,
/// through the real checker/evaluator — backing FR-153-AC-2 and FR-153-AC-4's
/// `Option<Reference<T>>` Outputs shape for `empty` mode specifically.
#[test]
#[trace("TC-198", "FR-153-AC-2", "FR-153-AC-4")]
fn l15_lookup_expression_empty_mode() {
    let scenario = scenario();
    let package = package(&scenario);
    let target = ValueType::Reference(node_key(&scenario.a));
    let parameters = [
        ("p", ValueType::Population(3)),
        ("r", ValueType::Reference(node_key(&scenario.b))),
    ];
    let expression = lookup(target.clone(), AbsenceMode::Empty);

    let present_reference = object_reference(&scenario.universe, &scenario.b, "b1");
    let (present, _) = run(
        &package,
        &parameters,
        &expression,
        vec![
            Value::Population(Arc::new(scenario.binding.clone())),
            Value::Reference(present_reference.clone()),
        ],
        SCALAR_UNLIMITED,
        &objects(&scenario),
    );
    match present {
        Outcome::Completed(Value::Option(option)) => {
            assert_eq!(option.payload_type(), &target);
            match option.payload() {
                Some(Value::Reference(reference)) => assert_eq!(*reference, present_reference),
                other => panic!("expected a present reference payload, got {other:?}"),
            }
        }
        other => panic!("expected a completed option, got {other:?}"),
    }

    let absent_reference = object_reference(&scenario.universe, &scenario.a, "c9");
    let parameters_absent = [
        ("p", ValueType::Population(3)),
        ("r", ValueType::Reference(node_key(&scenario.a))),
    ];
    let (absent, _) = run(
        &package,
        &parameters_absent,
        &lookup(target.clone(), AbsenceMode::Empty),
        vec![
            Value::Population(Arc::new(scenario.binding.clone())),
            Value::Reference(absent_reference),
        ],
        SCALAR_UNLIMITED,
        &objects(&scenario),
    );
    match absent {
        Outcome::Completed(Value::Option(option)) => {
            assert_eq!(option.payload_type(), &target);
            assert!(option.payload().is_none());
        }
        other => panic!("expected a completed none option, got {other:?}"),
    }
}

/// TC-198 L16: `lookup<M::A>(p, r) absent refused` refuses outright on an
/// absent key, and the real evaluator's `Outcome::Refused` carries the
/// exact same closed `code`/`cause` pair
/// (`invalid_runtime_input`/`absent-key`) `crate::model::population::lookup`
/// itself raises — backing FR-153-AC-2 (per-mode presence, `refused` mode)
/// through the identity bridge's refusal path
/// (`crate::value::model_query::model_refusal`), not only its success path.
#[test]
#[trace("TC-198", "FR-153-AC-2", "FR-153-AC-4")]
fn l16_lookup_expression_refused_mode() {
    let scenario = scenario();
    let package = package(&scenario);
    let target = ValueType::Reference(node_key(&scenario.a));
    let absent_reference = object_reference(&scenario.universe, &scenario.a, "c9");
    let parameters = [
        ("p", ValueType::Population(3)),
        ("r", ValueType::Reference(node_key(&scenario.a))),
    ];
    let expression = lookup(target, AbsenceMode::Refused);

    let (outcome, _) = run(
        &package,
        &parameters,
        &expression,
        vec![
            Value::Population(Arc::new(scenario.binding.clone())),
            Value::Reference(absent_reference),
        ],
        SCALAR_UNLIMITED,
        &objects(&scenario),
    );
    match outcome {
        Outcome::Refused(refusal) => {
            assert_eq!(refusal.code(), Some("invalid_runtime_input"));
            assert_eq!(refusal.cause(), Some("absent-key"));
        }
        other => panic!("expected a refused absent-key lookup, got {other:?}"),
    }
}

/// FR-153 names a population binding only as the direct operand of
/// `allInstances`/`lookup`, or a bare parameter's own declared type; every
/// other named-type context refuses it, never merely leaving `p == p` (or an
/// `Option`/collection/record/object-type context that names it) to reach
/// `Refusal::CheckedInvariant` at evaluation for an admitted program.
#[test]
fn population_refused_as_equality_operand() {
    let scenario = scenario();
    let package = package(&scenario);
    let parameters = [("p", ValueType::Population(3))];
    let expression = Expression::Binary {
        operator: BinaryOperator::Equal,
        left: Box::new(population_name()),
        right: Box::new(population_name()),
    };
    let refusal = check_refusal(&package, &parameters, &expression);
    assert_eq!(
        refusal.cause,
        CheckCause::IllTyped(IllTypedCause::OperatorIneligible)
    );
}

#[test]
fn population_refused_as_option_payload() {
    let scenario = scenario();
    let package = package(&scenario);
    let parameters = [("p", ValueType::Population(3))];
    let expression = Expression::Convert {
        target: ValueType::Option(Box::new(ValueType::Population(3))),
        operand: Box::new(Expression::Boolean(true)),
    };
    let refusal = check_refusal(&package, &parameters, &expression);
    assert_eq!(
        refusal.cause,
        CheckCause::IllTyped(IllTypedCause::OperatorIneligible)
    );
}

#[test]
fn population_refused_as_collection_element() {
    let scenario = scenario();
    let package = package(&scenario);
    let parameters = [("p", ValueType::Population(3))];
    let expression = Expression::Convert {
        target: ValueType::collection(CollectionType::new(
            CollectionKind::Set,
            ValueType::Population(3),
            CardinalityBound::new(0, 3).unwrap(),
        )),
        operand: Box::new(Expression::Boolean(true)),
    };
    let refusal = check_refusal(&package, &parameters, &expression);
    assert_eq!(
        refusal.cause,
        CheckCause::IllTyped(IllTypedCause::OperatorIneligible)
    );
}

/// Unlike the cases above, a record field or object-type attribute names its
/// member types at declaration time, checked once by
/// `TypeEnvironment::new`'s own `check_member_types` walk, never by
/// `check_expression` -- this is a different entry point into the same
/// `TypeEnvironment::type_refusal` this file's other `population_refused_*`
/// cases exercise through the checker.
#[test]
fn population_refused_as_record_field() {
    let scenario = scenario();
    let declaration = CompositeDeclaration::new(
        fixed_key(0xAA),
        "Holder",
        CompositeShape::Record(vec![FieldDeclaration::new(
            "value",
            ValueType::Population(3),
            Presence::Required,
        )]),
    );
    let result = TypeEnvironment::new(
        [declaration],
        [
            ObjectTypeDeclaration::new(node_key(&scenario.a), "M::A", vec![]),
            ObjectTypeDeclaration::new(node_key(&scenario.b), "M::B", vec![]),
        ],
    );
    match result {
        Err(invalid) => assert_eq!(
            invalid.cause,
            DeclarationCause::Type(IllTypedCause::OperatorIneligible)
        ),
        Ok(_) => panic!("expected a refused declaration for a Population record field"),
    }
}

#[test]
fn population_refused_as_object_attribute() {
    let scenario = scenario();
    let declaration = ObjectTypeDeclaration::new(
        fixed_key(0xBB),
        "M::Holder",
        vec![FieldDeclaration::new(
            "value",
            ValueType::Population(3),
            Presence::Required,
        )],
    );
    let result = TypeEnvironment::new(
        [],
        [
            ObjectTypeDeclaration::new(node_key(&scenario.a), "M::A", vec![]),
            ObjectTypeDeclaration::new(node_key(&scenario.b), "M::B", vec![]),
            declaration,
        ],
    );
    match result {
        Err(invalid) => assert_eq!(
            invalid.cause,
            DeclarationCause::Type(IllTypedCause::OperatorIneligible)
        ),
        Ok(_) => panic!("expected a refused declaration for a Population object attribute"),
    }
}

/// FR-153/TC-198 L06: the population operand is the wrong *kind*, not merely
/// the wrong type name, so both `allInstances`/`lookup` refuse it
/// `ill_typed`/`operator-ineligible`, never `type-mismatch`.
#[test]
fn all_instances_refuses_non_population_operand_as_ineligible() {
    let scenario = scenario();
    let package = package(&scenario);
    let parameters = [("p", ValueType::Integer)];
    let expression = all_instances(ValueType::Reference(node_key(&scenario.a)));
    let refusal = check_refusal(&package, &parameters, &expression);
    assert_eq!(
        refusal.cause,
        CheckCause::IllTyped(IllTypedCause::OperatorIneligible)
    );
}

#[test]
fn lookup_refuses_non_population_operand_as_ineligible() {
    let scenario = scenario();
    let package = package(&scenario);
    let parameters = [
        ("p", ValueType::Integer),
        ("r", ValueType::Reference(node_key(&scenario.a))),
    ];
    let expression = lookup(
        ValueType::Reference(node_key(&scenario.a)),
        AbsenceMode::Undefined,
    );
    let refusal = check_refusal(&package, &parameters, &expression);
    assert_eq!(
        refusal.cause,
        CheckCause::IllTyped(IllTypedCause::OperatorIneligible)
    );
}

/// FR-153-AC-3: a package may declare an object type its checked
/// `TypeEnvironment` admits that a particular runtime `PopulationBinding`'s
/// model never exports -- the checker cannot decide that (the
/// "TypeEnvironment island", `crate::value::model_query`'s module docs), so
/// `crate::value::model_query::resolve_target` must itself refuse it at
/// evaluation, `ill_typed`/`type-mismatch`, the same cause
/// `crate::model::population::all_instances`'s own `is_object_type` gives a
/// declared type the model doesn't recognize -- never
/// `Refusal::CheckedInvariant`.
#[test]
#[trace("TC-198", "FR-153-AC-3")]
fn all_instances_expression_target_declared_but_not_in_model_is_type_mismatch() {
    let scenario = scenario();
    let foreign_key = fixed_key(0xCC);
    let types = TypeEnvironment::new(
        [],
        [
            ObjectTypeDeclaration::new(node_key(&scenario.a), "M::A", vec![]),
            ObjectTypeDeclaration::new(node_key(&scenario.b), "M::B", vec![]),
            ObjectTypeDeclaration::new(foreign_key, "M::C", vec![]),
        ],
    )
    .unwrap();
    let package = PackageDeclarations {
        types,
        ..PackageDeclarations::default()
    }
    .check(CheckingLimits::default())
    .unwrap();
    let parameters = [("p", ValueType::Population(3))];
    let expression = all_instances(ValueType::Reference(foreign_key));

    let (outcome, _) = run(
        &package,
        &parameters,
        &expression,
        vec![Value::Population(Arc::new(scenario.binding.clone()))],
        SCALAR_UNLIMITED,
        &ObjectEnvironment::default(),
    );
    match outcome {
        Outcome::Refused(refusal) => {
            assert_eq!(refusal.code(), Some("ill_typed"));
            assert_eq!(refusal.cause(), Some("type-mismatch"));
        }
        other => {
            panic!("expected a refused type-mismatch for an undeclared model type, got {other:?}")
        }
    }
}

/// FR-153-AC-3: `lookup`'s own `type_conforms(S, T)` runs "before any
/// charge" (TC-198 L03), so a malformed reference cannot skip it -- the
/// checked static type `S` (`M::B`) must still conform to the queried `T`
/// (`M::A`) for this probe to reach the bridge at all. Once it does, a
/// 5-byte universe (never this model's own 32-byte
/// `quire.model.object-universe/v1` digest) can never be bridged into a
/// well-formed key at all, so
/// `crate::value::model_query::evaluate_unresolvable_lookup` decides it
/// directly, without ever calling the real
/// `crate::model::population::lookup` or substituting a derived value: the
/// same `type_conforms` check, the same single `lookup.key` charge, then
/// `foreign-universe` -- reporting the 5 bytes actually supplied, not a
/// fabricated complement.
#[test]
#[trace("TC-198", "FR-153-AC-3")]
fn lookup_expression_malformed_universe_is_foreign_universe_after_one_work_unit() {
    let scenario = scenario();
    let package = package(&scenario);
    let target = ValueType::Reference(node_key(&scenario.a));
    let parameters = [
        ("p", ValueType::Population(3)),
        ("r", ValueType::Reference(node_key(&scenario.b))),
    ];
    let expression = lookup(target, AbsenceMode::Undefined);

    let malformed_reference = ObjectReference::new(
        UniverseIdentity::new(&[1, 2, 3, 4, 5]).unwrap(),
        node_key(&scenario.b),
        ObjectIdentity::new(b"b1").unwrap(),
    );
    let object_world =
        ObjectEnvironment::new(&types(&scenario), [(malformed_reference.clone(), vec![])]).unwrap();

    let (outcome, meter) = run(
        &package,
        &parameters,
        &expression,
        vec![
            Value::Population(Arc::new(scenario.binding.clone())),
            Value::Reference(malformed_reference),
        ],
        SCALAR_UNLIMITED,
        &object_world,
    );
    match outcome {
        Outcome::Refused(refusal) => {
            assert_eq!(refusal.code(), Some("foreign_reference"));
            assert_eq!(refusal.cause(), Some("foreign-universe"));
        }
        other => panic!("expected a refused foreign-universe lookup, got {other:?}"),
    }
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 1);
}

/// A reference whose object-identity bytes are not valid UTF-8 can never
/// name a real population member (every member's own identity is a JSON
/// string, `crate::model::population::PopulationDocument`), so
/// `crate::value::model_query::evaluate_unresolvable_lookup` decides absence
/// directly -- `none` in `empty` mode -- without substituting a lossy-decoded
/// string and delegating to the real `lookup`. This population happens to
/// admit no member whose identity collides with that lossy decode; see
/// [`lookup_expression_malformed_identity_never_aliases_a_lossy_decoded_member`]
/// for the case where one does.
#[test]
#[trace("TC-198", "FR-153-AC-3")]
fn lookup_expression_malformed_identity_is_none_in_empty_mode() {
    let scenario = scenario();
    let package = package(&scenario);
    let target = ValueType::Reference(node_key(&scenario.a));
    let parameters = [
        ("p", ValueType::Population(3)),
        ("r", ValueType::Reference(node_key(&scenario.a))),
    ];
    let expression = lookup(target.clone(), AbsenceMode::Empty);

    let malformed_reference = ObjectReference::new(
        UniverseIdentity::new(scenario.universe.as_bytes()).unwrap(),
        node_key(&scenario.a),
        ObjectIdentity::new(&[0xFF, 0xFE]).unwrap(),
    );
    let object_world =
        ObjectEnvironment::new(&types(&scenario), [(malformed_reference.clone(), vec![])]).unwrap();

    let (outcome, _) = run(
        &package,
        &parameters,
        &expression,
        vec![
            Value::Population(Arc::new(scenario.binding.clone())),
            Value::Reference(malformed_reference),
        ],
        SCALAR_UNLIMITED,
        &object_world,
    );
    match outcome {
        Outcome::Completed(Value::Option(option)) => {
            assert_eq!(option.payload_type(), &target);
            assert!(option.payload().is_none());
        }
        other => panic!("expected a completed none option, got {other:?}"),
    }
}

/// PR #158 re-review finding 1 (HIGH): before this fix,
/// `crate::value::model_query`'s identity bridge substituted a lossy UTF-8
/// decode of a malformed reference's identity bytes and delegated to the
/// real `lookup`, as if the reference had always carried that decoded
/// string. `String::from_utf8_lossy(&[0xFF, 0xFE])` is
/// `"\u{FFFD}\u{FFFD}"` (two U+FFFD replacement characters) -- a value a
/// population is free to admit as a real member's own identity, so that
/// substitution let a producer-supplied malformed reference (these same two
/// invalid bytes) be mistaken for that real member, wrongly returning
/// "present" in all three absence modes. This population admits exactly that
/// member; `evaluate_unresolvable_lookup` must now see the malformed
/// reference as absent in all three modes, never present.
#[test]
#[trace("TC-198", "FR-153-AC-2", "FR-153-AC-3", "FR-153-AC-4")]
fn lookup_expression_malformed_identity_never_aliases_a_lossy_decoded_member() {
    let bundle = fixture_f1();
    let view = view_of(&bundle);
    let universe = object_universe(&bundle).unwrap().identity();
    let a = type_id(&view, "model.A");
    let b = type_id(&view, "model.B");
    let document = PopulationDocument {
        closed_world: true,
        model_identity: "bundle.n01".to_owned(),
        members: vec![
            member("a1", "model.A"),
            member("\u{FFFD}\u{FFFD}", "model.A"),
        ],
    };
    let binding = admitted_binding(&bundle, &view, &document);
    let scenario = Scenario {
        universe,
        a,
        b,
        binding,
    };
    let package = package(&scenario);
    let target = ValueType::Reference(node_key(&scenario.a));
    let parameters = [
        ("p", ValueType::Population(3)),
        ("r", ValueType::Reference(node_key(&scenario.a))),
    ];

    let malformed_reference = ObjectReference::new(
        UniverseIdentity::new(scenario.universe.as_bytes()).unwrap(),
        node_key(&scenario.a),
        ObjectIdentity::new(&[0xFF, 0xFE]).unwrap(),
    );
    let object_world =
        ObjectEnvironment::new(&types(&scenario), [(malformed_reference.clone(), vec![])]).unwrap();

    let (undefined_outcome, _) = run(
        &package,
        &parameters,
        &lookup(target.clone(), AbsenceMode::Undefined),
        vec![
            Value::Population(Arc::new(scenario.binding.clone())),
            Value::Reference(malformed_reference.clone()),
        ],
        SCALAR_UNLIMITED,
        &object_world,
    );
    match undefined_outcome {
        Outcome::Undefined(reason) => assert_eq!(reason, Undefined::AbsentKey),
        other => panic!("expected Undefined(AbsentKey), got {other:?}"),
    }

    let (refused_outcome, _) = run(
        &package,
        &parameters,
        &lookup(target.clone(), AbsenceMode::Refused),
        vec![
            Value::Population(Arc::new(scenario.binding.clone())),
            Value::Reference(malformed_reference.clone()),
        ],
        SCALAR_UNLIMITED,
        &object_world,
    );
    match refused_outcome {
        Outcome::Refused(refusal) => {
            assert_eq!(refusal.code(), Some("invalid_runtime_input"));
            assert_eq!(refusal.cause(), Some("absent-key"));
        }
        other => panic!("expected a refused absent-key lookup, got {other:?}"),
    }

    let (empty_outcome, _) = run(
        &package,
        &parameters,
        &lookup(target.clone(), AbsenceMode::Empty),
        vec![
            Value::Population(Arc::new(scenario.binding.clone())),
            Value::Reference(malformed_reference),
        ],
        SCALAR_UNLIMITED,
        &object_world,
    );
    match empty_outcome {
        Outcome::Completed(Value::Option(option)) => {
            assert_eq!(option.payload_type(), &target);
            assert!(option.payload().is_none());
        }
        other => panic!("expected a completed none option, got {other:?}"),
    }
}

/// PR #158 re-review finding 3 (LOW): `set[lookup<M::A>(p, r) absent
/// refused]` preserves the looked-up reference's own runtime most-specific
/// type inside the checked `Set<Reference<M::A>>` -- the collection's single
/// element has `object_type` `M::B`, not `M::A`, even though the collection's
/// declared/checked element type is `Reference<M::A>`. FR-143's identity
/// triple and FR-153's Outputs both require the most-specific type (TC-198
/// L01); `OptionValue::from_admitted`'s soundness chain (its own doc comment)
/// is exactly why this collection-literal path is sound too --
/// `crate::value::collection::from_admitted` bypasses `admits()` the same way
/// for the identical reason.
#[test]
#[trace("TC-198", "FR-143", "FR-153-AC-5")]
fn lookup_expression_inside_a_set_literal_keeps_the_most_specific_element_type() {
    let scenario = scenario();
    let package = package(&scenario);
    let target = ValueType::Reference(node_key(&scenario.a));
    let parameters = [
        ("p", ValueType::Population(3)),
        ("r", ValueType::Reference(node_key(&scenario.b))),
    ];
    let expression = Expression::Collection {
        kind: CollectionKind::Set,
        elements: vec![lookup(target.clone(), AbsenceMode::Refused)],
    };
    let expected = ValueType::collection(CollectionType::new(
        CollectionKind::Set,
        target,
        CardinalityBound::new(1, 1).unwrap(),
    ));

    let present_reference = object_reference(&scenario.universe, &scenario.b, "b1");
    let checked = package
        .check_expression(
            parameters
                .iter()
                .map(|(name, value_type)| ((*name).to_owned(), value_type.clone()))
                .collect(),
            &expression,
            Some(&expected),
            CheckMode::Kernel,
            CheckingLimits::default(),
        )
        .unwrap();
    let mut meter = Meter::new(SCALAR_UNLIMITED);
    let evaluation = package
        .evaluate(
            &checked,
            vec![
                Value::Population(Arc::new(scenario.binding.clone())),
                Value::Reference(present_reference.clone()),
            ],
            &objects(&scenario),
            &mut meter,
        )
        .unwrap();
    match evaluation.outcome {
        Outcome::Completed(value) => {
            let elements = reference_elements(&value);
            assert_eq!(elements.len(), 1);
            assert_eq!(elements[0], present_reference);
            assert_eq!(elements[0].object_type(), node_key(&scenario.b));
        }
        other => panic!("expected a completed collection, got {other:?}"),
    }
}
