// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-196: exact standalone conformance axes and composed refinement
//! through genuinely admitted source clauses. All pricing uses the owning
//! meter and the actual normalized qualify/inherit facts.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::model::accounting::{Meter, ModelNormalizationLimits};
use qsl_semantics::model::conformance::{
    check_field_redefinition, check_operation_redefinition, check_subsetting,
    resolve_redefinition_target, AxisFailure, ConformanceCheckOutcome, ConformanceOutcome,
    RedefinitionTargetOutcome,
};
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, FieldMemberRecord, Multiplicity,
    ObjectTypeRecord, OperationEffect, OperationMemberRecord, OperationParameterRecord,
    OperationResult, ScalarTypeRecord, ValueTypeRef,
};
use qsl_semantics::model::index::ModelIndex;
use qsl_semantics::model::key::{DeclarationKey, EffectiveId, RULE_REDEFINE};
use qsl_semantics::model::normalize::{
    normalize, EffectiveView, ModelRefusalCause, NormalizeOutcome, ViewEntry,
};

fn mult(lower: u64, upper: Option<u64>) -> Multiplicity {
    Multiplicity {
        lower,
        upper,
        ordered: false,
        unique: true,
    }
}

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

fn field_member(
    identity: &str,
    owner: &str,
    value_type: &str,
    m: Multiplicity,
) -> DomainPackageRecord {
    field_member_redefining(identity, owner, value_type, m, None, vec![])
}

/// `field_member`, plus this field's own inline `redefines`
/// (`model-complete.md`:162) and `subsets` (`model-complete.md`:161)
/// properties -- never a separate redefinition or subsetting record.
fn field_member_redefining(
    identity: &str,
    owner: &str,
    value_type: &str,
    m: Multiplicity,
    redefines: Option<&str>,
    subsets: Vec<&str>,
) -> DomainPackageRecord {
    DomainPackageRecord::FieldMember(FieldMemberRecord {
        key: DeclarationKey::fixture(identity),
        owner: DeclarationKey::fixture(owner),
        value_type: ValueTypeRef::Package(DeclarationKey::fixture(value_type)),
        multiplicity: m,
        presence: quire_exact::Presence::Required,
        subsets: subsets.into_iter().map(DeclarationKey::fixture).collect(),
        redefines: redefines.map(DeclarationKey::fixture),
    })
}

fn scalar_type(identity: &str, lower: i128, upper: i128) -> DomainPackageRecord {
    DomainPackageRecord::ScalarType(ScalarTypeRecord {
        key: DeclarationKey::fixture(identity),
        lower,
        upper,
    })
}

#[allow(clippy::too_many_arguments)]
fn operation(
    identity: &str,
    owner: &str,
    parameters: Vec<(&str, &str, Multiplicity)>,
    result: Option<(&str, Multiplicity)>,
    modifies: Vec<&str>,
    creates: Vec<&str>,
    deletes: Vec<&str>,
) -> DomainPackageRecord {
    operation_redefining(
        identity,
        owner,
        parameters,
        result,
        modifies,
        creates,
        deletes,
        None,
    )
}

/// `operation`, plus this operation's own inline `redefines`
/// (`model-complete.md`:162) property -- never a separate redefinition
/// record.
#[allow(clippy::too_many_arguments)]
fn operation_redefining(
    identity: &str,
    owner: &str,
    parameters: Vec<(&str, &str, Multiplicity)>,
    result: Option<(&str, Multiplicity)>,
    modifies: Vec<&str>,
    creates: Vec<&str>,
    deletes: Vec<&str>,
    redefines: Option<&str>,
) -> DomainPackageRecord {
    DomainPackageRecord::OperationMember(OperationMemberRecord {
        key: DeclarationKey::fixture(identity),
        owner: DeclarationKey::fixture(owner),
        parameters: parameters
            .into_iter()
            .map(|(id, ty, m)| OperationParameterRecord {
                key: DeclarationKey::fixture(id),
                value_type: ValueTypeRef::Package(DeclarationKey::fixture(ty)),
                multiplicity: m,
            })
            .collect(),
        result: result.map(|(ty, m)| OperationResult {
            value_type: ValueTypeRef::Package(DeclarationKey::fixture(ty)),
            multiplicity: m,
        }),
        effect: OperationEffect {
            modifies: modifies.into_iter().map(DeclarationKey::fixture).collect(),
            creates: creates.into_iter().map(DeclarationKey::fixture).collect(),
            deletes: deletes.into_iter().map(DeclarationKey::fixture).collect(),
        },
        has_body: true,
        redefines: redefines.map(DeclarationKey::fixture),
    })
}

/// H (domain package `bundle.h`): types `A`, `B` (`B` <= `A`); field `model.A.x`
/// typed `model.A` `{0,1}`; field `model.B.y` typed `model.A` `{0,1}`;
/// operation `model.A.op(p1: model.A {0,1})`: `model.B {1,1}`, effect
/// `{modifies: [model.A.x], creates: [model.A], deletes: []}`.
fn fixture_h_base() -> Vec<DomainPackageRecord> {
    vec![
        object_type("model.A", vec![]),
        object_type("model.B", vec!["model.A"]),
        field_member("model.A.x", "model.A", "model.A", mult(0, Some(1))),
        field_member("model.B.y", "model.B", "model.A", mult(0, Some(1))),
        operation(
            "model.A.op",
            "model.A",
            vec![("model.A.op.p1", "model.A", mult(0, Some(1)))],
            Some(("model.B", mult(1, Some(1)))),
            vec!["model.A.x"],
            vec!["model.A"],
            vec![],
        ),
    ]
}

#[trace("FR-151-AC-4", "FR-151-AC-11", "QSpec-TC-196")]
#[test]
fn operation_axes_charge_the_selected_type_facts_and_complete_effect_comparisons() {
    use qsl_semantics::model::accounting::LimitKind;

    for (row, expected_work, expected_failures) in [(2, 10, 0), (3, 8, 5), (4, 8, 1), (9, 10, 0)] {
        let mut records = fixture_h_base();
        if row == 9 {
            records.push(field_member_redefining("model.B.w", "model.B", "model.A",
                mult(0, Some(1)), Some("model.A.x"), vec![]));
        }
        records.push(operation_redefining("model.B.op", "model.B",
            if row == 4 { vec![] } else { vec![("model.B.op.p1",
                if row == 3 { "model.B" } else { "model.A" },
                if row == 3 { mult(1, Some(1)) } else { mult(0, Some(2)) })] },
            Some((if row == 3 { "model.A" } else { "model.B" },
                if row == 3 { mult(0, Some(1)) } else { mult(1, Some(1)) })),
            if row == 3 { vec!["model.B.y"] } else if row == 9 { vec!["model.B.w"] } else { vec![] },
            if [2, 4].contains(&row) { vec!["model.B"] } else { vec![] },
            vec![], Some("model.A.op")));
        let package = DomainPackage::new(DomainPackageRef::fixture("bundle.h"), records);
        let view = match normalize(&package, ModelNormalizationLimits::UNLIMITED) {
            NormalizeOutcome::Completed(view) => view,
            other => panic!("structural normalization must admit R{row}: {other:?}"),
        };
        let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
        let result = check_operation_redefinition(view.model_index(),
            &DeclarationKey::fixture("model.B.op"), &DeclarationKey::fixture("model.A.op"), &mut meter);
        let failures = match result {
            ConformanceCheckOutcome::Completed(ConformanceOutcome::Compatible) => 0,
            ConformanceCheckOutcome::Completed(ConformanceOutcome::Refused(failures)) => failures.len(),
            other => panic!("all charges must complete: {other:?}"),
        };
        assert_eq!(failures, expected_failures, "R{row} independent failures");
        assert_eq!(meter.consumed(LimitKind::WorkUnits), expected_work, "R{row} work pricing");
    }
}

