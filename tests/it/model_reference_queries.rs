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

use ix_trace_rs::trace;
use qsl_foundation::absence::AbsenceMode;
use quire_exact::{
    CardinalityBound, ChargePoint, CollectionKind, Integer, LimitKind, Meter, ScalarLimits,
};
use quire_spec_language::model::accounting::ModelNormalizationLimits;
use quire_spec_language::model::dispatch::GeneralizationClosure;
use quire_spec_language::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, Extent, FieldMemberRecord, Multiplicity,
    ObjectTypeRecord, OperationEffect, PopulationRecord, ValueTypeRef,
};
use quire_spec_language::model::key::{DeclarationKey, EffectiveId};
use quire_spec_language::model::normalize::{
    normalize, object_universe, EffectiveView, NormalizeOutcome,
};
use quire_spec_language::model::population::{
    admit_binding, admit_invocation, AdmissionMeter, AdmissionOutcome, InvocationContext,
    InvocationDelta, PopulationAdmissionLimits, PopulationBinding, PopulationDocument,
    PopulationMember,
};
use quire_spec_language::value::{
    BinaryOperator, CallFailure, CheckCause, CheckMode, CheckRefusal, CheckedExpression,
    CheckedPackage, CheckedPackageEvaluation, CheckingLimits, CollectionType, CompositeDeclaration,
    CompositeShape, DeclarationCause, Expression, FieldDeclaration, FunctionDeclaration,
    IllTypedCause, InputRefusal, NodeKey, ObjectEnvironment, ObjectIdentity, ObjectReference,
    ObjectTypeDeclaration, Outcome, PackageDeclarations, Presence, QualifiedName, Refusal,
    TypeEnvironment, Undefined, UniverseIdentity, Value, ValueType, WrongSnapshotCause,
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

fn object_type(identity: &str, supertypes: Vec<&str>) -> DomainPackageRecord {
    DomainPackageRecord::ObjectType(ObjectTypeRecord {
        key: DeclarationKey::fixture(identity),
        interface_features: None,
        abstract_type: false,
        supertypes: supertypes
            .into_iter()
            .map(DeclarationKey::fixture)
            .collect(),
    })
}

fn field_member(identity: &str, owner: &str, value_type: &str) -> DomainPackageRecord {
    DomainPackageRecord::FieldMember(FieldMemberRecord {
        key: DeclarationKey::fixture(identity),
        owner: DeclarationKey::fixture(owner),
        value_type: ValueTypeRef::Package(DeclarationKey::fixture(value_type)),
        multiplicity: MULTIPLICITY_0_1,
        subsets: Vec::new(),
        redefines: None,
    })
}

/// TC-195 F1, imported as `M` by TC-198 (see `tests/model_population.rs`):
/// types `A`, `B`; field `A.x` of `A`; generalization `B -> A`; plus its own
/// FR-153 population declaration `model.pop.p1` (member types `A`, `B`),
/// closed, which [`admit_binding`]/[`admit_invocation`] resolve by
/// [`p1_population_key`] rather than take as a caller-supplied record.
fn fixture_f1() -> DomainPackage {
    DomainPackage::new(
        DomainPackageRef::fixture("bundle.n01"),
        vec![
            object_type("model.A", vec![]),
            object_type("model.B", vec!["model.A"]),
            field_member("model.A.x", "model.A", "model.A"),
            DomainPackageRecord::Population(PopulationRecord {
                key: DeclarationKey::fixture("model.pop.p1"),
                member_types: vec![
                    DeclarationKey::fixture("model.A"),
                    DeclarationKey::fixture("model.B"),
                ],
                extent: Extent::Closed,
            }),
        ],
    )
}

fn view_of(domain_package: &DomainPackage) -> EffectiveView {
    match normalize(domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Completed(view) => view,
        other => panic!("expected a completed effective view, got {other:?}"),
    }
}

fn type_id(view: &EffectiveView, identity: &str) -> EffectiveId {
    let original = DeclarationKey::fixture(identity);
    view.declarations()
        .iter()
        .find(|entry| {
            entry.preimage.owner_effective_type.is_none() && entry.preimage.original == original
        })
        .map(|entry| entry.effective_id)
        .unwrap_or_else(|| panic!("{identity} has no type-level effective declaration"))
}

fn member(object: &str, type_identity: &str) -> PopulationMember {
    PopulationMember {
        object: object.to_owned(),
        type_identity: DeclarationKey::fixture(type_identity),
        field_values: Vec::new(),
    }
}

/// FCD FR-121 document P1: members `a1`/`a2` of `model.A`, `b1` of `model.B`.
fn p1(model_identity: &str) -> PopulationDocument {
    PopulationDocument {
        model_identity: model_identity.to_owned(),
        members: vec![
            member("a1", "model.A"),
            member("a2", "model.A"),
            member("b1", "model.B"),
        ],
    }
}

/// [`fixture_f1`]'s own `model.pop.p1` declaration key, backing [`p1`]/
/// [`p1_minus_a2`].
fn p1_population_key() -> DeclarationKey {
    DeclarationKey::fixture("model.pop.p1")
}

/// [`fixture_f1_with_second_population`]'s own second population
/// declaration key, `model.pop.second` -- FR-089-AC-1's own distinct-
/// `population_key` case (TC-291, TC-293) needs two population
/// declarations on the *same* domain package. Same key spelling as
/// `tests/it/model_population.rs`'s own `SECOND_POPULATION` (PR #326
/// review finding S4): the two files' `fixture_f1_with_second_population`
/// helpers build on each file's own separately authored `fixture_f1`, so
/// they are not merged into one shared helper, but nothing justifies them
/// naming the same declared population differently.
fn p2_population_key() -> DeclarationKey {
    DeclarationKey::fixture("model.pop.second")
}

/// [`fixture_f1`], plus a second `Population` declaration
/// ([`p2_population_key`], same member types) alongside its own
/// `model.pop.p1` -- FR-089-AC-1/AC-3's tests admit two distinct bindings
/// against the same domain package under two different population keys.
fn fixture_f1_with_second_population() -> DomainPackage {
    let mut domain_package = fixture_f1();
    domain_package
        .records
        .push(DomainPackageRecord::Population(PopulationRecord {
            key: p2_population_key(),
            member_types: vec![
                DeclarationKey::fixture("model.A"),
                DeclarationKey::fixture("model.B"),
            ],
            extent: Extent::Closed,
        }));
    domain_package
}

fn admitted_binding(
    domain_package: &DomainPackage,
    view: &EffectiveView,
    document: &PopulationDocument,
) -> PopulationBinding {
    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    match admit_binding(
        domain_package,
        view,
        document,
        &p1_population_key(),
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
    let object_type = NodeKey::from_digest(*type_identity.as_bytes());
    let identity = ObjectIdentity::new(object.as_bytes()).unwrap();
    ObjectReference::new(universe, object_type, identity)
}

fn node_key(identity: &EffectiveId) -> NodeKey {
    NodeKey::from_digest(*identity.as_bytes())
}

struct Scenario {
    universe: EffectiveId,
    a: EffectiveId,
    b: EffectiveId,
    binding: PopulationBinding,
}

fn scenario() -> Scenario {
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let universe = object_universe(&domain_package).unwrap().identity();
    let a = type_id(&view, "model.A");
    let b = type_id(&view, "model.B");
    let binding = admitted_binding(&domain_package, &view, &p1("test/orders"));
    Scenario {
        universe,
        a,
        b,
        binding,
    }
}

/// TC-198 P1 (see [`p1`]) after `model.A.remove` deletes `a2`: `a1`/`b1`
/// survive unchanged.
fn p1_minus_a2(model_identity: &str) -> PopulationDocument {
    PopulationDocument {
        model_identity: model_identity.to_owned(),
        members: vec![member("a1", "model.A"), member("b1", "model.B")],
    }
}

/// TC-198 L07's own scenario, as a real invocation admission: pre P1,
/// post P1-without-`a2`, under `model.A.remove`'s declared frame
/// `{modifies: [], creates: [], deletes: [model.A]}`. The returned
/// `Scenario`'s `binding` is the admitted *post* binding with the admitted
/// *pre* binding attached as its `pre_anchor` -- exactly the
/// `Value::Population` argument a real `pre(..)` source expression reads.
fn l07_scenario() -> Scenario {
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let universe = object_universe(&domain_package).unwrap().identity();
    let a = type_id(&view, "model.A");
    let b = type_id(&view, "model.B");
    let effect = OperationEffect {
        modifies: Vec::new(),
        creates: Vec::new(),
        deletes: vec![DeclarationKey::fixture("model.A")],
    };
    let population = p1_population_key();
    let context = InvocationContext {
        domain_package: &domain_package,
        view: &view,
        population: &population,
        subtype_closure: GeneralizationClosure::Closed,
        declared_maximum: Some(3),
    };
    let declared = InvocationDelta {
        effect: &effect,
        declared_created: &[],
        declared_deleted: &["a2".to_owned()],
    };
    let mut pre_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let mut post_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let binding = match admit_invocation(
        context,
        &p1("test/orders"),
        &p1_minus_a2("test/orders"),
        &declared,
        &mut pre_meter,
        &mut post_meter,
    ) {
        AdmissionOutcome::Admitted(binding) => binding,
        other => panic!("expected an admitted invocation, got {other:?}"),
    };
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
    let graph = PackageDeclarations {
        types: types(scenario),
        ..PackageDeclarations::default()
    }
    .check(CheckingLimits::default())
    .unwrap();
    CheckedPackage::link(graph)
}

/// Like [`package`], plus one declared function `F(p: Population<M::A>[3]):
/// Integer { size(allInstances<M::A>(p)) }` -- for item 2's `pre(F(p))`
/// test. `F`'s own body is never itself a postcondition (a function body is
/// checked with `postcondition: false`, `CheckedPackage::check`'s own
/// comment at its `Typer::new` call sites), so this package alone already
/// proves a function body cannot see `pre(...)`; the `pre(F(p))` test below
/// proves the complementary runtime half, that calling `F` from inside a
/// `pre(...)` operand does not leak the caller's pre anchor into `F`'s own
/// body.
fn package_with_function(scenario: &Scenario) -> CheckedPackage {
    let target = ValueType::Reference(node_key(&scenario.a));
    let graph = PackageDeclarations {
        types: types(scenario),
        functions: vec![FunctionDeclaration::new(
            "F",
            vec![("p".to_owned(), ValueType::Population(3))],
            ValueType::Integer,
            None,
            Expression::Size(Box::new(all_instances(target))),
        )],
        ..PackageDeclarations::default()
    }
    .check(CheckingLimits::default())
    .unwrap();
    CheckedPackage::link(graph)
}

/// Like [`package`], plus one declared function `F2(elements:
/// Set<Reference<M::A>>[0,3]): Integer { size(elements) }` -- the review's
/// required positive counterpart to the finding-1 refusal test: `elements`'
/// declared type is exactly `allInstances<M::A>(p)`'s own checked result
/// type, so `pre(F2(allInstances(p)))` passes an eligible read (the
/// `AllInstances` node itself, not the bare parameter `p`) as the call's
/// argument -- FR-042-AC-1's `pre(P(self.version, delta))` analogue, legal
/// because the eligible read happens before the call, not inside `F2`'s own
/// body (which, like `F`'s, is checked as `ClauseKind::Body` and could
/// never see `pre(...)` regardless).
fn package_with_collection_function(scenario: &Scenario) -> CheckedPackage {
    let element = ValueType::Reference(node_key(&scenario.a));
    let elements_type = ValueType::collection(CollectionType::new(
        CollectionKind::Set,
        element,
        CardinalityBound::new(0, 3).unwrap(),
    ));
    let graph = PackageDeclarations {
        types: types(scenario),
        functions: vec![FunctionDeclaration::new(
            "F2",
            vec![("elements".to_owned(), elements_type)],
            ValueType::Integer,
            None,
            Expression::Size(Box::new(Expression::Name("elements".to_owned()))),
        )],
        ..PackageDeclarations::default()
    }
    .check(CheckingLimits::default())
    .unwrap();
    CheckedPackage::link(graph)
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
    .with_population(scenario.binding.clone())
    .unwrap()
}

/// FR-089: the `Value::Population` argument a real source expression's `p`
/// parameter reads -- `scenario.binding`'s own minted `PopulationId`, never
/// the binding itself, which the evaluator now resolves by lookup in the
/// recorded correspondence ([`population_environment`]) rather than reading
/// directly off the `Value`.
fn population_argument(scenario: &Scenario) -> Value {
    Value::Population(scenario.binding.population_id())
}

/// An otherwise-empty `ObjectEnvironment` recording `scenario.binding`'s own
/// FR-089 `PopulationId` correspondence -- what a bare `ObjectEnvironment::
/// default()` supplied before the evaluator needed a correspondence to
/// resolve [`population_argument`] through.
fn population_environment(scenario: &Scenario) -> ObjectEnvironment {
    ObjectEnvironment::default()
        .with_population(scenario.binding.clone())
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
        .graph()
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
    match package.graph().check_expression(
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

/// Like [`check`], but as an operation's postcondition (`pre(...)` legal;
/// see [`CheckedPackage::check_postcondition_expression`]'s own docs) --
/// every `pre(..)` test in this file needs this instead of [`check`], since
/// [`check`] itself must stay a non-postcondition context (it backs every
/// other test in this file, several of which now double as negative
/// coverage that `pre` is refused outside a postcondition).
fn check_postcondition(
    package: &CheckedPackage,
    parameters: &[(&str, ValueType)],
    expression: &Expression,
) -> CheckedExpression {
    let parameters = parameters
        .iter()
        .map(|(name, value_type)| ((*name).to_owned(), value_type.clone()))
        .collect();
    package
        .graph()
        .check_postcondition_expression(
            parameters,
            expression,
            None,
            CheckMode::Kernel,
            CheckingLimits::default(),
        )
        .unwrap()
}

/// Like [`check_refusal`], but as an operation's postcondition (via
/// [`CheckedPackage::check_postcondition_expression`]) -- every negative
/// `pre(...)`-eligibility test in this file needs this instead of
/// [`check_refusal`], since a plain [`check`]/[`check_refusal`] context
/// already refuses `pre(...)` outright on clause context alone (see
/// `pre_refuses_outside_a_postcondition_context`), which would mask the
/// eligible-operand refusal these tests exist to pin.
fn check_refusal_as_postcondition(
    package: &CheckedPackage,
    parameters: &[(&str, ValueType)],
    expression: &Expression,
) -> CheckRefusal {
    let parameters = parameters
        .iter()
        .map(|(name, value_type)| ((*name).to_owned(), value_type.clone()))
        .collect();
    match package.graph().check_postcondition_expression(
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
    NodeKey::from_digest([byte; 32])
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

/// Like [`run`], checked as a postcondition (via [`check_postcondition`])
/// so `pre(...)` is legal.
fn run_postcondition(
    package: &CheckedPackage,
    parameters: &[(&str, ValueType)],
    expression: &Expression,
    arguments: Vec<Value>,
    limits: ScalarLimits,
    objects: &ObjectEnvironment,
) -> (Outcome<Value>, Meter) {
    let checked = check_postcondition(package, parameters, expression);
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

/// `pre(inner)` (FR-153).
fn pre(inner: Expression) -> Expression {
    Expression::Pre(Box::new(inner))
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
    let arguments = vec![population_argument(&scenario)];

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
        &population_environment(&scenario),
    );
    let value = match outcome {
        Outcome::Completed(value) => value,
        other => panic!("expected a completed collection, got {other:?}"),
    };
    // `A`'s effective-id hash sorts before `B`'s, so canonical reference-key
    // order is every `A` member then every `B` member.
    let expected = vec![
        object_reference(&scenario.universe, &scenario.a, "a1"),
        object_reference(&scenario.universe, &scenario.a, "a2"),
        object_reference(&scenario.universe, &scenario.b, "b1"),
    ];
    assert_eq!(reference_elements(&value), expected);
    assert!(meter.consumed(LimitKind::WorkUnits) > 0);

    // FR-153-AC-6: the same object, `b1`, selected under `M::B` directly, is the
    // identical reference the `M::A` selection above already carries.
    let arguments_b = vec![population_argument(&scenario)];
    let expression_b = all_instances(ValueType::Reference(node_key(&scenario.b)));
    let (outcome_b, _) = run(
        &package,
        &parameters,
        &expression_b,
        arguments_b,
        SCALAR_UNLIMITED,
        &population_environment(&scenario),
    );
    let value_b = match outcome_b {
        Outcome::Completed(value) => value,
        other => panic!("expected a completed collection, got {other:?}"),
    };
    let b1_via_b = object_reference(&scenario.universe, &scenario.b, "b1");
    assert_eq!(reference_elements(&value_b), vec![b1_via_b.clone()]);
    // B now sorts last among the three members (see this test's ordering
    // comment above), so the shared `b1` reference is `value`'s last
    // element, not its first.
    assert_eq!(*reference_elements(&value).last().unwrap(), b1_via_b);
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
        vec![population_argument(&scenario)],
        SCALAR_UNLIMITED,
        &population_environment(&scenario),
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
        vec![population_argument(&scenario)],
        limited,
        &population_environment(&scenario),
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
            population_argument(&scenario),
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
            population_argument(&scenario),
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
            population_argument(&scenario),
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
            population_argument(&scenario),
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
            population_argument(&scenario),
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
    let graph = PackageDeclarations {
        types,
        ..PackageDeclarations::default()
    }
    .check(CheckingLimits::default())
    .unwrap();
    let package = CheckedPackage::link(graph);
    let parameters = [("p", ValueType::Population(3))];
    let expression = all_instances(ValueType::Reference(foreign_key));

    let (outcome, _) = run(
        &package,
        &parameters,
        &expression,
        vec![population_argument(&scenario)],
        SCALAR_UNLIMITED,
        &population_environment(&scenario),
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
/// (`M::A`) for this probe to reach `crate::model::population::lookup` at
/// all. `crate::value::model_query::bridge_lookup_key` carries a 5-byte
/// universe (never this model's own 32-byte
/// `quire.model.object-universe/v1` digest) as its own raw bytes rather than
/// bridging it into an `EffectiveId`, so `lookup` itself -- the single owner
/// of the FR-153 lookup order, well-formed reference or not -- reaches this
/// same outcome by its own raw-byte universe comparison: the same
/// `type_conforms` check, the same single `lookup.key` charge, then
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
        ObjectEnvironment::new(&types(&scenario), [(malformed_reference.clone(), vec![])])
            .unwrap()
            .with_population(scenario.binding.clone())
            .unwrap();

    let (outcome, meter) = run(
        &package,
        &parameters,
        &expression,
        vec![
            population_argument(&scenario),
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
/// `crate::value::model_query::bridge_lookup_key` carries the raw bytes
/// unchanged, never a lossy-decoded string, and
/// `crate::model::population::lookup` -- the same function a well-formed
/// reference reaches -- decides absence directly, by its own
/// `std::str::from_utf8` check: `none` in `empty` mode. This population
/// happens to admit no member whose identity collides with that lossy
/// decode; see
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
        ObjectEnvironment::new(&types(&scenario), [(malformed_reference.clone(), vec![])])
            .unwrap()
            .with_population(scenario.binding.clone())
            .unwrap();

    let (outcome, _) = run(
        &package,
        &parameters,
        &expression,
        vec![
            population_argument(&scenario),
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
/// member; `crate::model::population::lookup`'s own `std::str::from_utf8`
/// check on the raw bytes (never a lossy decode) must now see the malformed
/// reference as absent in all three modes, never present.
#[test]
#[trace("TC-198", "FR-153-AC-2", "FR-153-AC-3", "FR-153-AC-4")]
fn lookup_expression_malformed_identity_never_aliases_a_lossy_decoded_member() {
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let universe = object_universe(&domain_package).unwrap().identity();
    let a = type_id(&view, "model.A");
    let b = type_id(&view, "model.B");
    let document = PopulationDocument {
        model_identity: "test/orders".to_owned(),
        members: vec![
            member("a1", "model.A"),
            member("\u{FFFD}\u{FFFD}", "model.A"),
        ],
    };
    let binding = admitted_binding(&domain_package, &view, &document);
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
        ObjectEnvironment::new(&types(&scenario), [(malformed_reference.clone(), vec![])])
            .unwrap()
            .with_population(scenario.binding.clone())
            .unwrap();

    let (undefined_outcome, _) = run(
        &package,
        &parameters,
        &lookup(target.clone(), AbsenceMode::Undefined),
        vec![
            population_argument(&scenario),
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
            population_argument(&scenario),
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
            population_argument(&scenario),
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

/// PR #158 round-3 review finding (MEDIUM), still guarded now that
/// `crate::model::population::lookup` is the single owner of the FR-153
/// lookup order for every reference, well-formed or not (#166): a malformed
/// (non-UTF-8) identity naming a universe that is not this binding's own
/// must refuse `foreign_reference`/`foreign-universe`, exactly like a
/// well-formed reference naming that same foreign universe, in every absence
/// mode -- never fall through to that mode's absence outcome. `lookup`
/// checks the universe (by raw bytes, see `LookupKey`'s own doc comment)
/// immediately after `lookup.key` and before membership, for a malformed
/// (non-UTF-8) identity exactly as for a well-formed one.
#[test]
#[trace("TC-198", "FR-153-AC-2", "FR-153-AC-3", "FR-153-AC-4")]
fn lookup_expression_malformed_identity_in_a_foreign_universe_is_refused_not_absent() {
    let scenario = scenario();
    let package = package(&scenario);
    let target = ValueType::Reference(node_key(&scenario.a));
    let parameters = [
        ("p", ValueType::Population(3)),
        ("r", ValueType::Reference(node_key(&scenario.a))),
    ];

    // 32 repeats of 0x07: well-formed length, but not `scenario.universe`.
    let foreign_universe = UniverseIdentity::new(&[0x07; 32]).unwrap();
    let malformed_reference = ObjectReference::new(
        foreign_universe,
        node_key(&scenario.a),
        ObjectIdentity::new(&[0xFF, 0xFE]).unwrap(),
    );
    let object_world =
        ObjectEnvironment::new(&types(&scenario), [(malformed_reference.clone(), vec![])])
            .unwrap()
            .with_population(scenario.binding.clone())
            .unwrap();

    for absence in [
        AbsenceMode::Undefined,
        AbsenceMode::Refused,
        AbsenceMode::Empty,
    ] {
        let (outcome, meter) = run(
            &package,
            &parameters,
            &lookup(target.clone(), absence),
            vec![
                population_argument(&scenario),
                Value::Reference(malformed_reference.clone()),
            ],
            SCALAR_UNLIMITED,
            &object_world,
        );
        match outcome {
            Outcome::Refused(refusal) => {
                assert_eq!(refusal.code(), Some("foreign_reference"));
                assert_eq!(refusal.cause(), Some("foreign-universe"));
            }
            other => panic!(
                "expected a refused foreign-universe lookup in {absence:?} mode, got {other:?}"
            ),
        }
        assert_eq!(meter.consumed(LimitKind::WorkUnits), 1);
    }
}

/// #166: `crate::model::population::lookup` is now the FR-153 lookup order's
/// one owner for every reference, well-formed or malformed alike (see
/// `crate::value::model_query`'s own module docs, "Malformed references").
/// This table proves well-formed-absent and malformed references are
/// indistinguishable: a well-formed reference naming an object the binding
/// never admitted, and a malformed reference that can never name one either
/// (a non-UTF-8 identity, or a universe some length other than 32), reach
/// byte-identical `Outcome<Value>`s and a byte-identical `Meter` at every
/// combination this table drives -- three absence modes and three scalar
/// budgets (unlimited; zero work units, denying before the first charge; one
/// work unit, enough for `lookup.key` but not a second charge), against
/// three reference-kind pairs: a malformed identity in the binding's own
/// universe, a malformed identity in a foreign universe, and a malformed
/// (5-byte) universe compared against that same well-formed foreign-universe
/// reference. The evaluator boundary's own `ModelQueryRefusal`
/// (`crate::value::outcome`) carries only a refusal's closed `code`/`cause`
/// tags, never the model refusal's free-text `detail` that reports the
/// caller's own bytes, so a direct `Outcome<Value>` comparison is exact
/// here, not merely a same-shape comparison; `Meter` derives `PartialEq`, so
/// one `assert_eq!` on the whole meter also covers every accounting field a
/// narrower, field-by-field comparison could still miss.
#[test]
#[trace("TC-198", "FR-153-AC-2", "FR-153-AC-3", "FR-153-AC-4")]
fn lookup_expression_malformed_and_absent_well_formed_references_are_indistinguishable() {
    let scenario = scenario();
    let package = package(&scenario);
    let target = ValueType::Reference(node_key(&scenario.a));
    let parameters = [
        ("p", ValueType::Population(3)),
        ("r", ValueType::Reference(node_key(&scenario.a))),
    ];

    let zero_work_units = ScalarLimits {
        work_units: 0,
        ..SCALAR_UNLIMITED
    };
    let one_work_unit = ScalarLimits {
        work_units: 1,
        ..SCALAR_UNLIMITED
    };
    let budgets = [
        ("unlimited", SCALAR_UNLIMITED),
        ("zero work units", zero_work_units),
        ("one work unit", one_work_unit),
    ];

    let own_universe = scenario.universe.as_bytes().to_vec();
    // 32 repeats of 0x07: well-formed length, but not `scenario.universe`.
    let foreign_universe = vec![0x07; 32];

    let well_formed_own = ObjectReference::new(
        UniverseIdentity::new(&own_universe).unwrap(),
        node_key(&scenario.a),
        ObjectIdentity::new(b"zz9-never-admitted").unwrap(),
    );
    let malformed_identity_own = ObjectReference::new(
        UniverseIdentity::new(&own_universe).unwrap(),
        node_key(&scenario.a),
        ObjectIdentity::new(&[0xFF, 0xFE]).unwrap(),
    );
    let well_formed_foreign = ObjectReference::new(
        UniverseIdentity::new(&foreign_universe).unwrap(),
        node_key(&scenario.a),
        ObjectIdentity::new(b"zz9-never-admitted").unwrap(),
    );
    let malformed_identity_foreign = ObjectReference::new(
        UniverseIdentity::new(&foreign_universe).unwrap(),
        node_key(&scenario.a),
        ObjectIdentity::new(&[0xFF, 0xFE]).unwrap(),
    );
    // 5 bytes: never a well-formed universe (every real universe is exactly
    // 32 bytes), compared against `well_formed_foreign` -- a well-formed but
    // different universe -- rather than against `well_formed_own`, so only
    // the universe axis varies between this pair.
    let malformed_universe = ObjectReference::new(
        UniverseIdentity::new(&[1, 2, 3, 4, 5]).unwrap(),
        node_key(&scenario.a),
        ObjectIdentity::new(b"zz9-never-admitted").unwrap(),
    );

    for (case_label, well_formed, other) in [
        (
            "own universe, malformed identity",
            &well_formed_own,
            &malformed_identity_own,
        ),
        (
            "foreign universe, malformed identity",
            &well_formed_foreign,
            &malformed_identity_foreign,
        ),
        (
            "foreign universe, malformed universe",
            &well_formed_foreign,
            &malformed_universe,
        ),
    ] {
        let object_world = ObjectEnvironment::new(
            &types(&scenario),
            [(well_formed.clone(), vec![]), (other.clone(), vec![])],
        )
        .unwrap()
        .with_population(scenario.binding.clone())
        .unwrap();

        for absence in [
            AbsenceMode::Undefined,
            AbsenceMode::Refused,
            AbsenceMode::Empty,
        ] {
            let expression = lookup(target.clone(), absence);
            for (budget_label, limits) in budgets {
                let (outcome_wf, meter_wf) = run(
                    &package,
                    &parameters,
                    &expression,
                    vec![
                        population_argument(&scenario),
                        Value::Reference(well_formed.clone()),
                    ],
                    limits,
                    &object_world,
                );
                let (outcome_other, meter_other) = run(
                    &package,
                    &parameters,
                    &expression,
                    vec![
                        population_argument(&scenario),
                        Value::Reference(other.clone()),
                    ],
                    limits,
                    &object_world,
                );

                assert_eq!(
                    format!("{outcome_wf:?}"),
                    format!("{outcome_other:?}"),
                    "{case_label} / {absence:?} / {budget_label}: outcome diverged \
                     between an absent well-formed reference and the other one"
                );
                assert_eq!(
                    meter_wf, meter_other,
                    "{case_label} / {absence:?} / {budget_label}: meter diverged \
                     between an absent well-formed reference and the other one"
                );
            }
        }
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
        .graph()
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
                population_argument(&scenario),
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

// ---------------------------------------------------------------------------
// TC-198 L07: `pre(..)` as a real source expression (EXPR-020/021/022)
// ---------------------------------------------------------------------------

/// TC-198 L07: `allInstances<M::A>(p)` reads the post population (`a2`
/// deleted), and `pre(allInstances<M::A>(p))` reads the invocation's pre
/// population (`a2` still present) -- the identical expression, differing
/// only in whether it is wrapped in `pre(..)`, over the identical
/// `Value::Population` argument (`l07_scenario`'s post binding, carrying its
/// pre anchor).
///
/// Mutation used: in `Machine::select_anchor`, changed `Anchor::Pre =>
/// binding.pre_anchor().ok_or_else(invariant)` to always return `Ok(binding)`
/// (ignoring the anchor entirely). `pre(allInstances(p))`'s result went from
/// `[a1, a2, b1]` to `[a1, b1]`, so the assertion below on the pre-anchored
/// result went red as expected; reverted.
#[test]
#[trace("TC-198", "FR-153-AC-7")]
fn l07_pre_all_instances_reads_the_invocation_pre_population() {
    let scenario = l07_scenario();
    let package = package(&scenario);
    let parameters = [("p", ValueType::Population(3))];
    let target = ValueType::Reference(node_key(&scenario.a));

    let (post_outcome, _) = run(
        &package,
        &parameters,
        &all_instances(target.clone()),
        vec![population_argument(&scenario)],
        SCALAR_UNLIMITED,
        &population_environment(&scenario),
    );
    // `A`'s effective-id hash sorts before `B`'s.
    let post_expected = vec![
        object_reference(&scenario.universe, &scenario.a, "a1"),
        object_reference(&scenario.universe, &scenario.b, "b1"),
    ];
    match post_outcome {
        Outcome::Completed(value) => assert_eq!(reference_elements(&value), post_expected),
        other => panic!("expected a completed post-anchored collection, got {other:?}"),
    }

    let (pre_outcome, _) = run_postcondition(
        &package,
        &parameters,
        &pre(all_instances(target)),
        vec![population_argument(&scenario)],
        SCALAR_UNLIMITED,
        &population_environment(&scenario),
    );
    let pre_expected = vec![
        object_reference(&scenario.universe, &scenario.a, "a1"),
        object_reference(&scenario.universe, &scenario.a, "a2"),
        object_reference(&scenario.universe, &scenario.b, "b1"),
    ];
    match pre_outcome {
        Outcome::Completed(value) => assert_eq!(reference_elements(&value), pre_expected),
        other => panic!("expected a completed pre-anchored collection, got {other:?}"),
    }
}

/// TC-198 L07: `lookup<M::A>(p, r2) absent empty` is `none` for the deleted
/// `a2` over the post population, and `pre(lookup<M::A>(p, r2) absent
/// empty)` is present, `a2` keeping its pre most-specific type `M::A`.
#[test]
#[trace("TC-198", "FR-153-AC-7")]
fn l07_pre_lookup_reads_the_invocation_pre_population_and_a2_keeps_its_pre_type() {
    let scenario = l07_scenario();
    let package = package(&scenario);
    let target = ValueType::Reference(node_key(&scenario.a));
    let parameters = [
        ("p", ValueType::Population(3)),
        ("r", ValueType::Reference(node_key(&scenario.a))),
    ];
    let r2 = object_reference(&scenario.universe, &scenario.a, "a2");
    let object_world = ObjectEnvironment::new(&types(&scenario), [(r2.clone(), vec![])])
        .unwrap()
        .with_population(scenario.binding.clone())
        .unwrap();

    let (post_outcome, _) = run(
        &package,
        &parameters,
        &lookup(target.clone(), AbsenceMode::Empty),
        vec![population_argument(&scenario), Value::Reference(r2.clone())],
        SCALAR_UNLIMITED,
        &object_world,
    );
    match post_outcome {
        Outcome::Completed(Value::Option(option)) => assert!(option.payload().is_none()),
        other => panic!("expected a completed none option for the deleted a2, got {other:?}"),
    }

    let (pre_outcome, _) = run_postcondition(
        &package,
        &parameters,
        &pre(lookup(target.clone(), AbsenceMode::Empty)),
        vec![population_argument(&scenario), Value::Reference(r2.clone())],
        SCALAR_UNLIMITED,
        &object_world,
    );
    match pre_outcome {
        Outcome::Completed(Value::Option(option)) => {
            assert_eq!(option.payload_type(), &target);
            match option.payload() {
                Some(Value::Reference(reference)) => {
                    assert_eq!(*reference, r2);
                    assert_eq!(reference.object_type(), node_key(&scenario.a));
                }
                other => panic!("expected a present a2 payload, got {other:?}"),
            }
        }
        other => panic!("expected a completed present option for pre(a2), got {other:?}"),
    }
}

/// EXPR-021: a `pre(..)` anchor's effect is scoped exactly to its own
/// operand -- once `pre(allInstances(p))` finishes evaluating (bound to
/// `pre_count` here), a sibling `allInstances(p)` in the same body reads the
/// post population again, not a drifted pre anchor left over from the
/// preceding `let` value.
///
/// Mutation used: in `Machine::eval`'s `NodeKind::Pre` arm, dropped the
/// `Task::RestoreAnchor` push (only setting `self.anchor = Anchor::Pre`,
/// never restoring it). The body's `allInstances(p)` then also read the pre
/// population (3 members, matching `pre_count`), so the `NotEqual` assertion
/// below went red as expected (`false`, not `true`); reverted.
#[test]
#[trace("TC-198", "FR-153-AC-7")]
fn pre_anchor_does_not_leak_into_a_sibling_post_anchored_query() {
    let scenario = l07_scenario();
    let package = package(&scenario);
    let target = ValueType::Reference(node_key(&scenario.a));
    let parameters = [("p", ValueType::Population(3))];

    // let pre_count = size(pre(allInstances(p))) in
    //   size(allInstances(p)) != pre_count
    let expression = Expression::Let {
        name: "pre_count".to_owned(),
        value: Box::new(Expression::Size(Box::new(pre(all_instances(
            target.clone(),
        ))))),
        body: Box::new(Expression::Binary {
            operator: BinaryOperator::NotEqual,
            left: Box::new(Expression::Size(Box::new(all_instances(target)))),
            right: Box::new(Expression::Name("pre_count".to_owned())),
        }),
    };

    let (outcome, _) = run_postcondition(
        &package,
        &parameters,
        &expression,
        vec![population_argument(&scenario)],
        SCALAR_UNLIMITED,
        &population_environment(&scenario),
    );
    match outcome {
        Outcome::Completed(Value::Boolean(result)) => assert!(
            result,
            "post-anchored allInstances(p) after a pre(..) sibling must not drift to the \
             pre population (expected 2 post members != 3 pre members)"
        ),
        other => panic!("expected a completed boolean, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// PR #168 review: `pre(...)`'s eligible-operand rule, clause-context gating
// and `Call`'s anchor reset (FR-042, shared-grammar.md).
// ---------------------------------------------------------------------------

/// Item 1's first capture-drift shape: `let v = allInstances(p) in pre(v)`
/// must refuse -- `v`'s own value was already read eagerly against the
/// *post* population before `pre(..)` ever runs, so re-tagging it `pre`
/// would return post data under a `pre` label (FR-042-AC-3's capture-drift
/// refusal) rather than actually re-evaluating the query against the pre
/// population. `pre`'s operand here is a bare `Name`, which
/// `contains_pre_eligible_read` never treats as eligible (only a direct
/// `allInstances`/`lookup`/`pre`/`Call` node is).
///
/// Mutation used: in `Typer::infer_form`'s `Expression::Pre` arm, removed
/// the `contains_pre_eligible_read` guard (keeping only the postcondition
/// guard). This test went red as expected (a checked expression instead of
/// a refusal); reverted.
#[test]
#[trace("FR-042-AC-3")]
fn pre_refuses_a_let_bound_query_result_capture_drift() {
    let scenario = l07_scenario();
    let package = package(&scenario);
    let target = ValueType::Reference(node_key(&scenario.a));
    let parameters = [("p", ValueType::Population(3))];

    // let v = allInstances(p) in pre(v)
    let expression = Expression::Let {
        name: "v".to_owned(),
        value: Box::new(all_instances(target)),
        body: Box::new(pre(Expression::Name("v".to_owned()))),
    };
    let refusal = check_refusal_as_postcondition(&package, &parameters, &expression);
    assert_eq!(
        refusal.cause,
        CheckCause::WrongSnapshot(WrongSnapshotCause::ForbiddenPreRead)
    );
}

/// PR #168 review round 2, finding 4: `let q = p in pre(size(allInstances(q)))`
/// must refuse the same way `pre(p)` itself does -- FR-042-AC-3's own `let s
/// = self in pre(s.version)` analogue. `q` is a `let`-bound alias of the
/// parameter `p`, captured *before* `pre(...)`'s own boundary; re-anchoring
/// a query over `q` inside `pre(...)` would look eligible to a purely
/// syntactic reader (`allInstances(q)` is a direct `AllInstances` node,
/// exactly the shape `contains_pre_eligible_read` treats as eligible), but
/// `q` was never re-bound at the `pre(...)` boundary, so this must still
/// refuse: `contains_captured_pre_alias` walks `size(allInstances(q))`,
/// finds `allInstances`'s `population` operand is the bare `Name("q")`, and
/// resolves `q` against `Typer`'s own local scope to a `let`-bound
/// (`LocalKind::Bound`) alias whose slot predates the `pre(...)`'s boundary
/// -- a captured alias, not a parameter reference (`pre(p)`'s own bare-name
/// case) and not a `let` introduced inside the `pre(...)` operand itself
/// (which `contains_captured_pre_alias`'s shadowing tracking exempts).
///
/// Mutation used: in `Typer`'s `Expression::Pre` arm, dropped the
/// `contains_captured_pre_alias` check (keeping only the `contains_pre_-
/// eligible_read` guard). This test went red as expected (a checked
/// expression instead of a refusal); reverted.
#[test]
#[trace("FR-042-AC-3")]
fn pre_refuses_a_let_bound_population_alias_capture_drift() {
    let scenario = l07_scenario();
    let package = package(&scenario);
    let target = ValueType::Reference(node_key(&scenario.a));
    let parameters = [("p", ValueType::Population(3))];

    // let q = p in pre(size(allInstances(q)))
    let expression = Expression::Let {
        name: "q".to_owned(),
        value: Box::new(population_name()),
        body: Box::new(pre(Expression::Size(Box::new(Expression::AllInstances {
            target,
            population: Box::new(Expression::Name("q".to_owned())),
        })))),
    };
    let refusal = check_refusal_as_postcondition(&package, &parameters, &expression);
    assert_eq!(
        refusal.cause,
        CheckCause::WrongSnapshot(WrongSnapshotCause::ForbiddenPreRead)
    );
}

/// PR #168 review round 3, finding 1 (alias-of-alias): `let q = p in
/// pre(let r = q in size(allInstances(r)))` must refuse the same way
/// `pre(size(allInstances(q)))` itself does -- `r` is a `let` introduced
/// *inside* the `pre(...)` operand, but its own value is a bare reference to
/// `q`, itself a captured alias of the parameter `p`. Aliasing an alias is
/// still aliasing, transitively, however many `let`s sit in between:
/// `contains_captured_pre_alias` must carry `q`'s alias status onto `r`,
/// never treat `r`'s binding as a fresh, unrelated shadow the way a `let`
/// over a genuinely fresh value (say `allInstances(other)`) would be.
///
/// Mutation used: in `contains_captured_pre_alias`'s `Expression::Let` arm,
/// changed `bindings.push((name.clone(), is_captured_alias(value)))` to
/// always push `false` (as the original, pre-round-3 code effectively did,
/// treating every `let` introduced inside the operand as a fresh, safe
/// shadow regardless of its own value). This test went red as expected (a
/// checked, evaluated `Completed(Integer(3))`, the pre population's size,
/// instead of a refusal); reverted.
#[test]
#[trace("FR-042-AC-3")]
fn pre_refuses_a_let_bound_alias_of_a_let_bound_population_alias() {
    let scenario = l07_scenario();
    let package = package(&scenario);
    let target = ValueType::Reference(node_key(&scenario.a));
    let parameters = [("p", ValueType::Population(3))];

    // let q = p in pre(let r = q in size(allInstances(r)))
    let expression = Expression::Let {
        name: "q".to_owned(),
        value: Box::new(population_name()),
        body: Box::new(pre(Expression::Let {
            name: "r".to_owned(),
            value: Box::new(Expression::Name("q".to_owned())),
            body: Box::new(Expression::Size(Box::new(Expression::AllInstances {
                target,
                population: Box::new(Expression::Name("r".to_owned())),
            }))),
        })),
    };
    let refusal = check_refusal_as_postcondition(&package, &parameters, &expression);
    assert_eq!(
        refusal.cause,
        CheckCause::WrongSnapshot(WrongSnapshotCause::ForbiddenPreRead)
    );
}

/// PR #168 review round 4, finding 1, shape 1: the alias can hide behind a
/// `let` written *directly as* `allInstances`'s own operand, not only
/// behind a `let` that wraps the whole `pre(...)` body the way the
/// alias-of-alias test above does. `let q = p in
/// pre(size(allInstances(let s = q in s)))`: `allInstances`'s operand is
/// itself `let s = q in s`, not a bare `Name`, so the old direct check
/// (`Expression::Name` only) never looked inside it at all.
/// `resolves_to_captured_alias`'s own `Expression::Let` arm now resolves
/// through the operand's value (`q`, a captured alias) and carries that
/// status onto `s`, so the operand as a whole still resolves to a captured
/// alias.
///
/// Mutation used: in `resolves_to_captured_alias`'s `Expression::Let` arm,
/// forced `alias` to always be `false` rather than
/// `self.resolves_to_captured_alias(value, boundary, bindings)` (treating a
/// `let` value as never itself an alias, the same blind spot the
/// alias-of-alias fix closed for `contains_captured_pre_alias`'s own `Let`
/// arm, but here inside the operand-resolution helper instead). This test
/// went red as expected (a checked, evaluated `Completed(Integer(3))`
/// instead of a refusal); reverted.
#[test]
#[trace("FR-042-AC-3")]
fn pre_refuses_a_let_expression_used_directly_as_the_all_instances_operand() {
    let scenario = l07_scenario();
    let package = package(&scenario);
    let target = ValueType::Reference(node_key(&scenario.a));
    let parameters = [("p", ValueType::Population(3))];

    // let q = p in pre(size(allInstances(let s = q in s)))
    let expression = Expression::Let {
        name: "q".to_owned(),
        value: Box::new(population_name()),
        body: Box::new(pre(Expression::Size(Box::new(Expression::AllInstances {
            target,
            population: Box::new(Expression::Let {
                name: "s".to_owned(),
                value: Box::new(Expression::Name("q".to_owned())),
                body: Box::new(Expression::Name("s".to_owned())),
            }),
        })))),
    };
    let refusal = check_refusal_as_postcondition(&package, &parameters, &expression);
    assert_eq!(
        refusal.cause,
        CheckCause::WrongSnapshot(WrongSnapshotCause::ForbiddenPreRead)
    );
}

/// PR #168 review round 4, finding 1, shape 2: the alias can hide behind an
/// `if` written directly as `allInstances`'s own operand, both arms naming
/// the same captured alias. `let q = p in
/// pre(size(allInstances(if true then q else q)))`: `resolves_to_captured_-
/// alias`'s own `Expression::If` arm resolves `true` when either branch
/// does, so an operand that is an `if` over a captured alias on both arms
/// still resolves to one.
///
/// Mutation used: removed `resolves_to_captured_alias`'s `Expression::If`
/// arm entirely, so an `if` operand falls through to the catch-all `_ =>
/// false` arm instead (recreating the exact pre-fix blind spot: the old
/// helper only ever recognized a bare `Expression::Name`). This test went
/// red as expected (a checked, evaluated `Completed(Integer(3))` instead of
/// a refusal); reverted.
#[test]
#[trace("FR-042-AC-3")]
fn pre_refuses_an_if_expression_used_directly_as_the_all_instances_operand() {
    let scenario = l07_scenario();
    let package = package(&scenario);
    let target = ValueType::Reference(node_key(&scenario.a));
    let parameters = [("p", ValueType::Population(3))];

    // let q = p in pre(size(allInstances(if true then q else q)))
    let expression = Expression::Let {
        name: "q".to_owned(),
        value: Box::new(population_name()),
        body: Box::new(pre(Expression::Size(Box::new(Expression::AllInstances {
            target,
            population: Box::new(Expression::If {
                condition: Box::new(Expression::Boolean(true)),
                then: Box::new(Expression::Name("q".to_owned())),
                otherwise: Box::new(Expression::Name("q".to_owned())),
            }),
        })))),
    };
    let refusal = check_refusal_as_postcondition(&package, &parameters, &expression);
    assert_eq!(
        refusal.cause,
        CheckCause::WrongSnapshot(WrongSnapshotCause::ForbiddenPreRead)
    );
}

/// PR #168 review round 4, finding 1, shape 3: the alias can hide behind an
/// `if` whose result is then bound by a `let` introduced *inside* the
/// `pre(...)` operand, one level removed from `allInstances`'s own operand
/// (a bare `Name`). `let q = p in pre(let r = if true then q else q in
/// size(allInstances(r)))`: `contains_captured_pre_alias`'s own
/// `Expression::Let` arm now computes `r`'s own alias status via
/// `resolves_to_captured_alias(value, ..)`, where `value` is the `if`
/// expression -- resolving `true` via the same `Expression::If` arm shape 2
/// exercises -- and carries it onto `r`, so `allInstances(r)`'s direct
/// check (`r` looked up in `bindings`) still finds the alias.
///
/// Mutation used: same as
/// `pre_refuses_an_if_expression_used_directly_as_the_all_instances_operand`'s
/// own (removing `resolves_to_captured_alias`'s `Expression::If` arm
/// entirely) -- `r`'s own value resolution goes through the same helper and
/// the same `If` arm, one call site further out than the direct-operand
/// case. This test went red as expected (a checked, evaluated
/// `Completed(Integer(3))` instead of a refusal); reverted.
#[test]
#[trace("FR-042-AC-3")]
fn pre_refuses_a_let_bound_if_expression_alias_of_a_population_alias() {
    let scenario = l07_scenario();
    let package = package(&scenario);
    let target = ValueType::Reference(node_key(&scenario.a));
    let parameters = [("p", ValueType::Population(3))];

    // let q = p in pre(let r = if true then q else q in size(allInstances(r)))
    let expression = Expression::Let {
        name: "q".to_owned(),
        value: Box::new(population_name()),
        body: Box::new(pre(Expression::Let {
            name: "r".to_owned(),
            value: Box::new(Expression::If {
                condition: Box::new(Expression::Boolean(true)),
                then: Box::new(Expression::Name("q".to_owned())),
                otherwise: Box::new(Expression::Name("q".to_owned())),
            }),
            body: Box::new(Expression::Size(Box::new(Expression::AllInstances {
                target,
                population: Box::new(Expression::Name("r".to_owned())),
            }))),
        })),
    };
    let refusal = check_refusal_as_postcondition(&package, &parameters, &expression);
    assert_eq!(
        refusal.cause,
        CheckCause::WrongSnapshot(WrongSnapshotCause::ForbiddenPreRead)
    );
}

/// PR #168 review round 4, finding 1, shape 4: the alias can hide behind a
/// further nested `let`, itself the value of a `let` introduced inside the
/// `pre(...)` operand. `let q = p in pre(let r = (let s = q in s) in
/// size(allInstances(r)))`: `r`'s own value is `let s = q in s`, so
/// resolving `r`'s alias status requires `resolves_to_captured_alias`'s own
/// `Expression::Let` arm to recurse into a nested `Let`, not just a bare
/// `Name` -- the same recursive shape shape 1 exercises for `allInstances`'s
/// own operand, exercised here one level further out.
///
/// Mutation used: same as
/// `pre_refuses_a_let_expression_used_directly_as_the_all_instances_operand`'s
/// own (`resolves_to_captured_alias`'s `Expression::Let` arm's own `alias`
/// computation forced to `false` instead of recursing) -- `r`'s value
/// resolution goes through the same helper and the same `Let` arm, one call
/// site further out than shape 1's direct-operand case (`contains_captured_-
/// pre_alias`'s own `Let` arm, computing `r`'s pushed binding, versus
/// `resolves_to_captured_alias`'s own `Let` arm, computing `s`'s). This test
/// went red as expected (a checked, evaluated `Completed(Integer(3))`
/// instead of a refusal); reverted.
#[test]
#[trace("FR-042-AC-3")]
fn pre_refuses_a_let_bound_nested_let_alias_of_a_population_alias() {
    let scenario = l07_scenario();
    let package = package(&scenario);
    let target = ValueType::Reference(node_key(&scenario.a));
    let parameters = [("p", ValueType::Population(3))];

    // let q = p in pre(let r = (let s = q in s) in size(allInstances(r)))
    let expression = Expression::Let {
        name: "q".to_owned(),
        value: Box::new(population_name()),
        body: Box::new(pre(Expression::Let {
            name: "r".to_owned(),
            value: Box::new(Expression::Let {
                name: "s".to_owned(),
                value: Box::new(Expression::Name("q".to_owned())),
                body: Box::new(Expression::Name("s".to_owned())),
            }),
            body: Box::new(Expression::Size(Box::new(Expression::AllInstances {
                target,
                population: Box::new(Expression::Name("r".to_owned())),
            }))),
        })),
    };
    let refusal = check_refusal_as_postcondition(&package, &parameters, &expression);
    assert_eq!(
        refusal.cause,
        CheckCause::WrongSnapshot(WrongSnapshotCause::ForbiddenPreRead)
    );
}

/// PR #168 review round 3, finding 2: QSpec FR-012-AC-4's positive
/// retention half -- "a captured post reference retains its observation
/// inside `pre`" -- stays legal and correct, the complement to the
/// capture-drift refusals above. `let v = size(allInstances(p)) in
/// pre(size(allInstances(p)) != v)`: `v` is bound to the *post* population's
/// size (2) before `pre(...)` is ever reached, and is not itself a bare
/// alias of a raw population (its value is already a computed `Integer`,
/// not a `Name`), so referencing `v` inside `pre(...)` never re-anchors it
/// -- it keeps observing exactly the post value it captured. The sibling
/// `size(allInstances(p))` written directly inside `pre(...)` does read the
/// *pre* population (3), so the comparison is `3 != 2`, `true`.
///
/// Mutation used: in `select_anchor`, changed the `Anchor::Pre` arm to
/// `Ok(binding)` (the same binding `Anchor::Post` already returns), so
/// `pre(...)` would silently read the post population instead of the pre
/// one. `v` (2, the post size) is unaffected either way -- it was already
/// resolved before `pre(...)` ran -- but the sibling `size(allInstances(p))`
/// written directly inside `pre(...)` then also reads 2, so the comparison
/// becomes `2 != 2`, `false`. This test went red as expected; reverted.
#[test]
#[trace("FR-042-AC-2")]
fn pre_of_a_captured_post_reference_retains_its_post_observation() {
    let scenario = l07_scenario();
    let package = package(&scenario);
    let target = ValueType::Reference(node_key(&scenario.a));
    let parameters = [("p", ValueType::Population(3))];

    // let v = size(allInstances(p)) in pre(size(allInstances(p)) != v)
    let expression = Expression::Let {
        name: "v".to_owned(),
        value: Box::new(Expression::Size(Box::new(all_instances(target.clone())))),
        body: Box::new(pre(Expression::Binary {
            operator: BinaryOperator::NotEqual,
            left: Box::new(Expression::Size(Box::new(all_instances(target)))),
            right: Box::new(Expression::Name("v".to_owned())),
        })),
    };
    let (outcome, _) = run_postcondition(
        &package,
        &parameters,
        &expression,
        vec![population_argument(&scenario)],
        SCALAR_UNLIMITED,
        &population_environment(&scenario),
    );
    match outcome {
        Outcome::Completed(Value::Boolean(result)) => assert!(
            result,
            "v must keep observing the post population's size (2) even though it is read \
             inside pre(...), so pre's own size(allInstances(p)) (3) must compare unequal"
        ),
        other => panic!("expected a completed boolean, got {other:?}"),
    }
}

/// Item 1's second capture-drift shape: `pre(p)` itself (a bare population
/// *parameter*, not a direct `allInstances`/`lookup` call) refuses -- FR-042
/// refuses `pre` on a bare parameter/constant/capture directly, never only
/// on what a later query does with it. `let q = pre(p) in allInstances(q)`
/// (the review's own literal shape) never gets past this same check: `q`'s
/// own initializer, `pre(p)`, already refuses before the `let`'s body is
/// even considered.
///
/// Mutation used: same as
/// `pre_refuses_a_let_bound_query_result_capture_drift`'s own (removing
/// `contains_pre_eligible_read`'s guard from `Typer::infer_form`'s
/// `Expression::Pre` arm). This test went red the same way (a checked
/// `Population(3)` instead of a refusal); reverted.
#[test]
#[trace("FR-042-AC-4")]
fn pre_refuses_a_bare_population_parameter() {
    let scenario = l07_scenario();
    let package = package(&scenario);
    let parameters = [("p", ValueType::Population(3))];

    let expression = pre(Expression::Name("p".to_owned()));
    let refusal = check_refusal_as_postcondition(&package, &parameters, &expression);
    assert_eq!(
        refusal.cause,
        CheckCause::WrongSnapshot(WrongSnapshotCause::ForbiddenPreRead)
    );
}

/// Item 1's third shape: `pre(1)`, a bare integer literal, refuses the same
/// way -- FR-042's Behavior clause never gives `pre` a bare constant
/// operand.
///
/// Mutation used: same as
/// `pre_refuses_a_let_bound_query_result_capture_drift`'s own. This test
/// went red the same way (a checked `Integer` instead of a refusal);
/// reverted.
#[test]
#[trace("FR-042-AC-4")]
fn pre_of_an_integer_literal_is_refused() {
    let scenario = l07_scenario();
    let package = package(&scenario);
    let parameters: [(&str, ValueType); 0] = [];

    let expression = pre(Expression::Integer(Integer::from(1_u64)));
    let refusal = check_refusal_as_postcondition(&package, &parameters, &expression);
    assert_eq!(
        refusal.cause,
        CheckCause::WrongSnapshot(WrongSnapshotCause::ForbiddenPreRead)
    );
}

/// Item 2: `pre(...)` is refused anywhere outside a postcondition context --
/// [`check`] (unlike [`check_postcondition`]) checks as a non-postcondition
/// expression, so `pre(allInstances(p))`, otherwise eligible, still refuses
/// here purely on clause context. FR-208 applies FR-042 to invariants and
/// preconditions and names this same wrong-clause case `forbidden-pre-read`
/// explicitly -- this is a checker-decided refusal over `pre(...)`'s own
/// syntax, never the runtime-only `wrong-anchor` cause `select_anchor`
/// reserves for a population value with no attached pre binding.
///
/// Mutation used: in `CheckedPackage::check_expression`, changed its
/// delegating `check_expression_as(..., false)` call to pass `true`
/// (treating every plain `check_expression` call as a postcondition). This
/// test went red as expected (a checked expression instead of a refusal);
/// reverted.
#[test]
#[trace("FR-042-AC-4")]
fn pre_refuses_outside_a_postcondition_context() {
    let scenario = l07_scenario();
    let package = package(&scenario);
    let target = ValueType::Reference(node_key(&scenario.a));
    let parameters = [("p", ValueType::Population(3))];

    let refusal = check_refusal(&package, &parameters, &pre(all_instances(target)));
    assert_eq!(
        refusal.cause,
        CheckCause::WrongSnapshot(WrongSnapshotCause::ForbiddenPreRead)
    );
}

/// Item 1/2: nested `pre` is idempotent (FR-042-AC-2) -- `pre(pre(e))`
/// checks (the outer `pre`'s operand is itself a `Pre` node, which
/// `contains_pre_eligible_read` always treats as eligible) and evaluates to
/// exactly the same result as the single `pre(e)`.
#[test]
#[trace("FR-042-AC-2", "TC-198", "FR-153-AC-7")]
fn nested_pre_is_idempotent() {
    let scenario = l07_scenario();
    let package = package(&scenario);
    let target = ValueType::Reference(node_key(&scenario.a));
    let parameters = [("p", ValueType::Population(3))];

    let (single, _) = run_postcondition(
        &package,
        &parameters,
        &pre(all_instances(target.clone())),
        vec![population_argument(&scenario)],
        SCALAR_UNLIMITED,
        &population_environment(&scenario),
    );
    let (nested, _) = run_postcondition(
        &package,
        &parameters,
        &pre(pre(all_instances(target))),
        vec![population_argument(&scenario)],
        SCALAR_UNLIMITED,
        &population_environment(&scenario),
    );
    match (single, nested) {
        (Outcome::Completed(single), Outcome::Completed(nested)) => {
            assert_eq!(reference_elements(&single), reference_elements(&nested));
        }
        other => panic!("expected two completed collections, got {other:?}"),
    }
}

/// PR #168 review round 2, finding 1: `pre(F(p))` -- a bare population
/// *parameter* passed as a call argument, never itself an eligible read --
/// must refuse the same way a bare `pre(p)` does. `contains_pre_eligible_read`
/// no longer treats `Expression::Call` itself as eligible (only
/// `AllInstances`/`Lookup`/`Pre` are direct-eligible); a call's own
/// arguments are still reached through `Expression::children()`'s ordinary
/// recursion, but here the sole argument is the bare `Name("p")`, which is
/// never eligible on its own. Admitting `pre(F(p))` here would have silently
/// returned `F`'s *post*-anchored result (`F`'s own body is checked with
/// `postcondition: false` and always reads the post population, per
/// `package_with_function`'s own doc) under a `pre(...)` label, exactly the
/// silent-no-op hazard FR-042 exists to refuse.
///
/// Mutation used: in `contains_pre_eligible_read`, re-added
/// `Expression::Call { .. }` to the direct `matches!` arm. This test went
/// red as expected (a checked, evaluated `Completed(Integer(2))` instead of
/// a refusal); reverted.
#[test]
#[trace("FR-042-AC-4", "TC-198")]
fn pre_of_a_function_call_over_a_bare_parameter_argument_refuses_forbidden_pre_read() {
    let scenario = l07_scenario();
    let package = package_with_function(&scenario);
    let parameters = [("p", ValueType::Population(3))];

    let expression = pre(Expression::Call {
        name: "F".to_owned(),
        arguments: vec![population_name()],
    });
    let refusal = check_refusal_as_postcondition(&package, &parameters, &expression);
    assert_eq!(
        refusal.cause,
        CheckCause::WrongSnapshot(WrongSnapshotCause::ForbiddenPreRead)
    );
}

/// PR #168 review round 2, finding 1's positive counterpart: `pre(F2(elements))`
/// where `elements` is itself an eligible read (`allInstances(p)`, passed as
/// the call's argument, not `p` itself) stays legal -- FR-042-AC-1's
/// `pre(P(self.version, delta))` analogue. `Expression::children()`'s
/// ordinary recursion into `Call`'s `arguments` still finds this eligible
/// read even though `Call` itself is no longer direct-eligible, so
/// `pre(F2(allInstances(p)))` checks and evaluates `allInstances(p)` against
/// the *pre* population (3 members) before handing that already-pre-read
/// collection into `F2`'s own (always post-anchored) body, which merely
/// takes its `size`.
///
/// Mutation used: in `contains_pre_eligible_read`, changed the direct
/// `matches!` arm to always return `false` (as if no read were ever
/// eligible). This test went red as expected (a refusal instead of a
/// completed integer); reverted.
#[test]
#[trace("FR-042-AC-1", "TC-198", "FR-153-AC-7")]
fn pre_of_a_function_call_over_an_eligible_read_argument_stays_legal() {
    let scenario = l07_scenario();
    let package = package_with_collection_function(&scenario);
    let target = ValueType::Reference(node_key(&scenario.a));
    let parameters = [("p", ValueType::Population(3))];

    let expression = pre(Expression::Call {
        name: "F2".to_owned(),
        arguments: vec![all_instances(target)],
    });
    let (outcome, _) = run_postcondition(
        &package,
        &parameters,
        &expression,
        vec![population_argument(&scenario)],
        SCALAR_UNLIMITED,
        &population_environment(&scenario),
    );
    match outcome {
        Outcome::Completed(Value::Integer(size)) => assert_eq!(
            size,
            Integer::from(3_u64),
            "F2's argument must read the pre population (3 members) before F2's own always- \
             post-anchored body takes its size"
        ),
        other => panic!("expected a completed integer, got {other:?}"),
    }
}

/// PR #168 review round 2, finding 2: evaluating `pre(allInstances(p))`
/// against a population admitted directly by [`admit_binding`] (never
/// through [`admit_invocation`], so it carries no `pre_anchor` at all) is a
/// real, caller-input-reachable `Refusal::WrongSnapshot(WrongAnchor)` --
/// never `Refusal::CheckedInvariant`, which is reserved for a genuinely
/// broken evaluator invariant. The checker only gates `pre(...)`'s own
/// syntax (clause context, operand eligibility -- this file's
/// `check_postcondition`); it has no way to see, at checking time, which
/// admission path a caller's runtime population argument will actually
/// take, so `select_anchor`'s missing-pre-anchor case is exactly the one
/// `wrong-anchor` case this crate's checker cannot decide for itself.
#[test]
#[trace("FR-042-AC-4", "TC-198")]
fn pre_of_a_binding_with_no_pre_anchor_refuses_wrong_anchor() {
    let scenario = scenario();
    let package = package(&scenario);
    let target = ValueType::Reference(node_key(&scenario.a));
    let parameters = [("p", ValueType::Population(3))];

    let (outcome, _) = run_postcondition(
        &package,
        &parameters,
        &pre(all_instances(target)),
        vec![population_argument(&scenario)],
        SCALAR_UNLIMITED,
        &population_environment(&scenario),
    );
    assert!(
        matches!(
            outcome,
            Outcome::Refused(Refusal::WrongSnapshot(WrongSnapshotCause::WrongAnchor))
        ),
        "expected Refused(WrongSnapshot(WrongAnchor)) for a pre(..) anchor with no admitted pre \
         binding, got {outcome:?}"
    );
}

/// TC-293 (FR-089-AC-3): the evaluator resolves a `Value::Population`
/// argument to the exact admitted `PopulationBinding` its own `PopulationId`
/// was minted for, by lookup in the recorded correspondence
/// (`ObjectEnvironment::resolve_population`) -- never a payload the `Value`
/// itself carries. Two distinct bindings, `B1` (`population_key` K1) and
/// `B2` (K2), are admitted and recorded in the *same* environment; an
/// `allInstances(p)`-shaped expression bound to `B1`'s id reads exactly
/// `B1`'s members, and the identical expression shape bound to `B2`'s id
/// reads exactly `B2`'s -- catching an evaluator that resolved every
/// `PopulationId` to whichever binding was admitted last.
#[test]
#[trace("TC-293", "FR-089-AC-3")]
fn tc_293_evaluator_resolves_population_id_through_recorded_correspondence() {
    let domain_package = fixture_f1_with_second_population();
    let view = view_of(&domain_package);
    let universe = object_universe(&domain_package).unwrap().identity();
    let a = type_id(&view, "model.A");
    let b = type_id(&view, "model.B");

    let b1_document = PopulationDocument {
        model_identity: "test/orders".to_owned(),
        members: vec![member("a1", "model.A")],
    };
    let b2_document = PopulationDocument {
        model_identity: "test/orders".to_owned(),
        members: vec![member("a9", "model.A")],
    };
    let mut meter1 = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let b1_binding = match admit_binding(
        &domain_package,
        &view,
        &b1_document,
        &p1_population_key(),
        GeneralizationClosure::Closed,
        Some(3),
        &mut meter1,
    ) {
        AdmissionOutcome::Admitted(binding) => binding,
        other => panic!("expected an admitted B1 binding, got {other:?}"),
    };
    let mut meter2 = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let b2_binding = match admit_binding(
        &domain_package,
        &view,
        &b2_document,
        &p2_population_key(),
        GeneralizationClosure::Closed,
        Some(3),
        &mut meter2,
    ) {
        AdmissionOutcome::Admitted(binding) => binding,
        other => panic!("expected an admitted B2 binding, got {other:?}"),
    };
    let id1 = b1_binding.population_id();
    let id2 = b2_binding.population_id();
    assert_ne!(
        id1, id2,
        "two distinct population_keys must mint distinct ids"
    );

    let scenario = Scenario {
        universe,
        a,
        b,
        binding: b1_binding.clone(),
    };
    let environment = ObjectEnvironment::default()
        .with_population(b1_binding)
        .unwrap()
        .with_population(b2_binding)
        .unwrap();
    let package = package(&scenario);
    let parameters = [("p", ValueType::Population(3))];
    let expression = all_instances(ValueType::Reference(node_key(&a)));

    let (outcome1, _) = run(
        &package,
        &parameters,
        &expression,
        vec![Value::Population(id1)],
        SCALAR_UNLIMITED,
        &environment,
    );
    match outcome1 {
        Outcome::Completed(value) => {
            assert_eq!(
                reference_elements(&value),
                vec![object_reference(&universe, &a, "a1")]
            );
        }
        other => panic!("expected B1's own member for id1, got {other:?}"),
    }

    let (outcome2, _) = run(
        &package,
        &parameters,
        &expression,
        vec![Value::Population(id2)],
        SCALAR_UNLIMITED,
        &environment,
    );
    match outcome2 {
        Outcome::Completed(value) => {
            assert_eq!(
                reference_elements(&value),
                vec![object_reference(&universe, &a, "a9")]
            );
        }
        other => panic!("expected B2's own member for id2, got {other:?}"),
    }
}

/// PR #326 review finding F2: two admissions that collide on one
/// `PopulationId` (same domain package, `population_key` and admission
/// role -- FR-089's own preimage, Linear QSL-131's open question over
/// whether declared maximum and document content should also distinguish
/// two bindings) but carry different declared maxima refuse loudly when
/// the second is recorded under the first's id, rather than the first
/// silently being overwritten. `ObjectEnvironment::with_population` is
/// keyed by the binding's own id, so recording an *equal* binding under an
/// id already bound is `Ok` (idempotent), but a *different* one is
/// `Err(PopulationConflict)`.
#[test]
fn with_population_refuses_a_conflicting_binding_under_a_shared_id() {
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);

    let mut meter_3 = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let binding_3 = match admit_binding(
        &domain_package,
        &view,
        &p1("test/orders"),
        &p1_population_key(),
        GeneralizationClosure::Closed,
        Some(3),
        &mut meter_3,
    ) {
        AdmissionOutcome::Admitted(binding) => binding,
        other => panic!("expected an admitted binding, got {other:?}"),
    };
    let mut meter_7 = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let binding_7 = match admit_binding(
        &domain_package,
        &view,
        &p1("test/orders"),
        &p1_population_key(),
        GeneralizationClosure::Closed,
        Some(7),
        &mut meter_7,
    ) {
        AdmissionOutcome::Admitted(binding) => binding,
        other => panic!("expected an admitted binding, got {other:?}"),
    };
    assert_eq!(
        binding_3.population_id(),
        binding_7.population_id(),
        "same domain package/population_key/role must mint the same id, \
         even though the two bindings' declared maxima differ"
    );
    let id = binding_3.population_id();

    let environment = ObjectEnvironment::default()
        .with_population(binding_3.clone())
        .unwrap();

    // Re-recording the identical binding under its own id is idempotent.
    let environment = environment.with_population(binding_3).unwrap();
    assert_eq!(
        environment
            .resolve_population(id)
            .and_then(PopulationBinding::declared_maximum),
        Some(3)
    );

    // Recording a *different* binding (declared maximum 7) under the same
    // id refuses, rather than silently overwriting binding_3's own entry.
    match environment.with_population(binding_7) {
        Err(conflict) => assert_eq!(conflict.population_id, id),
        Ok(_) => panic!("expected Err(PopulationConflict), got Ok -- the conflicting binding overwrote the first silently"),
    }
}

/// TC-294 (FR-089-AC-4): a `Value::Population(population_id)` whose
/// `population_id` names no binding recorded in the current evaluation's
/// correspondence refuses at argument admission -- `InputRefusal::
/// WrongValueKind`, naming the parameter, never a panic and never
/// `Undefined`. PR #326 review finding F1: this now runs inside
/// `CheckedPackage::evaluate`'s own `validate` (which already receives
/// `objects: &ObjectEnvironment` for the analogous `DanglingReference`
/// check), not only inside the evaluator's `Machine::resolve_population` --
/// `ValueType::admits` alone cannot perform it, since it has no access to
/// the recorded correspondence, so admission by presence alone let this
/// case through whenever nothing in the body consumed the parameter (see
/// [`tc_294_unresolved_population_id_refuses_even_when_unconsumed`]).
/// `scenario.binding`'s own minted id is genuine (admitted by a real
/// `admit_binding` call), but the environment this evaluation actually
/// runs against never recorded it -- exactly TC-294's "a distinct,
/// independently-admitted identity from a separate evaluation run".
#[test]
#[trace("TC-294", "FR-089-AC-4")]
fn tc_294_unresolved_population_id_refuses_typed() {
    let scenario = scenario();
    let package = package(&scenario);
    let parameters = [("p", ValueType::Population(3))];
    let expression = all_instances(ValueType::Reference(node_key(&scenario.a)));
    let unresolved_id = scenario.binding.population_id();

    let checked = check(&package, &parameters, &expression);
    let mut meter = Meter::new(SCALAR_UNLIMITED);
    let result = package.evaluate(
        &checked,
        vec![Value::Population(unresolved_id)],
        &ObjectEnvironment::default(),
        &mut meter,
    );
    match result {
        Err(CallFailure::Input(InputRefusal::WrongValueKind { parameter })) => {
            assert_eq!(parameter, 0)
        }
        other => {
            panic!("expected Err(CallFailure::Input(InputRefusal::WrongValueKind)), got {other:?}")
        }
    }
}

/// TC-294 (FR-089-AC-4)/FR-049-AC-2, PR #326 review finding F1: the same
/// unresolved id refuses at admission even when the checked body never
/// reads the parameter at all. Before this fix, `ValueType::admits`'s own
/// presence-only check let an unresolved (or mismatched-maximum, see
/// [`tc_295_population_maximum_mismatch_refuses_even_when_unconsumed`])
/// population argument through whenever nothing consumed it, completing
/// with the body's own unrelated value (the review's own probe: "a
/// population parameter that is never consumed... `Completed(Boolean(true))`").
/// A body that ignores `p` entirely still refuses now.
#[test]
#[trace("TC-294", "FR-089-AC-4", "FR-049-AC-2")]
fn tc_294_unresolved_population_id_refuses_even_when_unconsumed() {
    let scenario = scenario();
    let package = package(&scenario);
    let parameters = [("p", ValueType::Population(3))];
    let expression = Expression::Boolean(true);
    let unresolved_id = scenario.binding.population_id();

    let checked = check(&package, &parameters, &expression);
    let mut meter = Meter::new(SCALAR_UNLIMITED);
    let result = package.evaluate(
        &checked,
        vec![Value::Population(unresolved_id)],
        &ObjectEnvironment::default(),
        &mut meter,
    );
    match result {
        Err(CallFailure::Input(InputRefusal::WrongValueKind { parameter })) => {
            assert_eq!(parameter, 0)
        }
        other => {
            panic!("expected Err(CallFailure::Input(InputRefusal::WrongValueKind)), got {other:?}")
        }
    }
}

/// TC-295 (FR-089-AC-5): the QSL layer treats `ValueType::Population(maximum)`
/// as admitting a `Value::Population(population_id)` exactly when the
/// binding `population_id` resolves to (through the recorded correspondence)
/// has a declared maximum equal to `maximum`. A binding admitted with
/// declared maximum 5 is admitted under `Population<5>` and refused under
/// `Population<6>` -- the same resolved binding both times, only the
/// checked parameter's own declared maximum differs, catching an
/// implementation that admits every `Value::Population(_)` regardless of
/// `maximum`.
#[test]
#[trace("TC-295", "FR-089-AC-5")]
fn tc_295_population_type_pairing_checks_the_resolved_maximum() {
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let universe = object_universe(&domain_package).unwrap().identity();
    let a = type_id(&view, "model.A");
    let b = type_id(&view, "model.B");

    let mut meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let binding = match admit_binding(
        &domain_package,
        &view,
        &p1("test/orders"),
        &p1_population_key(),
        GeneralizationClosure::Closed,
        Some(5),
        &mut meter,
    ) {
        AdmissionOutcome::Admitted(binding) => binding,
        other => panic!("expected an admitted binding, got {other:?}"),
    };
    let id = binding.population_id();
    let scenario = Scenario {
        universe,
        a,
        b,
        binding: binding.clone(),
    };
    let environment = ObjectEnvironment::default()
        .with_population(binding)
        .unwrap();
    let package = package(&scenario);
    let expression = all_instances(ValueType::Reference(node_key(&a)));

    let parameters_5 = [("p", ValueType::Population(5))];
    let (outcome_5, _) = run(
        &package,
        &parameters_5,
        &expression,
        vec![Value::Population(id)],
        SCALAR_UNLIMITED,
        &environment,
    );
    match outcome_5 {
        Outcome::Completed(_) => {}
        other => panic!("expected admission under a matching Population<5>, got {other:?}"),
    }

    let parameters_6 = [("p", ValueType::Population(6))];
    let checked_6 = check(&package, &parameters_6, &expression);
    let mut meter_6 = Meter::new(SCALAR_UNLIMITED);
    let result_6 = package.evaluate(
        &checked_6,
        vec![Value::Population(id)],
        &environment,
        &mut meter_6,
    );
    match result_6 {
        Err(CallFailure::Input(InputRefusal::WrongValueKind { parameter })) => {
            assert_eq!(parameter, 0)
        }
        other => panic!(
            "expected Err(CallFailure::Input(InputRefusal::WrongValueKind)) under a mismatched \
             Population<6>, got {other:?}"
        ),
    }
}

/// TC-295 (FR-089-AC-5)/FR-049-AC-2, PR #326 review finding F1: the same
/// declared-maximum mismatch refuses at admission even when the checked
/// body never reads the parameter at all -- companion to
/// [`tc_294_unresolved_population_id_refuses_even_when_unconsumed`] for the
/// "resolved, wrong maximum" case rather than "not recorded".
#[test]
#[trace("TC-295", "FR-089-AC-5", "FR-049-AC-2")]
fn tc_295_population_maximum_mismatch_refuses_even_when_unconsumed() {
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let universe = object_universe(&domain_package).unwrap().identity();
    let a = type_id(&view, "model.A");
    let b = type_id(&view, "model.B");

    let mut meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let binding = match admit_binding(
        &domain_package,
        &view,
        &p1("test/orders"),
        &p1_population_key(),
        GeneralizationClosure::Closed,
        Some(5),
        &mut meter,
    ) {
        AdmissionOutcome::Admitted(binding) => binding,
        other => panic!("expected an admitted binding, got {other:?}"),
    };
    let id = binding.population_id();
    let scenario = Scenario {
        universe,
        a,
        b,
        binding: binding.clone(),
    };
    let environment = ObjectEnvironment::default()
        .with_population(binding)
        .unwrap();
    let package = package(&scenario);
    let expression = Expression::Boolean(true);

    let parameters_6 = [("p", ValueType::Population(6))];
    let checked = check(&package, &parameters_6, &expression);
    let mut meter = Meter::new(SCALAR_UNLIMITED);
    let result = package.evaluate(
        &checked,
        vec![Value::Population(id)],
        &environment,
        &mut meter,
    );
    match result {
        Err(CallFailure::Input(InputRefusal::WrongValueKind { parameter })) => {
            assert_eq!(parameter, 0)
        }
        other => {
            panic!("expected Err(CallFailure::Input(InputRefusal::WrongValueKind)), got {other:?}")
        }
    }
}

/// TC-294 (FR-089-AC-4)/S3 (PR #326 review): the same unresolved-id
/// admission refusal holds for `lookup<T>(p, r)`, not only `allInstances`
/// -- the two consumption sites `evaluate.rs`'s `Machine::resolve_population`
/// doc names (`:921`/`:934`). `admits_a_valid_reference_but_unresolved_
/// population` supplies a well-formed `r` (present in the environment, so
/// `DanglingReference` never fires) alongside an unresolved `p`, isolating
/// the population check from the reference check.
#[test]
#[trace("TC-294", "FR-089-AC-4")]
fn tc_294_lookup_refuses_an_unresolved_population_id() {
    let scenario = scenario();
    let package = package(&scenario);
    let target = ValueType::Reference(node_key(&scenario.a));
    let parameters = [
        ("p", ValueType::Population(3)),
        ("r", ValueType::Reference(node_key(&scenario.b))),
    ];
    let expression = lookup(target, AbsenceMode::Empty);
    let unresolved_id = scenario.binding.population_id();
    let present_reference = object_reference(&scenario.universe, &scenario.b, "b1");
    let environment =
        ObjectEnvironment::new(&types(&scenario), [(present_reference.clone(), vec![])]).unwrap();

    let checked = check(&package, &parameters, &expression);
    let mut meter = Meter::new(SCALAR_UNLIMITED);
    let result = package.evaluate(
        &checked,
        vec![
            Value::Population(unresolved_id),
            Value::Reference(present_reference),
        ],
        &environment,
        &mut meter,
    );
    match result {
        Err(CallFailure::Input(InputRefusal::WrongValueKind { parameter })) => {
            assert_eq!(parameter, 0)
        }
        other => {
            panic!("expected Err(CallFailure::Input(InputRefusal::WrongValueKind)), got {other:?}")
        }
    }
}

/// TC-391 (FR-090-AC-10): `CheckedPackage::call` -- not only `evaluate`,
/// which TC-294 already covers above -- refuses a `Value::Population`
/// argument whose id names no recorded binding as `Err(CallFailure::
/// Input(_))`, before S6a runs and before any charge. `F`'s own declared
/// parameter is `p: Population<M::A>[3]` ([`package_with_function`]).
#[test]
#[trace("TC-391", "FR-090-AC-10")]
fn tc_391_call_refuses_an_unresolved_population_id_at_admission() {
    let scenario = scenario();
    let package = package_with_function(&scenario);
    let unresolved_id = scenario.binding.population_id();
    let mut meter = Meter::new(SCALAR_UNLIMITED);

    let result = package.call(
        &QualifiedName::unqualified("F").unwrap(),
        vec![Value::Population(unresolved_id)],
        &ObjectEnvironment::default(),
        &mut meter,
    );
    match result {
        Err(CallFailure::Input(InputRefusal::WrongValueKind { parameter })) => {
            assert_eq!(parameter, 0)
        }
        other => {
            panic!("expected Err(CallFailure::Input(InputRefusal::WrongValueKind)), got {other:?}")
        }
    }
    assert!(
        meter.admitted_charges().is_empty(),
        "admission refuses before any charge"
    );
}

/// TC-391 (FR-090-AC-10): the companion admission refusal, a resolved
/// binding whose own declared maximum (2) differs from `F`'s declared
/// `Population<3>` -- also `Err(CallFailure::Input(_))` from `call`, before
/// S6a runs and before any charge.
#[test]
#[trace("TC-391", "FR-090-AC-10")]
fn tc_391_call_refuses_a_population_maximum_mismatch_at_admission() {
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let universe = object_universe(&domain_package).unwrap().identity();
    let a = type_id(&view, "model.A");
    let b = type_id(&view, "model.B");
    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let binding = match admit_binding(
        &domain_package,
        &view,
        &p1("test/orders"),
        &p1_population_key(),
        GeneralizationClosure::Closed,
        Some(2),
        &mut admission,
    ) {
        AdmissionOutcome::Admitted(binding) => binding,
        other => panic!("expected an admitted binding, got {other:?}"),
    };
    let id = binding.population_id();
    let scenario = Scenario {
        universe,
        a,
        b,
        binding: binding.clone(),
    };
    let objects = ObjectEnvironment::default()
        .with_population(binding)
        .unwrap();
    let package = package_with_function(&scenario);
    let mut meter = Meter::new(SCALAR_UNLIMITED);

    let result = package.call(
        &QualifiedName::unqualified("F").unwrap(),
        vec![Value::Population(id)],
        &objects,
        &mut meter,
    );
    match result {
        Err(CallFailure::Input(InputRefusal::WrongValueKind { parameter })) => {
            assert_eq!(parameter, 0)
        }
        other => {
            panic!("expected Err(CallFailure::Input(InputRefusal::WrongValueKind)), got {other:?}")
        }
    }
    assert!(
        meter.admitted_charges().is_empty(),
        "admission refuses before any charge"
    );
}
