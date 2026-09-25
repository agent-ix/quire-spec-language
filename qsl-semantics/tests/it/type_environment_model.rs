// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-57: the checker's `TypeEnvironment` against the model it checks.
//!
//! Three parts:
//!
//! - The divergence table: for a supertype naming no admitted type, a
//!   generalization cycle and a chain past the `ancestor_steps` ceiling, the
//!   check-time environment and the model's own evaluation-time walk give
//!   the same verdict over the same edges.
//! - Declaration-time flattening of object-type attributes with FR-151 field
//!   redefinition: inherited fields, hidden redefined fields, the most
//!   derived redefiner, conflicts, same-name fields, bad targets and renamed
//!   redefinitions.
//! - `object_type_supertypes`' injective re-key.

use ix_trace_rs::trace;
use qsl_foundation::absence::AbsenceMode;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::diagnostic::{CatalogCode, CatalogCoded, LimitExceeded, LimitKind};
use qsl_semantics::check::object_type_supertypes;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::dispatch::GeneralizationClosure;
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, Extent, ObjectTypeRecord,
    PopulationRecord,
};
use qsl_semantics::model::key::{DeclarationKey, EffectiveId};
use qsl_semantics::model::normalize::{
    normalize, EffectiveView, ModelRefusalCause, NormalizeOutcome,
};
use qsl_semantics::model::object_environment::{
    ObjectEnvironment, ObjectEnvironmentCause, ObjectEnvironmentRefusal,
};
use qsl_semantics::model::population::{
    admit_binding, lookup, AdmissionMeter, AdmissionOutcome, LookupKey, LookupOutcome,
    PopulationAdmissionLimits, PopulationBinding, PopulationDocument,
};
use qsl_semantics::value::declaration::{
    Admission, Component, CompositeDeclaration, CompositeShape, ConstructionCause,
    ConstructionRefusal, DeclarationCause, FieldDeclaration, FieldRef, ObjectTypeDeclaration,
    TypeEnvironment, TypeEnvironmentLimits, DEFAULT_WORK_UNITS,
};
use quire_exact::{
    FieldValue, Integer, IntegerInterval, Meter, ObjectId, ObjectReference, Presence, ScalarLimits,
    UniverseId, Value, ValueType,
};
use sha2::{Digest, Sha256};

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

// ---- model fixtures ---------------------------------------------------------

fn object_type_record(identity: &str, supertypes: &[&str]) -> DomainPackageRecord {
    DomainPackageRecord::ObjectType(ObjectTypeRecord {
        key: DeclarationKey::fixture(identity),
        interface_features: None,
        abstract_type: false,
        supertypes: supertypes
            .iter()
            .map(|general| DeclarationKey::fixture(*general))
            .collect(),
    })
}

fn population_key() -> DeclarationKey {
    DeclarationKey::fixture("model.pop")
}

/// A closed population over `member_types`.
fn population_record(member_types: &[&str]) -> DomainPackageRecord {
    DomainPackageRecord::Population(PopulationRecord {
        key: population_key(),
        member_types: member_types
            .iter()
            .map(|member| DeclarationKey::fixture(*member))
            .collect(),
        extent: Extent::Closed,
    })
}

fn package(label: &str, records: Vec<DomainPackageRecord>) -> DomainPackage {
    DomainPackage::new(DomainPackageRef::fixture(label), records)
}

fn view(domain_package: &DomainPackage, ancestor_steps: u64) -> EffectiveView {
    let limits = ModelNormalizationLimits {
        ancestor_steps,
        ..ModelNormalizationLimits::UNLIMITED
    };
    match normalize(domain_package, limits) {
        NormalizeOutcome::Completed(view) => view,
        other => panic!("expected a completed effective view, got {other:?}"),
    }
}

fn type_id(view: &EffectiveView, identity: &str) -> EffectiveId {
    view.type_identities()
        .get(&DeclarationKey::fixture(identity))
        .copied()
        .unwrap_or_else(|| panic!("{identity} has no effective type identity"))
}

/// The check-time environment a model intake builds from `view`: every
/// object type under its effective identity, with its supertypes re-keyed
/// by `object_type_supertypes`, admitted under `ancestor_steps`.
fn environment_of(view: &EffectiveView, ancestor_steps: u64) -> Admission<TypeEnvironment> {
    let supertypes = object_type_supertypes(view).unwrap();
    TypeEnvironment::bounded(
        [],
        supertypes.into_iter().map(|(key, generals)| {
            ObjectTypeDeclaration::new(key, key.to_string(), vec![]).with_supertypes(generals)
        }),
        TypeEnvironmentLimits {
            ancestor_steps,
            ..TypeEnvironmentLimits::default()
        },
    )
}

/// An empty binding of `view`'s closed population, admitted under
/// `ancestor_steps`: `lookup` walks conformance under that same ceiling.
fn binding(view: &EffectiveView, ancestor_steps: u64) -> PopulationBinding {
    let mut meter = AdmissionMeter::new(PopulationAdmissionLimits {
        ancestor_steps,
        ..PopulationAdmissionLimits::UNLIMITED
    });
    let document = PopulationDocument {
        model_identity: "test/orders".to_owned(),
        members: vec![],
    };
    match admit_binding(
        view,
        &document,
        &population_key(),
        GeneralizationClosure::Closed,
        Some(3),
        &mut meter,
    ) {
        AdmissionOutcome::Admitted(binding) => binding,
        other => panic!("expected an admitted binding, got {other:?}"),
    }
}

