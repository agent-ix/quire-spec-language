// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-198: closed model lookup (FR-153).
//!
//! Fixtures reuse TC-195 F1 (`model.A`, `model.B`, generalization `B -> A`)
//! exactly as TC-198 imports it as `M`, and population documents P1/P2 as
//! TC-198 states them. L07 (`pre(allInstances<T>(p))` in a postcondition),
//! L08 (FR-149 reference equality/upcast) and L10/L11 (`reaches`/`deref`,
//! FR-043's graph navigation) are out of this rung's scope: L07's
//! pre/post-state anchor needs real operation-effect execution this crate
//! has no evaluator for yet, so FR-153-AC-7 stays unbacked here and is
//! tracked on QSL #147 instead; see the crate's `src/model/population.rs`
//! module docs.

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use quire_spec_language::diagnostic::Code;
use quire_spec_language::model::accounting::ModelNormalizationLimits;
use quire_spec_language::model::bundle::{
    Bundle, BundleRecord, FieldMemberRecord, GeneralizationRecord, ModelSelection, Multiplicity,
    ObjectTypeRecord, SubsettingRecord,
};
use quire_spec_language::model::dispatch::GeneralizationClosure;
use quire_spec_language::model::key::{EffectiveId, ProducerKey, Revision};
use quire_spec_language::model::normalize::{
    normalize, object_universe, EffectiveView, NormalizeOutcome,
};
use quire_spec_language::model::population::{
    admit_binding, all_instances, lookup, AbsenceMode, AdmissionChargePoint, AdmissionLimitKind,
    AdmissionMeter, AdmissionOutcome, AllInstancesOutcome, LookupKey, LookupOutcome,
    MemberFieldValues, PopulationAdmissionLimits, PopulationBinding, PopulationDocument,
    PopulationMember, ReferenceKey, TypedReference,
};
use quire_spec_language::value::{ChargePoint, LimitKind, Meter, ScalarLimits};

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

fn field_member_mult(
    identity: &str,
    owner: &str,
    value_type: &str,
    lower: u64,
    upper: Option<u64>,
) -> BundleRecord {
    BundleRecord::FieldMember(FieldMemberRecord {
        key: ProducerKey::fixture(identity),
        owner: ProducerKey::fixture(owner),
        value_type: ProducerKey::fixture(value_type),
        multiplicity: Multiplicity {
            lower,
            upper,
            ordered: false,
            unique: true,
        },
    })
}

fn generalization(identity: &str, specific: &str, general: &str) -> BundleRecord {
    BundleRecord::Generalization(GeneralizationRecord {
        key: ProducerKey::fixture(identity),
        specific: ProducerKey::fixture(specific),
        general: ProducerKey::fixture(general),
    })
}

fn subsetting(identity: &str, owner: &str, subsetting: &str, subsetted: &str) -> BundleRecord {
    BundleRecord::Subsetting(SubsettingRecord {
        key: ProducerKey::fixture(identity),
        owner: ProducerKey::fixture(owner),
        subsetting: ProducerKey::fixture(subsetting),
        subsetted: ProducerKey::fixture(subsetted),
    })
}

/// TC-195 F1 (bundle `bundle.n01`), imported as `M` by TC-198: types `A`,
/// `B`; field `A.x` of `A`; generalization `B -> A`.
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

/// A second, distinct bundle so its object universe genuinely differs from
/// F1's (TC-198 L04's foreign-universe key).
fn fixture_other_universe() -> Bundle {
    Bundle::new(
        ModelSelection::fixture("bundle.n02"),
        vec![object_type("model.A")],
    )
}

/// A revision-only variant of [`fixture_f1`], for the view/bundle
/// correspondence test below: same `export.identity` and, since
/// `ModelSelection::fixture` derives its digest from the identity string
/// alone, the same `export.digest` too — only `export.revision` differs.
fn fixture_f1_with_revision(revision: &str) -> Bundle {
    let mut bundle = fixture_f1();
    bundle.model_selection.export.revision = Revision::producer_object(revision);
    bundle
}

fn view_of(bundle: &Bundle) -> EffectiveView {
    match normalize(bundle, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Completed(view) => view,
        other => panic!("expected a completed effective view, got {other:?}"),
    }
}

/// The effective identity of the type-level declaration named `identity`.
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
        field_values: Vec::new(),
    }
}

/// [`member`], plus declared field values (FCD FR-121 data, not
/// expressions): each `(field, values)` pair becomes one
/// [`MemberFieldValues`] entry.
fn member_with_fields(
    object: &str,
    type_identity: &str,
    fields: Vec<(&str, Vec<&str>)>,
) -> PopulationMember {
    PopulationMember {
        field_values: fields
            .into_iter()
            .map(|(field, values)| MemberFieldValues {
                field: ProducerKey::fixture(field),
                values: values.into_iter().map(str::to_owned).collect(),
            })
            .collect(),
        ..member(object, type_identity)
    }
}

/// FCD FR-121 document **P1**: members `a1`/`a2` of `model.A`, `b1` of `model.B`.
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

fn reference_key(
    universe: &EffectiveId,
    type_identity: &EffectiveId,
    object: &str,
) -> ReferenceKey {
    ReferenceKey {
        universe: universe.clone(),
        type_identity: type_identity.clone(),
        object: object.to_owned(),
    }
}

