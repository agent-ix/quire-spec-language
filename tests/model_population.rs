// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-198: closed model lookup (FR-153).
//!
//! Fixtures reuse TC-195 F1 (`model.A`, `model.B`, generalization `B -> A`)
//! exactly as TC-198 imports it as `M`, and population documents P1/P2 as
//! TC-198 states them. L07 (`pre(allInstances<T>(p))` in a postcondition,
//! `admit_invocation`'s created/deleted-identity and operation-frame
//! enforcement) is backed at this model layer below (the
//! `l07_invocation_*` tests); the source-expression form of the same
//! scenario (`Expression::Pre`, the real `Typer`/`Machine`) is
//! `tests/model_reference_queries.rs`'s own `l07_pre_*` tests. L08 (FR-149
//! reference equality/upcast) and L10/L11 (`reaches`/`deref`, FR-043's graph
//! navigation) stay out of this rung's scope.

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use quire_spec_language::diagnostic::Code;
use quire_spec_language::model::accounting::ModelNormalizationLimits;
use quire_spec_language::model::dispatch::GeneralizationClosure;
use quire_spec_language::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, Extent, FieldMemberRecord, Multiplicity,
    ObjectTypeRecord, OperationEffect, PopulationRecord, RedefinitionRecord, SubsettingRecord,
    SupertypeRecord,
};
use quire_spec_language::model::key::{DeclarationKey, EffectiveId};
use quire_spec_language::model::normalize::{
    normalize, object_universe, EffectiveView, ModelRefusal, ModelRefusalCause, NormalizeOutcome,
    OfferedSelection,
};
use quire_spec_language::model::population::{
    admit_binding, admit_invocation, all_instances, lookup, AbsenceMode, AdmissionChargePoint,
    AdmissionLimitKind, AdmissionMeter, AdmissionOutcome, AllInstancesOutcome, InvocationContext,
    InvocationDelta, LookupKey, LookupOutcome, MemberFieldValues, PopulationAdmissionLimits,
    PopulationBinding, PopulationDocument, PopulationMember, ReferenceKey, TypedReference,
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

fn object_type(identity: &str) -> DomainPackageRecord {
    DomainPackageRecord::ObjectType(ObjectTypeRecord {
        key: DeclarationKey::fixture(identity),
        interface_features: None,
        abstract_type: false,
    })
}

fn field_member(identity: &str, owner: &str, value_type: &str) -> DomainPackageRecord {
    DomainPackageRecord::FieldMember(FieldMemberRecord {
        key: DeclarationKey::fixture(identity),
        owner: DeclarationKey::fixture(owner),
        value_type: DeclarationKey::fixture(value_type),
        multiplicity: MULTIPLICITY_0_1,
    })
}

fn field_member_mult(
    identity: &str,
    owner: &str,
    value_type: &str,
    lower: u64,
    upper: Option<u64>,
) -> DomainPackageRecord {
    DomainPackageRecord::FieldMember(FieldMemberRecord {
        key: DeclarationKey::fixture(identity),
        owner: DeclarationKey::fixture(owner),
        value_type: DeclarationKey::fixture(value_type),
        multiplicity: Multiplicity {
            lower,
            upper,
            ordered: false,
            unique: true,
        },
    })
}

fn supertype(identity: &str, specific: &str, general: &str) -> DomainPackageRecord {
    DomainPackageRecord::Supertype(SupertypeRecord {
        key: DeclarationKey::fixture(identity),
        specific: DeclarationKey::fixture(specific),
        general: DeclarationKey::fixture(general),
    })
}

fn subsetting(
    identity: &str,
    owner: &str,
    subsetting: &str,
    subsetted: &str,
) -> DomainPackageRecord {
    DomainPackageRecord::Subsetting(SubsettingRecord {
        key: DeclarationKey::fixture(identity),
        owner: DeclarationKey::fixture(owner),
        subsetting: DeclarationKey::fixture(subsetting),
        subsetted: DeclarationKey::fixture(subsetted),
    })
}

fn redefinition(
    identity: &str,
    owner: &str,
    redefining: &str,
    redefined: &str,
) -> DomainPackageRecord {
    DomainPackageRecord::Redefinition(RedefinitionRecord {
        key: DeclarationKey::fixture(identity),
        owner: DeclarationKey::fixture(owner),
        redefining: DeclarationKey::fixture(redefining),
        redefined: DeclarationKey::fixture(redefined),
    })
}

/// A population declaration record naming `member_types`, at the given
/// `extent`. `admit_binding` resolves this record from the admitting domain
/// package's own `records` by key (FR-153's "Its declaration key must belong
/// to the binding's ModelSelection") and reads only `extent` from it (object
/// closure holds exactly when it is [`Extent::Closed`], FR-153:68); the
/// foreign-type check below still reads the whole effective view, not
/// `member_types`, so `member_types` here need not enumerate every type a
/// fixture document actually uses.
fn population_record(identity: &str, member_types: &[&str], extent: Extent) -> DomainPackageRecord {
    DomainPackageRecord::Population(PopulationRecord {
        key: DeclarationKey::fixture(identity),
        member_types: member_types
            .iter()
            .map(|type_name| DeclarationKey::fixture(*type_name))
            .collect(),
        extent,
    })
}

/// The identity of the closed population declaration backing every
/// P1/P2-shaped fixture document in this file ([`fixture_f1`]'s own
/// `Population` record).
const P1_POPULATION: &str = "model.pop.p1";

/// [`P1_POPULATION`]'s own declaration key, the argument every
/// [`admit_binding`]/[`invocation_context`] call in this file passes.
fn p1_population_key() -> DeclarationKey {
    DeclarationKey::fixture(P1_POPULATION)
}

/// TC-195 F1 (domain package `bundle.n01`), imported as `M` by TC-198: types
/// `A`, `B`; field `A.x` of `A`; generalization `B -> A`; plus its own FR-153
/// population declaration [`P1_POPULATION`] (member types `A`, `B`), at the
/// given `extent`.
fn fixture_f1_with_extent(extent: Extent) -> DomainPackage {
    DomainPackage::new(
        DomainPackageRef::fixture("bundle.n01"),
        vec![
            object_type("model.A"),
            object_type("model.B"),
            field_member("model.A.x", "model.A", "model.A"),
            supertype("model.gen.B-A", "model.B", "model.A"),
            population_record(P1_POPULATION, &["model.A", "model.B"], extent),
        ],
    )
}

/// [`fixture_f1_with_extent`] at [`Extent::Closed`], the extent every test in
/// this file other than `l02_unknown_closure_is_incomplete_not_refused` uses.
fn fixture_f1() -> DomainPackage {
    fixture_f1_with_extent(Extent::Closed)
}

/// A second, distinct domain package so its object universe genuinely differs from
/// F1's (TC-198 L04's foreign-universe key).
fn fixture_other_universe() -> DomainPackage {
    DomainPackage::new(
        DomainPackageRef::fixture("bundle.n02"),
        vec![object_type("model.A")],
    )
}

/// A version-only variant of [`fixture_f1`], for the view/domain package
/// correspondence test below: same `identity` and digest as
/// `fixture_f1()` — only `version` differs.
fn fixture_f1_with_version(version: &str) -> DomainPackage {
    let mut domain_package = fixture_f1();
    domain_package.model_selection.version = version.to_owned();
    domain_package
}

fn view_of(domain_package: &DomainPackage) -> EffectiveView {
    match normalize(domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Completed(view) => view,
        other => panic!("expected a completed effective view, got {other:?}"),
    }
}

/// The effective identity of the type-level declaration named `identity`.
fn type_id(view: &EffectiveView, identity: &str) -> EffectiveId {
    let original = DeclarationKey::fixture(identity);
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
        type_identity: DeclarationKey::fixture(type_identity),
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
                field: DeclarationKey::fixture(field),
                values: values.into_iter().map(str::to_owned).collect(),
            })
            .collect(),
        ..member(object, type_identity)
    }
}