/// `lookup<t>(p, r) absent empty` for a reference `r` statically typed
/// `s`, naming no member: the evaluation-time verdict on `s` against `t`.
fn evaluate_lookup(
    binding: &PopulationBinding,
    view: &EffectiveView,
    s: &str,
    t: &str,
) -> LookupOutcome {
    let key = LookupKey {
        static_type: DeclarationKey::fixture(s),
        universe: binding.universe().as_bytes().to_vec(),
        type_identity: type_id(view, s),
        object: b"nobody".to_vec(),
    };
    lookup(
        binding,
        &DeclarationKey::fixture(t),
        &key,
        AbsenceMode::Empty,
        &mut Meter::new(SCALAR_UNLIMITED),
    )
}

/// Whether `outcome` is `lookup`'s own non-conformance verdict
/// (`ill_typed`/`type-mismatch`).
fn is_type_mismatch(outcome: &LookupOutcome) -> bool {
    matches!(
        outcome,
        LookupOutcome::Refused(refusal)
            if refusal.code == Code::IllTyped && refusal.cause == ModelRefusalCause::TypeMismatch
    )
}

/// Whether `outcome` is the FR-082 `resource_exhausted`/`ancestor-steps`
/// refusal naming `limit`.
fn is_ancestor_steps(outcome: &LookupOutcome, limit: u64) -> bool {
    matches!(
        outcome,
        LookupOutcome::Refused(refusal)
            if refusal.code == Code::ResourceExhausted
                && matches!(refusal.cause, ModelRefusalCause::AncestorSteps { limit: l, .. } if l == limit)
    )
}

// ---- divergence table -------------------------------------------------------

/// Divergence row 1: a supertype naming no admitted type. The model refuses
/// the package at normalization (`dangling_reference`/`UnknownGeneral`), so
/// no binding exists to evaluate against; the checker refuses the same
/// edge, re-keyed, as `UnknownObjectType`. Both refuse; neither admits the
/// type with the edge silently dropped.
#[trace("TC-219", "FR-082-AC-6")]
#[test]
fn divergence_unknown_supertype_refuses_at_check_and_at_evaluation() {
    let domain_package = package(
        "bundle.qsl57-unknown",
        vec![
            object_type_record("model.B", &["model.Missing"]),
            population_record(&["model.B"]),
        ],
    );
    match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusals) => {
            let refusal = refusals.into_first();
            assert_eq!(refusal.code, Code::DanglingReference);
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::UnknownGeneral {
                    supertype: DeclarationKey::fixture("model.B"),
                    general: DeclarationKey::fixture("model.Missing"),
                }
            );
        }
        other => panic!("expected normalization to refuse, got {other:?}"),
    }

    let b = effective("model.B");
    let missing = effective("model.Missing");
    let refusal = TypeEnvironment::new(
        [],
        [ObjectTypeDeclaration::new(b, "M::B", vec![]).with_supertypes(vec![missing])],
    )
    .unwrap_err()
    .into_refused()
    .unwrap();
    assert_eq!(refusal.cause, DeclarationCause::UnknownObjectType(missing));
    assert_eq!(refusal.code(), "invalid_semantic_graph");
}

/// Divergence row 1, positive: the same type with its supertype declared is
/// admitted on both sides, and both answer `B` conforms to `A`.
#[trace("TC-219", "FR-082-AC-6")]
#[test]
fn divergence_known_supertype_admits_at_check_and_at_evaluation() {
    let domain_package = package(
        "bundle.qsl57-known",
        vec![
            object_type_record("model.A", &[]),
            object_type_record("model.B", &["model.A"]),
            population_record(&["model.A"]),
        ],
    );
    let view = view(&domain_package, 10);
    let environment = environment_of(&view, 10).unwrap();
    let (a, b) = (type_id(&view, "model.A"), type_id(&view, "model.B"));
    assert!(environment.conforms(b, a));
    assert!(!environment.conforms(a, b));
    let binding = binding(&view, 10);
    assert_eq!(
        evaluate_lookup(&binding, &view, "model.B", "model.A"),
        LookupOutcome::Completed(None)
    );
    assert!(is_type_mismatch(&evaluate_lookup(
        &binding, &view, "model.A", "model.B"
    )));
}

/// Divergence row 2: a generalization cycle. The model refuses the package
/// at normalization (`SpecializationCycle`); the checker refuses the same
/// cycle as `GeneralizationCycle`. Neither tolerates it.
#[trace("TC-219", "FR-082-AC-2", "FR-082-AC-6")]
#[test]
fn divergence_generalization_cycle_refuses_at_check_and_at_evaluation() {
    let domain_package = package(
        "bundle.qsl57-cycle",
        vec![
            object_type_record("model.M", &["model.N"]),
            object_type_record("model.N", &["model.M"]),
            population_record(&["model.M"]),
        ],
    );
    match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusals) => {
            assert!(
                refusals.iter().all(|refusal| matches!(
                    refusal.cause,
                    ModelRefusalCause::SpecializationCycle { .. }
                )),
                "every refusal is the cycle: {refusals:?}"
            );
        }
        other => panic!("expected normalization to refuse the cycle, got {other:?}"),
    }

    let (m, n) = (effective("model.M"), effective("model.N"));
    let refusal = TypeEnvironment::new(
        [],
        [
            ObjectTypeDeclaration::new(m, "M::M", vec![]).with_supertypes(vec![n]),
            ObjectTypeDeclaration::new(n, "M::N", vec![]).with_supertypes(vec![m]),
        ],
    )
    .unwrap_err()
    .into_refused()
    .unwrap();
    assert!(matches!(
        refusal.cause,
        DeclarationCause::GeneralizationCycle { .. }
    ));
    assert_eq!(refusal.code(), "ill_typed");
}

