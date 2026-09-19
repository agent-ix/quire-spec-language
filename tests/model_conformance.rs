// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-196: conformance, redefinition and subsetting axes (FR-151), the
//! static/structural subset this rung builds. See
//! `src/model/conformance.rs`'s module docs for the recorded scope
//! decisions (no FR-146 evaluator, no exact per-axis work-unit costs).

use ix_trace_rs::trace;
use quire_spec_language::diagnostic::Code;
use quire_spec_language::model::accounting::{Meter, ModelNormalizationLimits};
use quire_spec_language::model::conformance::{
    check_field_redefinition, check_field_refinement_obligation, check_operation_redefinition,
    check_subsetting, resolve_redefinition_target, AxisFailure, ConformanceCheckOutcome,
    ConformanceOutcome, RedefinitionTargetOutcome,
};
use quire_spec_language::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, FieldMemberRecord, Multiplicity,
    ObjectTypeRecord, OperationEffect, OperationMemberRecord, OperationParameterRecord,
    OperationResult, PostconditionClause, RedefinitionRecord, ScalarTypeRecord, SubsettingRecord,
    SupertypeRecord,
};
use quire_spec_language::model::key::{DeclarationKey, EffectiveId, RULE_REDEFINE};
use quire_spec_language::model::normalize::{
    normalize, EffectiveView, ModelRefusalCause, NormalizeOutcome, ViewEntry,
};
use quire_spec_language::value::OrderingOperator;

fn mult(lower: u64, upper: Option<u64>) -> Multiplicity {
    Multiplicity {
        lower,
        upper,
        ordered: false,
        unique: true,
    }
}

fn object_type(identity: &str) -> DomainPackageRecord {
    DomainPackageRecord::ObjectType(ObjectTypeRecord {
        key: DeclarationKey::fixture(identity),
        interface_features: None,
        abstract_type: false,
    })
}

fn field_member(
    identity: &str,
    owner: &str,
    value_type: &str,
    m: Multiplicity,
) -> DomainPackageRecord {
    DomainPackageRecord::FieldMember(FieldMemberRecord {
        key: DeclarationKey::fixture(identity),
        owner: DeclarationKey::fixture(owner),
        value_type: DeclarationKey::fixture(value_type),
        multiplicity: m,
    })
}

fn supertype(identity: &str, specific: &str, general: &str) -> DomainPackageRecord {
    DomainPackageRecord::Supertype(SupertypeRecord {
        key: DeclarationKey::fixture(identity),
        specific: DeclarationKey::fixture(specific),
        general: DeclarationKey::fixture(general),
    })
}