/// FCD FR-121 document **P1**: members `a1`/`a2` of `model.A`, `b1` of `model.B`.
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
        Some(3),
        &mut admission,
    ) {
        AdmissionOutcome::Admitted(binding) => binding,
        other => panic!("expected an admitted binding, got {other:?}"),
    };

    let mut meter_a = Meter::new(SCALAR_UNLIMITED);
    let selected_a =
        match all_instances(&binding, &DeclarationKey::fixture("model.A"), &mut meter_a) {
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
            &reference_key(&universe, &a, "a1"),
            &reference_key(&universe, &a, "a2"),
            &reference_key(&universe, &b, "b1"),
        ],
        // #131's DeclarationKey reshape changed every effective id here, and
        // with it `type_identity`'s ascending order between A and B.
        "canonical reference-key order: every A member precedes every B member"
    );
    // Outputs clause: a typed reference to the queried type, bounded [0, 3]
    // (this binding's declared_maximum), never a bare unbounded set.
    assert_eq!(
        selected_a.element_type(),
        &DeclarationKey::fixture("model.A")
    );
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
    let selected_b =
        match all_instances(&binding, &DeclarationKey::fixture("model.B"), &mut meter_b) {
            AllInstancesOutcome::Completed(set) => set,
            other => panic!("expected a completed M::B selection, got {other:?}"),
        };
    let b1_via_b = reference_key(&universe, &b, "b1");
    assert_eq!(
        selected_b.members(),
        &[b1_via_b.clone()].into_iter().collect::<BTreeSet<_>>()
    );
    assert_eq!(
        selected_b.element_type(),
        &DeclarationKey::fixture("model.B")
    );
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
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let binding = match admit_binding(
        &domain_package,
        &view,
        &p1("test/orders"),
        &p1_population_key(),
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
    let outcome = all_instances(&binding, &DeclarationKey::fixture("model.A"), &mut meter);
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
/// Mutation used: inverted `population.extent != Extent::Closed` to
/// `population.extent == Extent::Closed` in `admit_binding`, so the
/// open-extent population fell through instead of returning
/// `UnknownClosure` — red as expected, reverted.
#[test]
#[trace("TC-198", "FR-153-AC-2")]
fn l02_unknown_closure_is_incomplete_not_refused() {
    let open_extent = fixture_f1_with_extent(Extent::Open);
    let open_view = view_of(&open_extent);

    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    assert_eq!(
        admit_binding(
            &open_extent,
            &open_view,
            &p1("test/orders"),
            &p1_population_key(),
            GeneralizationClosure::Closed,
            Some(3),
            &mut admission,
        ),
        AdmissionOutcome::UnknownClosure(ModelRefusal {
            code: Code::IncompletePopulation,
            cause: ModelRefusalCause::IncompleteScope {
                selection: "test/orders".to_string(),
            },
            detail: "population model.pop.p1 for test/orders does not declare extent: closed"
                .to_string(),
        })
    );
    assert!(
        admission.admitted_charges().is_empty(),
        "no charge before a binding exists"
    );

    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let mut open_subtypes = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    assert_eq!(
        admit_binding(
            &domain_package,
            &view,
            &p1("test/orders"),
            &p1_population_key(),
            GeneralizationClosure::Open,
            Some(3),
            &mut open_subtypes,
        ),
        AdmissionOutcome::UnknownClosure(ModelRefusal {
            code: Code::IncompletePopulation,
            cause: ModelRefusalCause::UnclosedSubtypes {
                selection: "test/orders".to_string(),
                type_name: Some(DeclarationKey::fixture("model.A")),
            },
            detail: "model selection test/orders naming model.A does not have a closed \
                     generalization graph"
                .to_string(),
        })
    );
    assert!(open_subtypes.admitted_charges().is_empty());
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
/// `Completed(Some(TypedReference::new(DeclarationKey::fixture("model.A"), ...)))`
/// assertion went red as expected, reverted.
#[test]
#[trace("TC-198", "FR-153-AC-2", "FR-153-AC-4")]
fn l03_lookup_undefined_mode() {
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let universe = object_universe(&domain_package).unwrap().identity();
    let a = type_id(&view, "model.A");
    let b = type_id(&view, "model.B");
    let binding = admitted_binding(&domain_package, &view, &p1("test/orders"));

    let rb = LookupKey {
        static_type: DeclarationKey::fixture("model.B"),
        key: reference_key(&universe, &b, "b1"),
    };
    let mut meter_present = Meter::new(SCALAR_UNLIMITED);
    let present = lookup(
        &binding,
        &DeclarationKey::fixture("model.A"),
        &rb,
        AbsenceMode::Undefined,
        &mut meter_present,
    );
    assert_eq!(
        present,
        LookupOutcome::Completed(Some(TypedReference::new(
            DeclarationKey::fixture("model.A"),
            reference_key(&universe, &b, "b1")
        )))
    );
    assert_eq!(meter_present.consumed(LimitKind::WorkUnits), 2);
    assert_eq!(meter_present.consumed(LimitKind::ResultUnits), 1);

    // `rc` is bound to `q` (document P2), never admitted into `p`, so it is
    // absent from `p`'s binding.
    let rc = LookupKey {
        static_type: DeclarationKey::fixture("model.A"),
        key: reference_key(&universe, &a, "c9"),
    };
    let mut meter_absent = Meter::new(SCALAR_UNLIMITED);
    let absent = lookup(
        &binding,
        &DeclarationKey::fixture("model.A"),
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
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let universe = object_universe(&domain_package).unwrap().identity();
    let a = type_id(&view, "model.A");
    let b = type_id(&view, "model.B");
    let binding = admitted_binding(&domain_package, &view, &p1("test/orders"));

    let rb = LookupKey {
        static_type: DeclarationKey::fixture("model.B"),
        key: reference_key(&universe, &b, "b1"),
    };
    let mut meter_present = Meter::new(SCALAR_UNLIMITED);
    let present = lookup(
        &binding,
        &DeclarationKey::fixture("model.A"),
        &rb,
        AbsenceMode::Empty,
        &mut meter_present,
    );
    assert_eq!(
        present,
        LookupOutcome::Completed(Some(TypedReference::new(
            DeclarationKey::fixture("model.A"),
            reference_key(&universe, &b, "b1")
        )))
    );
    assert_eq!(meter_present.consumed(LimitKind::WorkUnits), 2);
    assert_eq!(meter_present.consumed(LimitKind::ResultUnits), 2);

    let rc = LookupKey {
        static_type: DeclarationKey::fixture("model.A"),
        key: reference_key(&universe, &a, "c9"),
    };
    let mut meter_absent = Meter::new(SCALAR_UNLIMITED);
    let absent = lookup(
        &binding,
        &DeclarationKey::fixture("model.A"),
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
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let universe = object_universe(&domain_package).unwrap().identity();
    let a = type_id(&view, "model.A");
    let binding = admitted_binding(&domain_package, &view, &p1("test/orders"));

    let rc = LookupKey {
        static_type: DeclarationKey::fixture("model.A"),
        key: reference_key(&universe, &a, "c9"),
    };
    let mut meter = Meter::new(SCALAR_UNLIMITED);
    let outcome = lookup(
        &binding,
        &DeclarationKey::fixture("model.A"),
        &rc,
        AbsenceMode::Refused,
        &mut meter,
    );
    match outcome {
        LookupOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::InvalidRuntimeInput);
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::AbsentKey {
                    key: b"c9".to_vec(),
                }
            );
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
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let universe = object_universe(&domain_package).unwrap().identity();
    let a = type_id(&view, "model.A");
    let binding = admitted_binding(&domain_package, &view, &p1("test/orders"));

    let ra = LookupKey {
        static_type: DeclarationKey::fixture("model.A"),
        key: reference_key(&universe, &a, "a1"),
    };
    let mut meter = Meter::new(SCALAR_UNLIMITED);
    let outcome = lookup(
        &binding,
        &DeclarationKey::fixture("model.B"),
        &ra,
        AbsenceMode::Empty,
        &mut meter,
    );
    match outcome {
        LookupOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::IllTyped);
            assert_eq!(refusal.cause, ModelRefusalCause::TypeMismatch);
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
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let a = type_id(&view, "model.A");
    let binding = admitted_binding(&domain_package, &view, &p1("test/orders"));

    let other = fixture_other_universe();
    let foreign_universe = object_universe(&other).unwrap().identity();
    assert_ne!(&foreign_universe, binding.universe());

    let rx = LookupKey {
        static_type: DeclarationKey::fixture("model.A"),
        key: reference_key(&foreign_universe, &a, "a1"),
    };
    let mut meter = Meter::new(SCALAR_UNLIMITED);
    let outcome = lookup(
        &binding,
        &DeclarationKey::fixture("model.A"),
        &rx,
        AbsenceMode::Empty,
        &mut meter,
    );
    match outcome {
        LookupOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::ForeignReference);
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::ForeignUniverse {
                    actual: foreign_universe.as_bytes().to_vec(),
                    expected: binding.universe().clone(),
                }
            );
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
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let mut document = p1("test/orders");
    document.members.push(member("a1", "model.B"));

    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let outcome = admit_binding(
        &domain_package,
        &view,
        &document,
        &p1_population_key(),
        GeneralizationClosure::Closed,
        Some(3),
        &mut admission,
    );
    match outcome {
        AdmissionOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::InvalidRuntimeInput);
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::ConflictingIdentity {
                    object: "a1".to_string(),
                    existing_type: type_id(&view, "model.A"),
                    declared_type: type_id(&view, "model.B"),
                }
            );
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
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let mut document = p1("test/orders");
    document.members.push(member("a1", "model.A"));

    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let binding = match admit_binding(
        &domain_package,
        &view,
        &document,
        &p1_population_key(),
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
    let selected = match all_instances(&binding, &DeclarationKey::fixture("model.A"), &mut meter) {
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
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let mut document = p1("test/orders");
    document.members.push(member("z1", "model.Z"));

    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let outcome = admit_binding(
        &domain_package,
        &view,
        &document,
        &p1_population_key(),
        GeneralizationClosure::Closed,
        Some(3),
        &mut admission,
    );
    match outcome {
        AdmissionOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::ForeignReference);
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::ForeignType {
                    member: "z1".to_string(),
                    type_name: DeclarationKey::fixture("model.Z"),
                }
            );
        }
        other => panic!("expected Refused(foreign_reference/foreign-type), got {other:?}"),
    }
}