/// A linear chain `model.c.0 -> model.c.1 -> ... -> model.c.{depth}` plus an
/// unrelated `model.z`: `model.c.0` has `depth` ancestors, so the model's
/// walk from it to `model.z` expands `depth + 1` types.
fn chain_package(depth: u64) -> DomainPackage {
    let names: Vec<String> = (0..=depth).map(|i| format!("model.c.{i}")).collect();
    let mut records: Vec<DomainPackageRecord> = names
        .iter()
        .enumerate()
        .map(|(i, name)| match names.get(i + 1) {
            Some(general) => object_type_record(name, &[general.as_str()]),
            None => object_type_record(name, &[]),
        })
        .collect();
    records.push(object_type_record("model.z", &[]));
    let top = format!("model.c.{depth}");
    records.push(population_record(&[top.as_str(), "model.z"]));
    package("bundle.qsl57-chain", records)
}

/// Divergence row 3, at the ceiling: the deepest type's worst-case walk
/// expands `depth + 1` types, exactly the ceiling. The checker admits the
/// environment and every conformance question it answers completes at
/// evaluation with the same verdict: `c.0` conforms to `c.{depth}`, and not
/// to `z`.
#[trace("TC-220", "FR-082-AC-3", "FR-082-AC-6")]
#[test]
fn divergence_chain_within_the_ceiling_agrees_at_check_and_at_evaluation() {
    const DEPTH: u64 = 5;
    const CEILING: u64 = DEPTH + 1;
    let domain_package = chain_package(DEPTH);
    let view = view(&domain_package, CEILING);
    let environment = environment_of(&view, CEILING).unwrap();
    let binding = binding(&view, CEILING);
    let top = format!("model.c.{DEPTH}");

    assert!(environment.conforms(type_id(&view, "model.c.0"), type_id(&view, &top)));
    assert_eq!(
        evaluate_lookup(&binding, &view, "model.c.0", &top),
        LookupOutcome::Completed(None)
    );
    assert!(!environment.conforms(type_id(&view, "model.c.0"), type_id(&view, "model.z")));
    assert!(is_type_mismatch(&evaluate_lookup(
        &binding,
        &view,
        "model.c.0",
        "model.z"
    )));
}

/// Divergence row 3, one past the ceiling: the same chain under a ceiling of
/// `depth`. The model still normalizes it (its longest path is `depth`
/// steps) but refuses the walk from `c.0` to `z` with `resource_exhausted`/
/// `ancestor-steps`; the checker refuses the environment with the same code
/// and ceiling rather than answering `false` from a closure the model
/// cannot compute.
#[trace("TC-220", "FR-082-AC-3", "FR-082-AC-6")]
#[test]
fn divergence_chain_past_the_ceiling_refuses_at_check_and_at_evaluation() {
    const DEPTH: u64 = 5;
    const CEILING: u64 = DEPTH;
    let domain_package = chain_package(DEPTH);
    let view = view(&domain_package, CEILING);
    let binding = binding(&view, CEILING);
    assert!(is_ancestor_steps(
        &evaluate_lookup(&binding, &view, "model.c.0", "model.z"),
        CEILING
    ));

    // Check time: a stage limit (ADR-014 B-3), node count, the ceiling
    // plus one, and no locus (FR-082).
    let limit = environment_of(&view, CEILING)
        .unwrap_err()
        .into_refused()
        .unwrap_err();
    assert_eq!(
        limit,
        LimitExceeded::new(LimitKind::NodeCount, CEILING, u128::from(CEILING) + 1)
    );
    assert_eq!(
        limit.catalog_code(),
        CatalogCode::new("stage_limit_exceeded", "node-count-exceeded")
    );
    assert_eq!(limit.locus(), None);
}

/// `TypeEnvironment::new` admits under the same default ceiling the model's
/// own limits default to, so a caller that sets neither gets one bound.
#[trace("TC-220", "FR-082-AC-6")]
#[test]
fn check_and_evaluation_share_one_default_ancestor_ceiling() {
    use qsl_semantics::value::declaration::DEFAULT_ANCESTOR_STEPS;
    assert_eq!(
        ModelNormalizationLimits::default().ancestor_steps,
        DEFAULT_ANCESTOR_STEPS
    );
    assert_eq!(
        PopulationAdmissionLimits::default().ancestor_steps,
        DEFAULT_ANCESTOR_STEPS
    );
}

// ---- flattening and redefinition --------------------------------------------

fn effective(label: &str) -> EffectiveId {
    EffectiveId::from_digest(Sha256::digest(label.as_bytes()).into())
}

fn integer_field(name: &str) -> FieldDeclaration {
    FieldDeclaration::new(name, ValueType::Integer, Presence::Required)
}

fn object(
    label: &str,
    fields: Vec<FieldDeclaration>,
    supertypes: &[&str],
) -> ObjectTypeDeclaration {
    ObjectTypeDeclaration::new(effective(label), label, fields).with_supertypes(
        supertypes
            .iter()
            .map(|general| effective(general))
            .collect(),
    )
}