/// TC-198 L01: closed all-instances over P1 returns every and only member of
/// the selected typed population, once, in canonical reference-key order,
/// including the subtype `B` population under `M::A` — and `b1`'s reference
/// key is identical whether queried through `M::A` or its own type `M::B`.
/// Also backs FR-153's Outputs clause directly: the result is a typed,
/// bounded [`ReferenceSet`], not a bare unbounded set — `element_type()`
/// names the queried type and `bound()` carries `[0, declared_maximum]`.
///
/// Mutation used (selection): in `all_instances`, inverted the
/// `type_conforms` match (`Ok(true) => selected.insert(...)` swapped to fire
/// on `Ok(false)`), which drove the `M::A` selection empty instead of
/// `{b1, a1, a2}` — red as expected, reverted.
///
/// Mutation used (bound): in `all_instances`, changed the returned
/// `ReferenceSet`'s `bound` from `CardinalityBound::new(0, declared_maximum)`
/// to `CardinalityBound::new(0, u64::MAX)`, which left the binding's own
/// declared maximum unreflected in the Outputs — the
/// `selected_a.bound().maximum() == 3` assertion went red as expected,
/// reverted.
#[test]
#[trace("TC-198", "FR-153-AC-1", "FR-153-AC-5", "FR-153-AC-6")]
fn l01_all_instances_selects_subtype_population_once() {
    let bundle = fixture_f1();
    let view = view_of(&bundle);
    let universe = object_universe(&bundle).unwrap().identity();
    let a = type_id(&view, "model.A");
    let b = type_id(&view, "model.B");

    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let binding = match admit_binding(
        &bundle,
        &view,
        &p1("bundle.n01"),
        GeneralizationClosure::Closed,
        Some(3),
        &mut admission,
    ) {
        AdmissionOutcome::Admitted(binding) => binding,
        other => panic!("expected an admitted binding, got {other:?}"),
    };

    let mut meter_a = Meter::new(SCALAR_UNLIMITED);
    let selected_a = match all_instances(&binding, &ProducerKey::fixture("model.A"), &mut meter_a) {
        AllInstancesOutcome::Completed(set) => set,
        other => panic!("expected a completed M::A selection, got {other:?}"),
    };
    let expected_a: BTreeSet<ReferenceKey> = [
        reference_key(&universe, &b, "b1"),
        reference_key(&universe, &a, "a1"),
        reference_key(&universe, &a, "a2"),
    ]
    .into_iter()
    .collect();
    assert_eq!(selected_a.members(), &expected_a);
    assert_eq!(
        selected_a.members().iter().collect::<Vec<_>>(),
        [
            &reference_key(&universe, &b, "b1"),
            &reference_key(&universe, &a, "a1"),
            &reference_key(&universe, &a, "a2"),
        ],
        "canonical reference-key order: every B member precedes every A member"
    );
    // Outputs clause: a typed reference to the queried type, bounded [0, 3]
    // (this binding's declared_maximum), never a bare unbounded set.
    assert_eq!(selected_a.element_type(), &ProducerKey::fixture("model.A"));
    assert_eq!(selected_a.bound().minimum(), 0);
    assert_eq!(selected_a.bound().maximum(), 3);
    assert_eq!(selected_a.len(), 3);
    assert!(!selected_a.is_empty());
    assert_eq!(meter_a.consumed(LimitKind::WorkUnits), 5);
    assert_eq!(meter_a.consumed(LimitKind::ResultUnits), 4);
    assert_eq!(meter_a.consumed(LimitKind::ValueOccurrences), 4);
    assert_eq!(
        meter_a
            .admitted_charges()
            .iter()
            .filter(|point| **point == ChargePoint::PopulationVisit)
            .count(),
        3
    );

    let mut meter_b = Meter::new(SCALAR_UNLIMITED);
    let selected_b = match all_instances(&binding, &ProducerKey::fixture("model.B"), &mut meter_b) {
        AllInstancesOutcome::Completed(set) => set,
        other => panic!("expected a completed M::B selection, got {other:?}"),
    };
    let b1_via_b = reference_key(&universe, &b, "b1");
    assert_eq!(
        selected_b.members(),
        &[b1_via_b.clone()].into_iter().collect::<BTreeSet<_>>()
    );
    assert_eq!(selected_b.element_type(), &ProducerKey::fixture("model.B"));
    assert_eq!(selected_b.bound().maximum(), 3);
    assert_eq!(meter_b.consumed(LimitKind::WorkUnits), 5);
    assert_eq!(meter_b.consumed(LimitKind::ResultUnits), 2);

    // AC-6: one reference key, identical, regardless of which type queried it.
    let b1_via_a = selected_a
        .members()
        .iter()
        .find(|key| key.object == "b1")
        .expect("b1 selected under M::A");
    assert_eq!(*b1_via_a, b1_via_b);
}

/// TC-198 L01's boundary: with `work_units: 4`, `allInstances<M::A>(p)`
/// denies exactly at `collection.result-retain`.
///
/// Mutation used: removed the `population.visit` charge from the walk loop,
/// which left only two charges (bound + retain) under `work_units: 4` — the
/// call went `Completed` instead of `Incomplete`, red as expected, reverted.
#[test]
#[trace("TC-198", "FR-153-AC-3")]
fn l01_all_instances_incomplete_at_result_retain() {
    let bundle = fixture_f1();
    let view = view_of(&bundle);
    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let binding = match admit_binding(
        &bundle,
        &view,
        &p1("bundle.n01"),
        GeneralizationClosure::Closed,
        Some(3),
        &mut admission,
    ) {
        AdmissionOutcome::Admitted(binding) => binding,
        other => panic!("expected an admitted binding, got {other:?}"),
    };

    let limits = ScalarLimits {
        work_units: 4,
        ..SCALAR_UNLIMITED
    };
    let mut meter = Meter::new(limits);
    let outcome = all_instances(&binding, &ProducerKey::fixture("model.A"), &mut meter);
    let incomplete = match outcome {
        AllInstancesOutcome::Incomplete(incomplete) => incomplete,
        other => panic!("expected an incomplete result, got {other:?}"),
    };
    assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
    assert_eq!(incomplete.limit, 4);
    assert_eq!(incomplete.consumed, 4);
    assert_eq!(
        incomplete.next_charge,
        quire_spec_language::value::Integer::from(1_u64)
    );
    assert_eq!(incomplete.charge_point, ChargePoint::CollectionResultRetain);
}