#[trace("FR-082-AC-3", "QSpec-TC-196")]
#[test]
fn ancestor_availability_is_per_walk_and_preserves_the_exact_denied_edge_record() {
    use qsl_semantics::model::accounting::{ChargePoint, Incomplete, LimitKind};

    for branching in [false, true] {
        let mut records = vec![object_type("model.A", vec![]),
            object_type("model.B", vec!["model.A"]),
            object_type("model.C", if branching { vec!["model.B", "model.D"] } else { vec!["model.B"] }),
            field_member("model.A.n", "model.A", "model.A", mult(0, Some(1))),
            field_member_redefining("model.B.n", "model.B", "model.C", mult(0, Some(1)),
                Some("model.A.n"), vec![])];
        if branching { records.push(object_type("model.D", vec![])); }
        let package = DomainPackage::new(DomainPackageRef::fixture("test/ancestor-axis"), records);
        let view = match normalize(&package, ModelNormalizationLimits::UNLIMITED) {
            NormalizeOutcome::Completed(view) => view,
            other => panic!("structural fixture must normalize: {other:?}"),
        };
        let edges = if branching { 3 } else { 2 };
        let work = if branching { 4 } else { 3 };
        for limit in [0, edges - 1, edges] {
            let normalized = normalize(&package, ModelNormalizationLimits {
                ancestor_steps: limit, ..ModelNormalizationLimits::UNLIMITED
            });
            if limit == edges {
                assert!(matches!(normalized, NormalizeOutcome::Completed(_)));
            } else {
                assert_eq!(normalized, NormalizeOutcome::Incomplete(Incomplete {
                    limit_kind: LimitKind::AncestorSteps, limit, consumed: limit, next_charge: 1,
                    charge_point: ChargePoint::ModelAncestorEdge,
                }));
            }
            let mut meter = Meter::new(ModelNormalizationLimits {
                ancestor_steps: limit, ..ModelNormalizationLimits::UNLIMITED
            });
            let result = check_field_redefinition(view.model_index(), &DeclarationKey::fixture("model.B.n"),
                &DeclarationKey::fixture("model.A.n"), &mut meter);
            if limit == edges {
                assert_eq!(result, ConformanceCheckOutcome::Completed(ConformanceOutcome::Compatible));
                assert_eq!(meter.consumed(LimitKind::WorkUnits), work + 1);
            } else {
                assert_eq!(result, ConformanceCheckOutcome::Incomplete(Incomplete {
                    limit_kind: LimitKind::AncestorSteps, limit, consumed: limit, next_charge: 1,
                    charge_point: ChargePoint::ModelAncestorEdge,
                }));
                assert_eq!(meter.consumed(LimitKind::WorkUnits), work,
                    "admitted owning axis is retained before the denied edge");
            }
        }
    }
}

#[trace("FR-082-AC-3", "QSpec-TC-196")]
#[test]
fn an_unavailable_owning_axis_wins_before_an_unavailable_ancestor_edge() {
    use qsl_semantics::model::accounting::{ChargePoint, Incomplete, LimitKind};
    let package = DomainPackage::new(DomainPackageRef::fixture("test/ancestor-precedence"), vec![
        object_type("model.A", vec![]), object_type("model.B", vec!["model.A"]),
        field_member("model.A.n", "model.A", "model.A", mult(0, Some(1))),
        field_member_redefining("model.B.n", "model.B", "model.B", mult(0, Some(1)),
            Some("model.A.n"), vec![]),
    ]);
    let index = ModelIndex::build(package);
    for work_units in [0, 2] {
        let mut meter = Meter::new(ModelNormalizationLimits {
            ancestor_steps: 0, work_units, ..ModelNormalizationLimits::UNLIMITED
        });
        let result = check_field_redefinition(&index, &DeclarationKey::fixture("model.B.n"),
            &DeclarationKey::fixture("model.A.n"), &mut meter);
        let expected = if work_units == 0 {
            Incomplete { limit_kind: LimitKind::WorkUnits, limit: 0, consumed: 0,
                next_charge: 2, charge_point: ChargePoint::ConformanceAxis }
        } else {
            Incomplete { limit_kind: LimitKind::AncestorSteps, limit: 0, consumed: 0,
                next_charge: 1, charge_point: ChargePoint::ModelAncestorEdge }
        };
        assert_eq!(result, ConformanceCheckOutcome::Incomplete(expected));
        assert_eq!(meter.consumed(LimitKind::WorkUnits), work_units);
    }
}

fn bundle_h(mut records: Vec<DomainPackageRecord>) -> DomainPackage {
    records.extend(fixture_h_base());
    DomainPackage::new(DomainPackageRef::fixture("bundle.h"), records)
}

/// Finds the effective TYPE entry (no owner) declared with original identity
/// `identity` — mirrors `tests/model_normalization.rs`'s own helper, which
/// this integration-test binary cannot import directly.
fn find_type<'a>(view: &'a EffectiveView, identity: &str) -> &'a ViewEntry {
    view.declarations()
        .iter()
        .find(|entry| {
            entry.preimage.owner_effective_type.is_none()
                && entry.preimage.original.node == identity
        })
        .unwrap_or_else(|| panic!("no effective type {identity} in {view:?}"))
}

/// Finds the effective MEMBER entry declared with original identity
/// `original_identity` under owner effective type `owner`, regardless of its
/// `visible` bit — phase 4 retains hidden entries in the view.
fn find_member<'a>(
    view: &'a EffectiveView,
    owner: &EffectiveId,
    original_identity: &str,
) -> &'a ViewEntry {
    view.declarations()
        .iter()
        .find(|entry| {
            entry.preimage.owner_effective_type.as_ref() == Some(owner)
                && entry.preimage.original.node == original_identity
        })
        .unwrap_or_else(|| panic!("no member {original_identity} owned by {owner:?} in {view:?}"))
}