fn field_ref(owner: &str, name: &str) -> FieldRef {
    FieldRef::new(effective(owner), name)
}

/// `(owner, name)` of every attribute of `label`'s effective set, in slot
/// order.
fn slots(environment: &TypeEnvironment, label: &str) -> Vec<(EffectiveId, String)> {
    environment
        .attributes(effective(label))
        .unwrap()
        .iter()
        .map(|attribute| (attribute.owner(), attribute.field().name().to_owned()))
        .collect()
}

fn reference(label: &str, object: &str) -> ObjectReference {
    ObjectReference::new(
        UniverseId::from_digest([0x42; 32]),
        effective(label),
        ObjectId::new(object).unwrap(),
    )
}

fn int(value: i64) -> FieldValue {
    FieldValue::Present(Value::Integer(Integer::from(value)))
}

fn slot_integer(slot: Option<&FieldValue>) -> Integer {
    match slot {
        Some(FieldValue::Present(Value::Integer(value))) => value.clone(),
        other => panic!("a present integer slot, not {other:?}"),
    }
}

/// A subtype's effective attribute set holds its own fields and every
/// ancestor's, so an object of the subtype has a slot for the inherited
/// field, and the slot is found through the ancestor's field identity.
#[trace("TC-196", "FR-151-AC-1")]
#[test]
fn an_inherited_field_is_flattened_into_the_subtype() {
    let environment = TypeEnvironment::new(
        [],
        [
            object("M::A", vec![integer_field("x")], &[]),
            object("M::B", vec![integer_field("y")], &["M::A"]),
        ],
    )
    .unwrap();
    assert_eq!(
        slots(&environment, "M::B"),
        vec![
            (effective("M::B"), "y".to_owned()),
            (effective("M::A"), "x".to_owned()),
        ]
    );
    let inherited = environment.attribute(effective("M::B"), "x").unwrap();
    assert_eq!(inherited.identity(), field_ref("M::A", "x"));

    let b1 = reference("M::B", "b1");
    let objects = ObjectEnvironment::new(
        &environment,
        [(b1.clone(), vec![("x", int(7)), ("y", int(8))])],
    )
    .unwrap();
    assert_eq!(
        slot_integer(objects.attribute(&environment, &b1, &field_ref("M::A", "x"))),
        Integer::from(7_i64)
    );
    assert_eq!(
        slot_integer(objects.attribute(&environment, &b1, &field_ref("M::B", "y"))),
        Integer::from(8_i64)
    );
    // The inherited field is a required slot of `B`'s storage.
    assert_eq!(
        ObjectEnvironment::new(&environment, [(b1.clone(), vec![("y", int(8))])]).unwrap_err(),
        ObjectEnvironmentRefusal {
            object: Box::new(b1),
            cause: ObjectEnvironmentCause::Attribute(ConstructionRefusal {
                component: Component::Field("x".to_owned()),
                cause: ConstructionCause::MissingField,
            }),
        }
    );
}

/// FR-151 hide: `B.x` redefines `A.x`, so `B`'s set holds one `x`, owned by
/// `B`, and that one slot answers for `A.x` too. `A`'s own set is unchanged.
#[trace("TC-196", "FR-151-AC-1")]
#[test]
fn a_redefined_field_is_hidden_and_its_redefiner_takes_its_slot() {
    let environment = TypeEnvironment::new(
        [],
        [
            object("M::A", vec![integer_field("x")], &[]),
            object(
                "M::B",
                vec![integer_field("x").with_redefines(field_ref("M::A", "x"))],
                &["M::A"],
            ),
        ],
    )
    .unwrap();
    assert_eq!(
        slots(&environment, "M::A"),
        vec![(effective("M::A"), "x".to_owned())]
    );
    assert_eq!(
        slots(&environment, "M::B"),
        vec![(effective("M::B"), "x".to_owned())]
    );
    let exposed = environment.attribute(effective("M::B"), "x").unwrap();
    assert!(exposed.stands_for(&field_ref("M::A", "x")));
    assert!(exposed.stands_for(&field_ref("M::B", "x")));

    let b1 = reference("M::B", "b1");
    let objects =
        ObjectEnvironment::new(&environment, [(b1.clone(), vec![("x", int(3))])]).unwrap();
    assert_eq!(
        slot_integer(objects.attribute(&environment, &b1, &field_ref("M::A", "x"))),
        Integer::from(3_i64)
    );
}

/// FR-151 "only the redefining member of the most derived owner is
/// exposed": `B.x` and `C.x` both redefine `A.x`, and `C -> B`. `C`'s set
/// exposes `C.x` alone, standing for `A.x` and `B.x`.
#[trace("TC-196", "FR-151-AC-1")]
#[test]
fn the_most_derived_redefinition_wins() {
    let environment = TypeEnvironment::new(
        [],
        [
            object("M::A", vec![integer_field("x")], &[]),
            object(
                "M::B",
                vec![integer_field("x").with_redefines(field_ref("M::A", "x"))],
                &["M::A"],
            ),
            object(
                "M::C",
                vec![integer_field("x").with_redefines(field_ref("M::A", "x"))],
                &["M::B"],
            ),
        ],
    )
    .unwrap();
    assert_eq!(
        slots(&environment, "M::C"),
        vec![(effective("M::C"), "x".to_owned())]
    );
    let exposed = environment.attribute(effective("M::C"), "x").unwrap();
    for hidden in [field_ref("M::A", "x"), field_ref("M::B", "x")] {
        assert!(exposed.stands_for(&hidden), "C.x stands for {hidden:?}");
    }
}