fn scalar_type(identity: &str, lower: i64, upper: i64) -> DomainPackageRecord {
    DomainPackageRecord::ScalarType(ScalarTypeRecord {
        key: DeclarationKey::fixture(identity),
        lower,
        upper,
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

#[allow(clippy::too_many_arguments)]
fn operation(
    identity: &str,
    owner: &str,
    parameters: Vec<(&str, &str, Multiplicity)>,
    result: Option<(&str, Multiplicity)>,
    modifies: Vec<&str>,
    creates: Vec<&str>,
    deletes: Vec<&str>,
    own_postcondition_clauses: Vec<PostconditionClause>,
) -> DomainPackageRecord {
    DomainPackageRecord::OperationMember(OperationMemberRecord {
        key: DeclarationKey::fixture(identity),
        owner: DeclarationKey::fixture(owner),
        parameters: parameters
            .into_iter()
            .map(|(id, ty, m)| OperationParameterRecord {
                key: DeclarationKey::fixture(id),
                value_type: DeclarationKey::fixture(ty),
                multiplicity: m,
            })
            .collect(),
        result: result.map(|(ty, m)| OperationResult {
            value_type: DeclarationKey::fixture(ty),
            multiplicity: m,
        }),
        effect: OperationEffect {
            modifies: modifies.into_iter().map(DeclarationKey::fixture).collect(),
            creates: creates.into_iter().map(DeclarationKey::fixture).collect(),
            deletes: deletes.into_iter().map(DeclarationKey::fixture).collect(),
        },
        has_own_precondition: false,
        own_postcondition_clauses,
        has_body: true,
    })
}

/// H (domain package `bundle.h`): types `A`, `B` (`B` <= `A`); field `model.A.x`
/// typed `model.A` `{0,1}`; field `model.B.y` typed `model.A` `{0,1}`;
/// operation `model.A.op(p1: model.A {0,1})`: `model.B {1,1}`, effect
/// `{modifies: [model.A.x], creates: [model.A], deletes: []}`.
fn fixture_h_base() -> Vec<DomainPackageRecord> {
    vec![
        object_type("model.A"),
        object_type("model.B"),
        supertype("model.gen.B-A", "model.B", "model.A"),
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
            vec![],
        ),
    ]
}

fn bundle_h(mut records: Vec<DomainPackageRecord>) -> DomainPackage {
    records.extend(fixture_h_base());
    DomainPackage::new(DomainPackageRef::fixture("bundle.h"), records)
}

/// Finds the effective TYPE entry (no owner) declared with original identity
/// `identity` — mirrors `tests/model_normalization.rs`'s own helper, which
/// this integration-test binary cannot import directly.
fn find_type<'a>(view: &'a EffectiveView, identity: &str) -> &'a ViewEntry {
    view.declarations
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
    view.declarations
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
#[trace("TC-196", "FR-151-AC-1")]
#[test]
fn r01_a_compatible_field_redefinition_yields_one_effective_member_with_complete_provenance() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.r01"),
        vec![
            object_type("model.A"),
            object_type("model.B"),
            supertype("model.gen.B-A", "model.B", "model.A"),
            field_member("model.A.n", "model.A", "model.A", mult(0, Some(5))),
            field_member("model.B.n", "model.B", "model.A", mult(1, Some(3))),
            redefinition("model.redef.B.n", "model.B", "model.B.n", "model.A.n"),
        ],
    );
    let record = RedefinitionRecord {
        key: DeclarationKey::fixture("model.redef.B.n"),
        owner: DeclarationKey::fixture("model.B"),
        redefining: DeclarationKey::fixture("model.B.n"),
        redefined: DeclarationKey::fixture("model.A.n"),
    };

    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    match check_field_redefinition(&domain_package, &record, &mut meter) {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Compatible) => {}
        other => panic!("expected Compatible ({{1,3}} conforms to {{0,5}}), got {other:?}"),
    }

    let view = match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Completed(view) => view,
        other => panic!("expected a completed view, got {other:?}"),
    };
    let owner_b = find_type(&view, "model.B").effective_id.clone();

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
        .declarations
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
#[trace("TC-196")]
#[test]
fn two_field_members_sharing_one_declaration_key_refuse_conflicting_binding() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.f2-conformance-conflict"),
        vec![
            object_type("model.A"),
            object_type("model.B"),
            field_member("model.A.x", "model.A", "model.A", mult(0, Some(1))),
            field_member("model.A.x", "model.A", "model.A", mult(0, Some(5))),
            field_member("model.B.y", "model.B", "model.A", mult(0, Some(3))),
        ],
    );
    match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
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
#[trace("TC-196", "FR-151-AC-4")]
#[test]
fn r02_a_compatible_operation_redefinition_admits_every_axis() {
    let domain_package = bundle_h(vec![
        operation(
            "model.B.op",
            "model.B",
            vec![("model.B.op.p1", "model.A", mult(0, Some(2)))],
            Some(("model.B", mult(1, Some(1)))),
            vec![],
            vec!["model.B"],
            vec![],
            vec![],
        ),
        redefinition("model.redef.B.op", "model.B", "model.B.op", "model.A.op"),
    ]);
    let record = RedefinitionRecord {
        key: DeclarationKey::fixture("model.redef.B.op"),
        owner: DeclarationKey::fixture("model.B"),
        redefining: DeclarationKey::fixture("model.B.op"),
        redefined: DeclarationKey::fixture("model.A.op"),
    };
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    match check_operation_redefinition(&domain_package, &record, &mut meter) {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Compatible) => {}
        other => panic!("expected Compatible, got {other:?}"),
    }
}