/// A compatible field redefinition (FR-151's own variance rule admits it)
/// must fold, under FR-150's phase 4, to exactly one visible effective
/// member at the redefining owner — with complete provenance: the winner's
/// derivation names both the redefinition record and what it redefines, and
/// the redefined original is retained (for provenance) but hidden. This is
/// FR-151-AC-1's own language ("one effective member with complete
/// provenance"), checked as the join of `check_field_redefinition`'s
/// Compatible outcome and `normalize`'s resulting view — distinct from
/// FR-150-AC-1's own test (`tests/model_normalization.rs`'s n06 case),
/// which exercises a *conflict* resolved by dominance, not a single
/// uncontested redefiner.
#[trace("QSpec-TC-196", "QSpec-FR-151-AC-1")]
#[test]
fn r01_a_compatible_field_redefinition_yields_one_effective_member_with_complete_provenance() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.r01"),
        vec![
            object_type("model.A", vec![]),
            object_type("model.B", vec!["model.A"]),
            field_member("model.A.n", "model.A", "model.A", mult(0, Some(5))),
            field_member_redefining(
                "model.B.n",
                "model.B",
                "model.A",
                mult(1, Some(3)),
                Some("model.A.n"),
                vec![],
            ),
        ],
    );
    let redefining_key = DeclarationKey::fixture("model.B.n");
    let redefined_key = DeclarationKey::fixture("model.A.n");

    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    match check_field_redefinition(
        &ModelIndex::build(domain_package.clone()),
        &redefining_key,
        &redefined_key,
        &mut meter,
    ) {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Compatible) => {}
        other => panic!("expected Compatible ({{1,3}} conforms to {{0,5}}), got {other:?}"),
    }

    let view = match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Completed(view) => view,
        other => panic!("expected a completed view, got {other:?}"),
    };
    let owner_b = find_type(&view, "model.B").effective_id;

    let winner = find_member(&view, &owner_b, "model.B.n");
    assert!(
        winner.visible,
        "B.n is the sole, uncontested redefiner of A.n at B: it must win outright"
    );
    let winner_redefine: Vec<_> = winner
        .preimage
        .derivation
        .iter()
        .filter(|fact| fact.rule == RULE_REDEFINE)
        .collect();
    assert_eq!(
        winner_redefine.len(),
        1,
        "exactly one redefine fact links B.n to what it redefines"
    );
    assert_eq!(
        winner_redefine[0].inputs,
        vec![
            DeclarationKey::fixture("model.B.n"),
            DeclarationKey::fixture("model.A.n"),
        ]
    );

    let hidden = find_member(&view, &owner_b, "model.A.n");
    assert!(
        !hidden.visible,
        "A.n is retained at B for provenance but is not the effective member there"
    );
    let hidden_redefine: Vec<_> = hidden
        .preimage
        .derivation
        .iter()
        .filter(|fact| fact.rule == RULE_REDEFINE)
        .collect();
    assert_eq!(
        hidden_redefine.len(),
        1,
        "exactly one redefine fact, from B.n's own edge"
    );
    assert_eq!(
        hidden_redefine[0].inputs,
        vec![
            DeclarationKey::fixture("model.B.n"),
            DeclarationKey::fixture("model.A.n"),
        ]
    );

    let visible_members_at_b: Vec<_> = view
        .declarations()
        .iter()
        .filter(|entry| {
            entry.preimage.owner_effective_type.as_ref() == Some(&owner_b) && entry.visible
        })
        .collect();
    assert_eq!(
        visible_members_at_b.len(),
        1,
        "B has exactly one visible effective member for this slot: B.n, never a second silently-surviving one"
    );
}

/// FR-154: "Two nodes share one identity" refuses
/// `invalid_model_binding`/`conflicting-binding`. Retargets the pre-#131
/// `f2_field_members_sharing_an_identity_but_differing_in_revision_both_survive`
/// regression test: under the dropped `revision`/`digest` fields, two
/// `FieldMember` records that once differed only in `revision` now share the
/// exact same `DeclarationKey` and `normalize` must refuse before this
/// module's own conformance check ever runs.
#[trace("QSpec-TC-196")]
#[test]
fn two_field_members_sharing_one_declaration_key_refuse_conflicting_binding() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.f2-conformance-conflict"),
        vec![
            object_type("model.A", vec![]),
            object_type("model.B", vec![]),
            field_member("model.A.x", "model.A", "model.A", mult(0, Some(1))),
            field_member("model.A.x", "model.A", "model.A", mult(0, Some(5))),
            field_member("model.B.y", "model.B", "model.A", mult(0, Some(3))),
        ],
    );
    match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.len(),
                1,
                "expected exactly one refusal: {refusal:?}"
            );
            let refusal = refusal[0].clone();
            assert_eq!(refusal.code, Code::InvalidModelBinding);
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::ConflictingBinding {
                    key: DeclarationKey::fixture("model.A.x"),
                }
            );
        }
        other => {
            panic!("expected Refused(invalid_model_binding/conflicting-binding), got {other:?}")
        }
    }
}

// AC-1 is `r01`'s own subject (`check_operation_redefinition` builds no
// effective member — `normalize.rs`'s own module doc says operation
// redefinition builds no phase there). AC-4 is this test's real subject:
// every variance axis admitting independently.
#[trace("QSpec-TC-196", "QSpec-FR-151-AC-4")]
#[test]
fn r02_a_compatible_operation_redefinition_admits_every_axis() {
    let domain_package = bundle_h(vec![operation_redefining(
        "model.B.op",
        "model.B",
        vec![("model.B.op.p1", "model.A", mult(0, Some(2)))],
        Some(("model.B", mult(1, Some(1)))),
        vec![],
        vec!["model.B"],
        vec![],
        Some("model.A.op"),
    )]);
    let redefining_key = DeclarationKey::fixture("model.B.op");
    let redefined_key = DeclarationKey::fixture("model.A.op");
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    match check_operation_redefinition(
        &ModelIndex::build(domain_package.clone()),
        &redefining_key,
        &redefined_key,
        &mut meter,
    ) {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Compatible) => {}
        other => panic!("expected Compatible, got {other:?}"),
    }
}

#[trace("TC-218", "FR-082-AC-1")]
#[test]
fn r03_an_incompatible_operation_redefinition_reports_every_failing_axis() {
    let domain_package = bundle_h(vec![operation_redefining(
        "model.B.op",
        "model.B",
        vec![("model.B.op.p1", "model.B", mult(1, Some(1)))],
        Some(("model.A", mult(0, Some(1)))),
        vec!["model.B.y"],
        vec![],
        vec![],
        Some("model.A.op"),
    )]);
    let redefining_key = DeclarationKey::fixture("model.B.op");
    let redefined_key = DeclarationKey::fixture("model.A.op");
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    match check_operation_redefinition(
        &ModelIndex::build(domain_package.clone()),
        &redefining_key,
        &redefined_key,
        &mut meter,
    ) {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Refused(failures)) => {
            let causes: Vec<ModelRefusalCause> = failures.iter().map(|f| f.cause.clone()).collect();
            assert_eq!(causes.len(), 5);
            assert_eq!(
                causes[0],
                ModelRefusalCause::VarianceParameter {
                    index: 1,
                    declared: ValueTypeRef::Package(DeclarationKey::fixture("model.A")),
                    redefined: ValueTypeRef::Package(DeclarationKey::fixture("model.B")),
                }
            );
            assert_eq!(
                causes[1],
                ModelRefusalCause::MultiplicityNarrowing {
                    from: mult(0, Some(1)),
                    to: mult(1, Some(1)),
                }
            );
            assert_eq!(causes[2], ModelRefusalCause::VarianceResult);
            assert_eq!(
                causes[3],
                ModelRefusalCause::MultiplicityNarrowing {
                    from: mult(0, Some(1)),
                    to: mult(1, Some(1)),
                }
            );
            assert_eq!(
                causes[4],
                ModelRefusalCause::EffectEscape {
                    field: DeclarationKey::fixture("model.B.y"),
                }
            );
            assert!(failures.iter().all(|f| f.code == Code::IllTyped));
        }
        other => panic!("expected Refused, got {other:?}"),
    }
}