/// TC-198 L02: an unclosed object world or an open subtype closure is
/// admission's `incomplete_population` outcome, never a refusal, and never a
/// binding.
///
/// Mutation used: inverted `!document.closed_world` to `document.closed_world`
/// in `admit_binding`, so the `closedWorld: false` document fell through
/// instead of returning `UnknownClosure` — red as expected, reverted.
#[test]
#[trace("TC-198", "FR-153-AC-2")]
fn l02_unknown_closure_is_incomplete_not_refused() {
    let bundle = fixture_f1();
    let view = view_of(&bundle);

    let mut open_world = p1("bundle.n01");
    open_world.closed_world = false;
    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    match admit_binding(
        &bundle,
        &view,
        &open_world,
        GeneralizationClosure::Closed,
        Some(3),
        &mut admission,
    ) {
        AdmissionOutcome::UnknownClosure(refusal) => {
            assert_eq!(refusal.code, Code::IncompletePopulation);
            assert_eq!(refusal.cause, "incomplete-scope");
        }
        other => panic!("expected UnknownClosure(incomplete-scope), got {other:?}"),
    }
    assert!(
        admission.admitted_charges().is_empty(),
        "no charge before a binding exists"
    );

    let mut open_subtypes = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    match admit_binding(
        &bundle,
        &view,
        &p1("bundle.n01"),
        GeneralizationClosure::Open,
        Some(3),
        &mut open_subtypes,
    ) {
        AdmissionOutcome::UnknownClosure(refusal) => {
            assert_eq!(refusal.code, Code::IncompletePopulation);
            assert_eq!(refusal.cause, "unclosed-subtypes");
        }
        other => panic!("expected UnknownClosure(unclosed-subtypes), got {other:?}"),
    }
    assert!(open_subtypes.admitted_charges().is_empty());
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

/// TC-198 L03's `undefined` mode: present returns the member reference;
/// absent returns `LookupOutcome::Undefined` with no result-retain charge.
/// The present result also backs FR-153's Outputs clause: a typed
/// [`TypedReference`] naming the queried type `T`, not a bare `ReferenceKey`.
///
/// Mutation used (absence): in `lookup`, changed the `AbsenceMode::Undefined`
/// arm to return `LookupOutcome::Completed(None)` instead of `Undefined` —
/// the absent-`rc` assertion went red as expected, reverted.
///
/// Mutation used (typed result): in `lookup`, changed the present branch's
/// `TypedReference::new(t.clone(), r.key.clone())` to
/// `TypedReference::new(r.static_type.clone(), r.key.clone())`, swapping the
/// queried type `M::A` for the reference key's own static type `M::B` — the
/// `Completed(Some(TypedReference::new(ProducerKey::fixture("model.A"), ...)))`
/// assertion went red as expected, reverted.
#[test]
#[trace("TC-198", "FR-153-AC-2", "FR-153-AC-4")]
fn l03_lookup_undefined_mode() {
    let bundle = fixture_f1();
    let view = view_of(&bundle);
    let universe = object_universe(&bundle).unwrap().identity();
    let a = type_id(&view, "model.A");
    let b = type_id(&view, "model.B");
    let binding = admitted_binding(&bundle, &view, &p1("bundle.n01"));

    let rb = LookupKey {
        static_type: ProducerKey::fixture("model.B"),
        key: reference_key(&universe, &b, "b1"),
    };
    let mut meter_present = Meter::new(SCALAR_UNLIMITED);
    let present = lookup(
        &binding,
        &ProducerKey::fixture("model.A"),
        &rb,
        AbsenceMode::Undefined,
        &mut meter_present,
    );
    assert_eq!(
        present,
        LookupOutcome::Completed(Some(TypedReference::new(
            ProducerKey::fixture("model.A"),
            reference_key(&universe, &b, "b1")
        )))
    );
    assert_eq!(meter_present.consumed(LimitKind::WorkUnits), 2);
    assert_eq!(meter_present.consumed(LimitKind::ResultUnits), 1);

    // `rc` is bound to `q` (document P2), never admitted into `p`, so it is
    // absent from `p`'s binding.
    let rc = LookupKey {
        static_type: ProducerKey::fixture("model.A"),
        key: reference_key(&universe, &a, "c9"),
    };
    let mut meter_absent = Meter::new(SCALAR_UNLIMITED);
    let absent = lookup(
        &binding,
        &ProducerKey::fixture("model.A"),
        &rc,
        AbsenceMode::Undefined,
        &mut meter_absent,
    );
    assert_eq!(absent, LookupOutcome::Undefined);
    assert_eq!(meter_absent.consumed(LimitKind::WorkUnits), 1);
    assert_eq!(meter_absent.consumed(LimitKind::ResultUnits), 0);
}

/// TC-198 L03's `empty` mode: a present result retains two occurrences (the
/// `Option` wrapper plus the reference); an absent result is `none`, still
/// retaining one occurrence (the wrapper alone). The present result also
/// backs the same typed-Outputs property as `l03_lookup_undefined_mode`.
///
/// Mutation used (retain count): in `lookup`, hardcoded the present-branch
/// retain amount to `1` regardless of mode instead of `2` for `Empty` — the
/// `meter.consumed(ResultUnits) == 2` assertion went red as expected, reverted.
///
/// Mutation used (typed result): same as `l03_lookup_undefined_mode`'s typed-
/// result mutation, confirmed independently red here too, reverted.
#[test]
#[trace("TC-198", "FR-153-AC-2", "FR-153-AC-4")]
fn l03_lookup_empty_mode() {
    let bundle = fixture_f1();
    let view = view_of(&bundle);
    let universe = object_universe(&bundle).unwrap().identity();
    let a = type_id(&view, "model.A");
    let b = type_id(&view, "model.B");
    let binding = admitted_binding(&bundle, &view, &p1("bundle.n01"));

    let rb = LookupKey {
        static_type: ProducerKey::fixture("model.B"),
        key: reference_key(&universe, &b, "b1"),
    };
    let mut meter_present = Meter::new(SCALAR_UNLIMITED);
    let present = lookup(
        &binding,
        &ProducerKey::fixture("model.A"),
        &rb,
        AbsenceMode::Empty,
        &mut meter_present,
    );
    assert_eq!(
        present,
        LookupOutcome::Completed(Some(TypedReference::new(
            ProducerKey::fixture("model.A"),
            reference_key(&universe, &b, "b1")
        )))
    );
    assert_eq!(meter_present.consumed(LimitKind::WorkUnits), 2);
    assert_eq!(meter_present.consumed(LimitKind::ResultUnits), 2);

    let rc = LookupKey {
        static_type: ProducerKey::fixture("model.A"),
        key: reference_key(&universe, &a, "c9"),
    };
    let mut meter_absent = Meter::new(SCALAR_UNLIMITED);
    let absent = lookup(
        &binding,
        &ProducerKey::fixture("model.A"),
        &rc,
        AbsenceMode::Empty,
        &mut meter_absent,
    );
    assert_eq!(absent, LookupOutcome::Completed(None));
    assert_eq!(meter_absent.consumed(LimitKind::WorkUnits), 2);
    assert_eq!(meter_absent.consumed(LimitKind::ResultUnits), 1);
}

/// TC-198 L03's `refused` mode: an absent key is a typed refusal naming
/// `invalid_runtime_input`/`absent-key`, never a substitute value.
///
/// Mutation used: in `lookup`, changed the `AbsenceMode::Refused` arm's
/// `Code::InvalidRuntimeInput` to `Code::IllTyped` — the code assertion went
/// red as expected, reverted.
#[test]
#[trace("TC-198", "FR-153-AC-2", "FR-153-AC-4")]
fn l03_lookup_refused_mode() {
    let bundle = fixture_f1();
    let view = view_of(&bundle);
    let universe = object_universe(&bundle).unwrap().identity();
    let a = type_id(&view, "model.A");
    let binding = admitted_binding(&bundle, &view, &p1("bundle.n01"));

    let rc = LookupKey {
        static_type: ProducerKey::fixture("model.A"),
        key: reference_key(&universe, &a, "c9"),
    };
    let mut meter = Meter::new(SCALAR_UNLIMITED);
    let outcome = lookup(
        &binding,
        &ProducerKey::fixture("model.A"),
        &rc,
        AbsenceMode::Refused,
        &mut meter,
    );
    match outcome {
        LookupOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::InvalidRuntimeInput);
            assert_eq!(refusal.cause, "absent-key");
        }
        other => panic!("expected Refused(invalid_runtime_input/absent-key), got {other:?}"),
    }
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 1);
}