/// FR-153:57/:72: a member type present in the effective view but not
/// covered by the population's own declared `member_types` (itself, or a
/// type conforming to one of them) is refused `foreign_reference`/
/// `foreign-type`, distinct from [`l05_foreign_type_refuses`]'s absent-type
/// case: `model.C` is a real declared object type here, just not one
/// [`P1_POPULATION`] lists.
#[test]
#[trace("TC-198", "FR-153-AC-3")]
fn l05b_member_type_not_covered_by_population_member_types_refuses() {
    let mut domain_package = fixture_f1();
    domain_package.records.push(object_type("model.C"));

    let view = view_of(&domain_package);
    let mut document = p1("test/orders");
    document.members.push(member("c1", "model.C"));

    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let outcome = admit_binding(
        &domain_package,
        &view,
        &document,
        &p1_population_key(),
        GeneralizationClosure::Closed,
        Some(4),
        &mut admission,
    );
    match outcome {
        AdmissionOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::ForeignReference);
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::ForeignType {
                    member: "c1".to_string(),
                    type_name: DeclarationKey::fixture("model.C"),
                }
            );
        }
        other => panic!("expected Refused(foreign_reference/foreign-type), got {other:?}"),
    }
}

/// D05 (`model-complete.md:156`): a member whose most-specific type is
/// declared `abstract` is refused `invalid_runtime_input`/`abstract-instance`,
/// even though that type is a real declared object type covered by the
/// population's own `member_types`.
#[test]
#[trace("TC-198", "FR-153-AC-3")]
fn l05c_abstract_instance_refuses() {
    let mut domain_package = fixture_f1();
    for record in &mut domain_package.records {
        if let DomainPackageRecord::ObjectType(object) = record {
            if object.key == DeclarationKey::fixture("model.B") {
                object.abstract_type = true;
            }
        }
    }

    let view = view_of(&domain_package);
    let document = p1("test/orders"); // "b1" names `model.B`, now abstract.

    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let outcome = admit_binding(
        &domain_package,
        &view,
        &document,
        &p1_population_key(),
        GeneralizationClosure::Closed,
        Some(3),
        &mut admission,
    );
    match outcome {
        AdmissionOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::InvalidRuntimeInput);
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::AbstractInstance {
                    member: "b1".to_string(),
                    abstract_type: DeclarationKey::fixture("model.B"),
                }
            );
        }
        other => panic!("expected Refused(invalid_runtime_input/abstract-instance), got {other:?}"),
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
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
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

    let mut meter = Meter::new(SCALAR_UNLIMITED);
    let outcome = all_instances(&binding, &DeclarationKey::fixture("model.A"), &mut meter);
    match outcome {
        AllInstancesOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::CardinalityOutOfBound);
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::AboveMaximum {
                    selected: 3,
                    maximum: 2,
                }
            );
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
        &domain_package,
        &view,
        &p1("test/orders"),
        &p1_population_key(),
        GeneralizationClosure::Closed,
        Some(3),
        &mut AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED),
    ) {
        AdmissionOutcome::Admitted(binding) => binding,
        other => panic!("expected an admitted binding, got {other:?}"),
    };
    let incomplete_outcome = all_instances(
        &bounded,
        &DeclarationKey::fixture("model.A"),
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
// `domain_package` argument against the binding's admitted `DomainPackageRef` header
// (identity, revision, digest), rather than against the binding's own
// admitted `records`. Nothing in this crate verifies `export.digest`
// against a domain package's actual `records` anywhere, and the test-only
// `DomainPackageRef::fixture` derives that digest from the identity string
// alone — so a caller could present a domain package with the exact admitted header
// but different `records` (e.g. F1 with its `B -> A` supertype record
// removed) and the header check would pass while `all_instances`/`lookup`
// silently answered against the wrong content (`b1` dropping out of
// `allInstances<M::A>(p)` instead of the query refusing).
//
// This is now unrepresentable, not merely refused: `all_instances` and
// `lookup` no longer take a `domain_package` argument at all (see the module docs'
// "Binding/domain package correspondence"), so there is no way for a caller to
// evaluate a query against anything other than the exact domain package
// `admit_binding` admitted. The prior fix cycle's `fixture_f1_with_
// foreign_digest` fixture and its query-time test are deleted along with
// `require_bundle_matches` itself, rather than adapted — there is no longer
// a second domain package parameter for either one to exercise. The reachable
// sibling of this defect class — a `LookupKey` naming a foreign universe,
// independent of which domain package is bound — stays covered by
// `l04_lookup_foreign_universe_refuses` above, whose `LookupKey.key.universe`
// field a caller can still set to anything regardless of the bound domain package.

/// TC-198's own admission-time `modelIdentity` check: a population document
/// naming a `modelIdentity` other than the admitting domain package's own refuses
/// before any `binding.member` charge. Every other test in this file admits
/// `p1("test/orders")` against `fixture_f1()`, whose own `modelIdentity` is
/// also `test/orders` (`DomainPackageRef::fixture`'s fixed identity), so
/// this path was untested.
///
/// Mutation used: in `admit_binding`, changed the `modelIdentity` guard's
/// condition from `document.model_identity != domain_package.model_selection.
/// identity` to `false` (never fires), which let the mismatched document
/// proceed to `AdmissionOutcome::Admitted` instead of refusing — the
/// `Refused(foreign_reference/foreign-model-selection)` assertion went red
/// as expected, reverted.
#[test]
#[trace("TC-198", "FR-153-AC-3")]
fn l05_foreign_model_selection_refuses_at_admission() {
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let mismatched = p1("bundle.other");

    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let outcome = admit_binding(
        &domain_package,
        &view,
        &mismatched,
        &p1_population_key(),
        GeneralizationClosure::Closed,
        Some(3),
        &mut admission,
    );
    match outcome {
        AdmissionOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::ForeignReference);
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::ForeignModelSelection {
                    actual: OfferedSelection::Document("bundle.other".to_string()),
                    expected: domain_package.model_selection.clone(),
                }
            );
        }
        other => {
            panic!("expected Refused(foreign_reference/foreign-model-selection), got {other:?}")
        }
    }
    assert!(admission.admitted_charges().is_empty());
}