#[trace("TC-239", "FR-082-AC-5")]
#[test]
fn r04_an_arity_mismatch_refuses_without_checking_parameter_axes() {
    let domain_package = bundle_h(vec![operation_redefining(
        "model.B.op",
        "model.B",
        vec![],
        Some(("model.B", mult(1, Some(1)))),
        vec![],
        vec!["model.B"],
        vec![],
        Some("model.A.op"),
    )]);
    let redefining_key = DeclarationKey::fixture("model.B.op");
    let redefined_key = DeclarationKey::fixture("model.A.op");
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    match check_operation_redefinition(
        &ModelIndex::build(domain_package.clone()),
        &redefining_key,
        &redefined_key,
        &mut meter,
    ) {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Refused(failures)) => {
            assert_eq!(failures.len(), 1);
            assert_eq!(failures[0].cause, ModelRefusalCause::TypeMismatch);
            assert_eq!(failures[0].code, Code::IllTyped);
        }
        other => panic!("expected Refused, got {other:?}"),
    }
}

#[trace("QSpec-TC-196", "QSpec-FR-151-AC-2")]
#[test]
fn r05_field_multiplicity_narrowing_refuses_and_the_boundary_admits() {
    let records = |a_mult: Multiplicity, b_mult: Multiplicity| {
        vec![
            object_type("model.A", vec![]),
            object_type("model.B", vec!["model.A"]),
            field_member("model.A.n", "model.A", "model.A", a_mult),
            field_member_redefining(
                "model.B.n",
                "model.B",
                "model.A",
                b_mult,
                Some("model.A.n"),
                vec![],
            ),
        ]
    };
    let redefining_key = DeclarationKey::fixture("model.B.n");
    let redefined_key = DeclarationKey::fixture("model.A.n");

    let narrowing = DomainPackage::new(
        DomainPackageRef::fixture("bundle.r05a"),
        records(mult(1, Some(3)), mult(0, Some(5))),
    );
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    match check_field_redefinition(
        &ModelIndex::build(narrowing.clone()),
        &redefining_key,
        &redefined_key,
        &mut meter,
    ) {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Refused(failures)) => {
            assert_eq!(failures.len(), 1);
            assert_eq!(
                failures[0].cause,
                ModelRefusalCause::MultiplicityNarrowing {
                    from: mult(0, Some(5)),
                    to: mult(1, Some(3)),
                }
            );
        }
        other => panic!("expected Refused, got {other:?}"),
    }

    let widening = DomainPackage::new(
        DomainPackageRef::fixture("bundle.r05b"),
        records(mult(0, Some(5)), mult(1, Some(3))),
    );
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    match check_field_redefinition(
        &ModelIndex::build(widening.clone()),
        &redefining_key,
        &redefined_key,
        &mut meter,
    ) {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Compatible) => {}
        other => panic!("expected Compatible, got {other:?}"),
    }
}

#[trace("QSpec-TC-196", "QSpec-FR-151-AC-2")]
#[test]
fn r06_subsetting_type_and_multiplicity_axes() {
    let bundle_of = |some_type: &str, some_mult: Multiplicity| {
        DomainPackage::new(
            DomainPackageRef::fixture("bundle.r06"),
            vec![
                object_type("model.A", vec![]),
                object_type("model.B", vec!["model.A"]),
                object_type("model.C", vec![]),
                field_member("model.A.all", "model.A", "model.A", mult(0, Some(5))),
                field_member_redefining(
                    "model.A.some",
                    "model.A",
                    some_type,
                    some_mult,
                    None,
                    vec!["model.A.all"],
                ),
            ],
        )
    };
    let subsetting_key = DeclarationKey::fixture("model.A.some");
    let subsetted_key = DeclarationKey::fixture("model.A.all");

    // (i) same type, wider multiplicity: multiplicity-narrowing.
    let domain_package = bundle_of("model.A", mult(0, Some(9)));
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    match check_subsetting(
        &ModelIndex::build(domain_package.clone()),
        &subsetting_key,
        &subsetted_key,
        &mut meter,
    ) {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Refused(failures)) => {
            assert_eq!(failures.len(), 1);
            assert_eq!(
                failures[0].cause,
                ModelRefusalCause::MultiplicityNarrowing {
                    from: mult(0, Some(9)),
                    to: mult(0, Some(5)),
                }
            );
        }
        other => panic!("expected Refused, got {other:?}"),
    }

    // (ii) unrelated type: subsetting-type.
    let domain_package = bundle_of("model.C", mult(0, Some(5)));
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    match check_subsetting(
        &ModelIndex::build(domain_package.clone()),
        &subsetting_key,
        &subsetted_key,
        &mut meter,
    ) {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Refused(failures)) => {
            assert_eq!(failures.len(), 1);
            assert_eq!(
                failures[0].cause,
                ModelRefusalCause::SubsettingType {
                    subsetting: ValueTypeRef::Package(DeclarationKey::fixture("model.C")),
                    subsetted: ValueTypeRef::Package(DeclarationKey::fixture("model.A")),
                }
            );
        }
        other => panic!("expected Refused, got {other:?}"),
    }

    // (iii) conforming subtype, narrower multiplicity: admitted.
    let domain_package = bundle_of("model.B", mult(0, Some(3)));
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    match check_subsetting(
        &ModelIndex::build(domain_package.clone()),
        &subsetting_key,
        &subsetted_key,
        &mut meter,
    ) {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Compatible) => {}
        other => panic!("expected Compatible, got {other:?}"),
    }
}

// Only the "zero inherited targets" shape is directly testable through
// `resolve_redefinition_target` under QSpec's inline `redefines` property
// (`model-complete.md`:162): a field or operation member declares at most
// one `redefines`, never several competing candidates. TC-196 R07's "two
// distinct, both-legitimate inherited targets" shape is therefore
// structurally unreachable here and is dropped rather than adapted.
#[trace("TC-219", "FR-082-AC-2")]
#[test]
fn r07_zero_inherited_targets_refuses_redefinition_target() {
    // Zero inherited targets: B.z claims to redefine C.w, but B does not
    // conform to C.
    let zero = DomainPackage::new(
        DomainPackageRef::fixture("bundle.r07a"),
        vec![
            object_type("model.A", vec![]),
            object_type("model.B", vec!["model.A"]),
            object_type("model.C", vec![]),
            field_member("model.C.w", "model.C", "model.A", mult(0, Some(1))),
            field_member_redefining(
                "model.B.z",
                "model.B",
                "model.A",
                mult(0, Some(1)),
                Some("model.C.w"),
                vec![],
            ),
        ],
    );
    match resolve_redefinition_target(
        &ModelIndex::build(zero.clone()),
        &DeclarationKey::fixture("model.B.z"),
        ModelNormalizationLimits::UNLIMITED.ancestor_steps,
    ) {
        Ok(RedefinitionTargetOutcome::Refused { cause, candidate }) => {
            assert_eq!(
                cause,
                ModelRefusalCause::RedefinitionTarget {
                    redefiners: vec![DeclarationKey::fixture("model.B.z")],
                    target: DeclarationKey::fixture("model.C.w"),
                }
            );
            assert_eq!(
                candidate,
                Some((
                    DeclarationKey::fixture("model.B.z"),
                    DeclarationKey::fixture("model.C.w"),
                ))
            );
        }
        other => panic!("expected Refused(zero targets), got {other:?}"),
    }
}