/// TC-198 L03's final vector: `lookup<M::B>(p, ra)` refuses `ill_typed`/
/// `type-mismatch` before any charge, because `M::A` does not conform to `M::B`.
///
/// Mutation used: in `lookup`, changed `Ok(false) => return Refused(...)` to
/// `Ok(false) => {}` (fall through instead of refusing) — the call then
/// charged `lookup.key` and returned a lookup result instead of refusing,
/// red as expected, reverted.
#[test]
#[trace("TC-198", "FR-153-AC-3")]
fn l03_lookup_type_mismatch_before_any_charge() {
    let bundle = fixture_f1();
    let view = view_of(&bundle);
    let universe = object_universe(&bundle).unwrap().identity();
    let a = type_id(&view, "model.A");
    let binding = admitted_binding(&bundle, &view, &p1("bundle.n01"));

    let ra = LookupKey {
        static_type: ProducerKey::fixture("model.A"),
        key: reference_key(&universe, &a, "a1"),
    };
    let mut meter = Meter::new(SCALAR_UNLIMITED);
    let outcome = lookup(
        &binding,
        &ProducerKey::fixture("model.B"),
        &ra,
        AbsenceMode::Empty,
        &mut meter,
    );
    match outcome {
        LookupOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::IllTyped);
            assert_eq!(refusal.cause, "type-mismatch");
        }
        other => panic!("expected Refused(ill_typed/type-mismatch), got {other:?}"),
    }
    assert!(
        meter.admitted_charges().is_empty(),
        "no charge before any type check completes"
    );
}

/// TC-198 L04: a reference key naming a foreign universe refuses
/// `foreign_reference`/`foreign-universe`, after `lookup.key` has already
/// charged once.
///
/// Mutation used: in `lookup`, moved the universe comparison ahead of the
/// `lookup.key` charge, so the refusal fired with zero work units consumed
/// instead of one — the `consumed(WorkUnits) == 1` assertion went red as
/// expected, reverted.
#[test]
#[trace("TC-198", "FR-153-AC-3")]
fn l04_lookup_foreign_universe_refuses() {
    let bundle = fixture_f1();
    let view = view_of(&bundle);
    let a = type_id(&view, "model.A");
    let binding = admitted_binding(&bundle, &view, &p1("bundle.n01"));

    let other = fixture_other_universe();
    let foreign_universe = object_universe(&other).unwrap().identity();
    assert_ne!(&foreign_universe, binding.universe());

    let rx = LookupKey {
        static_type: ProducerKey::fixture("model.A"),
        key: reference_key(&foreign_universe, &a, "a1"),
    };
    let mut meter = Meter::new(SCALAR_UNLIMITED);
    let outcome = lookup(
        &binding,
        &ProducerKey::fixture("model.A"),
        &rx,
        AbsenceMode::Empty,
        &mut meter,
    );
    match outcome {
        LookupOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::ForeignReference);
            assert_eq!(refusal.cause, "foreign-universe");
        }
        other => panic!("expected Refused(foreign_reference/foreign-universe), got {other:?}"),
    }
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 1);
}