/// Round-3 review finding (PR #148): `admit_binding` takes `view` and
/// `domain_package` separately and, before this test, never checked that they
/// correspond — the same mismatch class removed from `all_instances`/
/// `lookup` themselves, just moved one level up. Pins the check with a
/// version-only divergence (same `identity` and digest as
/// `fixture_f1()` — `DomainPackageRef::fixture` derives the digest from the
/// selection placeholder alone, so an identity-only comparison would miss
/// this) to prove the check compares the full `DomainPackageRef` header,
/// not just `identity`.
///
/// Mutation used: narrowed the check from `view.model_selection !=
/// domain_package.model_selection` to `view.model_selection.identity !=
/// domain_package.model_selection.identity`, which let this version-only
/// mismatch admit instead of refusing — red as expected, reverted.
#[test]
#[trace("TC-198", "FR-153-AC-3")]
fn l05_view_from_a_different_bundle_version_refuses_at_admission() {
    let view = view_of(&fixture_f1_with_version("1"));
    let domain_package = fixture_f1_with_version("2");

    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let outcome = admit_binding(
        &domain_package,
        &view,
        &p1("test/orders"),
        &p1_population_key(),
        GeneralizationClosure::Closed,
        Some(3),
        &mut admission,
    );
    match outcome {
        AdmissionOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::ForeignReference);
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::ForeignModelSelection {
                    actual: OfferedSelection::View(view.model_selection.clone()),
                    expected: domain_package.model_selection.clone(),
                }
            );
        }
        other => {
            panic!("expected Refused(foreign_reference/foreign-model-selection), got {other:?}")
        }
    }
    assert!(admission.admitted_charges().is_empty());
}

/// #196 review finding 1, happy path: `admit_binding` resolves
/// `p1_population_key()` against `fixture_f1()`'s own `Population` record
/// and admits normally -- the by-key resolution this fix added does not
/// itself change an already-passing admission.
#[test]
#[trace("TC-198", "FR-153-AC-1")]
fn admission_admits_when_the_population_key_resolves_in_the_domain_package() {
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);

    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let outcome = admit_binding(
        &domain_package,
        &view,
        &p1("test/orders"),
        &p1_population_key(),
        GeneralizationClosure::Closed,
        Some(3),
        &mut admission,
    );
    let AdmissionOutcome::Admitted(binding) = outcome else {
        panic!("expected Admitted, got {outcome:?}");
    };
    assert_eq!(binding.declared_maximum(), Some(3));
    assert_eq!(binding.members().len(), 3);
    assert_eq!(
        admission.admitted_charges().len(),
        3,
        "one binding.member charge per P1 member"
    );
}

/// #196 review finding 1: extent is read from the `Population` record
/// `admit_binding` itself resolves by key, never a value a caller states
/// independently of it -- `admit_binding` takes only `population_key`, a
/// `DeclarationKey`, so there is no parameter left through which a caller
/// could claim `Closed` for a population the domain package itself declares
/// `Open`.
///
/// Mutation used: in `admit_binding`, changed `population.extent !=
/// Extent::Closed` to `false` (the resolved record's own extent never
/// denies), which let the open-extent population admit instead of returning
/// `UnknownClosure` -- red as expected, reverted.
#[test]
#[trace("TC-198", "FR-153-AC-3")]
fn admission_reads_extent_from_the_resolved_record_never_a_caller_claim() {
    let open_extent = fixture_f1_with_extent(Extent::Open);
    let view = view_of(&open_extent);

    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    assert_eq!(
        admit_binding(
            &open_extent,
            &view,
            &p1("test/orders"),
            &p1_population_key(),
            GeneralizationClosure::Closed,
            Some(3),
            &mut admission,
        ),
        AdmissionOutcome::UnknownClosure(ModelRefusal {
            code: Code::IncompletePopulation,
            cause: ModelRefusalCause::IncompleteScope {
                selection: "test/orders".to_string(),
            },
            detail: "population model.pop.p1 for test/orders does not declare extent: closed"
                .to_string(),
        })
    );
    assert!(admission.admitted_charges().is_empty());
}

/// #196 review finding 1: a population key naming another domain package's
/// `Population` declaration -- not one of the admitting `domain_package`'s
/// own records -- refuses `foreign_reference`/`foreign-model-selection`
/// rather than resolving against nothing or admitting with no population
/// declaration behind it at all.
///
/// Mutation used: in `admit_binding`, changed the resolving `find_map`'s
/// predicate from `population.key == *population_key` to `true` (matches
/// whichever `Population` record is declared first, regardless of key),
/// which let a foreign key resolve to `fixture_f1()`'s own `model.pop.p1`
/// record instead of refusing -- red as expected, reverted.
#[test]
#[trace("TC-198", "FR-153-AC-3")]
fn admission_refuses_a_population_key_from_another_domain_package() {
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let foreign_key = DeclarationKey::fixture("model.pop.g1");

    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    assert_eq!(
        admit_binding(
            &domain_package,
            &view,
            &p1("test/orders"),
            &foreign_key,
            GeneralizationClosure::Closed,
            Some(3),
            &mut admission,
        ),
        AdmissionOutcome::Refused(ModelRefusal {
            code: Code::ForeignReference,
            cause: ModelRefusalCause::ForeignModelSelection {
                actual: OfferedSelection::Population(foreign_key.clone()),
                expected: domain_package.model_selection.clone(),
            },
            detail: format!(
                "population key {} names no Population declaration of domain package test/orders",
                foreign_key.node
            ),
        })
    );
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
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let limits = PopulationAdmissionLimits {
        population_members: 2,
        ..PopulationAdmissionLimits::UNLIMITED
    };
    let mut admission = AdmissionMeter::new(limits);
    let outcome = admit_binding(
        &domain_package,
        &view,
        &p1("test/orders"),
        &p1_population_key(),
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
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let limits = PopulationAdmissionLimits {
        work_units: 2,
        ..PopulationAdmissionLimits::UNLIMITED
    };
    let mut admission = AdmissionMeter::new(limits);
    let outcome = admit_binding(
        &domain_package,
        &view,
        &p1("test/orders"),
        &p1_population_key(),
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
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);

    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let binding = match admit_binding(
        &domain_package,
        &view,
        &p1("test/orders"),
        &p1_population_key(),
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
    let selected_a =
        match all_instances(&binding, &DeclarationKey::fixture("model.A"), &mut meter_a) {
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
fn r06_bundle() -> DomainPackage {
    DomainPackage::new(
        DomainPackageRef::fixture("bundle.r06pop"),
        vec![
            object_type("model.A"),
            object_type("model.B"),
            supertype("model.gen.B-A", "model.B", "model.A"),
            field_member_mult("model.A.all", "model.A", "model.A", 0, Some(5)),
            field_member_mult("model.A.some", "model.A", "model.B", 0, Some(3)),
            subsetting(
                "model.subset.some-all",
                "model.A",
                "model.A.some",
                "model.A.all",
            ),
            population_record(P1_POPULATION, &["model.A", "model.B"], Extent::Closed),
        ],
    )
}

#[test]
#[trace("TC-196", "FR-151-AC-10")]
fn r06_subsetting_violation_refuses_after_the_charged_subset_value() {
    let domain_package = r06_bundle();
    let view = view_of(&domain_package);
    let document = PopulationDocument {
        model_identity: "test/orders".to_owned(),
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
        &domain_package,
        &view,
        &document,
        &p1_population_key(),
        GeneralizationClosure::Closed,
        Some(3),
        &mut admission,
    );
    match outcome {
        AdmissionOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::InvalidRuntimeInput);
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::SubsettingViolation {
                    object: "a1".to_owned(),
                    record: DeclarationKey::fixture("model.subset.some-all"),
                    subsetting: DeclarationKey::fixture("model.A.some"),
                    subsetted: DeclarationKey::fixture("model.A.all"),
                }
            );
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
    let domain_package = r06_bundle();
    let view = view_of(&domain_package);
    let document = PopulationDocument {
        model_identity: "test/orders".to_owned(),
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
        &domain_package,
        &view,
        &document,
        &p1_population_key(),
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
    let domain_package = r06_bundle();
    let view = view_of(&domain_package);
    let document = PopulationDocument {
        model_identity: "test/orders".to_owned(),
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
        &domain_package,
        &view,
        &document,
        &p1_population_key(),
        GeneralizationClosure::Closed,
        Some(3),
        &mut admission,
    );
    match outcome {
        AdmissionOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::InvalidRuntimeInput);
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::DuplicateMember {
                    object: "a1".to_owned(),
                    field: DeclarationKey::fixture("model.A.all"),
                }
            );
            assert!(refusal.detail.contains("a1"));
            assert!(refusal.detail.contains("model.A.all"));
        }
        other => {
            panic!("expected Refused(invalid_runtime_input/duplicate-member), got {other:?}")
        }
    }
}

// ---------------------------------------------------------------------------
// TC-198 L07: admit_invocation -- created/deleted identities and the
// operation frame (EXPR-020/021/022)
// ---------------------------------------------------------------------------

/// TC-198 P1 (see [`p1`]) after the operation's own effect deletes `a2`:
/// `a1`/`b1` survive unchanged.
fn p1_minus_a2(model_identity: &str) -> PopulationDocument {
    PopulationDocument {
        model_identity: model_identity.to_owned(),
        members: vec![member("a1", "model.A"), member("b1", "model.B")],
    }
}

/// TC-198 L07's own `model.A.remove` effect: `{modifies: [], creates: [],
/// deletes: [model.A]}`.
fn deletes_a_effect() -> OperationEffect {
    OperationEffect {
        modifies: Vec::new(),
        creates: Vec::new(),
        deletes: vec![DeclarationKey::fixture("model.A")],
    }
}

/// An operation effect declaring no frame at all: every create, delete and
/// field write it did not itself grant is unauthorized.
fn empty_effect() -> OperationEffect {
    OperationEffect {
        modifies: Vec::new(),
        creates: Vec::new(),
        deletes: Vec::new(),
    }
}

fn invocation_context<'a>(
    domain_package: &'a DomainPackage,
    view: &'a EffectiveView,
    population: &'a DeclarationKey,
) -> InvocationContext<'a> {
    InvocationContext {
        domain_package,
        view,
        population,
        subtype_closure: GeneralizationClosure::Closed,
        declared_maximum: Some(3),
    }
}

