// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-196: conformance, redefinition and subsetting axes (FR-151), the
//! static/structural subset this rung builds. See
//! `src/model/conformance.rs`'s module docs for the recorded scope
//! decisions (no FR-146 evaluator, no exact per-axis work-unit costs).

use ix_trace_rs::trace;
use quire_spec_language::diagnostic::Code;
use quire_spec_language::model::accounting::{Meter, ModelNormalizationLimits};
use quire_spec_language::model::bundle::{
    Bundle, BundleRecord, FieldMemberRecord, GeneralizationRecord, ModelSelection, Multiplicity,
    ObjectTypeRecord, OperationEffect, OperationMemberRecord, OperationParameterRecord,
    OperationResult, PostconditionClause, RedefinitionRecord, ScalarTypeRecord, SubsettingRecord,
};
use quire_spec_language::model::conformance::{
    check_field_redefinition, check_field_refinement_obligation, check_operation_redefinition,
    check_subsetting, resolve_redefinition_target, ConformanceCheckOutcome, ConformanceOutcome,
    RedefinitionTargetOutcome,
};
use quire_spec_language::model::key::{EffectiveId, ProducerKey, RULE_REDEFINE};
use quire_spec_language::model::normalize::{
    normalize, EffectiveView, NormalizeOutcome, ViewEntry,
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

fn object_type(identity: &str) -> BundleRecord {
    BundleRecord::ObjectType(ObjectTypeRecord {
        key: ProducerKey::fixture(identity),
        interface_features: None,
    })
}

fn field_member(identity: &str, owner: &str, value_type: &str, m: Multiplicity) -> BundleRecord {
    BundleRecord::FieldMember(FieldMemberRecord {
        key: ProducerKey::fixture(identity),
        owner: ProducerKey::fixture(owner),
        value_type: ProducerKey::fixture(value_type),
        multiplicity: m,
    })
}

fn generalization(identity: &str, specific: &str, general: &str) -> BundleRecord {
    BundleRecord::Generalization(GeneralizationRecord {
        key: ProducerKey::fixture(identity),
        specific: ProducerKey::fixture(specific),
        general: ProducerKey::fixture(general),
    })
}

fn scalar_type(identity: &str, lower: i64, upper: i64) -> BundleRecord {
    BundleRecord::ScalarType(ScalarTypeRecord {
        key: ProducerKey::fixture(identity),
        lower,
        upper,
    })
}

fn redefinition(identity: &str, owner: &str, redefining: &str, redefined: &str) -> BundleRecord {
    BundleRecord::Redefinition(RedefinitionRecord {
        key: ProducerKey::fixture(identity),
        owner: ProducerKey::fixture(owner),
        redefining: ProducerKey::fixture(redefining),
        redefined: ProducerKey::fixture(redefined),
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

#[allow(clippy::too_many_arguments)]
fn operation(
    identity: &str,
    owner: &str,
    parameters: Vec<(&str, &str, Multiplicity)>,
    result: Option<(&str, Multiplicity)>,
    field_writes: Vec<&str>,
    creates: Vec<&str>,
    deletes: Vec<&str>,
    own_postcondition_clauses: Vec<PostconditionClause>,
) -> BundleRecord {
    BundleRecord::OperationMember(OperationMemberRecord {
        key: ProducerKey::fixture(identity),
        owner: ProducerKey::fixture(owner),
        parameters: parameters
            .into_iter()
            .map(|(id, ty, m)| OperationParameterRecord {
                key: ProducerKey::fixture(id),
                value_type: ProducerKey::fixture(ty),
                multiplicity: m,
            })
            .collect(),
        result: result.map(|(ty, m)| OperationResult {
            value_type: ProducerKey::fixture(ty),
            multiplicity: m,
        }),
        effect: OperationEffect {
            field_writes: field_writes.into_iter().map(ProducerKey::fixture).collect(),
            creates: creates.into_iter().map(ProducerKey::fixture).collect(),
            deletes: deletes.into_iter().map(ProducerKey::fixture).collect(),
        },
        has_own_precondition: false,
        own_postcondition_clauses,
        has_body: true,
    })
}

/// H (bundle `bundle.h`): types `A`, `B` (`B` <= `A`); field `model.A.x`
/// typed `model.A` `{0,1}`; field `model.B.y` typed `model.A` `{0,1}`;
/// operation `model.A.op(p1: model.A {0,1})`: `model.B {1,1}`, effect
/// `{fieldWrites: [model.A.x], creates: [model.A], deletes: []}`.
fn fixture_h_base() -> Vec<BundleRecord> {
    vec![
        object_type("model.A"),
        object_type("model.B"),
        generalization("model.gen.B-A", "model.B", "model.A"),
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

fn bundle_h(mut records: Vec<BundleRecord>) -> Bundle {
    records.extend(fixture_h_base());
    Bundle::new(ModelSelection::fixture("bundle.h"), records)
}

/// Finds the effective TYPE entry (no owner) declared with original identity
/// `identity` — mirrors `tests/model_normalization.rs`'s own helper, which
/// this integration-test binary cannot import directly.
fn find_type<'a>(view: &'a EffectiveView, identity: &str) -> &'a ViewEntry {
    view.declarations
        .iter()
        .find(|entry| {
            entry.preimage.owner_effective_type.is_none()
                && entry.preimage.original.identity == identity
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
                && entry.preimage.original.identity == original_identity
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
    let bundle = Bundle::new(
        ModelSelection::fixture("bundle.r01"),
        vec![
            object_type("model.A"),
            object_type("model.B"),
            generalization("model.gen.B-A", "model.B", "model.A"),
            field_member("model.A.n", "model.A", "model.A", mult(0, Some(5))),
            field_member("model.B.n", "model.B", "model.A", mult(1, Some(3))),
            redefinition("model.redef.B.n", "model.B", "model.B.n", "model.A.n"),
        ],
    );
    let record = RedefinitionRecord {
        key: ProducerKey::fixture("model.redef.B.n"),
        owner: ProducerKey::fixture("model.B"),
        redefining: ProducerKey::fixture("model.B.n"),
        redefined: ProducerKey::fixture("model.A.n"),
    };

    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    match check_field_redefinition(&bundle, &record, &mut meter) {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Compatible) => {}
        other => panic!("expected Compatible ({{1,3}} conforms to {{0,5}}), got {other:?}"),
    }

    let view = match normalize(&bundle, ModelNormalizationLimits::UNLIMITED) {
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
            ProducerKey::fixture("model.redef.B.n"),
            ProducerKey::fixture("model.A.n"),
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
            ProducerKey::fixture("model.redef.B.n"),
            ProducerKey::fixture("model.A.n"),
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

// AC-1 is `r01`'s own subject (`check_operation_redefinition` builds no
// effective member — `normalize.rs`'s own module doc says operation
// redefinition builds no phase there). AC-4 is this test's real subject:
// every variance axis admitting independently.
#[trace("TC-196", "FR-151-AC-4")]
#[test]
fn r02_a_compatible_operation_redefinition_admits_every_axis() {
    let bundle = bundle_h(vec![
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
        key: ProducerKey::fixture("model.redef.B.op"),
        owner: ProducerKey::fixture("model.B"),
        redefining: ProducerKey::fixture("model.B.op"),
        redefined: ProducerKey::fixture("model.A.op"),
    };
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    match check_operation_redefinition(&bundle, &record, &mut meter) {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Compatible) => {}
        other => panic!("expected Compatible, got {other:?}"),
    }
}

#[trace("TC-196", "FR-151-AC-2", "FR-151-AC-4")]
#[test]
fn r03_an_incompatible_operation_redefinition_reports_every_failing_axis() {
    let bundle = bundle_h(vec![
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
        key: ProducerKey::fixture("model.redef.B.op"),
        owner: ProducerKey::fixture("model.B"),
        redefining: ProducerKey::fixture("model.B.op"),
        redefined: ProducerKey::fixture("model.A.op"),
    };
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    match check_operation_redefinition(&bundle, &record, &mut meter) {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Refused(failures)) => {
            let causes: Vec<&str> = failures.iter().map(|f| f.cause).collect();
            assert_eq!(
                causes,
                vec![
                    "variance-parameter",
                    "multiplicity-narrowing",
                    "variance-result",
                    "multiplicity-narrowing",
                    "effect-escape",
                ]
            );
            assert!(failures.iter().all(|f| f.code == Code::IllTyped));
        }
        other => panic!("expected Refused, got {other:?}"),
    }
}

#[trace("TC-196", "FR-151-AC-4")]
#[test]
fn r04_an_arity_mismatch_refuses_without_checking_parameter_axes() {
    let bundle = bundle_h(vec![
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
        key: ProducerKey::fixture("model.redef.B.op"),
        owner: ProducerKey::fixture("model.B"),
        redefining: ProducerKey::fixture("model.B.op"),
        redefined: ProducerKey::fixture("model.A.op"),
    };
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    match check_operation_redefinition(&bundle, &record, &mut meter) {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Refused(failures)) => {
            assert_eq!(failures.len(), 1);
            assert_eq!(failures[0].cause, "type-mismatch");
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
            generalization("model.gen.B-A", "model.B", "model.A"),
            field_member("model.A.n", "model.A", "model.A", a_mult),
            field_member("model.B.n", "model.B", "model.A", b_mult),
            redefinition("model.redef.B.n", "model.B", "model.B.n", "model.A.n"),
        ]
    };
    let record = RedefinitionRecord {
        key: ProducerKey::fixture("model.redef.B.n"),
        owner: ProducerKey::fixture("model.B"),
        redefining: ProducerKey::fixture("model.B.n"),
        redefined: ProducerKey::fixture("model.A.n"),
    };

    let narrowing = Bundle::new(
        ModelSelection::fixture("bundle.r05a"),
        records(mult(1, Some(3)), mult(0, Some(5))),
    );
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    match check_field_redefinition(&narrowing, &record, &mut meter) {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Refused(failures)) => {
            assert_eq!(failures.len(), 1);
            assert_eq!(failures[0].cause, "multiplicity-narrowing");
        }
        other => panic!("expected Refused, got {other:?}"),
    }

    let widening = Bundle::new(
        ModelSelection::fixture("bundle.r05b"),
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
        Bundle::new(
            ModelSelection::fixture("bundle.r06"),
            vec![
                object_type("model.A"),
                object_type("model.B"),
                object_type("model.C"),
                generalization("model.gen.B-A", "model.B", "model.A"),
                field_member("model.A.all", "model.A", "model.A", mult(0, Some(5))),
                field_member("model.A.some", "model.A", some_type, some_mult),
                subsetting("model.sub.some", "model.A", "model.A.some", "model.A.all"),
            ],
        )
    };
    let record = SubsettingRecord {
        key: ProducerKey::fixture("model.sub.some"),
        owner: ProducerKey::fixture("model.A"),
        subsetting: ProducerKey::fixture("model.A.some"),
        subsetted: ProducerKey::fixture("model.A.all"),
    };

    // (i) same type, wider multiplicity: multiplicity-narrowing.
    let bundle = bundle_of("model.A", mult(0, Some(9)));
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    match check_subsetting(&bundle, &record, &mut meter) {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Refused(failures)) => {
            assert_eq!(failures.len(), 1);
            assert_eq!(failures[0].cause, "multiplicity-narrowing");
        }
        other => panic!("expected Refused, got {other:?}"),
    }

    // (ii) unrelated type: subsetting-type.
    let bundle = bundle_of("model.C", mult(0, Some(5)));
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    match check_subsetting(&bundle, &record, &mut meter) {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Refused(failures)) => {
            assert_eq!(failures.len(), 1);
            assert_eq!(failures[0].cause, "subsetting-type");
        }
        other => panic!("expected Refused, got {other:?}"),
    }

    // (iii) conforming subtype, narrower multiplicity: admitted.
    let bundle = bundle_of("model.B", mult(0, Some(3)));
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    match check_subsetting(&bundle, &record, &mut meter) {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Compatible) => {}
        other => panic!("expected Compatible, got {other:?}"),
    }
}

#[trace("TC-196", "FR-151-AC-2")]
#[test]
fn r07_zero_or_multiple_inherited_targets_refuse_redefinition_target() {
    // Zero inherited targets: B.z claims to redefine C.w, but B does not
    // conform to C.
    let zero = Bundle::new(
        ModelSelection::fixture("bundle.r07a"),
        vec![
            object_type("model.A"),
            object_type("model.B"),
            object_type("model.C"),
            generalization("model.gen.B-A", "model.B", "model.A"),
            field_member("model.C.w", "model.C", "model.A", mult(0, Some(1))),
            field_member("model.B.z", "model.B", "model.A", mult(0, Some(1))),
            redefinition("model.redef.z", "model.B", "model.B.z", "model.C.w"),
        ],
    );
    match resolve_redefinition_target(
        &zero,
        &ProducerKey::fixture("model.B"),
        &ProducerKey::fixture("model.B.z"),
    ) {
        Ok(RedefinitionTargetOutcome::Refused {
            cause,
            candidates,
            valid_targets,
        }) => {
            assert_eq!(cause, "redefinition-target");
            assert_eq!(candidates.len(), 1);
            assert!(
                valid_targets.is_empty(),
                "zero-target shape must carry no valid targets, got {valid_targets:?}"
            );
        }
        other => panic!("expected Refused(zero targets), got {other:?}"),
    }

    // Two distinct, both-legitimate inherited targets: ambiguous.
    let two = Bundle::new(
        ModelSelection::fixture("bundle.r07b"),
        vec![
            object_type("model.A"),
            object_type("model.B"),
            generalization("model.gen.B-A", "model.B", "model.A"),
            field_member("model.A.x", "model.A", "model.A", mult(0, Some(1))),
            field_member("model.A.x2", "model.A", "model.A", mult(0, Some(1))),
            field_member("model.B.z", "model.B", "model.A", mult(0, Some(1))),
            redefinition("model.redef.z1", "model.B", "model.B.z", "model.A.x"),
            redefinition("model.redef.z2", "model.B", "model.B.z", "model.A.x2"),
        ],
    );
    match resolve_redefinition_target(
        &two,
        &ProducerKey::fixture("model.B"),
        &ProducerKey::fixture("model.B.z"),
    ) {
        Ok(RedefinitionTargetOutcome::Refused {
            cause,
            candidates,
            valid_targets,
        }) => {
            assert_eq!(cause, "redefinition-target");
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
fn r08_base() -> Vec<BundleRecord> {
    vec![
        object_type("model.A"),
        object_type("model.B"),
        generalization("model.gen.B-A", "model.B", "model.A"),
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
    let bundle = Bundle::new(ModelSelection::fixture("bundle.r08a"), records);
    let record = RedefinitionRecord {
        key: ProducerKey::fixture("model.redef.xb"),
        owner: ProducerKey::fixture("model.B"),
        redefining: ProducerKey::fixture("model.B.xb"),
        redefined: ProducerKey::fixture("model.A.x"),
    };
    match check_field_refinement_obligation(&bundle, &record) {
        Ok(ConformanceOutcome::Refused(failures)) => {
            assert_eq!(failures.len(), 1);
            assert_eq!(failures[0].cause, "unproved-refinement");
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
            field: ProducerKey::fixture("model.B.xb"),
        }],
    ));
    records.push(redefinition(
        "model.redef.set",
        "model.B",
        "model.B.set",
        "model.A.set",
    ));
    let bundle = Bundle::new(ModelSelection::fixture("bundle.r08b"), records);
    let record = RedefinitionRecord {
        key: ProducerKey::fixture("model.redef.xb"),
        owner: ProducerKey::fixture("model.B"),
        redefining: ProducerKey::fixture("model.B.xb"),
        redefined: ProducerKey::fixture("model.A.x"),
    };
    match check_field_refinement_obligation(&bundle, &record) {
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
    let bundle = Bundle::new(ModelSelection::fixture("bundle.r08c"), records);
    let record = RedefinitionRecord {
        key: ProducerKey::fixture("model.redef.xr"),
        owner: ProducerKey::fixture("model.B"),
        redefining: ProducerKey::fixture("model.B.xr"),
        redefined: ProducerKey::fixture("model.A.x"),
    };
    match check_field_refinement_obligation(&bundle, &record) {
        Ok(ConformanceOutcome::Refused(failures)) => {
            assert_eq!(failures[0].cause, "unproved-refinement");
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
    let bundle = Bundle::new(ModelSelection::fixture("bundle.r08d"), records);
    let record = RedefinitionRecord {
        key: ProducerKey::fixture("model.redef.cs"),
        owner: ProducerKey::fixture("model.B"),
        redefining: ProducerKey::fixture("model.B.cs"),
        redefined: ProducerKey::fixture("model.A.c"),
    };
    match check_field_refinement_obligation(&bundle, &record) {
        Ok(ConformanceOutcome::Refused(failures)) => {
            assert_eq!(failures[0].cause, "unproved-refinement");
            assert!(failures[0].detail.contains("field-domain"));
        }
        other => panic!("expected Refused(field-domain), got {other:?}"),
    }
}

#[trace("TC-196", "FR-151-AC-6")]
#[test]
fn r08e_and_r08f_an_established_interval_admits_only_when_contained() {
    let record = RedefinitionRecord {
        key: ProducerKey::fixture("model.redef.cs"),
        owner: ProducerKey::fixture("model.B"),
        redefining: ProducerKey::fixture("model.B.cs"),
        redefined: ProducerKey::fixture("model.A.c"),
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
                field: ProducerKey::fixture("model.B.cs"),
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
        Bundle::new(ModelSelection::fixture("bundle.r08ef"), records)
    };

    match check_field_refinement_obligation(&contained(5), &record) {
        Ok(ConformanceOutcome::Compatible) => {}
        other => panic!("expected Compatible (e), got {other:?}"),
    }
    match check_field_refinement_obligation(&contained(6), &record) {
        Ok(ConformanceOutcome::Refused(failures)) => {
            assert_eq!(failures[0].cause, "unproved-refinement");
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
            field: ProducerKey::fixture("model.B.cs"),
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
    let bundle = Bundle::new(ModelSelection::fixture("bundle.r08g"), records);
    let record = RedefinitionRecord {
        key: ProducerKey::fixture("model.redef.cs"),
        owner: ProducerKey::fixture("model.B"),
        redefining: ProducerKey::fixture("model.B.cs"),
        redefined: ProducerKey::fixture("model.A.c"),
    };
    match check_field_refinement_obligation(&bundle, &record) {
        Ok(ConformanceOutcome::Refused(failures)) => {
            assert_eq!(failures[0].cause, "unproved-refinement");
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
        field: ProducerKey::fixture("model.B.cs"),
        operator: OrderingOperator::GreaterOrEqual,
        literal: 0,
    };
    let le_five = PostconditionClause::Comparison {
        field: ProducerKey::fixture("model.B.cs"),
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
            generalization("model.gen.B-A", "model.B", "model.A"),
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
        let bundle = Bundle::new(ModelSelection::fixture("bundle.r08h"), records);
        let record = RedefinitionRecord {
            key: ProducerKey::fixture("model.redef.cs"),
            owner: ProducerKey::fixture("model.B"),
            redefining: ProducerKey::fixture("model.B.cs"),
            redefined: ProducerKey::fixture("model.A.c"),
        };
        match check_field_refinement_obligation(&bundle, &record) {
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
/// panic on this caller-supplied bundle data.
#[trace("TC-196", "FR-151-AC-6")]
#[test]
fn r08i_a_malformed_scalar_domain_refuses_rather_than_panicking() {
    let records = vec![
        object_type("model.A"),
        object_type("model.B"),
        generalization("model.gen.B-A", "model.B", "model.A"),
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
                field: ProducerKey::fixture("model.B.cs"),
                operator: OrderingOperator::LessOrEqual,
                literal: 5,
            }],
        ),
        redefinition("model.redef.set", "model.B", "model.B.set", "model.A.set"),
    ];
    let bundle = Bundle::new(ModelSelection::fixture("bundle.r08i"), records);
    let record = RedefinitionRecord {
        key: ProducerKey::fixture("model.redef.cs"),
        owner: ProducerKey::fixture("model.B"),
        redefining: ProducerKey::fixture("model.B.cs"),
        redefined: ProducerKey::fixture("model.A.c"),
    };
    match check_field_refinement_obligation(&bundle, &record) {
        Err(refusal) => {
            assert_eq!(refusal.code, Code::InvalidModelBinding);
            // FR-272's `invalid_model_binding` cause list is closed; there
            // is no dedicated scalar-domain variant, so this is the
            // catalogued `malformed-declaration`.
            assert_eq!(refusal.cause, "malformed-declaration");
        }
        other => panic!("expected Err(malformed-declaration), got {other:?}"),
    }
}

/// PR #144 review finding #2 regression: two field members sharing a
/// display identity but differing in revision must both survive
/// independently in `ConformanceIndex.fields` — never one collapsing/
/// overwriting the other by identity alone (the exact defect class PR #140
/// fixed in `normalize.rs`). Mirrors
/// `f2_field_members_sharing_an_identity_but_differing_in_revision_both_survive`
/// in `tests/model_normalization.rs`.
#[trace("TC-196")]
#[test]
fn f2_field_members_sharing_an_identity_but_differing_in_revision_both_survive() {
    let revision_1 = ProducerKey::fixture("model.A.x");
    let mut revision_2 = ProducerKey::fixture("model.A.x");
    revision_2.revision.value = "2".to_owned();

    let record = RedefinitionRecord {
        key: ProducerKey::fixture("model.redef.y"),
        owner: ProducerKey::fixture("model.B"),
        redefining: ProducerKey::fixture("model.B.y"),
        redefined: revision_2.clone(),
    };
    let bundle = Bundle::new(
        ModelSelection::fixture("bundle.f2-conformance-revision"),
        vec![
            object_type("model.A"),
            object_type("model.B"),
            // revision "1": a narrow {0,1} upper bound. If the index ever
            // collapsed onto this record instead of `revision_2` (the
            // record.redefined actually names), the redefining {0,3} field
            // below would wrongly fail to narrow it.
            BundleRecord::FieldMember(FieldMemberRecord {
                key: revision_1,
                owner: ProducerKey::fixture("model.A"),
                value_type: ProducerKey::fixture("model.A"),
                multiplicity: mult(0, Some(1)),
            }),
            // revision "2": the actual redefinition target, a wider {0,5}
            // upper bound that the redefining field genuinely narrows.
            BundleRecord::FieldMember(FieldMemberRecord {
                key: revision_2,
                owner: ProducerKey::fixture("model.A"),
                value_type: ProducerKey::fixture("model.A"),
                multiplicity: mult(0, Some(5)),
            }),
            field_member("model.B.y", "model.B", "model.A", mult(0, Some(3))),
        ],
    );

    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    match check_field_redefinition(&bundle, &record, &mut meter) {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Compatible) => {}
        other => {
            panic!("expected Compatible against revision \"2\"'s {{0,5}} bound, got {other:?}")
        }
    }
}