/// TC-198 L05: two member records naming the same object with different
/// types refuse `invalid_runtime_input`/`conflicting-identity`, decided at
/// the fourth `binding.member` charge.
///
/// Mutation used: in `admit_binding`, changed the conflicting-identity
/// lookup's predicate from `existing.object == key.object` to `false` (never
/// match), so the conflicting second `a1` record was admitted instead of
/// refused — the `AdmissionOutcome::Refused` assertion went red as expected,
/// reverted.
#[test]
#[trace("TC-198", "FR-153-AC-3")]
fn l05_conflicting_identity_refuses_after_fourth_member_charge() {
    let bundle = fixture_f1();
    let view = view_of(&bundle);
    let mut document = p1("bundle.n01");
    document.members.push(member("a1", "model.B"));

    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let outcome = admit_binding(
        &bundle,
        &view,
        &document,
        GeneralizationClosure::Closed,
        Some(3),
        &mut admission,
    );
    match outcome {
        AdmissionOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::InvalidRuntimeInput);
            assert_eq!(refusal.cause, "conflicting-identity");
        }
        other => {
            panic!("expected Refused(invalid_runtime_input/conflicting-identity), got {other:?}")
        }
    }
    assert_eq!(
        admission
            .admitted_charges()
            .iter()
            .filter(|point| **point == AdmissionChargePoint::BindingMember)
            .count(),
        4
    );
}

/// TC-198 L05: an exact duplicate `a1` record collapses rather than
/// conflicting, still consuming a fourth `binding.member` charge, and the
/// resulting binding evaluates identically to L01's undivided P1.
///
/// Mutation used: in `admit_binding`, removed the `if admitted.contains_key
/// (&key) { continue; }` duplicate-collapse guard, which made the exact
/// duplicate fall into the conflicting-identity check (it shares an object
/// with an already-admitted entry) and wrongly refuse — the
/// `AdmissionOutcome::Admitted` assertion went red as expected, reverted.
#[test]
#[trace("TC-198", "FR-153-AC-1")]
fn l05_duplicate_collapses_and_recovers_l01() {
    let bundle = fixture_f1();
    let view = view_of(&bundle);
    let mut document = p1("bundle.n01");
    document.members.push(member("a1", "model.A"));

    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let binding = match admit_binding(
        &bundle,
        &view,
        &document,
        GeneralizationClosure::Closed,
        Some(3),
        &mut admission,
    ) {
        AdmissionOutcome::Admitted(binding) => binding,
        other => panic!("expected an admitted binding, got {other:?}"),
    };
    assert_eq!(
        binding.members().len(),
        3,
        "the exact duplicate collapses, not a fourth member"
    );
    assert_eq!(admission.consumed(AdmissionLimitKind::PopulationMembers), 4);
    assert_eq!(admission.consumed(AdmissionLimitKind::WorkUnits), 4);

    let mut meter = Meter::new(SCALAR_UNLIMITED);
    let selected = match all_instances(&binding, &ProducerKey::fixture("model.A"), &mut meter) {
        AllInstancesOutcome::Completed(set) => set,
        other => panic!("expected a completed selection, got {other:?}"),
    };
    assert_eq!(selected.len(), 3);
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 5);
    assert_eq!(meter.consumed(LimitKind::ResultUnits), 4);
}

/// TC-198 L05: a member naming a type absent from the effective view refuses
/// `foreign_reference`/`foreign-type`.
///
/// Mutation used: in `admit_binding`, changed the effective-type lookup's
/// predicate to ignore `entry.preimage.original == member.type_identity`
/// (always matching the first type-level entry found), so the foreign
/// `model.Z` member no longer refused — the `AdmissionOutcome::Refused`
/// assertion went red as expected, reverted.
#[test]
#[trace("TC-198", "FR-153-AC-3")]
fn l05_foreign_type_refuses() {
    let bundle = fixture_f1();
    let view = view_of(&bundle);
    let mut document = p1("bundle.n01");
    document.members.push(member("z1", "model.Z"));

    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let outcome = admit_binding(
        &bundle,
        &view,
        &document,
        GeneralizationClosure::Closed,
        Some(3),
        &mut admission,
    );
    match outcome {
        AdmissionOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::ForeignReference);
            assert_eq!(refusal.cause, "foreign-type");
        }
        other => panic!("expected Refused(foreign_reference/foreign-type), got {other:?}"),
    }
}

/// TC-198 L06: a selected count above the declared maximum refuses
/// `cardinality_out_of_bound`/`above-maximum` after `collection.bound`, with
/// no result-retain charge; a `work_units: 3` ceiling instead denies exactly
/// at `collection.bound`.
///
/// Mutation used: in `all_instances`, removed the `if n > declared_maximum`
/// check, letting the over-bound selection proceed to `Completed` instead of
/// refusing — the `AllInstancesOutcome::Refused` assertion went red as
/// expected, reverted.
#[test]
#[trace("TC-198", "FR-153-AC-3")]
fn l06_cardinality_bound_and_incomplete() {
    let bundle = fixture_f1();
    let view = view_of(&bundle);
    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let binding = match admit_binding(
        &bundle,
        &view,
        &p1("bundle.n01"),
        GeneralizationClosure::Closed,
        Some(2),
        &mut admission,
    ) {
        AdmissionOutcome::Admitted(binding) => binding,
        other => panic!("expected an admitted binding, got {other:?}"),
    };

    let mut meter = Meter::new(SCALAR_UNLIMITED);
    let outcome = all_instances(&binding, &ProducerKey::fixture("model.A"), &mut meter);
    match outcome {
        AllInstancesOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::CardinalityOutOfBound);
            assert_eq!(refusal.cause, "above-maximum");
        }
        other => panic!("expected Refused(cardinality_out_of_bound/above-maximum), got {other:?}"),
    }
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 4);
    assert_eq!(meter.consumed(LimitKind::ResultUnits), 0);

    let limits = ScalarLimits {
        work_units: 3,
        ..SCALAR_UNLIMITED
    };
    let mut meter_incomplete = Meter::new(limits);
    let bounded = match admit_binding(
        &bundle,
        &view,
        &p1("bundle.n01"),
        GeneralizationClosure::Closed,
        Some(3),
        &mut AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED),
    ) {
        AdmissionOutcome::Admitted(binding) => binding,
        other => panic!("expected an admitted binding, got {other:?}"),
    };
    let incomplete_outcome = all_instances(
        &bounded,
        &ProducerKey::fixture("model.A"),
        &mut meter_incomplete,
    );
    match incomplete_outcome {
        AllInstancesOutcome::Incomplete(incomplete) => {
            assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
            assert_eq!(incomplete.limit, 3);
            assert_eq!(incomplete.consumed, 3);
            assert_eq!(incomplete.charge_point, ChargePoint::CollectionBound);
        }
        other => panic!("expected Incomplete at collection.bound, got {other:?}"),
    }
}