#[trace("TC-196", "FR-151-AC-2", "FR-151-AC-4")]
#[test]
fn r03_an_incompatible_operation_redefinition_reports_every_failing_axis() {
    let domain_package = bundle_h(vec![
        operation(
            "model.B.op",
            "model.B",
            vec![("model.B.op.p1", "model.B", mult(1, Some(1)))],
            Some(("model.A", mult(0, Some(1)))),
            vec!["model.B.y"],
            vec![],
            vec![],
            vec![],
        ),
        redefinition("model.redef.B.op", "model.B", "model.B.op", "model.A.op"),
    ]);
    let record = RedefinitionRecord {
        key: DeclarationKey::fixture("model.redef.B.op"),
        owner: DeclarationKey::fixture("model.B"),
        redefining: DeclarationKey::fixture("model.B.op"),
        redefined: DeclarationKey::fixture("model.A.op"),
    };
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    match check_operation_redefinition(&domain_package, &record, &mut meter) {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Refused(failures)) => {
            let causes: Vec<ModelRefusalCause> = failures.iter().map(|f| f.cause.clone()).collect();
            assert_eq!(causes.len(), 5);
            assert_eq!(
                causes[0],
                ModelRefusalCause::VarianceParameter {
                    index: 1,
                    declared: DeclarationKey::fixture("model.A"),
                    redefined: DeclarationKey::fixture("model.B"),
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

#[trace("TC-196", "FR-151-AC-4")]
#[test]
fn r04_an_arity_mismatch_refuses_without_checking_parameter_axes() {
    let domain_package = bundle_h(vec![
        operation(
            "model.B.op",
            "model.B",
            vec![],
            Some(("model.B", mult(1, Some(1)))),
            vec![],
            vec!["model.B"],
            vec![],
            vec![],
        ),
        redefinition("model.redef.B.op", "model.B", "model.B.op", "model.A.op"),
    ]);
    let record = RedefinitionRecord {
        key: DeclarationKey::fixture("model.redef.B.op"),
        owner: DeclarationKey::fixture("model.B"),
        redefining: DeclarationKey::fixture("model.B.op"),
        redefined: DeclarationKey::fixture("model.A.op"),
    };
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    match check_operation_redefinition(&domain_package, &record, &mut meter) {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Refused(failures)) => {
            assert_eq!(failures.len(), 1);
            assert_eq!(failures[0].cause, ModelRefusalCause::TypeMismatch);
            assert_eq!(failures[0].code, Code::IllTyped);
        }
        other => panic!("expected Refused, got {other:?}"),
    }
}

#[trace("TC-196", "FR-151-AC-2")]
#[test]
fn r05_field_multiplicity_narrowing_refuses_and_the_boundary_admits() {
    let records = |a_mult: Multiplicity, b_mult: Multiplicity| {
        vec![
            object_type("model.A"),
            object_type("model.B"),
            supertype("model.gen.B-A", "model.B", "model.A"),
            field_member("model.A.n", "model.A", "model.A", a_mult),
            field_member("model.B.n", "model.B", "model.A", b_mult),
            redefinition("model.redef.B.n", "model.B", "model.B.n", "model.A.n"),
        ]
    };
    let record = RedefinitionRecord {
        key: DeclarationKey::fixture("model.redef.B.n"),
        owner: DeclarationKey::fixture("model.B"),
        redefining: DeclarationKey::fixture("model.B.n"),
        redefined: DeclarationKey::fixture("model.A.n"),
    };

    let narrowing = DomainPackage::new(
        DomainPackageRef::fixture("bundle.r05a"),
        records(mult(1, Some(3)), mult(0, Some(5))),
    );
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    match check_field_redefinition(&narrowing, &record, &mut meter) {
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
    match check_field_redefinition(&widening, &record, &mut meter) {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Compatible) => {}
        other => panic!("expected Compatible, got {other:?}"),
    }
}

#[trace("TC-196", "FR-151-AC-2")]
#[test]
fn r06_subsetting_type_and_multiplicity_axes() {
    let bundle_of = |some_type: &str, some_mult: Multiplicity| {
        DomainPackage::new(
            DomainPackageRef::fixture("bundle.r06"),
            vec![
                object_type("model.A"),
                object_type("model.B"),
                object_type("model.C"),
                supertype("model.gen.B-A", "model.B", "model.A"),
                field_member("model.A.all", "model.A", "model.A", mult(0, Some(5))),
                field_member("model.A.some", "model.A", some_type, some_mult),
                subsetting("model.sub.some", "model.A", "model.A.some", "model.A.all"),
            ],
        )
    };
    let record = SubsettingRecord {
        key: DeclarationKey::fixture("model.sub.some"),
        owner: DeclarationKey::fixture("model.A"),
        subsetting: DeclarationKey::fixture("model.A.some"),
        subsetted: DeclarationKey::fixture("model.A.all"),
    };

    // (i) same type, wider multiplicity: multiplicity-narrowing.
    let domain_package = bundle_of("model.A", mult(0, Some(9)));
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    match check_subsetting(&domain_package, &record, &mut meter) {
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
    match check_subsetting(&domain_package, &record, &mut meter) {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Refused(failures)) => {
            assert_eq!(failures.len(), 1);
            assert_eq!(
                failures[0].cause,
                ModelRefusalCause::SubsettingType {
                    subsetting: DeclarationKey::fixture("model.C"),
                    subsetted: DeclarationKey::fixture("model.A"),
                }
            );
        }
        other => panic!("expected Refused, got {other:?}"),
    }

    // (iii) conforming subtype, narrower multiplicity: admitted.
    let domain_package = bundle_of("model.B", mult(0, Some(3)));
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    match check_subsetting(&domain_package, &record, &mut meter) {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Compatible) => {}
        other => panic!("expected Compatible, got {other:?}"),
    }
}

#[trace("TC-196", "FR-151-AC-2")]
#[test]
fn r07_zero_or_multiple_inherited_targets_refuse_redefinition_target() {
    // Zero inherited targets: B.z claims to redefine C.w, but B does not
    // conform to C.
    let zero = DomainPackage::new(
        DomainPackageRef::fixture("bundle.r07a"),
        vec![
            object_type("model.A"),
            object_type("model.B"),
            object_type("model.C"),
            supertype("model.gen.B-A", "model.B", "model.A"),
            field_member("model.C.w", "model.C", "model.A", mult(0, Some(1))),
            field_member("model.B.z", "model.B", "model.A", mult(0, Some(1))),
            redefinition("model.redef.z", "model.B", "model.B.z", "model.C.w"),
        ],
    );
    match resolve_redefinition_target(
        &zero,
        &DeclarationKey::fixture("model.B"),
        &DeclarationKey::fixture("model.B.z"),
    ) {
        Ok(RedefinitionTargetOutcome::Refused {
            cause,
            candidates,
            valid_targets,
        }) => {
            assert_eq!(cause, ModelRefusalCause::RedefinitionTarget);
            assert_eq!(candidates.len(), 1);
            assert!(
                valid_targets.is_empty(),
                "zero-target shape must carry no valid targets, got {valid_targets:?}"
            );
        }
        other => panic!("expected Refused(zero targets), got {other:?}"),
    }

    // Two distinct, both-legitimate inherited targets: ambiguous.
    let two = DomainPackage::new(
        DomainPackageRef::fixture("bundle.r07b"),
        vec![
            object_type("model.A"),
            object_type("model.B"),
            supertype("model.gen.B-A", "model.B", "model.A"),
            field_member("model.A.x", "model.A", "model.A", mult(0, Some(1))),
            field_member("model.A.x2", "model.A", "model.A", mult(0, Some(1))),
            field_member("model.B.z", "model.B", "model.A", mult(0, Some(1))),
            redefinition("model.redef.z1", "model.B", "model.B.z", "model.A.x"),
            redefinition("model.redef.z2", "model.B", "model.B.z", "model.A.x2"),
        ],
    );
    match resolve_redefinition_target(
        &two,
        &DeclarationKey::fixture("model.B"),
        &DeclarationKey::fixture("model.B.z"),
    ) {
        Ok(RedefinitionTargetOutcome::Refused {
            cause,
            candidates,
            valid_targets,
        }) => {
            assert_eq!(cause, ModelRefusalCause::RedefinitionTarget);
            assert_eq!(candidates.len(), 2);
            assert_eq!(
                valid_targets.len(),
                2,
                "ambiguous shape must carry both valid targets, got {valid_targets:?}"
            );
        }
        other => panic!("expected Refused(ambiguous), got {other:?}"),
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

/// R08's shared base: types `A`, `B` (`B` <= `A`); scalars `model.Count`
/// `[0,9]` and `model.Small` `[0,5]`; field `model.A.x` typed `model.A`
/// `{0,1}`; field `model.A.c` typed `model.Count` `{1,1}`; operation
/// `model.A.set()` with no result and effect writing `[model.A.x, model.A.c]`.
fn r08_base() -> Vec<DomainPackageRecord> {
    vec![
        object_type("model.A"),
        object_type("model.B"),
        supertype("model.gen.B-A", "model.B", "model.A"),
        scalar_type("model.Count", 0, 9),
        scalar_type("model.Small", 0, 5),
        field_member("model.A.x", "model.A", "model.A", mult(0, Some(1))),
        field_member("model.A.c", "model.A", "model.Count", mult(1, Some(1))),
        operation(
            "model.A.set",
            "model.A",
            vec![],
            None,
            vec!["model.A.x", "model.A.c"],
            vec![],
            vec![],
            vec![],
        ),
    ]
}

#[trace("TC-196", "FR-151-AC-6")]
#[test]
fn r08a_a_narrowing_field_redefinition_without_a_presence_fact_refuses() {
    let mut records = r08_base();
    records.push(field_member(
        "model.B.xb",
        "model.B",
        "model.A",
        mult(1, Some(1)),
    ));
    records.push(redefinition(
        "model.redef.xb",
        "model.B",
        "model.B.xb",
        "model.A.x",
    ));
    let domain_package = DomainPackage::new(DomainPackageRef::fixture("bundle.r08a"), records);
    let record = RedefinitionRecord {
        key: DeclarationKey::fixture("model.redef.xb"),
        owner: DeclarationKey::fixture("model.B"),
        redefining: DeclarationKey::fixture("model.B.xb"),
        redefined: DeclarationKey::fixture("model.A.x"),
    };
    match check_field_refinement_obligation(&domain_package, &record) {
        Ok(ConformanceOutcome::Refused(failures)) => {
            assert_eq!(failures.len(), 1);
            assert_eq!(failures[0].cause, ModelRefusalCause::UnprovedRefinement);
            assert!(failures[0].detail.contains("field-presence"));
        }
        other => panic!("expected Refused(field-presence), got {other:?}"),
    }
}

#[trace("TC-196", "FR-151-AC-6")]
#[test]
fn r08b_a_redefined_operation_with_the_presence_fact_discharges_the_obligation() {
    let mut records = r08_base();
    records.push(field_member(
        "model.B.xb",
        "model.B",
        "model.A",
        mult(1, Some(1)),
    ));
    records.push(redefinition(
        "model.redef.xb",
        "model.B",
        "model.B.xb",
        "model.A.x",
    ));
    records.push(operation(
        "model.B.set",
        "model.B",
        vec![],
        None,
        vec![],
        vec![],
        vec![],
        vec![PostconditionClause::Presence {
            field: DeclarationKey::fixture("model.B.xb"),
        }],
    ));
    records.push(redefinition(
        "model.redef.set",
        "model.B",
        "model.B.set",
        "model.A.set",
    ));
    let domain_package = DomainPackage::new(DomainPackageRef::fixture("bundle.r08b"), records);
    let record = RedefinitionRecord {
        key: DeclarationKey::fixture("model.redef.xb"),
        owner: DeclarationKey::fixture("model.B"),
        redefining: DeclarationKey::fixture("model.B.xb"),
        redefined: DeclarationKey::fixture("model.A.x"),
    };
    match check_field_refinement_obligation(&domain_package, &record) {
        Ok(ConformanceOutcome::Compatible) => {}
        other => panic!("expected Compatible, got {other:?}"),
    }
}

#[trace("TC-196", "FR-151-AC-6")]
#[test]
fn r08c_an_object_typed_narrowing_has_no_proof_form() {
    let mut records = r08_base();
    records.push(field_member(
        "model.B.xr",
        "model.B",
        "model.B",
        mult(0, Some(1)),
    ));
    records.push(redefinition(
        "model.redef.xr",
        "model.B",
        "model.B.xr",
        "model.A.x",
    ));
    let domain_package = DomainPackage::new(DomainPackageRef::fixture("bundle.r08c"), records);
    let record = RedefinitionRecord {
        key: DeclarationKey::fixture("model.redef.xr"),
        owner: DeclarationKey::fixture("model.B"),
        redefining: DeclarationKey::fixture("model.B.xr"),
        redefined: DeclarationKey::fixture("model.A.x"),
    };
    match check_field_refinement_obligation(&domain_package, &record) {
        Ok(ConformanceOutcome::Refused(failures)) => {
            assert_eq!(failures[0].cause, ModelRefusalCause::UnprovedRefinement);
            assert!(failures[0].detail.contains("no-proof-form"));
        }
        other => panic!("expected Refused(no-proof-form), got {other:?}"),
    }
}

#[trace("TC-196", "FR-151-AC-6")]
#[test]
fn r08d_a_narrowed_scalar_domain_without_an_interval_fact_refuses_field_domain() {
    let mut records = r08_base();
    records.push(field_member(
        "model.B.cs",
        "model.B",
        "model.Small",
        mult(1, Some(1)),
    ));
    records.push(redefinition(
        "model.redef.cs",
        "model.B",
        "model.B.cs",
        "model.A.c",
    ));
    let domain_package = DomainPackage::new(DomainPackageRef::fixture("bundle.r08d"), records);
    let record = RedefinitionRecord {
        key: DeclarationKey::fixture("model.redef.cs"),
        owner: DeclarationKey::fixture("model.B"),
        redefining: DeclarationKey::fixture("model.B.cs"),
        redefined: DeclarationKey::fixture("model.A.c"),
    };
    match check_field_refinement_obligation(&domain_package, &record) {
        Ok(ConformanceOutcome::Refused(failures)) => {
            assert_eq!(failures[0].cause, ModelRefusalCause::UnprovedRefinement);
            assert!(failures[0].detail.contains("field-domain"));
        }
        other => panic!("expected Refused(field-domain), got {other:?}"),
    }
}

#[trace("TC-196", "FR-151-AC-6")]
#[test]
fn r08e_and_r08f_an_established_interval_admits_only_when_contained() {
    let record = RedefinitionRecord {
        key: DeclarationKey::fixture("model.redef.cs"),
        owner: DeclarationKey::fixture("model.B"),
        redefining: DeclarationKey::fixture("model.B.cs"),
        redefined: DeclarationKey::fixture("model.A.c"),
    };

    let contained = |upper: i64| {
        let mut records = r08_base();
        records.push(field_member(
            "model.B.cs",
            "model.B",
            "model.Small",
            mult(1, Some(1)),
        ));
        records.push(redefinition(
            "model.redef.cs",
            "model.B",
            "model.B.cs",
            "model.A.c",
        ));
        records.push(operation(
            "model.B.set",
            "model.B",
            vec![],
            None,
            vec![],
            vec![],
            vec![],
            vec![PostconditionClause::Comparison {
                field: DeclarationKey::fixture("model.B.cs"),
                operator: OrderingOperator::LessOrEqual,
                literal: upper,
            }],
        ));
        records.push(redefinition(
            "model.redef.set",
            "model.B",
            "model.B.set",
            "model.A.set",
        ));
        DomainPackage::new(DomainPackageRef::fixture("bundle.r08ef"), records)
    };

    match check_field_refinement_obligation(&contained(5), &record) {
        Ok(ConformanceOutcome::Compatible) => {}
        other => panic!("expected Compatible (e), got {other:?}"),
    }
    match check_field_refinement_obligation(&contained(6), &record) {
        Ok(ConformanceOutcome::Refused(failures)) => {
            assert_eq!(failures[0].cause, ModelRefusalCause::UnprovedRefinement);
            assert!(failures[0].detail.contains("field-domain"));
        }
        other => panic!("expected Refused (f), got {other:?}"),
    }
}

/// A postcondition clause that does not actually establish the narrowed
/// domain (`model.B.cs >= 0`, which every value of a scalar with floor 0
/// already satisfies) must still refuse: the obligation is discharged by
/// real fact derivation over the declared clause, not by trusting that any
/// clause naming the field is sufficient.
#[trace("TC-196", "FR-151-AC-6")]
#[test]
fn r08g_an_unrelated_clause_over_the_same_field_does_not_discharge_the_obligation() {
    let mut records = r08_base();
    records.push(field_member(
        "model.B.cs",
        "model.B",
        "model.Small",
        mult(1, Some(1)),
    ));
    records.push(redefinition(
        "model.redef.cs",
        "model.B",
        "model.B.cs",
        "model.A.c",
    ));
    records.push(operation(
        "model.B.set",
        "model.B",
        vec![],
        None,
        vec![],
        vec![],
        vec![],
        vec![PostconditionClause::Comparison {
            field: DeclarationKey::fixture("model.B.cs"),
            operator: OrderingOperator::GreaterOrEqual,
            literal: 0,
        }],
    ));
    records.push(redefinition(
        "model.redef.set",
        "model.B",
        "model.B.set",
        "model.A.set",
    ));
    let domain_package = DomainPackage::new(DomainPackageRef::fixture("bundle.r08g"), records);
    let record = RedefinitionRecord {
        key: DeclarationKey::fixture("model.redef.cs"),
        owner: DeclarationKey::fixture("model.B"),
        redefining: DeclarationKey::fixture("model.B.cs"),
        redefined: DeclarationKey::fixture("model.A.c"),
    };
    match check_field_refinement_obligation(&domain_package, &record) {
        Ok(ConformanceOutcome::Refused(failures)) => {
            assert_eq!(failures[0].cause, ModelRefusalCause::UnprovedRefinement);
            assert!(failures[0].detail.contains("field-domain"));
        }
        other => panic!("expected Refused (g, unrelated clause), got {other:?}"),
    }
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
#[trace("TC-196", "FR-151-AC-6")]
#[test]
fn r08h_two_conjoined_clauses_together_establish_the_narrowed_interval() {
    let ge_zero = PostconditionClause::Comparison {
        field: DeclarationKey::fixture("model.B.cs"),
        operator: OrderingOperator::GreaterOrEqual,
        literal: 0,
    };
    let le_five = PostconditionClause::Comparison {
        field: DeclarationKey::fixture("model.B.cs"),
        operator: OrderingOperator::LessOrEqual,
        literal: 5,
    };

    for (order, clauses) in [
        ("ge-then-le", vec![ge_zero.clone(), le_five.clone()]),
        ("le-then-ge", vec![le_five, ge_zero]),
    ] {
        let records = vec![
            object_type("model.A"),
            object_type("model.B"),
            supertype("model.gen.B-A", "model.B", "model.A"),
            scalar_type("model.Count", -5, 9),
            scalar_type("model.Small", 0, 5),
            field_member("model.A.x", "model.A", "model.A", mult(0, Some(1))),
            field_member("model.A.c", "model.A", "model.Count", mult(1, Some(1))),
            field_member("model.B.cs", "model.B", "model.Small", mult(1, Some(1))),
            redefinition("model.redef.cs", "model.B", "model.B.cs", "model.A.c"),
            operation(
                "model.A.set",
                "model.A",
                vec![],
                None,
                vec!["model.A.x", "model.A.c"],
                vec![],
                vec![],
                vec![],
            ),
            operation(
                "model.B.set",
                "model.B",
                vec![],
                None,
                vec![],
                vec![],
                vec![],
                clauses,
            ),
            redefinition("model.redef.set", "model.B", "model.B.set", "model.A.set"),
        ];
        let domain_package = DomainPackage::new(DomainPackageRef::fixture("bundle.r08h"), records);
        let record = RedefinitionRecord {
            key: DeclarationKey::fixture("model.redef.cs"),
            owner: DeclarationKey::fixture("model.B"),
            redefining: DeclarationKey::fixture("model.B.cs"),
            redefined: DeclarationKey::fixture("model.A.c"),
        };
        match check_field_refinement_obligation(&domain_package, &record) {
            Ok(ConformanceOutcome::Compatible) => {}
            other => panic!(
                "expected Compatible regardless of clause order ({order}): the two \
                 clauses together establish [0,5], contained in Small's [0,5], \
                 got {other:?}"
            ),
        }
    }
}

/// Review of PR #157 finding #3: a malformed `ScalarTypeRecord` (its own
/// lower greater than its upper) must refuse when a narrowing
/// redefinition's obligation check seeds a synthetic guard from it, never
/// panic on this caller-supplied domain package data.
#[trace("TC-196", "FR-151-AC-6")]
#[test]
fn r08i_a_malformed_scalar_domain_refuses_rather_than_panicking() {
    let records = vec![
        object_type("model.A"),
        object_type("model.B"),
        supertype("model.gen.B-A", "model.B", "model.A"),
        scalar_type("model.Count", 9, 0), // malformed: lower > upper.
        scalar_type("model.Small", 0, 5),
        field_member("model.A.x", "model.A", "model.A", mult(0, Some(1))),
        field_member("model.A.c", "model.A", "model.Count", mult(1, Some(1))),
        field_member("model.B.cs", "model.B", "model.Small", mult(1, Some(1))),
        redefinition("model.redef.cs", "model.B", "model.B.cs", "model.A.c"),
        operation(
            "model.A.set",
            "model.A",
            vec![],
            None,
            vec!["model.A.x", "model.A.c"],
            vec![],
            vec![],
            vec![],
        ),
        operation(
            "model.B.set",
            "model.B",
            vec![],
            None,
            vec![],
            vec![],
            vec![],
            vec![PostconditionClause::Comparison {
                field: DeclarationKey::fixture("model.B.cs"),
                operator: OrderingOperator::LessOrEqual,
                literal: 5,
            }],
        ),
        redefinition("model.redef.set", "model.B", "model.B.set", "model.A.set"),
    ];
    let domain_package = DomainPackage::new(DomainPackageRef::fixture("bundle.r08i"), records);
    let record = RedefinitionRecord {
        key: DeclarationKey::fixture("model.redef.cs"),
        owner: DeclarationKey::fixture("model.B"),
        redefining: DeclarationKey::fixture("model.B.cs"),
        redefined: DeclarationKey::fixture("model.A.c"),
    };
    match check_field_refinement_obligation(&domain_package, &record) {
        Err(refusal) => {
            assert_eq!(refusal.code, Code::InvalidModelBinding);
            // FR-272's `invalid_model_binding` cause list is closed; there
            // is no dedicated scalar-domain variant, so this is the
            // catalogued `malformed-declaration`.
            assert_eq!(refusal.cause, ModelRefusalCause::MalformedDeclaration);
        }
        other => panic!("expected Err(malformed-declaration), got {other:?}"),
    }
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
#[trace("TC-196", "FR-151-AC-4")]
#[test]
fn r09_operation_redefinition_effect_axis_reaches_through_a_two_hop_field_redefinition_chain() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.redef.chain.op"),
        vec![
            object_type("model.A"),
            object_type("model.B"),
            object_type("model.C"),
            supertype("model.gen.B-A", "model.B", "model.A"),
            supertype("model.gen.C-B", "model.C", "model.B"),
            field_member("model.A.x", "model.A", "model.A", mult(0, Some(1))),
            field_member("model.B.x", "model.B", "model.A", mult(0, Some(1))),
            field_member("model.C.x", "model.C", "model.A", mult(0, Some(1))),
            redefinition("model.redef.B.x-A.x", "model.B", "model.B.x", "model.A.x"),
            redefinition("model.redef.C.x-B.x", "model.C", "model.C.x", "model.B.x"),
            operation(
                "model.A.op",
                "model.A",
                vec![],
                None,
                vec!["model.A.x"],
                vec![],
                vec![],
                vec![],
            ),
            operation(
                "model.C.op",
                "model.C",
                vec![],
                None,
                vec!["model.C.x"],
                vec![],
                vec![],
                vec![],
            ),
            redefinition(
                "model.redef.C.op-A.op",
                "model.C",
                "model.C.op",
                "model.A.op",
            ),
        ],
    );
    let record = RedefinitionRecord {
        key: DeclarationKey::fixture("model.redef.C.op-A.op"),
        owner: DeclarationKey::fixture("model.C"),
        redefining: DeclarationKey::fixture("model.C.op"),
        redefined: DeclarationKey::fixture("model.A.op"),
    };
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    assert_eq!(
        check_operation_redefinition(&domain_package, &record, &mut meter),
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
#[trace("TC-196", "FR-151-AC-4")]
#[test]
fn r10_operation_redefinition_effect_axis_refuses_a_write_at_a_package_the_grant_does_not_name() {
    let write_in_another_package = DeclarationKey {
        package: "other/pkg".to_owned(),
        node: "model.A.x".to_owned(),
    };
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.redef.op-package"),
        vec![
            object_type("model.A"),
            object_type("model.B"),
            supertype("model.gen.B-A", "model.B", "model.A"),
            field_member("model.A.x", "model.A", "model.A", mult(0, Some(1))),
            operation(
                "model.A.op",
                "model.A",
                vec![],
                None,
                vec!["model.A.x"],
                vec![],
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
                has_own_precondition: false,
                own_postcondition_clauses: vec![],
                has_body: true,
            }),
            redefinition(
                "model.redef.B.op-A.op",
                "model.B",
                "model.B.op",
                "model.A.op",
            ),
        ],
    );
    let record = RedefinitionRecord {
        key: DeclarationKey::fixture("model.redef.B.op-A.op"),
        owner: DeclarationKey::fixture("model.B"),
        redefining: DeclarationKey::fixture("model.B.op"),
        redefined: DeclarationKey::fixture("model.A.op"),
    };
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    assert_eq!(
        check_operation_redefinition(&domain_package, &record, &mut meter),
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
#[trace("TC-196", "FR-151-AC-4")]
#[test]
fn r11_operation_redefinition_effect_axis_refuses_a_chain_that_never_reaches_the_grant() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.redef.chain.no-grant"),
        vec![
            object_type("model.A"),
            object_type("model.B"),
            object_type("model.C"),
            supertype("model.gen.B-A", "model.B", "model.A"),
            supertype("model.gen.C-B", "model.C", "model.B"),
            field_member("model.A.x", "model.A", "model.A", mult(0, Some(1))),
            field_member("model.B.x", "model.B", "model.A", mult(0, Some(1))),
            field_member("model.C.x", "model.C", "model.A", mult(0, Some(1))),
            redefinition("model.redef.C.x-B.x", "model.C", "model.C.x", "model.B.x"),
            operation(
                "model.A.op",
                "model.A",
                vec![],
                None,
                vec!["model.A.x"],
                vec![],
                vec![],
                vec![],
            ),
            operation(
                "model.C.op",
                "model.C",
                vec![],
                None,
                vec!["model.C.x"],
                vec![],
                vec![],
                vec![],
            ),
            redefinition(
                "model.redef.C.op-A.op",
                "model.C",
                "model.C.op",
                "model.A.op",
            ),
        ],
    );
    let record = RedefinitionRecord {
        key: DeclarationKey::fixture("model.redef.C.op-A.op"),
        owner: DeclarationKey::fixture("model.C"),
        redefining: DeclarationKey::fixture("model.C.op"),
        redefined: DeclarationKey::fixture("model.A.op"),
    };
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    assert_eq!(
        check_operation_redefinition(&domain_package, &record, &mut meter),
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
#[trace("TC-196", "FR-151-AC-4")]
#[test]
fn r12_operation_redefinition_effect_axis_refuses_and_terminates_on_a_redefinition_cycle() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.redef.cycle"),
        vec![
            object_type("model.A"),
            object_type("model.B"),
            object_type("model.C"),
            supertype("model.gen.B-A", "model.B", "model.A"),
            supertype("model.gen.C-B", "model.C", "model.B"),
            field_member("model.A.x", "model.A", "model.A", mult(0, Some(1))),
            field_member("model.B.x", "model.B", "model.A", mult(0, Some(1))),
            field_member("model.C.x", "model.C", "model.A", mult(0, Some(1))),
            redefinition("model.redef.C.x-B.x", "model.C", "model.C.x", "model.B.x"),
            redefinition("model.redef.B.x-C.x", "model.B", "model.B.x", "model.C.x"),
            operation(
                "model.A.op",
                "model.A",
                vec![],
                None,
                vec!["model.A.x"],
                vec![],
                vec![],
                vec![],
            ),
            operation(
                "model.C.op",
                "model.C",
                vec![],
                None,
                vec!["model.C.x"],
                vec![],
                vec![],
                vec![],
            ),
            redefinition(
                "model.redef.C.op-A.op",
                "model.C",
                "model.C.op",
                "model.A.op",
            ),
        ],
    );
    let record = RedefinitionRecord {
        key: DeclarationKey::fixture("model.redef.C.op-A.op"),
        owner: DeclarationKey::fixture("model.C"),
        redefining: DeclarationKey::fixture("model.C.op"),
        redefined: DeclarationKey::fixture("model.A.op"),
    };
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    assert_eq!(
        check_operation_redefinition(&domain_package, &record, &mut meter),
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