/// TC-198 L07: `admit_invocation` admits P1 (pre) and P1-without-`a2` (post)
/// under `model.A.remove`'s declared frame -- `a2`'s deletion conforms to
/// the declared `deletes: [model.A]` grant -- and attaches the pre binding
/// as the post binding's own `pre_anchor`, so `pre(allInstances(p))`/
/// `pre(lookup(p, r) absent m)` (backed at the source-expression layer by
/// `tests/model_reference_queries.rs`'s `l07_pre_*` tests) can read it. The
/// deleted object, `a2`, keeps its pre most-specific type in the attached
/// pre anchor.
///
/// Mutation used: in `enforce_frame`, replaced the deleted-identity loop's
/// `if !allowed { return Err(..) }` with an unconditional no-op (accepting
/// every deletion regardless of the declared frame). This test still went
/// green (it never violates the frame), but
/// `l07_invocation_refuses_a_delete_outside_the_declared_frame` below went
/// green when it should have stayed red, confirming the mutation defeats
/// the guard that test exists to pin; reverted.
#[test]
#[trace("TC-198", "FR-153-AC-7")]
fn l07_invocation_admits_a_declared_delete_and_attaches_the_pre_anchor() {
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let universe = object_universe(&domain_package).unwrap().identity();
    let a = type_id(&view, "model.A");
    let effect = deletes_a_effect();
    let mut pre_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let mut post_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);

    let declared = InvocationDelta {
        effect: &effect,
        declared_created: &[],
        declared_deleted: &["a2".to_owned()],
    };
    let post = match admit_invocation(
        invocation_context(&domain_package, &view, &p1_population_key()),
        &p1("test/orders"),
        &p1_minus_a2("test/orders"),
        &declared,
        &mut pre_meter,
        &mut post_meter,
    ) {
        AdmissionOutcome::Admitted(binding) => binding,
        other => panic!("expected an admitted invocation, got {other:?}"),
    };

    let pre = post
        .pre_anchor()
        .expect("post binding must carry its pre anchor");
    assert_eq!(pre.members().len(), 3);
    assert_eq!(post.members().len(), 2);

    let a2_key = reference_key(&universe, &a, "a2");
    assert_eq!(
        pre.members().get(&a2_key),
        Some(&DeclarationKey::fixture("model.A"))
    );
    assert!(!post.members().contains_key(&a2_key));
}

/// A deletion the operation's own effect does not grant refuses
/// `Code::FrameViolation`/cause `unauthorized-change` -- the same TC-198 L07
/// scenario (deleting `a2`), but under `empty_effect()`'s empty `deletes:
/// []`.
#[test]
#[trace("TC-198", "FR-046-AC-3")]
fn l07_invocation_refuses_a_delete_outside_the_declared_frame() {
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let effect = empty_effect();
    let declared = InvocationDelta {
        effect: &effect,
        declared_created: &[],
        declared_deleted: &["a2".to_owned()],
    };
    let mut pre_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let mut post_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);

    let outcome = admit_invocation(
        invocation_context(&domain_package, &view, &p1_population_key()),
        &p1("test/orders"),
        &p1_minus_a2("test/orders"),
        &declared,
        &mut pre_meter,
        &mut post_meter,
    );
    match outcome {
        AdmissionOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::FrameViolation);
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::FrameDeleteOutsideGrant {
                    object: "a2".to_owned(),
                    type_name: DeclarationKey::fixture("model.A"),
                }
            );
            assert!(refusal.detail.contains("a2"));
        }
        other => panic!("expected Refused(frame_violation/unauthorized-change), got {other:?}"),
    }
}

/// A creation the operation's own effect does not grant refuses the same
/// way: post names `a9` (of `model.A`), absent from pre, under an effect
/// declaring no `creates` grant at all.
#[test]
#[trace("FR-046-AC-3")]
fn invocation_refuses_a_create_outside_the_declared_frame() {
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let effect = empty_effect();
    let post_document = PopulationDocument {
        model_identity: "test/orders".to_owned(),
        members: vec![
            member("a1", "model.A"),
            member("a2", "model.A"),
            member("b1", "model.B"),
            member("a9", "model.A"),
        ],
    };
    let declared = InvocationDelta {
        effect: &effect,
        declared_created: &["a9".to_owned()],
        declared_deleted: &[],
    };
    let mut pre_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let mut post_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);

    let outcome = admit_invocation(
        invocation_context(&domain_package, &view, &p1_population_key()),
        &p1("test/orders"),
        &post_document,
        &declared,
        &mut pre_meter,
        &mut post_meter,
    );
    match outcome {
        AdmissionOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::FrameViolation);
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::FrameCreateOutsideGrant {
                    object: "a9".to_owned(),
                    type_name: DeclarationKey::fixture("model.A"),
                }
            );
            assert!(refusal.detail.contains("a9"));
        }
        other => panic!("expected Refused(frame_violation/unauthorized-change), got {other:?}"),
    }
}