// Round-3 review finding (PR #148): the prior fix cycle's
// `require_bundle_matches` compared `all_instances`/`lookup`'s separate
// `bundle` argument against the binding's admitted `ModelSelection` header
// (identity, revision, digest), rather than against the binding's own
// admitted `records`. Nothing in this crate verifies `export.digest`
// against a bundle's actual `records` anywhere, and the test-only
// `ModelSelection::fixture` derives that digest from the identity string
// alone — so a caller could present a bundle with the exact admitted header
// but different `records` (e.g. F1 with its `B -> A` generalization record
// removed) and the header check would pass while `all_instances`/`lookup`
// silently answered against the wrong content (`b1` dropping out of
// `allInstances<M::A>(p)` instead of the query refusing).
//
// This is now unrepresentable, not merely refused: `all_instances` and
// `lookup` no longer take a `bundle` argument at all (see the module docs'
// "Binding/bundle correspondence"), so there is no way for a caller to
// evaluate a query against anything other than the exact bundle
// `admit_binding` admitted. The prior fix cycle's `fixture_f1_with_
// foreign_digest` fixture and its query-time test are deleted along with
// `require_bundle_matches` itself, rather than adapted — there is no longer
// a second bundle parameter for either one to exercise. The reachable
// sibling of this defect class — a `LookupKey` naming a foreign universe,
// independent of which bundle is bound — stays covered by
// `l04_lookup_foreign_universe_refuses` above, whose `LookupKey.key.universe`
// field a caller can still set to anything regardless of the bound bundle.

/// TC-198's own admission-time `modelIdentity` check: a population document
/// naming a `modelIdentity` other than the admitting bundle's own refuses
/// before any `binding.member` charge. Every other test in this file admits
/// `p1("bundle.n01")` against `fixture_f1()`, whose own `modelIdentity` is
/// also `bundle.n01`, so this path was untested.
///
/// Mutation used: in `admit_binding`, changed the `modelIdentity` guard's
/// condition from `document.model_identity != bundle.model_selection.export.
/// identity` to `false` (never fires), which let the mismatched document
/// proceed to `AdmissionOutcome::Admitted` instead of refusing — the
/// `Refused(foreign_reference/foreign-model-selection)` assertion went red
/// as expected, reverted.
#[test]
#[trace("TC-198", "FR-153-AC-3")]
fn l05_foreign_model_selection_refuses_at_admission() {
    let bundle = fixture_f1();
    let view = view_of(&bundle);
    let mismatched = p1("bundle.other");

    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let outcome = admit_binding(
        &bundle,
        &view,
        &mismatched,
        GeneralizationClosure::Closed,
        Some(3),
        &mut admission,
    );
    match outcome {
        AdmissionOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::ForeignReference);
            assert_eq!(refusal.cause, "foreign-model-selection");
        }
        other => {
            panic!("expected Refused(foreign_reference/foreign-model-selection), got {other:?}")
        }
    }
    assert!(admission.admitted_charges().is_empty());
}

/// Round-3 review finding (PR #148): `admit_binding` takes `view` and
/// `bundle` separately and, before this test, never checked that they
/// correspond — the same mismatch class removed from `all_instances`/
/// `lookup` themselves, just moved one level up. Pins the check with a
/// revision-only divergence (same `export.identity` and `export.digest` as
/// `fixture_f1()` — `ModelSelection::fixture` derives the digest from the
/// identity string alone, so an identity-only comparison would miss this)
/// to prove the check compares the full `ModelSelection` header, not just
/// `export.identity`.
///
/// Mutation used: narrowed the check from `view.model_selection !=
/// bundle.model_selection` to `view.model_selection.export.identity !=
/// bundle.model_selection.export.identity`, which let this revision-only
/// mismatch admit instead of refusing — red as expected, reverted.
#[test]
#[trace("TC-198", "FR-153-AC-3")]
fn l05_view_from_a_different_bundle_revision_refuses_at_admission() {
    let view = view_of(&fixture_f1_with_revision("1"));
    let bundle = fixture_f1_with_revision("2");

    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let outcome = admit_binding(
        &bundle,
        &view,
        &p1("bundle.n01"),
        GeneralizationClosure::Closed,
        Some(3),
        &mut admission,
    );
    match outcome {
        AdmissionOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::ForeignReference);
            assert_eq!(refusal.cause, "foreign-model-selection");
        }
        other => {
            panic!("expected Refused(foreign_reference/foreign-model-selection), got {other:?}")
        }
    }
    assert!(admission.admitted_charges().is_empty());
}

/// TC-198 AC-3's "a denied charge is incomplete", `population_members`
/// dimension: every prior test in this file left both
/// `PopulationAdmissionLimits` counters `UNLIMITED`, so this dimension was
/// never actually driven to denial. P1's third member (`b1`) charges
/// `binding.member` sized `3` (the running high-water member count);
/// `population_members: 2` admits the first two charges (sizes `1`, `2`)
/// and denies the third.
///
/// Mutation used: in `AdmissionMeter::charge`, changed `amount >
/// kind.limit(&self.limits)` to `false` (the `population_members` check
/// never fires), which let the third `b1` charge succeed and the whole
/// document admit instead of denying — the `AdmissionOutcome::Incomplete`
/// assertion went red as expected, reverted.
#[test]
#[trace("TC-198", "FR-153-AC-3")]
fn l02_population_members_limit_denies_the_third_member_charge() {
    let bundle = fixture_f1();
    let view = view_of(&bundle);
    let limits = PopulationAdmissionLimits {
        population_members: 2,
        ..PopulationAdmissionLimits::UNLIMITED
    };
    let mut admission = AdmissionMeter::new(limits);
    let outcome = admit_binding(
        &bundle,
        &view,
        &p1("bundle.n01"),
        GeneralizationClosure::Closed,
        Some(3),
        &mut admission,
    );
    match outcome {
        AdmissionOutcome::Incomplete(incomplete) => {
            assert_eq!(incomplete.limit_kind, AdmissionLimitKind::PopulationMembers);
            assert_eq!(incomplete.limit, 2);
            assert_eq!(incomplete.consumed, 2);
            assert_eq!(incomplete.next_charge, 3);
            assert_eq!(incomplete.charge_point, AdmissionChargePoint::BindingMember);
        }
        other => panic!(
            "expected Incomplete(population_members, limit 2, consumed 2, next 3), got {other:?}"
        ),
    }
    assert_eq!(
        admission
            .admitted_charges()
            .iter()
            .filter(|point| **point == AdmissionChargePoint::BindingMember)
            .count(),
        2
    );
}