// TC-196 R07's other shape — two distinct redefining members (`B/z` and
// `B/z2`) contending for the identical single inherited target — is no
// longer tested via `resolve_redefinition_target` directly: nothing in
// `src/` calls that function, so a per-redefiner query here can never see
// the sibling that contends with it (each of `B/z` and `B/z2` resolves, on
// its own, to a single valid target — `Resolved`, not `Refused`). That
// shape is covered instead where a model actually normalizes through it:
// `r07_two_redefiners_owned_by_the_same_type_refuse_redefinition_target_through_normalize`
// in `tests/model_normalization.rs`, against `normalize`'s own phase 4.

#[trace("TC-221", "FR-082-AC-4")]
#[test]
fn r08a_a_narrowing_field_redefinition_without_a_presence_fact_refuses() {
    assert_eq!(crate::model_conformance_boundary::refinement_vector(0), Err(vec![(Code::UndefinedExpression, Some("unproved-refinement"))]));
}

#[trace("TC-221", "FR-082-AC-4")]
#[test]
fn r08b_a_redefined_operation_with_the_presence_fact_discharges_the_obligation() {
    assert_eq!(crate::model_conformance_boundary::refinement_vector(1), Ok(()));
}

#[trace("QSpec-TC-196", "QSpec-FR-151-AC-6")]
#[test]
fn r08c_an_object_typed_narrowing_has_no_proof_form() {
    assert_eq!(crate::model_conformance_boundary::refinement_vector(2), Err(vec![(Code::UndefinedExpression, Some("unproved-refinement"))]));
}

#[trace("QSpec-TC-196", "QSpec-FR-151-AC-6")]
#[test]
fn r08d_a_narrowed_scalar_domain_without_an_interval_fact_refuses_field_domain() {
    assert_eq!(crate::model_conformance_boundary::refinement_vector(3), Err(vec![(Code::UndefinedExpression, Some("unproved-refinement"))]));
}

#[trace("QSpec-TC-196", "QSpec-FR-151-AC-6")]
#[test]
fn r08e_and_r08f_an_established_interval_admits_only_when_contained() {
    assert_eq!(crate::model_conformance_boundary::refinement_vector(4), Ok(()));
    assert_eq!(crate::model_conformance_boundary::refinement_vector(5),
        Err(vec![(Code::UndefinedExpression, Some("unproved-refinement"))]));
}

/// FR-082-AC-9 (TC-911 step 3): a field of scalar type `Wide`
/// (`[0, u64::MAX]`) redefined by a field of `Narrow` (`[0, i64::MAX + 1]`)
/// is decided exactly: a postcondition `self.f <= 9223372036854775808`
/// admits the redefinition, `self.f <= 9223372036854775809` refuses
/// `unproved-refinement` with obligation `field-domain`.
#[trace("TC-911", "FR-082-AC-9")]
#[test]
fn a_wide_domain_and_a_literal_above_i64_max_are_decided_exactly() {
    use qsl_semantics::check::{CheckCause, PackageDeclarations, RefinementObligation};
    use qsl_semantics::model::intake::SelectedModels;
    use quire_semantic_value::checking::CheckingLimits;
    use crate::model_operations::parse_and_build;

    // The exact record fixture avoids a lossy JCS encoding above 2^53,
    // while its source clauses use the normal parser and checker.
    for (literal, succeeds) in [(9_223_372_036_854_775_808_i128, true),
        (9_223_372_036_854_775_809, false)] {
        let package = DomainPackage::new(DomainPackageRef::fixture("bundle.wide"), vec![
            object_type("ix://test/orders/A", vec![]),
            object_type("ix://test/orders/B", vec!["ix://test/orders/A"]),
            scalar_type("ix://test/orders/Wide", 0, i128::from(u64::MAX)),
            scalar_type("ix://test/orders/Narrow", 0, 9_223_372_036_854_775_808),
            field_member("ix://test/orders/A/f", "ix://test/orders/A", "ix://test/orders/Wide", mult(1, Some(1))),
            field_member_redefining("ix://test/orders/B/fn", "ix://test/orders/B", "ix://test/orders/Narrow",
                mult(1, Some(1)), Some("ix://test/orders/A/f"), vec![]),
            operation("ix://test/orders/A/set", "ix://test/orders/A", vec![], None,
                vec!["ix://test/orders/A/f"], vec![], vec![]),
            operation_redefining("ix://test/orders/B/set", "ix://test/orders/B", vec![], None,
                vec!["ix://test/orders/A/f"], vec![], vec![], Some("ix://test/orders/A/set")),
        ]);
        let digest = qsl_semantics::model::key::hex(&package.model_selection.digest);
        let unit = format!("language \"ix:native\" edition \"1-draft\";\n\
            profile v = \"quire.value.complete/v1\";\n\
            model M = \"test/orders\" version \"1\" digest \"sha256-jcs:{digest}\";\n\
            post Q using v on M::B::set {{ self.fn <= {literal} }}\n");
        let selected = SelectedModels::fixture("M", qsl_foundation::Span { start: 0, end: 1 },
            package, ModelNormalizationLimits::UNLIMITED).expect("actual private normalization owner");
        let built = parse_and_build(&unit);
        let raw = qsl_cst::parse(qsl_foundation::SourceIdentity::new("test", "tc-911", "fixture", "1"),
            "unit.native", unit.as_bytes(), qsl_cst::Limits::default()).expect("source admits");
        let declarations = PackageDeclarations::assemble(raw.source().reference().clone(), built, selected, vec![])
            .expect("model-owned narrowing waits for its checked clause");
        let result = declarations.check(CheckingLimits::default());
        if succeeds { assert!(result.is_ok(), "exact 2^63 bound: {result:?}"); }
        else {
            let refused = result.expect_err("2^63+1 does not fit the narrowed interval");
            assert_eq!(refused.len(), 1);
            let CheckCause::ModelConformance(failure) = &refused[0].cause else {
                panic!("actual refinement failure: {refused:?}");
            };
            assert_eq!(failure.cause, ModelRefusalCause::UnprovedRefinement);
            assert_eq!(failure.obligation, Some(RefinementObligation::Domain));
            assert_eq!(failure.member, DeclarationKey::fixture("ix://test/orders/B/fn"));
            assert_eq!(failure.parent, DeclarationKey::fixture("ix://test/orders/A/f"));
            assert_eq!(failure.writer, Some(DeclarationKey::fixture("ix://test/orders/B/set")));
        }
    }
}

/// A postcondition clause that does not actually establish the narrowed
/// domain (`model.B.cs >= 0`, which every value of a scalar with floor 0
/// already satisfies) must still refuse: the obligation is discharged by
/// real fact derivation over the declared clause, not by trusting that any
/// clause naming the field is sufficient.
#[trace("QSpec-TC-196", "QSpec-FR-151-AC-6")]
#[test]
fn r08g_an_unrelated_clause_over_the_same_field_does_not_discharge_the_obligation() {
    crate::model_conformance_boundary::unrelated_numeric_clause();
}