/// A surviving member's field value changing pre to post refuses when the
/// operation's effect does not declare that field a `modifies` member,
/// and admits when it does -- both over the identical pre/post pair, so only
/// the declared frame decides the outcome.
#[test]
#[trace("FR-046-AC-3")]
fn invocation_field_write_outside_the_declared_frame_refuses_and_inside_it_admits() {
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let pre_document = PopulationDocument {
        model_identity: "test/orders".to_owned(),
        members: vec![member_with_fields(
            "a1",
            "model.A",
            vec![("model.A.x", vec!["a2"])],
        )],
    };
    let post_document = PopulationDocument {
        model_identity: "test/orders".to_owned(),
        members: vec![member_with_fields(
            "a1",
            "model.A",
            vec![("model.A.x", vec!["a9"])],
        )],
    };

    let undeclared = empty_effect();
    let undeclared_delta = InvocationDelta {
        effect: &undeclared,
        declared_created: &[],
        declared_deleted: &[],
    };
    let mut pre_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let mut post_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let outcome = admit_invocation(
        invocation_context(&domain_package, &view, &p1_population_key()),
        &pre_document,
        &post_document,
        &undeclared_delta,
        &mut pre_meter,
        &mut post_meter,
    );
    match outcome {
        AdmissionOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::FrameViolation);
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::FrameFieldWriteOutsideGrant {
                    object: "a1".to_owned(),
                    field: DeclarationKey::fixture("model.A.x"),
                }
            );
            assert!(refusal.detail.contains("a1"));
            assert!(refusal.detail.contains("model.A.x"));
        }
        other => panic!("expected Refused(frame_violation/unauthorized-change), got {other:?}"),
    }

    let declared_effect = OperationEffect {
        modifies: vec![DeclarationKey::fixture("model.A.x")],
        creates: Vec::new(),
        deletes: Vec::new(),
    };
    let declared = InvocationDelta {
        effect: &declared_effect,
        declared_created: &[],
        declared_deleted: &[],
    };
    let mut pre_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let mut post_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let outcome = admit_invocation(
        invocation_context(&domain_package, &view, &p1_population_key()),
        &pre_document,
        &post_document,
        &declared,
        &mut pre_meter,
        &mut post_meter,
    );
    match outcome {
        AdmissionOutcome::Admitted(_) => {}
        other => {
            panic!("expected Admitted once model.A.x is a declared modifies member, got {other:?}")
        }
    }
}

// ---------------------------------------------------------------------------
// PR #168 review: object identity vs. type, ordered/unordered field
// comparison, redefinition-reaching field-write coverage, subtype
// created/deleted under a supertype grant, and FR-046 delta agreement.
// ---------------------------------------------------------------------------

/// A dedicated domain package for the ordered/unordered field-comparison tests:
/// `model.A.ordered` (declared `ordered: true`, so pre/post comparison is
/// exact-sequence) and `model.A.unordered` (declared `ordered: false`, so
/// comparison is order-insensitive), both fields of `model.A`.
fn ordering_bundle() -> DomainPackage {
    DomainPackage::new(
        DomainPackageRef::fixture("bundle.ordering"),
        vec![
            object_type("model.A"),
            DomainPackageRecord::FieldMember(FieldMemberRecord {
                key: DeclarationKey::fixture("model.A.ordered"),
                owner: DeclarationKey::fixture("model.A"),
                value_type: DeclarationKey::fixture("model.A"),
                multiplicity: Multiplicity {
                    lower: 0,
                    upper: Some(5),
                    ordered: true,
                    unique: true,
                },
            }),
            DomainPackageRecord::FieldMember(FieldMemberRecord {
                key: DeclarationKey::fixture("model.A.unordered"),
                owner: DeclarationKey::fixture("model.A"),
                value_type: DeclarationKey::fixture("model.A"),
                multiplicity: Multiplicity {
                    lower: 0,
                    upper: Some(5),
                    ordered: false,
                    unique: true,
                },
            }),
            population_record(P1_POPULATION, &["model.A"], Extent::Closed),
        ],
    )
}

/// Item 6: reordering an *unordered* field's values between pre and post is
/// not a write at all -- `enforce_frame` compares by the field's own
/// declared collection kind (`Multiplicity::ordered`), never by `Vec`
/// sequence, for a field declared unordered. `empty_effect()` (no declared
/// `modifies`) still admits, because there is no write to authorize.
///
/// Mutation used: in `field_values_equal`, removed the `if
/// field_ordered(...)` branch entirely (always comparing `pre == post` as
/// plain sequences). This test went red as expected (`Refused(
/// frame_violation/unauthorized-change)` instead of `Admitted`); reverted.
#[test]
#[trace("FR-046-AC-3")]
fn enforce_frame_admits_an_unordered_field_reorder_without_a_write() {
    let domain_package = ordering_bundle();
    let view = view_of(&domain_package);
    let pre_document = PopulationDocument {
        model_identity: "test/orders".to_owned(),
        members: vec![member_with_fields(
            "a1",
            "model.A",
            vec![("model.A.unordered", vec!["a1", "a2"])],
        )],
    };
    let post_document = PopulationDocument {
        model_identity: "test/orders".to_owned(),
        members: vec![member_with_fields(
            "a1",
            "model.A",
            vec![("model.A.unordered", vec!["a2", "a1"])],
        )],
    };
    let effect = empty_effect();
    let declared = InvocationDelta {
        effect: &effect,
        declared_created: &[],
        declared_deleted: &[],
    };
    let mut pre_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let mut post_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let outcome = admit_invocation(
        invocation_context(&domain_package, &view, &p1_population_key()),
        &pre_document,
        &post_document,
        &declared,
        &mut pre_meter,
        &mut post_meter,
    );
    match outcome {
        AdmissionOutcome::Admitted(_) => {}
        other => panic!(
            "expected Admitted -- an unordered field's reordered values are not a write, got \
             {other:?}"
        ),
    }
}

/// Item 6's other half: the identical reorder over the *ordered* sibling
/// field is a write (an ordered field's declared sequence is exact, so a
/// different order is a different sequence), and `empty_effect()` declares
/// no `modifies`, so it refuses.
///
/// Mutation used: in `field_ordered`, replaced the body with an
/// unconditional `false` (treating every field, including this one's own
/// declared-`ordered: true` field, as order-insensitive). This test went
/// red as expected (`Admitted` instead of `Refused`); reverted.
#[test]
#[trace("FR-046-AC-3")]
fn enforce_frame_refuses_an_ordered_field_reorder_as_a_write() {
    let domain_package = ordering_bundle();
    let view = view_of(&domain_package);
    let pre_document = PopulationDocument {
        model_identity: "test/orders".to_owned(),
        members: vec![member_with_fields(
            "a1",
            "model.A",
            vec![("model.A.ordered", vec!["a1", "a2"])],
        )],
    };
    let post_document = PopulationDocument {
        model_identity: "test/orders".to_owned(),
        members: vec![member_with_fields(
            "a1",
            "model.A",
            vec![("model.A.ordered", vec!["a2", "a1"])],
        )],
    };
    let effect = empty_effect();
    let declared = InvocationDelta {
        effect: &effect,
        declared_created: &[],
        declared_deleted: &[],
    };
    let mut pre_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let mut post_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let outcome = admit_invocation(
        invocation_context(&domain_package, &view, &p1_population_key()),
        &pre_document,
        &post_document,
        &declared,
        &mut pre_meter,
        &mut post_meter,
    );
    match outcome {
        AdmissionOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::FrameViolation);
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::FrameFieldWriteOutsideGrant {
                    object: "a1".to_owned(),
                    field: DeclarationKey::fixture("model.A.ordered"),
                }
            );
            assert!(refusal.detail.contains("model.A.ordered"));
        }
        other => panic!(
            "expected Refused(frame_violation/unauthorized-change) -- an ordered field's \
             reordered values are a write, got {other:?}"
        ),
    }
}

/// A dedicated domain package for item 7's redefinition-reaching field-write test:
/// `model.B.x` redefines `model.A.x` (`B` a subtype of `A`).
fn redefinition_bundle() -> DomainPackage {
    DomainPackage::new(
        DomainPackageRef::fixture("bundle.redef"),
        vec![
            object_type("model.A"),
            object_type("model.B"),
            supertype("model.gen.B-A", "model.B", "model.A"),
            field_member("model.A.x", "model.A", "model.A"),
            field_member("model.B.x", "model.B", "model.A"),
            redefinition("model.redef.B.x-A.x", "model.B", "model.B.x", "model.A.x"),
            population_record(P1_POPULATION, &["model.A", "model.B"], Extent::Closed),
        ],
    )
}