/// TC-198 AC-3's "a denied charge is incomplete", `work_units` dimension:
/// each `binding.member` charge costs exactly one work unit regardless of
/// its `population_members` size, so `work_units: 2` admits P1's first two
/// members and denies the third on work units alone (its
/// `population_members` size, `3`, is within an otherwise-`UNLIMITED`
/// counter).
///
/// Mutation used: in `AdmissionMeter::charge`, changed
/// `total <= self.limits.work_units` to `true` (the `work_units` check
/// never denies), which let the third `b1` charge succeed and the whole
/// document admit instead of denying — the `AdmissionOutcome::Incomplete`
/// assertion went red as expected, reverted.
#[test]
#[trace("TC-198", "FR-153-AC-3")]
fn l02_work_units_limit_denies_the_third_member_charge() {
    let bundle = fixture_f1();
    let view = view_of(&bundle);
    let limits = PopulationAdmissionLimits {
        work_units: 2,
        ..PopulationAdmissionLimits::UNLIMITED
    };
    let mut admission = AdmissionMeter::new(limits);
    let outcome = admit_binding(
        &bundle,
        &view,
        &p1("bundle.n01"),
        GeneralizationClosure::Closed,
        Some(3),
        &mut admission,
    );
    match outcome {
        AdmissionOutcome::Incomplete(incomplete) => {
            assert_eq!(incomplete.limit_kind, AdmissionLimitKind::WorkUnits);
            assert_eq!(incomplete.limit, 2);
            assert_eq!(incomplete.consumed, 2);
            assert_eq!(incomplete.next_charge, 1);
            assert_eq!(incomplete.charge_point, AdmissionChargePoint::BindingMember);
        }
        other => {
            panic!("expected Incomplete(work_units, limit 2, consumed 2, next 1), got {other:?}")
        }
    }
    assert_eq!(
        admission
            .admitted_charges()
            .iter()
            .filter(|point| **point == AdmissionChargePoint::BindingMember)
            .count(),
        2
    );
}

// L07 (`pre(allInstances<M::A>(p))` reading pre-population state under a
// postcondition) is deleted here, not retagged: FR-153-AC-7 needs a real
// pre/post operation-effect evaluator this crate does not have (the
// two-independent-bindings model above only exercised L01's selection-count
// and L03's Empty-mode lookup behavior over two separately admitted
// documents, never an actual `pre()` anchor over one operation's effect).
// It backed no AC uniquely; FR-153-AC-7 stays unbacked here and is tracked
// on QSL #147.

/// Review finding (PR #148): every other `Completed` test in this file admits
/// P1 (3 members: `a1`, `a2`, `b1`) with `declared_maximum: Some(3)`, so a
/// hard-coded `3` and P1's own member count are indistinguishable from the
/// binding's actual declared maximum — both give the right answer for the
/// wrong reason. This admits P1 the same way but with `declared_maximum:
/// Some(5)`, a value equal to neither the constant `3` nor `binding.members()
/// .len()` (still 3), so `bound().maximum()` can only be `5` if
/// `all_instances` is genuinely reading the binding's own declared maximum.
///
/// Mutation used (constant): in `all_instances`, changed
/// `CardinalityBound::new(0, declared_maximum)` to
/// `CardinalityBound::new(0, 3)` — `selected_a.bound().maximum() == 5` went
/// red as expected (got `3`), reverted.
///
/// Mutation used (member count): in `all_instances`, changed
/// `CardinalityBound::new(0, declared_maximum)` to
/// `CardinalityBound::new(0, length_amount(binding.members().len()))` —
/// `selected_a.bound().maximum() == 5` went red as expected (got `3`),
/// reverted.
#[test]
#[trace("TC-198", "FR-153-AC-1", "FR-153-AC-5")]
fn l08_bound_reflects_declared_maximum_not_member_count_or_a_constant() {
    let bundle = fixture_f1();
    let view = view_of(&bundle);

    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let binding = match admit_binding(
        &bundle,
        &view,
        &p1("bundle.n01"),
        GeneralizationClosure::Closed,
        Some(5),
        &mut admission,
    ) {
        AdmissionOutcome::Admitted(binding) => binding,
        other => panic!("expected an admitted binding, got {other:?}"),
    };
    assert_eq!(binding.declared_maximum(), Some(5));
    assert_eq!(binding.members().len(), 3);

    let mut meter_a = Meter::new(SCALAR_UNLIMITED);
    let selected_a = match all_instances(&binding, &ProducerKey::fixture("model.A"), &mut meter_a) {
        AllInstancesOutcome::Completed(set) => set,
        other => panic!("expected a completed M::A selection, got {other:?}"),
    };
    assert_eq!(selected_a.len(), 3);
    assert_eq!(selected_a.bound().minimum(), 0);
    assert_eq!(
        selected_a.bound().maximum(),
        5,
        "bound().maximum() must be the binding's own declared maximum (5), \
         not the constant 3 or the 3-member selection count"
    );
}