/// Review of PR #157 finding #1 (and its re-review, finding #5): the
/// effective postcondition is a conjunction, so two clauses over the same
/// field must be proved *together*, not by keeping only whichever one
/// happened to be derived first — and, unlike `r08_base`'s `Count [0,9]`,
/// this test's own domain (`Count [-5,9]`) makes neither clause alone
/// sufficient, in *either* declaration order: `model.B.cs >= 0` alone
/// establishes only `[0,9]` (the domain's own upper bound, not narrowed by
/// this clause) and `model.B.cs <= 5` alone establishes only `[-5,5]` (the
/// domain's own lower bound, not narrowed by this clause) — neither is
/// contained in `Small`'s `[0,5]`. Folded into one `cs >= 0 AND cs <= 5`
/// guard, they establish exactly `[0,5]`, which is. The old `find_map`-first
/// bug refused in both orders under this domain (a domain of `[0,9]` let
/// `cs <= 5` alone slip through as already-sufficient when declared first,
/// which is why this test does not reuse `r08_base`).
#[trace("QSpec-TC-196", "QSpec-FR-151-AC-6")]
#[test]
fn r08h_two_conjoined_clauses_together_establish_the_narrowed_interval() {
    crate::model_conformance_boundary::conjoined_numeric_clauses();
}

/// Review of PR #157 finding #3: a malformed `ScalarTypeRecord` (its own
/// lower greater than its upper) must refuse when a narrowing
/// domain reader admits any checked facts, never panic on malformed source data.
#[trace("QSpec-TC-196", "QSpec-FR-151-AC-6")]
#[test]
fn r08i_a_malformed_scalar_domain_refuses_rather_than_panicking() {
    crate::model_conformance_boundary::malformed_numeric_domain();
}

/// QSL #171: `check_operation_redefinition`'s effect axis must walk a
/// field's *full* redefinition chain, not just one hop, exactly like
/// `redefinition_reaches` already does for the runtime frame check (QSL
/// #168, `field_write_covered`). Types `A <- B <- C`; `model.B.x` redefines
/// `model.A.x`, `model.C.x` redefines `model.B.x` -- per model-complete.md:56
/// ("the redefining feature replaces the *one* inherited redefined
/// feature"), there is no direct `model.C.x -> model.A.x` record, only the
/// two-hop chain. `model.A.op` declares `modifies: [model.A.x]`;
/// `model.C.op` redefines it with `modifies: [model.C.x]`. `model.C.x`
/// reaches the `model.A.x` grant only through both hops, so this must admit
/// `Compatible`, not refuse `EffectEscape`.
#[trace("QSpec-TC-196", "QSpec-FR-151-AC-4")]
#[test]
fn r09_operation_redefinition_effect_axis_reaches_through_a_two_hop_field_redefinition_chain() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.redef.chain.op"),
        vec![
            object_type("model.A", vec![]),
            object_type("model.B", vec!["model.A"]),
            object_type("model.C", vec!["model.B"]),
            field_member("model.A.x", "model.A", "model.A", mult(0, Some(1))),
            field_member_redefining(
                "model.B.x",
                "model.B",
                "model.A",
                mult(0, Some(1)),
                Some("model.A.x"),
                vec![],
            ),
            field_member_redefining(
                "model.C.x",
                "model.C",
                "model.A",
                mult(0, Some(1)),
                Some("model.B.x"),
                vec![],
            ),
            operation(
                "model.A.op",
                "model.A",
                vec![],
                None,
                vec!["model.A.x"],
                vec![],
                vec![],
            ),
            operation_redefining(
                "model.C.op",
                "model.C",
                vec![],
                None,
                vec!["model.C.x"],
                vec![],
                vec![],
                Some("model.A.op"),
            ),
        ],
    );
    let redefining_key = DeclarationKey::fixture("model.C.op");
    let redefined_key = DeclarationKey::fixture("model.A.op");
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    assert_eq!(
        check_operation_redefinition(
            &ModelIndex::build(domain_package.clone()),
            &redefining_key,
            &redefined_key,
            &mut meter
        ),
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Compatible)
    );
}

/// PR #177 review finding 1: the effect axis's `redefinition_reaches` call
/// must compare `DeclarationKey`s by their full derived `PartialEq`
/// (`package`, `node`), not `.node` alone -- a write naming `model.A.x` in
/// package `other/pkg` does not reach a grant for `model.A.x` in package
/// `test/orders`, even though both share the display node `model.A.x`.
/// Before this PR the effect axis compared `.node`/`.identity` only, so this
/// exact scenario wrongly admitted; every other fixture in this file uses
/// `DeclarationKey::fixture`'s fixed `test/orders` package on both the write
/// and the grant, so none of them can tell full-key equality apart from
/// node-only comparison the way this one does. Retargets the pre-#131
/// revision-differing regression test: under the dropped `revision`/
/// `digest` fields, a `package`-differing key is now the only way to
/// construct two `DeclarationKey`s that share a display node but are not
/// equal.
///
/// Mutation used: in `check_operation_redefinition`'s effect closure,
/// compared `redefined.effect.modifies.iter().any(|w| w.node ==
/// candidate.node)` instead of `.contains(candidate)`. Every other test in
/// this file stayed green; this one went from `Refused` to `Compatible`;
/// reverted.
#[trace("QSpec-TC-196", "QSpec-FR-151-AC-4")]
#[test]
fn r10_operation_redefinition_effect_axis_refuses_a_write_at_a_package_the_grant_does_not_name() {
    let write_in_another_package = DeclarationKey {
        package: "other/pkg".to_owned(),
        node: "model.A.x".to_owned(),
    };
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.redef.op-package"),
        vec![
            object_type("model.A", vec![]),
            object_type("model.B", vec!["model.A"]),
            field_member("model.A.x", "model.A", "model.A", mult(0, Some(1))),
            operation(
                "model.A.op",
                "model.A",
                vec![],
                None,
                vec!["model.A.x"],
                vec![],
                vec![],
            ),
            DomainPackageRecord::OperationMember(OperationMemberRecord {
                key: DeclarationKey::fixture("model.B.op"),
                owner: DeclarationKey::fixture("model.B"),
                parameters: vec![],
                result: None,
                effect: OperationEffect {
                    modifies: vec![write_in_another_package.clone()],
                    creates: Vec::new(),
                    deletes: Vec::new(),
                },
                has_body: true,
                redefines: Some(DeclarationKey::fixture("model.A.op")),
            }),
        ],
    );
    let redefining_key = DeclarationKey::fixture("model.B.op");
    let redefined_key = DeclarationKey::fixture("model.A.op");
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    assert_eq!(
        check_operation_redefinition(
            &ModelIndex::build(domain_package.clone()),
            &redefining_key,
            &redefined_key,
            &mut meter
        ),
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Refused(vec![AxisFailure {
            axis: "effect",
            code: Code::IllTyped,
            cause: ModelRefusalCause::EffectEscape {
                field: write_in_another_package.clone(),
            },
            detail: format!(
                "write {} is not covered by the redefined effect",
                write_in_another_package.node
            ),
        }]))
    );
}