/// A dedicated domain package for the review's own chained-redefinition test:
/// `model.A <- model.B <- model.C`, with `model.B.x` redefining `model.A.x`
/// and `model.C.x` redefining `model.B.x` -- no direct `model.C.x ->
/// model.A.x` record exists, per model-complete.md:56 ("the redefining
/// feature replaces the *one* inherited redefined feature").
fn redefinition_chain_bundle() -> DomainPackage {
    DomainPackage::new(
        DomainPackageRef::fixture("bundle.redef.chain"),
        vec![
            object_type("model.A"),
            object_type("model.B"),
            object_type("model.C"),
            supertype("model.gen.B-A", "model.B", "model.A"),
            supertype("model.gen.C-B", "model.C", "model.B"),
            field_member("model.A.x", "model.A", "model.A"),
            field_member("model.B.x", "model.B", "model.A"),
            field_member("model.C.x", "model.C", "model.A"),
            redefinition("model.redef.B.x-A.x", "model.B", "model.B.x", "model.A.x"),
            redefinition("model.redef.C.x-B.x", "model.C", "model.C.x", "model.B.x"),
            population_record(
                P1_POPULATION,
                &["model.A", "model.B", "model.C"],
                Extent::Closed,
            ),
        ],
    )
}

/// Item 7: a field write is covered by `effect.modifies` when it
/// "reaches one through redefinition records" (FR-151's own effect-
/// inclusion rule), not only by an exact key match. The operation declares
/// only the redefined ancestor member, `model.A.x`; the population document
/// writes the redefining member, `model.B.x` -- admitted because it reaches
/// `model.A.x` through the one `RedefinitionRecord` above.
///
/// Mutation used: in `field_write_covered`, removed the `bundle.records...`
/// redefinition-reaching branch (direct match only). This test went red as
/// expected (`Refused(frame_violation/unauthorized-change)` instead of
/// `Admitted`); reverted.
#[test]
#[trace("FR-046-AC-3")]
fn enforce_frame_admits_a_field_write_that_reaches_a_declared_grant_through_redefinition() {
    let domain_package = redefinition_bundle();
    let view = view_of(&domain_package);
    let pre_document = PopulationDocument {
        model_identity: "test/orders".to_owned(),
        members: vec![member_with_fields(
            "b1",
            "model.B",
            vec![("model.B.x", vec!["a1"])],
        )],
    };
    let post_document = PopulationDocument {
        model_identity: "test/orders".to_owned(),
        members: vec![member_with_fields(
            "b1",
            "model.B",
            vec![("model.B.x", vec!["a9"])],
        )],
    };
    let effect = OperationEffect {
        modifies: vec![DeclarationKey::fixture("model.A.x")],
        creates: Vec::new(),
        deletes: Vec::new(),
    };
    let declared = InvocationDelta {
        effect: &effect,
        declared_created: &[],
        declared_deleted: &[],
    };
    let mut pre_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let mut post_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let outcome = admit_invocation(
        invocation_context(&domain_package, &view, &p1_population_key()),
        &pre_document,
        &post_document,
        &declared,
        &mut pre_meter,
        &mut post_meter,
    );
    match outcome {
        AdmissionOutcome::Admitted(_) => {}
        other => panic!(
            "expected Admitted -- model.B.x reaches the declared model.A.x grant through \
             redefinition, got {other:?}"
        ),
    }
}

/// Re-review finding 3: one redefinition hop is not enough. `model.C.x`
/// reaches `model.A.x` only through a *chain* (`model.C.x -> model.B.x ->
/// model.A.x`, two hops, `redefinition_chain_bundle`'s own two records) --
/// there is no direct `model.C.x -> model.A.x` record to satisfy a one-hop
/// lookup, yet the write must still admit under `modifies: [model.A.x]`,
/// since model-complete.md:56 makes this chain shape legal and ordinary.
///
/// Mutation used: in `redefinition_reaches`, replaced the loop bound
/// `0..=bound` with `0..=0` (a single iteration: check `field` itself, hop
/// once, then stop without checking the hop's own result). This test went
/// red as expected (`Refused(frame_violation/unauthorized-change)` instead
/// of `Admitted` -- the walk found `model.C.x -> model.B.x` but never
/// checked whether `model.B.x` itself reached a grant); reverted.
#[test]
#[trace("FR-046-AC-3")]
fn enforce_frame_admits_a_field_write_that_reaches_a_declared_grant_through_a_redefinition_chain() {
    let domain_package = redefinition_chain_bundle();
    let view = view_of(&domain_package);
    let pre_document = PopulationDocument {
        model_identity: "test/orders".to_owned(),
        members: vec![member_with_fields(
            "c1",
            "model.C",
            vec![("model.C.x", vec!["a1"])],
        )],
    };
    let post_document = PopulationDocument {
        model_identity: "test/orders".to_owned(),
        members: vec![member_with_fields(
            "c1",
            "model.C",
            vec![("model.C.x", vec!["a9"])],
        )],
    };
    let effect = OperationEffect {
        modifies: vec![DeclarationKey::fixture("model.A.x")],
        creates: Vec::new(),
        deletes: Vec::new(),
    };
    let declared = InvocationDelta {
        effect: &effect,
        declared_created: &[],
        declared_deleted: &[],
    };
    let mut pre_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let mut post_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let outcome = admit_invocation(
        invocation_context(&domain_package, &view, &p1_population_key()),
        &pre_document,
        &post_document,
        &declared,
        &mut pre_meter,
        &mut post_meter,
    );
    match outcome {
        AdmissionOutcome::Admitted(_) => {}
        other => panic!(
            "expected Admitted -- model.C.x reaches the declared model.A.x grant through the \
             two-hop chain model.C.x -> model.B.x -> model.A.x, got {other:?}"
        ),
    }
}

/// Item 5: an object that changes its most-specific type between pre and
/// post (`a1: model.A -> a1: model.B`) has no authorization path under any
/// declared frame -- FR-151's effect vocabulary grants only `modifies`/
/// `creates`/`deletes`, never a retype, and FR-143 binds the most-specific
/// type into an object reference's own identity -- so it always refuses,
/// even though `model.A`/`model.B` both appear in `creates`/`deletes` (which
/// would admit it if it were silently reinterpreted as an unrelated
/// delete-plus-create rather than caught as a type change).
///
/// Mutation used: in `enforce_frame`, replaced the type-change condition
/// (`*pre_type != *post_type`) with `false` (never refusing a type change).
/// This test went red as expected (`Admitted` instead of `Refused` -- the
/// invocation was silently admitted, `a1` ending up typed `model.B` in the
/// returned post binding); reverted.
#[test]
#[trace("FR-046-AC-3")]
fn enforce_frame_refuses_an_object_that_changes_type_between_pre_and_post() {
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let pre_document = PopulationDocument {
        model_identity: "test/orders".to_owned(),
        members: vec![member("a1", "model.A"), member("b1", "model.B")],
    };
    let post_document = PopulationDocument {
        model_identity: "test/orders".to_owned(),
        members: vec![member("a1", "model.B"), member("b1", "model.B")],
    };
    let effect = OperationEffect {
        modifies: Vec::new(),
        creates: vec![
            DeclarationKey::fixture("model.A"),
            DeclarationKey::fixture("model.B"),
        ],
        deletes: vec![
            DeclarationKey::fixture("model.A"),
            DeclarationKey::fixture("model.B"),
        ],
    };
    let declared = InvocationDelta {
        effect: &effect,
        declared_created: &[],
        declared_deleted: &[],
    };
    let mut pre_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let mut post_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let outcome = admit_invocation(
        invocation_context(&domain_package, &view, &p1_population_key()),
        &pre_document,
        &post_document,
        &declared,
        &mut pre_meter,
        &mut post_meter,
    );
    match outcome {
        AdmissionOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::FrameViolation);
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::FrameTypeChanged {
                    object: "a1".to_owned(),
                    pre_type: DeclarationKey::fixture("model.A"),
                    post_type: DeclarationKey::fixture("model.B"),
                }
            );
            assert!(refusal.detail.contains("a1"));
        }
        other => panic!(
            "expected Refused(frame_violation/unauthorized-change) -- a mid-lifetime type \
             change has no authorization path, got {other:?}"
        ),
    }
}