/// A plain diamond: `B` and `C` both inherit `A.x`, and `D` inherits both.
/// `A.x` is one slot of `D`, not two, and a read resolved through either
/// path (`x` in `B`'s set, `x` in `C`'s) finds that one slot. `D`'s slots
/// are its own fields first, then `B`'s, then `C`'s (its supertypes in
/// declaration order), each attribute once.
#[trace("TC-196", "FR-151-AC-1")]
#[test]
fn a_diamond_shares_one_slot_and_orders_supertypes_by_declaration() {
    let environment = TypeEnvironment::new(
        [],
        [
            object("M::A", vec![integer_field("x")], &[]),
            object("M::B", vec![integer_field("b")], &["M::A"]),
            object("M::C", vec![integer_field("c")], &["M::A"]),
            object("M::D", vec![integer_field("d")], &["M::B", "M::C"]),
        ],
    )
    .unwrap();
    assert_eq!(
        slots(&environment, "M::D"),
        vec![
            (effective("M::D"), "d".to_owned()),
            (effective("M::B"), "b".to_owned()),
            (effective("M::A"), "x".to_owned()),
            (effective("M::C"), "c".to_owned()),
        ]
    );

    let d1 = reference("M::D", "d1");
    let objects = ObjectEnvironment::new(
        &environment,
        [(
            d1.clone(),
            vec![("x", int(5)), ("b", int(6)), ("c", int(7)), ("d", int(8))],
        )],
    )
    .unwrap();
    for path in ["M::B", "M::C"] {
        let resolved = environment.attribute(effective(path), "x").unwrap();
        assert_eq!(resolved.identity(), field_ref("M::A", "x"), "via {path}");
        assert_eq!(
            slot_integer(objects.attribute(&environment, &d1, &resolved.identity())),
            Integer::from(5_i64),
            "via {path}"
        );
    }

    // Reversing `D`'s supertypes reverses the inherited order.
    let reversed = TypeEnvironment::new(
        [],
        [
            object("M::A", vec![integer_field("x")], &[]),
            object("M::B", vec![integer_field("b")], &["M::A"]),
            object("M::C", vec![integer_field("c")], &["M::A"]),
            object("M::D", vec![integer_field("d")], &["M::C", "M::B"]),
        ],
    )
    .unwrap();
    assert_eq!(
        slots(&reversed, "M::D"),
        vec![
            (effective("M::D"), "d".to_owned()),
            (effective("M::C"), "c".to_owned()),
            (effective("M::A"), "x".to_owned()),
            (effective("M::B"), "b".to_owned()),
        ]
    );
}

/// FR-151 conflict: `B.x` and `C.x` both redefine `A.x`, and `D` inherits
/// both with neither owner more derived than the other. Refused
/// `RedefinitionConflict` naming `A.x`; a `D.x` redefining `A.x` itself
/// resolves it.
#[trace("TC-196", "FR-151-AC-1")]
#[test]
fn conflicting_redefinitions_refuse_until_a_more_derived_one_resolves_them() {
    let diamond = |d_fields: Vec<FieldDeclaration>| {
        TypeEnvironment::new(
            [],
            [
                object("M::A", vec![integer_field("x")], &[]),
                object(
                    "M::B",
                    vec![integer_field("x").with_redefines(field_ref("M::A", "x"))],
                    &["M::A"],
                ),
                object(
                    "M::C",
                    vec![integer_field("x").with_redefines(field_ref("M::A", "x"))],
                    &["M::A"],
                ),
                object("M::D", d_fields, &["M::B", "M::C"]),
            ],
        )
    };
    let refusal = diamond(vec![]).unwrap_err().into_refused().unwrap();
    assert_eq!(refusal.declaration, "M::D");
    assert_eq!(
        refusal.cause,
        DeclarationCause::RedefinitionConflict(field_ref("M::A", "x"))
    );
    assert_eq!(refusal.code(), "invalid_semantic_graph");

    let resolved = diamond(vec![
        integer_field("x").with_redefines(field_ref("M::A", "x"))
    ])
    .unwrap();
    assert_eq!(
        slots(&resolved, "M::D"),
        vec![(effective("M::D"), "x".to_owned())]
    );
    let exposed = resolved.attribute(effective("M::D"), "x").unwrap();
    for hidden in [
        field_ref("M::A", "x"),
        field_ref("M::B", "x"),
        field_ref("M::C", "x"),
    ] {
        assert!(exposed.stands_for(&hidden), "D.x stands for {hidden:?}");
    }
}

/// A subtype field that shares an inherited field's name without
/// redefining it is a second field of that name: refused `DuplicateMember`.
#[trace("TC-196", "FR-151-AC-1")]
#[test]
fn a_same_name_field_that_does_not_redefine_is_refused() {
    let refusal = TypeEnvironment::new(
        [],
        [
            object("M::A", vec![integer_field("x")], &[]),
            object("M::B", vec![integer_field("x")], &["M::A"]),
        ],
    )
    .unwrap_err()
    .into_refused()
    .unwrap();
    assert_eq!(refusal.declaration, "M::B");
    assert_eq!(
        refusal.cause,
        DeclarationCause::DuplicateMember("x".to_owned())
    );
}