/// PR #177 review finding 2a: a redefinition chain that never reaches the
/// declared grant must still refuse. This is distinct from r03's coverage
/// (a write with no redefinition record at all -- zero hops): here
/// `model.C.x` redefines `model.B.x` (one real hop), but `model.B.x`
/// redefines nothing, so the walk takes its one hop, finds no further
/// redefinition record and no match against `modifies: [model.A.x]`, and
/// refuses -- it must not, e.g., stop after zero hops and admit by mistake,
/// or walk past the chain's actual end.
#[trace("QSpec-TC-196", "QSpec-FR-151-AC-4")]
#[test]
fn r11_operation_redefinition_effect_axis_refuses_a_chain_that_never_reaches_the_grant() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.redef.chain.no-grant"),
        vec![
            object_type("model.A", vec![]),
            object_type("model.B", vec!["model.A"]),
            object_type("model.C", vec!["model.B"]),
            field_member("model.A.x", "model.A", "model.A", mult(0, Some(1))),
            field_member("model.B.x", "model.B", "model.A", mult(0, Some(1))),
            field_member_redefining(
                "model.C.x",
                "model.C",
                "model.A",
                mult(0, Some(1)),
                Some("model.B.x"),
                vec![],
            ),
            operation(
                "model.A.op",
                "model.A",
                vec![],
                None,
                vec!["model.A.x"],
                vec![],
                vec![],
            ),
            operation_redefining(
                "model.C.op",
                "model.C",
                vec![],
                None,
                vec!["model.C.x"],
                vec![],
                vec![],
                Some("model.A.op"),
            ),
        ],
    );
    let redefining_key = DeclarationKey::fixture("model.C.op");
    let redefined_key = DeclarationKey::fixture("model.A.op");
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    assert_eq!(
        check_operation_redefinition(
            &ModelIndex::build(domain_package.clone()),
            &redefining_key,
            &redefined_key,
            &mut meter
        ),
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Refused(vec![AxisFailure {
            axis: "effect",
            code: Code::IllTyped,
            cause: ModelRefusalCause::EffectEscape {
                field: DeclarationKey::fixture("model.C.x"),
            },
            detail: "write model.C.x is not covered by the redefined effect".to_owned(),
        }]))
    );
}

/// PR #177 review finding 2b: a redefinition cycle must terminate, not hang,
/// and still refuse. `model.C.x` redefines `model.B.x`; `model.B.x`
/// redefines `model.C.x` right back -- a malformed chain (never a real
/// generalization-respecting redefinition, but nothing upstream of this
/// walk rules it out). The walk from `model.C.x` never reaches `model.A.x`,
/// and `redefinition_reaches`'s own `records.len()` bound stops it after
/// finitely many hops rather than cycling `C.x -> B.x -> C.x -> ...`
/// forever.
///
/// Mutation used: in `redefinition_reaches`, replaced `for _ in 0..=bound`
/// with `loop` (no bound) -- hand-edited and restored, never committed. This
/// test hung under that mutation (killed by a 15s external `timeout`,
/// having produced no result) instead of failing fast; the source was
/// restored to the bounded loop before this file was committed.
#[trace("QSpec-TC-196", "QSpec-FR-151-AC-4")]
#[test]
fn r12_operation_redefinition_effect_axis_refuses_and_terminates_on_a_redefinition_cycle() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.redef.cycle"),
        vec![
            object_type("model.A", vec![]),
            object_type("model.B", vec!["model.A"]),
            object_type("model.C", vec!["model.B"]),
            field_member("model.A.x", "model.A", "model.A", mult(0, Some(1))),
            field_member_redefining(
                "model.B.x",
                "model.B",
                "model.A",
                mult(0, Some(1)),
                Some("model.C.x"),
                vec![],
            ),
            field_member_redefining(
                "model.C.x",
                "model.C",
                "model.A",
                mult(0, Some(1)),
                Some("model.B.x"),
                vec![],
            ),
            operation(
                "model.A.op",
                "model.A",
                vec![],
                None,
                vec!["model.A.x"],
                vec![],
                vec![],
            ),
            operation_redefining(
                "model.C.op",
                "model.C",
                vec![],
                None,
                vec!["model.C.x"],
                vec![],
                vec![],
                Some("model.A.op"),
            ),
        ],
    );
    let redefining_key = DeclarationKey::fixture("model.C.op");
    let redefined_key = DeclarationKey::fixture("model.A.op");
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    assert_eq!(
        check_operation_redefinition(
            &ModelIndex::build(domain_package.clone()),
            &redefining_key,
            &redefined_key,
            &mut meter
        ),
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Refused(vec![AxisFailure {
            axis: "effect",
            code: Code::IllTyped,
            cause: ModelRefusalCause::EffectEscape {
                field: DeclarationKey::fixture("model.C.x"),
            },
            detail: "write model.C.x is not covered by the redefined effect".to_owned(),
        }]))
    );
}

/// A genuine second selected model with matching display names owns its
/// own writer and checked postcondition; its facts cannot discharge the
/// first selection's immediate-parent obligation.
#[trace("QSpec-TC-196", "QSpec-FR-151-AC-6")]
#[test]
fn r13_field_refinement_same_type_check_does_not_confuse_two_packages_scalar_of_the_same_node() {
    crate::model_conformance_boundary::cross_selection_refinement_decoy(0);
}

/// A genuine second selected model with matching display names owns its
/// own writer and checked postcondition; its facts cannot discharge the
/// first selection's immediate-parent obligation.
#[trace("QSpec-TC-196", "QSpec-FR-151-AC-6")]
#[test]
fn r14a_field_refinement_writer_search_does_not_confuse_a_decoy_matching_the_redefined_field_node()
{
    crate::model_conformance_boundary::cross_selection_refinement_decoy(1);
}

/// A genuine second selected model with matching display names owns its
/// own writer and checked postcondition; its facts cannot discharge the
/// first selection's immediate-parent obligation.
#[trace("QSpec-TC-196", "QSpec-FR-151-AC-6")]
#[test]
fn r14b_field_refinement_writer_search_does_not_confuse_a_decoy_matching_the_redefining_field_node()
{
    crate::model_conformance_boundary::cross_selection_refinement_decoy(2);
}

/// A genuine second selected model with matching display names owns its
/// own writer and checked postcondition; its facts cannot discharge the
/// first selection's immediate-parent obligation.
#[trace("QSpec-TC-196", "QSpec-FR-151-AC-6")]
#[test]
fn r15a_field_refinement_chain_extension_does_not_confuse_a_decoy_matching_the_writers_node() {
    crate::model_conformance_boundary::cross_selection_refinement_decoy(3);
}

/// A genuine second selected model with matching display names owns its
/// own writer and checked postcondition; its facts cannot discharge the
/// first selection's immediate-parent obligation.
#[trace("QSpec-TC-196", "QSpec-FR-151-AC-6")]
#[test]
fn r15b_field_refinement_chain_extension_does_not_confuse_a_decoy_matching_the_owners_node() {
    crate::model_conformance_boundary::cross_selection_refinement_decoy(4);
}

/// A genuine second selected model with matching display names owns its
/// own writer and checked postcondition; its facts cannot discharge the
/// first selection's immediate-parent obligation.
#[trace("QSpec-TC-196", "QSpec-FR-151-AC-6")]
#[test]
fn r16a_field_refinement_names_field_filter_does_not_confuse_a_clause_matching_the_redefining_field_node(
) {
    crate::model_conformance_boundary::cross_selection_refinement_decoy(5);
}

/// A genuine second selected model with matching display names owns its
/// own writer and checked postcondition; its facts cannot discharge the
/// first selection's immediate-parent obligation.
#[trace("QSpec-TC-196", "QSpec-FR-151-AC-6")]
#[test]
fn r16b_field_refinement_names_field_filter_does_not_confuse_a_clause_matching_the_redefined_field_node(
) {
    crate::model_conformance_boundary::cross_selection_refinement_decoy(6);
}