/// TC-196 R06's admission-binding half: types `A`, `B <= A`; field
/// `model.A.all` typed `model.A` `{0,5}`; field `model.A.some` typed
/// `model.B` `{0,3}` with a subsetting record (`A`, `A.some` subsets
/// `A.all`); a closed population where `a1.all = [a2]` and `a1.some = [a3]`.
/// `a3` is not among `a1.all`'s values, so binding admission refuses
/// `invalid_runtime_input`/`subsetting-violation` naming the record, `a1` and
/// `a3`, after three `binding.member` charges and one `binding.subset-value`
/// charge for `a1`'s one `some` value (`n = 1`): four admission work units.
/// The type-level axes (multiplicity-narrowing, subsetting-type, admitted)
/// are already covered by `model_conformance.rs`'s
/// `r06_subsetting_type_and_multiplicity_axes`; this backs only the runtime
/// `binding.subset-value` charge and refusal FR-151-AC-10 adds.
fn r06_bundle() -> Bundle {
    Bundle::new(
        ModelSelection::fixture("bundle.r06pop"),
        vec![
            object_type("model.A"),
            object_type("model.B"),
            generalization("model.gen.B-A", "model.B", "model.A"),
            field_member_mult("model.A.all", "model.A", "model.A", 0, Some(5)),
            field_member_mult("model.A.some", "model.A", "model.B", 0, Some(3)),
            subsetting(
                "model.subset.some-all",
                "model.A",
                "model.A.some",
                "model.A.all",
            ),
        ],
    )
}

#[test]
#[trace("TC-196", "FR-151-AC-10")]
fn r06_subsetting_violation_refuses_after_the_charged_subset_value() {
    let bundle = r06_bundle();
    let view = view_of(&bundle);
    let document = PopulationDocument {
        closed_world: true,
        model_identity: "bundle.r06pop".to_owned(),
        members: vec![
            member_with_fields(
                "a1",
                "model.A",
                vec![("model.A.all", vec!["a2"]), ("model.A.some", vec!["a3"])],
            ),
            member("a2", "model.A"),
            member("a3", "model.A"),
        ],
    };

    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let outcome = admit_binding(
        &bundle,
        &view,
        &document,
        GeneralizationClosure::Closed,
        Some(3),
        &mut admission,
    );
    match outcome {
        AdmissionOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::InvalidRuntimeInput);
            assert_eq!(refusal.cause, "subsetting-violation");
            assert!(refusal.detail.contains("a1"));
            assert!(refusal.detail.contains("a3"));
            assert!(refusal.detail.contains("model.A.some"));
            assert!(refusal.detail.contains("model.A.all"));
            assert!(
                refusal.detail.contains("model.subset.some-all"),
                "the detail must name the subsetting record, got: {}",
                refusal.detail
            );
        }
        other => {
            panic!("expected Refused(invalid_runtime_input/subsetting-violation), got {other:?}")
        }
    }
    assert_eq!(
        admission
            .admitted_charges()
            .iter()
            .filter(|point| **point == AdmissionChargePoint::BindingMember)
            .count(),
        3
    );
    assert_eq!(
        admission
            .admitted_charges()
            .iter()
            .filter(|point| **point == AdmissionChargePoint::BindingSubsetValue)
            .count(),
        1
    );
    assert_eq!(admission.consumed(AdmissionLimitKind::WorkUnits), 4);
}

/// R06's satisfying counterpart: `a1.some = [a2]`, and `a2` is among
/// `a1.all`'s one value, so the same one `binding.subset-value` charge
/// (`n = 1`) discharges instead of refusing, and admission completes.
#[test]
#[trace("TC-196", "FR-151-AC-10")]
fn r06_subsetting_satisfied_admits_with_the_charged_subset_value() {
    let bundle = r06_bundle();
    let view = view_of(&bundle);
    let document = PopulationDocument {
        closed_world: true,
        model_identity: "bundle.r06pop".to_owned(),
        members: vec![
            member_with_fields(
                "a1",
                "model.A",
                vec![("model.A.all", vec!["a2"]), ("model.A.some", vec!["a2"])],
            ),
            member("a2", "model.A"),
        ],
    };

    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let outcome = admit_binding(
        &bundle,
        &view,
        &document,
        GeneralizationClosure::Closed,
        Some(3),
        &mut admission,
    );
    match outcome {
        AdmissionOutcome::Admitted(binding) => assert_eq!(binding.members().len(), 2),
        other => panic!("expected an admitted binding, got {other:?}"),
    }
    assert_eq!(
        admission
            .admitted_charges()
            .iter()
            .filter(|point| **point == AdmissionChargePoint::BindingSubsetValue)
            .count(),
        1
    );
    assert_eq!(admission.consumed(AdmissionLimitKind::WorkUnits), 3);
}

/// A member declaring the same field twice in its own `field_values` must
/// refuse rather than silently keep only the first entry: `values_of`'s
/// `find` would otherwise discard the second `model.A.all` entry with no
/// signal at all, so [`admit_binding`] refuses `invalid_runtime_input`/
/// `duplicate-member` (FR-272's closed cause; there is no dedicated
/// duplicate-field variant) naming the object and the duplicated field.
#[test]
#[trace("TC-196", "FR-151-AC-10")]
fn r06_duplicate_field_values_refuse_rather_than_silently_keep_the_first() {
    let bundle = r06_bundle();
    let view = view_of(&bundle);
    let document = PopulationDocument {
        closed_world: true,
        model_identity: "bundle.r06pop".to_owned(),
        members: vec![
            member_with_fields(
                "a1",
                "model.A",
                vec![
                    ("model.A.all", vec!["a2"]),
                    ("model.A.all", vec!["a9"]),
                    ("model.A.some", vec!["a2"]),
                ],
            ),
            member("a2", "model.A"),
        ],
    };

    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let outcome = admit_binding(
        &bundle,
        &view,
        &document,
        GeneralizationClosure::Closed,
        Some(3),
        &mut admission,
    );
    match outcome {
        AdmissionOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::InvalidRuntimeInput);
            assert_eq!(refusal.cause, "duplicate-member");
            assert!(refusal.detail.contains("a1"));
            assert!(refusal.detail.contains("model.A.all"));
        }
        other => {
            panic!("expected Refused(invalid_runtime_input/duplicate-member), got {other:?}")
        }
    }
}
