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
//! `Machine` or `crate::value::model_query`'s FR-143 identity bridge — the
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
    CheckMode, CheckedPackage, CheckingLimits, Expression, LimitKind, Meter, NodeKey,
    ObjectEnvironment, ObjectIdentity, ObjectReference, ObjectTypeDeclaration, Outcome,
    PackageDeclarations, ScalarLimits, TypeEnvironment, UniverseIdentity, Value, ValueType,
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

/// The direct FR-143 byte transfer of a model type/universe/object identity
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
) -> quire_spec_language::value::CheckedExpression {
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
/// AC-5 (bounded, typed `Set<Reference<T>>` result) directly through the
/// checker/evaluator, and AC-6 (`b1`'s reference is identical whether
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

    // AC-6: the same object, `b1`, selected under `M::B` directly, is the
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
#[test]
#[trace("TC-198", "FR-153-AC-3")]
fn l13_all_instances_expression_incomplete_under_a_low_work_limit() {
    let scenario = scenario();
    let package = package(&scenario);
    let parameters = [("p", ValueType::Population(3))];
    let expression = all_instances(ValueType::Reference(node_key(&scenario.a)));
    let arguments = vec![Value::Population(Arc::new(scenario.binding.clone()))];

    let limited = ScalarLimits {
        work_units: 4,
        ..SCALAR_UNLIMITED
    };
    let (outcome, _) = run(
        &package,
        &parameters,
        &expression,
        arguments,
        limited,
        &ObjectEnvironment::default(),
    );
    match outcome {
        Outcome::Incomplete(_) => {}
        other => panic!("expected Incomplete under a work_units:4 ceiling, got {other:?}"),
    }
}

/// TC-198 L14: `lookup<M::A>(p, r) absent undefined` through the real
/// checker/evaluator returns the present member's reference for a member
/// key and `Outcome::Undefined` for an absent one, backing FR-153-AC-2
/// (per-mode presence) and AC-4 (typed present-result Outputs) at the
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
        Outcome::Undefined(_) => {}
        other => panic!("expected Undefined for an absent key, got {other:?}"),
    }
}

/// TC-198 L15: `lookup<M::A>(p, r) absent empty` wraps a present member's
/// reference as `Option::present` and an absent key as `Option::none`,
/// through the real checker/evaluator — backing FR-153-AC-2 and AC-4's
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
/// through the FR-143 identity bridge's refusal path
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