/// A linear chain `model.chain.0 -> model.chain.1 -> ... ->
/// model.chain.{depth}`, each type's own single direct supertype the next
/// in the chain; `model.chain.{depth}` itself has none. `model.chain.0` has
/// exactly `depth` ancestors. Two fields ride on the chain's ends: one on
/// `model.chain.0` typed `model.chain.0`, one on `model.chain.{depth}` typed
/// `model.chain.{depth}`, so checking the first as a redefinition of the
/// second walks `depth` generalization steps.
fn ancestor_chain_package(depth: u64) -> DomainPackage {
    let mut records: Vec<DomainPackageRecord> = (0..=depth)
        .map(|i| {
            let supertypes = if i < depth {
                vec![DeclarationKey::fixture(format!("model.chain.{}", i + 1))]
            } else {
                vec![]
            };
            DomainPackageRecord::ObjectType(ObjectTypeRecord {
                key: DeclarationKey::fixture(format!("model.chain.{i}")),
                interface_features: None,
                abstract_type: false,
                supertypes,
            })
        })
        .collect();
    records.push(field_member(
        "model.chain.redefining",
        "model.chain.0",
        "model.chain.0",
        mult(0, Some(1)),
    ));
    records.push(field_member(
        &format!("model.chain.{depth}.redefined"),
        &format!("model.chain.{depth}"),
        &format!("model.chain.{depth}"),
        mult(0, Some(1)),
    ));
    DomainPackage::new(DomainPackageRef::fixture("bundle.chain"), records)
}

fn check_chain(depth: u64, limits: ModelNormalizationLimits) -> ConformanceCheckOutcome {
    let mut meter = Meter::new(limits);
    check_field_redefinition(
        &ModelIndex::build(ancestor_chain_package(depth)),
        &DeclarationKey::fixture("model.chain.redefining"),
        &DeclarationKey::fixture(format!("model.chain.{depth}.redefined")),
        &mut meter,
    )
}

/// A valid model with more than 128 ancestors on one type
/// passes conformance at default (unlimited) limits. The removed fixed
/// ceiling of 128 refused this whatever the caller configured (ADR-011 §7.3;
/// NFR-001 "an implementation ceiling is not a domain bound").
#[trace("TC-220", "FR-082-AC-3")]
#[test]
fn a_type_with_more_than_128_ancestors_passes_conformance_at_default_limits() {
    assert_eq!(
        check_chain(129, ModelNormalizationLimits::UNLIMITED),
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Compatible),
    );
}

/// FR-082-AC-3: a 10,000-long `supertypes` chain's conformance walk completes
/// on a thread with a 512 KiB stack at the default `ancestor_steps`, and the
/// outcome names no depth.
#[trace("TC-220", "FR-082-AC-3")]
#[test]
fn a_ten_thousand_long_chain_conforms_on_a_small_stack_at_the_default_limits() {
    let outcome = std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(|| check_chain(10_000, ModelNormalizationLimits::default()))
        .expect("spawn a 512 KiB thread")
        .join()
        .expect("the walk must not overflow a 512 KiB stack");
    assert_eq!(
        outcome,
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Compatible)
    );
}

/// TC-220: a chain of exactly `ancestor_steps` `supertypes` edges
/// completes with a conformance verdict, and one step longer refuses with
/// the distinct `resource_exhausted` outcome naming the configured bound --
/// never a `Completed` verdict of either polarity, which is what a walk
/// truncated at the bound would report.
#[trace("TC-720", "FR-255-AC-1")]
#[trace("TC-220", "FR-082-AC-3")]
#[test]
fn an_ancestor_chain_at_the_configured_bound_is_admitted_and_one_longer_refuses() {
    const BOUND: u64 = 50;
    let limits = ModelNormalizationLimits {
        ancestor_steps: BOUND,
        ..ModelNormalizationLimits::UNLIMITED
    };

    assert_eq!(
        check_chain(BOUND, limits),
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Compatible),
        "a chain of exactly ancestor_steps generalization steps must be admitted"
    );

    match check_chain(BOUND + 1, limits) {
        ConformanceCheckOutcome::Incomplete(incomplete) => {
            assert_eq!(incomplete, qsl_semantics::model::accounting::Incomplete {
                limit_kind: qsl_semantics::model::accounting::LimitKind::AncestorSteps,
                limit: BOUND,
                consumed: BOUND,
                next_charge: 1,
                charge_point: qsl_semantics::model::accounting::ChargePoint::ModelAncestorEdge,
            });
        }
        other => panic!("expected Incomplete(AncestorSteps), got {other:?}"),
    }
}

/// End to end: a model with more than 128 ancestors on one
/// type normalizes at default (unlimited) limits, and the same model's
/// redefinition then passes conformance. The removed fixed ceiling of 128
/// on normalization's ancestor paths refused this before conformance ever
/// ran.
#[trace("TC-220", "FR-082-AC-3")]
#[test]
fn a_type_with_more_than_128_ancestors_normalizes_and_conforms_at_default_limits() {
    const DEPTH: u64 = 130;
    let domain_package = ancestor_chain_package(DEPTH);
    match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Completed(_) => {}
        other => panic!("expected a completed effective view, got {other:?}"),
    }
    assert_eq!(
        check_chain(DEPTH, ModelNormalizationLimits::UNLIMITED),
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Compatible),
    );
}

/// TC-220 at normalization: an ancestor path of exactly `ancestor_steps`
/// generalization steps normalizes, and one step longer refuses with
/// `resource_exhausted` naming the configured bound.
#[trace("TC-720", "FR-255-AC-1")]
#[trace("TC-220", "FR-082-AC-3")]
#[test]
fn normalization_admits_an_ancestor_path_at_the_bound_and_refuses_one_longer() {
    const BOUND: u64 = 40;
    let limits = ModelNormalizationLimits {
        ancestor_steps: BOUND,
        ..ModelNormalizationLimits::UNLIMITED
    };
    assert!(matches!(
        normalize(&ancestor_chain_package(BOUND), limits),
        NormalizeOutcome::Completed(_)
    ));
    match normalize(&ancestor_chain_package(BOUND + 1), limits) {
        NormalizeOutcome::Refused(refusals) => {
            assert_eq!(refusals.len(), 1);
            let refusal = refusals.into_first();
            assert_eq!(refusal.code, Code::ResourceExhausted);
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::ancestor_steps(
                    DeclarationKey::fixture("model.chain.0"),
                    BOUND,
                    qsl_foundation::Setting::ModelAncestorSteps,
                )
            );
            let exceeded = refusal
                .cause
                .limit_exceeded()
                .expect("a step ceiling is a stage limit");
            assert_eq!(
                exceeded.setting(),
                qsl_foundation::Setting::ModelAncestorSteps
            );
            assert_eq!(
                exceeded.actual(),
                u128::from(match &refusal.cause {
                    ModelRefusalCause::AncestorSteps { limit, .. }
                    | ModelRefusalCause::FamilySteps { limit, .. } => *limit,
                    other => panic!("unexpected {other:?}"),
                }) + 1
            );
        }
        other => panic!("expected Refused(AncestorSteps), got {other:?}"),
    }
}
