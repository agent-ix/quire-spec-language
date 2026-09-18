// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-198: closed model lookup (FR-153).
//!
//! Fixtures reuse TC-195 F1 (`model.A`, `model.B`, generalization `B -> A`)
//! exactly as TC-198 imports it as `M`, and population documents P1/P2 as
//! TC-198 states them. L08 (FR-149 reference equality/upcast) and L10/L11
//! (`reaches`/`deref`, FR-043's graph navigation) are out of this rung's
//! scope; see the crate's `src/model/population.rs` module docs.

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use quire_spec_language::diagnostic::Code;
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
    admit_binding, all_instances, lookup, AbsenceMode, AdmissionMeter, AdmissionOutcome,
    AllInstancesOutcome, LookupKey, LookupOutcome, PopulationAdmissionLimits, PopulationDocument,
    PopulationMember, ReferenceKey,
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

fn generalization(identity: &str, specific: &str, general: &str) -> BundleRecord {
    BundleRecord::Generalization(GeneralizationRecord {
        key: ProducerKey::fixture(identity),
        specific: ProducerKey::fixture(specific),
        general: ProducerKey::fixture(general),
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
///
/// Mutation used: in `all_instances`, inverted the `type_conforms` match
/// (`Ok(true) => selected.insert(...)` swapped to fire on `Ok(false)`),
/// which drove the `M::A` selection empty instead of `{b1, a1, a2}` — red as
/// expected, reverted.
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
    let selected_a = match all_instances(
        &bundle,
        &binding,
        &ProducerKey::fixture("model.A"),
        &mut meter_a,
    ) {
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
    assert_eq!(selected_a, expected_a);
    assert_eq!(
        selected_a.iter().collect::<Vec<_>>(),
        [
            &reference_key(&universe, &b, "b1"),
            &reference_key(&universe, &a, "a1"),
            &reference_key(&universe, &a, "a2"),
        ],
        "canonical reference-key order: every B member precedes every A member"
    );
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
    let selected_b = match all_instances(
        &bundle,
        &binding,
        &ProducerKey::fixture("model.B"),
        &mut meter_b,
    ) {
        AllInstancesOutcome::Completed(set) => set,
        other => panic!("expected a completed M::B selection, got {other:?}"),
    };
    let b1_via_b = reference_key(&universe, &b, "b1");
    assert_eq!(
        selected_b,
        [b1_via_b.clone()].into_iter().collect::<BTreeSet<_>>()
    );
    assert_eq!(meter_b.consumed(LimitKind::WorkUnits), 5);
    assert_eq!(meter_b.consumed(LimitKind::ResultUnits), 2);

    // AC-6: one reference key, identical, regardless of which type queried it.
    let b1_via_a = selected_a
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
    let outcome = all_instances(
        &bundle,
        &binding,
        &ProducerKey::fixture("model.A"),
        &mut meter,
    );
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
) -> quire_spec_language::model::population::PopulationBinding {
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
///
/// Mutation used: in `lookup`, changed the `AbsenceMode::Undefined` arm to
/// return `LookupOutcome::Completed(None)` instead of `Undefined` — the
/// absent-`rc` assertion went red as expected, reverted.
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
        &bundle,
        &binding,
        &ProducerKey::fixture("model.A"),
        &rb,
        AbsenceMode::Undefined,
        &mut meter_present,
    );
    assert_eq!(
        present,
        LookupOutcome::Completed(Some(reference_key(&universe, &b, "b1")))
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
        &bundle,
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
/// retaining one occurrence (the wrapper alone).
///
/// Mutation used: in `lookup`, hardcoded the present-branch retain amount to
/// `1` regardless of mode instead of `2` for `Empty` — the
/// `meter.consumed(ResultUnits) == 2` assertion went red as expected, reverted.
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
        &bundle,
        &binding,
        &ProducerKey::fixture("model.A"),
        &rb,
        AbsenceMode::Empty,
        &mut meter_present,
    );
    assert_eq!(
        present,
        LookupOutcome::Completed(Some(reference_key(&universe, &b, "b1")))
    );
    assert_eq!(meter_present.consumed(LimitKind::WorkUnits), 2);
    assert_eq!(meter_present.consumed(LimitKind::ResultUnits), 2);

    let rc = LookupKey {
        static_type: ProducerKey::fixture("model.A"),
        key: reference_key(&universe, &a, "c9"),
    };
    let mut meter_absent = Meter::new(SCALAR_UNLIMITED);
    let absent = lookup(
        &bundle,
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
        &bundle,
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
        &bundle,
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
    assert_ne!(foreign_universe, binding.universe);

    let rx = LookupKey {
        static_type: ProducerKey::fixture("model.A"),
        key: reference_key(&foreign_universe, &a, "a1"),
    };
    let mut meter = Meter::new(SCALAR_UNLIMITED);
    let outcome = lookup(
        &bundle,
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
            .filter(|point| **point
                == quire_spec_language::model::population::AdmissionChargePoint::BindingMember)
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
        binding.members.len(),
        3,
        "the exact duplicate collapses, not a fourth member"
    );
    assert_eq!(
        admission.consumed(
            quire_spec_language::model::population::AdmissionLimitKind::PopulationMembers
        ),
        4
    );
    assert_eq!(
        admission.consumed(quire_spec_language::model::population::AdmissionLimitKind::WorkUnits),
        4
    );

    let mut meter = Meter::new(SCALAR_UNLIMITED);
    let selected = match all_instances(
        &bundle,
        &binding,
        &ProducerKey::fixture("model.A"),
        &mut meter,
    ) {
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
    let outcome = all_instances(
        &bundle,
        &binding,
        &ProducerKey::fixture("model.A"),
        &mut meter,
    );
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
        &bundle,
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

/// TC-198 L07: `pre(allInstances<M::A>(p))` in a postcondition reads the pre
/// population, and a deleted object's pre `lookup` still reports it present
/// with its pre type — modeled here as two independently admitted bindings
/// over pre/post population documents, since this rung carries no operation-
/// effect execution.
///
/// Mutation used: in `lookup`, changed the presence test
/// `binding.members.contains_key(&r.key)` to always `true`, which made the
/// post-population lookup for the deleted `a2` report present instead of
/// `none` — the `LookupOutcome::Completed(None)` assertion went red as
/// expected, reverted.
#[test]
#[trace("TC-198", "FR-153-AC-7")]
fn l07_postcondition_pre_population_reads_pre_state() {
    let bundle = fixture_f1();
    let view = view_of(&bundle);
    let universe = object_universe(&bundle).unwrap().identity();
    let a = type_id(&view, "model.A");

    let pre = p1("bundle.n01");
    let mut post = p1("bundle.n01");
    post.members.retain(|member| member.object != "a2");

    let pre_binding = admitted_binding(&bundle, &view, &pre);
    let post_binding = admitted_binding(&bundle, &view, &post);

    let mut meter_pre = Meter::new(SCALAR_UNLIMITED);
    let pre_selection = match all_instances(
        &bundle,
        &pre_binding,
        &ProducerKey::fixture("model.A"),
        &mut meter_pre,
    ) {
        AllInstancesOutcome::Completed(set) => set,
        other => panic!("expected a completed pre selection, got {other:?}"),
    };
    assert_eq!(
        pre_selection.len(),
        3,
        "pre(allInstances) still includes the deleted a2"
    );

    let mut meter_post = Meter::new(SCALAR_UNLIMITED);
    let post_selection = match all_instances(
        &bundle,
        &post_binding,
        &ProducerKey::fixture("model.A"),
        &mut meter_post,
    ) {
        AllInstancesOutcome::Completed(set) => set,
        other => panic!("expected a completed post selection, got {other:?}"),
    };
    assert_eq!(
        post_selection.len(),
        2,
        "post allInstances excludes the deleted a2"
    );

    let r2 = LookupKey {
        static_type: ProducerKey::fixture("model.A"),
        key: reference_key(&universe, &a, "a2"),
    };
    let mut meter_post_lookup = Meter::new(SCALAR_UNLIMITED);
    let post_lookup = lookup(
        &bundle,
        &post_binding,
        &ProducerKey::fixture("model.A"),
        &r2,
        AbsenceMode::Empty,
        &mut meter_post_lookup,
    );
    assert_eq!(post_lookup, LookupOutcome::Completed(None));

    let mut meter_pre_lookup = Meter::new(SCALAR_UNLIMITED);
    let pre_lookup = lookup(
        &bundle,
        &pre_binding,
        &ProducerKey::fixture("model.A"),
        &r2,
        AbsenceMode::Empty,
        &mut meter_pre_lookup,
    );
    assert_eq!(
        pre_lookup,
        LookupOutcome::Completed(Some(reference_key(&universe, &a, "a2")))
    );
}