/// A `redefines` naming a field its owner does not inherit is refused
/// `RedefinitionTarget`: a missing field of an ancestor, a field of a type
/// that is no ancestor, and any `redefines` on a record field.
#[trace("TC-219", "FR-082-AC-2")]
#[test]
fn a_redefinition_of_a_field_not_inherited_is_refused() {
    let with_b_field = |target: FieldRef| {
        TypeEnvironment::new(
            [],
            [
                object("M::A", vec![integer_field("x")], &[]),
                object("M::Other", vec![integer_field("x")], &[]),
                object(
                    "M::B",
                    vec![integer_field("y").with_redefines(target)],
                    &["M::A"],
                ),
            ],
        )
    };
    for target in [field_ref("M::A", "nope"), field_ref("M::Other", "x")] {
        let refusal = with_b_field(target.clone())
            .unwrap_err()
            .into_refused()
            .unwrap();
        assert_eq!(refusal.declaration, "M::B");
        assert_eq!(refusal.cause, DeclarationCause::RedefinitionTarget(target));
    }
    // A type cannot redefine its own field.
    let own = TypeEnvironment::new(
        [],
        [object(
            "M::A",
            vec![
                integer_field("x"),
                integer_field("y").with_redefines(field_ref("M::A", "x")),
            ],
            &[],
        )],
    )
    .unwrap_err()
    .into_refused()
    .unwrap();
    assert_eq!(
        own.cause,
        DeclarationCause::RedefinitionTarget(field_ref("M::A", "x"))
    );

    let record = TypeEnvironment::new(
        [CompositeDeclaration::new(
            quire_exact::NodeKey::from_digest([0x11; 32]),
            "R",
            CompositeShape::Record(vec![
                integer_field("x").with_redefines(field_ref("M::A", "x"))
            ]),
        )],
        [object("M::A", vec![integer_field("x")], &[])],
    )
    .unwrap_err()
    .into_refused()
    .unwrap();
    assert_eq!(
        record.cause,
        DeclarationCause::RedefinitionTarget(field_ref("M::A", "x"))
    );
}

/// A renamed redefinition: `B.y` redefines `A.x`. `B`'s set holds `y` and
/// no `x`, so `x` does not resolve by name on `B`, while `A.x`'s identity
/// still finds `B.y`'s slot on a `B` object.
#[trace("TC-196", "FR-151-AC-1")]
#[test]
fn a_renamed_redefinition_replaces_the_inherited_name() {
    let environment = TypeEnvironment::new(
        [],
        [
            object("M::A", vec![integer_field("x")], &[]),
            object(
                "M::B",
                vec![integer_field("y").with_redefines(field_ref("M::A", "x"))],
                &["M::A"],
            ),
        ],
    )
    .unwrap();
    assert_eq!(
        slots(&environment, "M::B"),
        vec![(effective("M::B"), "y".to_owned())]
    );
    assert!(environment.attribute(effective("M::B"), "x").is_none());
    assert!(environment.attribute(effective("M::A"), "x").is_some());

    let b1 = reference("M::B", "b1");
    let objects =
        ObjectEnvironment::new(&environment, [(b1.clone(), vec![("y", int(5))])]).unwrap();
    assert_eq!(
        slot_integer(objects.attribute(&environment, &b1, &field_ref("M::A", "x"))),
        Integer::from(5_i64)
    );
    assert_eq!(
        ObjectEnvironment::new(&environment, [(b1.clone(), vec![("x", int(5))])]).unwrap_err(),
        ObjectEnvironmentRefusal {
            object: Box::new(b1),
            cause: ObjectEnvironmentCause::Attribute(ConstructionRefusal {
                component: Component::Field("x".to_owned()),
                cause: ConstructionCause::UndeclaredField,
            }),
        }
    );
}

fn int_field(name: &str, lower: i64, upper: i64) -> FieldDeclaration {
    FieldDeclaration::new(
        name,
        ValueType::Int(IntegerInterval::new(Integer::from(lower), Integer::from(upper)).unwrap()),
        Presence::Required,
    )
}

/// A redefinition may narrow its target: `Int[0, 9]` redefining `Integer`,
/// and a required field redefining an optional one, are admitted.
#[trace("TC-196", "FR-151-AC-2")]
#[test]
fn a_narrowing_redefinition_is_admitted() {
    let environment = TypeEnvironment::new(
        [],
        [
            object(
                "M::A",
                vec![
                    integer_field("x"),
                    FieldDeclaration::new("o", ValueType::Integer, Presence::Optional),
                ],
                &[],
            ),
            object(
                "M::B",
                vec![
                    int_field("x", 0, 9).with_redefines(field_ref("M::A", "x")),
                    integer_field("o").with_redefines(field_ref("M::A", "o")),
                ],
                &["M::A"],
            ),
        ],
    );
    assert!(environment.is_ok(), "{environment:?}");
}