/// Item 8: a subtype (`model.B`) created and deleted under a supertype
/// (`model.A`) grant admits -- `enforce_frame` decides `creates`/`deletes`
/// membership by conformance (`crate::model::conformance::type_conforms`),
/// not exact type equality, exactly like `all_instances`'s own subtype
/// selection (`tests/model_population.rs`'s `l01_*` test).
#[test]
#[trace("TC-198", "FR-046-AC-3")]
fn invocation_admits_a_subtype_created_and_deleted_under_a_supertype_grant() {
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let pre_document = p1("test/orders"); // a1, a2, b1
    let post_document = PopulationDocument {
        model_identity: "test/orders".to_owned(),
        members: vec![
            member("a1", "model.A"),
            member("a2", "model.A"),
            member("b9", "model.B"),
        ],
    };
    let effect = OperationEffect {
        modifies: Vec::new(),
        creates: vec![DeclarationKey::fixture("model.A")],
        deletes: vec![DeclarationKey::fixture("model.A")],
    };
    let declared = InvocationDelta {
        effect: &effect,
        declared_created: &["b9".to_owned()],
        declared_deleted: &["b1".to_owned()],
    };
    let mut pre_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let mut post_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let outcome = admit_invocation(
        invocation_context(&domain_package, &view, &p1_population_key()),
        &pre_document,
        &post_document,
        &declared,
        &mut pre_meter,
        &mut post_meter,
    );
    match outcome {
        AdmissionOutcome::Admitted(_) => {}
        other => panic!(
            "expected Admitted -- model.B conforms to the declared model.A creates/deletes \
             grant, got {other:?}"
        ),
    }
}

/// PR #168 review round 5: `check_declared_delta`'s own `DuplicateDeclaredIdentity`
/// branch (FR-046: "no duplicate within either list") had no test of its
/// own -- every other `delta_mismatch` branch (overlap, full-set mismatch)
/// was exercised, but declaring the *same* identity twice in one list
/// (`created = ["o1", "o1"]`) never was. `Code::PopulationDeltaMismatch`/
/// cause `delta-disagreement`, the typed `DuplicateDeclaredIdentity {
/// identity: "o1" }` variant specifically.
///
/// Mutation used: in `check_declared_delta`, deleted the
/// `if !identities.insert(identity.clone()) { return Err(delta_mismatch(...)); }`
/// block entirely (the whole `DuplicateDeclaredIdentity` branch), leaving
/// only the overlap and full-set-equality checks below it. This test went
/// red as expected (`Admitted` instead of `Refused` -- `o1` is created and
/// the duplicate declaration collapses into the same single-membered set
/// the complete computed set already agrees with, once nothing catches the
/// duplicate itself); reverted.
#[test]
#[trace("FR-046-AC-3")]
fn invocation_refuses_a_declared_delta_that_declares_the_same_identity_twice() {
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let effect = OperationEffect {
        modifies: Vec::new(),
        creates: vec![DeclarationKey::fixture("model.A")],
        deletes: Vec::new(),
    };
    let post_document = PopulationDocument {
        model_identity: "test/orders".to_owned(),
        members: vec![
            member("a1", "model.A"),
            member("a2", "model.A"),
            member("b1", "model.B"),
            member("o1", "model.A"),
        ],
    };
    let declared = InvocationDelta {
        effect: &effect,
        declared_created: &["o1".to_owned(), "o1".to_owned()],
        declared_deleted: &[],
    };
    let mut pre_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let mut post_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let outcome = admit_invocation(
        invocation_context(&domain_package, &view, &p1_population_key()),
        &p1("test/orders"),
        &post_document,
        &declared,
        &mut pre_meter,
        &mut post_meter,
    );
    match outcome {
        AdmissionOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::PopulationDeltaMismatch);
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::DuplicateDeclaredIdentity {
                    identity: "o1".to_owned(),
                }
            );
        }
        other => {
            panic!("expected Refused(population_delta_mismatch/delta-disagreement), got {other:?}")
        }
    }
}

/// Item 4 (FR-046): the invocation's own declared created/deleted identity
/// lists must equal the complete computed sets [`enforce_frame`] derives
/// from the two documents. Here the operation's own frame authorizes
/// deleting `a2` (so the frame check itself passes), but the invocation
/// declares no delta at all -- `Code::PopulationDeltaMismatch`/cause
/// `delta-disagreement`, the same cause the native runtime's own
/// `declared_deltas` uses for this violation.
///
/// Mutation used: in `admit_invocation`, replaced the `enforce_frame(...)`
/// call's `Err` arm with an unconditional fallthrough to
/// `AdmissionOutcome::Admitted(...)` (never propagating a delta refusal).
/// This test went red as expected (`Admitted` instead of `Refused`);
/// reverted.
#[test]
#[trace("FR-046-AC-3")]
fn invocation_refuses_a_declared_delta_that_disagrees_with_the_complete_populations() {
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let effect = deletes_a_effect();
    let declared = InvocationDelta {
        effect: &effect,
        declared_created: &[],
        declared_deleted: &[], // wrong: a2 is actually deleted
    };
    let mut pre_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let mut post_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let outcome = admit_invocation(
        invocation_context(&domain_package, &view, &p1_population_key()),
        &p1("test/orders"),
        &p1_minus_a2("test/orders"),
        &declared,
        &mut pre_meter,
        &mut post_meter,
    );
    match outcome {
        AdmissionOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::PopulationDeltaMismatch);
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::DeclaredDeltaMismatch {
                    declared_created: BTreeSet::new(),
                    declared_deleted: BTreeSet::new(),
                    computed_created: BTreeSet::new(),
                    computed_deleted: ["a2".to_owned()].into_iter().collect(),
                }
            );
        }
        other => {
            panic!("expected Refused(population_delta_mismatch/delta-disagreement), got {other:?}")
        }
    }
}

/// FR-046's other declared-delta requirement: the same identity may not be
/// declared both created and deleted in one invocation, regardless of what
/// the two documents actually show (here, pre and post are identical, so
/// there is no real change at all).
///
/// No isolated mutation pins this test to `check_declared_delta`'s own
/// overlap check specifically: `computed_created`/`computed_deleted` are
/// always disjoint by construction (an object can only be classified
/// created *or* deleted, never both, in `enforce_frame`'s own loops), so a
/// declared pair that overlaps can never coincidentally equal both computed
/// sets either -- the final full-set-equality check below the overlap check
/// already refuses this same input on its own. Disabling only the overlap
/// check (removing its `if let Some(overlap) = ...` block) leaves this test
/// green, still refused via the equality check, with an unchanged code/cause
/// (`PopulationDeltaMismatch`/`delta-disagreement`) but a different detail
/// message. The overlap check is kept for a clearer diagnostic message and
/// to mirror the native runtime's own `declared_deltas` shape, not because
/// this test independently proves it fires.
#[test]
#[trace("FR-046-AC-3")]
fn invocation_refuses_a_declared_delta_that_declares_the_same_identity_created_and_deleted() {
    let domain_package = fixture_f1();
    let view = view_of(&domain_package);
    let effect = OperationEffect {
        modifies: Vec::new(),
        creates: vec![DeclarationKey::fixture("model.A")],
        deletes: vec![DeclarationKey::fixture("model.A")],
    };
    let declared = InvocationDelta {
        effect: &effect,
        declared_created: &["a2".to_owned()],
        declared_deleted: &["a2".to_owned()],
    };
    let mut pre_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let mut post_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let outcome = admit_invocation(
        invocation_context(&domain_package, &view, &p1_population_key()),
        &p1("test/orders"),
        &p1("test/orders"),
        &declared,
        &mut pre_meter,
        &mut post_meter,
    );
    match outcome {
        AdmissionOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::PopulationDeltaMismatch);
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::DeclaredCreateDeleteOverlap {
                    identity: "a2".to_owned(),
                }
            );
            // PR #168 review round 2, finding 6: pin the overlap check's own
            // detail text too, not just its code/cause -- now that `cause`
            // is the typed `DeclaredCreateDeleteOverlap` variant, the
            // `assert_eq!` above already discriminates this branch from the
            // other `delta_mismatch` call sites on its own, but the detail
            // text stays pinned as well for the message-content guarantee.
            assert!(
                refusal.detail.contains("both created and deleted"),
                "expected the overlap check's own detail text, got {:?}",
                refusal.detail
            );
        }
        other => {
            panic!("expected Refused(population_delta_mismatch/delta-disagreement), got {other:?}")
        }
    }
}