/// A redefinition that widens its target is refused `RedefinitionWidens`
/// (`ill_typed`): a value type the target does not admit, and an optional
/// redefiner of a required field. Admitting either would let `deref(r).x`
/// through `A` read a value its checked type does not describe.
#[trace("TC-196", "FR-151-AC-2")]
#[test]
fn a_widening_redefinition_is_refused() {
    let widening = |b_field: FieldDeclaration| {
        TypeEnvironment::new(
            [],
            [
                object("M::A", vec![int_field("x", 0, 9)], &[]),
                object(
                    "M::B",
                    vec![b_field.with_redefines(field_ref("M::A", "x"))],
                    &["M::A"],
                ),
            ],
        )
        .unwrap_err()
        .into_refused()
        .unwrap()
    };
    for b_field in [
        integer_field("x"),
        int_field("x", 0, 10),
        FieldDeclaration::new("x", ValueType::Boolean, Presence::Required),
        FieldDeclaration::new(
            "x",
            int_field("x", 0, 9).value_type().clone(),
            Presence::Optional,
        ),
    ] {
        let refusal = widening(b_field.clone());
        assert_eq!(refusal.declaration, "M::B", "{b_field:?}");
        assert_eq!(
            refusal.cause,
            DeclarationCause::RedefinitionWidens(field_ref("M::A", "x")),
            "{b_field:?}"
        );
        assert_eq!(refusal.code(), "ill_typed");
    }
}

/// The most derived redefiner stands in every hidden redefinition's slot,
/// so it must narrow each of them, not only the target it names: `C.x:
/// Int[0, 100]` redefines `A.x: Integer` but would stand in for `B.x:
/// Int[0, 9]`, so a `C` read through `Reference<B>` could yield 50. Refused
/// `RedefinitionWidens` naming `B.x`.
#[trace("TC-196", "FR-151-AC-2")]
#[test]
fn the_most_derived_redefinition_must_narrow_every_field_it_hides() {
    let refusal = TypeEnvironment::new(
        [],
        [
            object("M::A", vec![integer_field("x")], &[]),
            object(
                "M::B",
                vec![int_field("x", 0, 9).with_redefines(field_ref("M::A", "x"))],
                &["M::A"],
            ),
            object(
                "M::C",
                vec![int_field("x", 0, 100).with_redefines(field_ref("M::A", "x"))],
                &["M::B"],
            ),
        ],
    )
    .unwrap_err()
    .into_refused()
    .unwrap();
    assert_eq!(refusal.declaration, "M::C");
    assert_eq!(
        refusal.cause,
        DeclarationCause::RedefinitionWidens(field_ref("M::B", "x"))
    );
}

fn reference_field(name: &str, target: &str) -> FieldDeclaration {
    FieldDeclaration::new(
        name,
        ValueType::Reference(effective(target)),
        Presence::Optional,
    )
}

/// A reference field is redefined only with its own reference type:
/// evaluation's `ValueType::admits` matches a reference's object type
/// exactly, so `B.o: Reference<B>?` redefining `A.o: Reference<A>?` would
/// pass the check and then refuse `CheckedInvariant` when `o` is read
/// through `A`. Refused `RedefinitionWidens` naming `A.o`; the same
/// redefinition typed `Reference<A>?` is admitted.
#[trace("TC-196", "FR-151-AC-2")]
#[test]
fn a_reference_field_is_redefined_only_with_its_own_reference_type() {
    let redefined = |b_field: FieldDeclaration| {
        TypeEnvironment::new(
            [],
            [
                object("M::A", vec![reference_field("o", "M::A")], &[]),
                object(
                    "M::B",
                    vec![b_field.with_redefines(field_ref("M::A", "o"))],
                    &["M::A"],
                ),
            ],
        )
    };
    let refusal = redefined(reference_field("o", "M::B"))
        .unwrap_err()
        .into_refused()
        .unwrap();
    assert_eq!(refusal.declaration, "M::B");
    assert_eq!(
        refusal.cause,
        DeclarationCause::RedefinitionWidens(field_ref("M::A", "o"))
    );
    assert_eq!(refusal.code(), "ill_typed");

    let environment = redefined(reference_field("o", "M::A")).unwrap();
    let exposed = environment.attribute(effective("M::B"), "o").unwrap();
    assert_eq!(exposed.owner(), effective("M::B"));
    assert!(exposed.stands_for(&field_ref("M::A", "o")));
}

// ---- admission work budget -------------------------------------------------

/// A linear chain `M::T0 <- M::T1 <- ...` of `depth` types, each declaring
/// one integer field of its own.
fn chain(depth: usize) -> Vec<ObjectTypeDeclaration> {
    (0..depth)
        .map(|level| {
            let label = format!("M::T{level}");
            let supertypes = match level.checked_sub(1) {
                Some(previous) => vec![effective(&format!("M::T{previous}"))],
                None => vec![],
            };
            ObjectTypeDeclaration::new(
                effective(&label),
                label,
                vec![integer_field(&format!("f{level}"))],
            )
            .with_supertypes(supertypes)
        })
        .collect()
}

/// The flattened slots of a `depth`-type chain: type `k` has `k + 1`.
fn chain_slots(depth: u64) -> u64 {
    depth * (depth + 1) / 2
}

/// The work-budget stage limit an admission stopped at (FR-082, ADR-014
/// B-3): the refused charge's cumulative total passes the bound, and it
/// carries no locus.
fn work_limit(admission: Admission<TypeEnvironment>) -> LimitExceeded {
    let limit = admission
        .unwrap_err()
        .into_refused()
        .expect_err("a work-budget stage limit, not a refusal");
    assert_eq!(limit.kind(), LimitKind::WorkBudget);
    assert!(limit.actual() > u128::from(limit.configured_bound()));
    assert_eq!(limit.locus(), None);
    limit
}

fn work_units(work_units: u64) -> TypeEnvironmentLimits {
    TypeEnvironmentLimits {
        work_units,
        ..TypeEnvironmentLimits::default()
    }
}

/// A 5,000-type chain holds 12.5 million flattened slots. Admission charges
/// its ancestor closure and flattening to the default `work_units` budget
/// and stops at a work-budget stage limit naming it, instead of running
/// unmetered; a 1,000-type chain fits and admits.
#[trace("TC-220", "FR-082-AC-7")]
#[test]
fn a_deep_chain_refuses_the_admission_work_budget() {
    let limit = work_limit(TypeEnvironment::new([], chain(5_000)));
    assert_eq!(limit.configured_bound(), DEFAULT_WORK_UNITS);
    assert_eq!(
        limit.catalog_code(),
        CatalogCode::new("stage_limit_exceeded", "work-budget-exceeded")
    );

    let environment = TypeEnvironment::new([], chain(1_000)).unwrap();
    assert_eq!(
        environment.attributes(effective("M::T999")).unwrap().len(),
        1_000
    );
}

/// Admission work grows with the flattened output, not faster: a linear
/// chain of every depth admits within four units a flattened slot, and
/// refuses when given fewer units than it has slots, so each slot is
/// charged. The pre-budget algorithm's all-pairs scans cost a cube of the
/// depth; so would any regression back to them.
#[trace("TC-220", "FR-082-AC-7")]
#[test]
fn a_linear_chain_costs_admission_work_linear_in_its_flattened_slots() {
    for depth in [50_u64, 100, 200, 400] {
        let types = || chain(usize::try_from(depth).unwrap());
        let slots = chain_slots(depth);
        assert!(
            TypeEnvironment::bounded([], types(), work_units(4 * slots)).is_ok(),
            "depth {depth} admits within 4 units a slot"
        );
        let limit = work_limit(TypeEnvironment::bounded([], types(), work_units(slots)));
        assert_eq!(limit.configured_bound(), slots, "depth {depth}");
    }
}

/// `M::W` with `width` direct supertypes `M::R0..`, each a root declaring
/// one field: `2 * width + 1` flattened slots in all.
fn wide(width: usize) -> Vec<ObjectTypeDeclaration> {
    let roots: Vec<String> = (0..width).map(|root| format!("M::R{root}")).collect();
    let mut types: Vec<ObjectTypeDeclaration> = roots
        .iter()
        .enumerate()
        .map(|(root, label)| object(label, vec![integer_field(&format!("r{root}"))], &[]))
        .collect();
    let supertypes: Vec<&str> = roots.iter().map(String::as_str).collect();
    types.push(object("M::W", vec![integer_field("w")], &supertypes));
    types
}

/// Wide multiple inheritance is charged what it costs: one type with 2,000
/// direct supertypes gathers their ancestor sets and sorts them once, and
/// admission stays within a fixed number of units a flattened slot (the
/// sort's `log` factor included); a budget below the slot count refuses.
#[trace("TC-220", "FR-082-AC-7")]
#[test]
fn wide_multiple_inheritance_costs_admission_work_bounded_in_its_slots() {
    for width in [500_u64, 2_000] {
        let types = || wide(usize::try_from(width).unwrap());
        let slots = 2 * width + 1;
        let environment = TypeEnvironment::bounded([], types(), work_units(24 * slots)).unwrap();
        assert_eq!(
            environment.attributes(effective("M::W")).unwrap().len(),
            usize::try_from(width + 1).unwrap()
        );
        let limit = work_limit(TypeEnvironment::bounded([], types(), work_units(slots)));
        assert_eq!(limit.configured_bound(), slots, "width {width}");
    }
}

// ---- object_type_supertypes -------------------------------------------------

/// The bridge re-keys every object type of a normalized view injectively.
/// Two object-type records under one key never reach it: normalization
/// refuses them `conflicting-binding` first. The bridge's own
/// `DuplicateObjectKey` refusal for a non-injective map is a unit test in
/// `check::checked_dispatch`, which can build such a map directly.
#[trace("TC-196")]
#[test]
fn object_type_supertypes_rekeys_a_view_and_normalization_refuses_a_shared_key() {
    let domain_package = package(
        "bundle.qsl57-bridge",
        vec![
            object_type_record("model.A", &[]),
            object_type_record("model.B", &["model.A"]),
            population_record(&["model.A"]),
        ],
    );
    let view = view(&domain_package, 10);
    let supertypes = object_type_supertypes(&view).unwrap();
    let (a, b) = (type_id(&view, "model.A"), type_id(&view, "model.B"));
    assert_eq!(supertypes.get(&b), Some(&vec![a]));
    assert_eq!(supertypes.get(&a), Some(&vec![]));

    let shared = package(
        "bundle.qsl57-shared",
        vec![
            object_type_record("model.A", &[]),
            object_type_record("model.A", &["model.A"]),
        ],
    );
    match normalize(&shared, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusals) => {
            assert!(
                refusals.iter().any(|refusal| refusal.cause
                    == ModelRefusalCause::ConflictingBinding {
                        key: DeclarationKey::fixture("model.A")
                    }),
                "expected conflicting-binding on model.A: {refusals:?}"
            );
        }
        other => panic!("expected normalization to refuse a shared key, got {other:?}"),
    }
}
