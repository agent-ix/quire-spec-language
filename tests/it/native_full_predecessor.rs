// SPDX-License-Identifier: AGPL-3.0-or-later
//! Complete native predecessor authority over genuine producer-issued reports.

use ix_trace_rs::trace;
use qsl_eval::temporal::evidence::{EvidenceRequest, ObligationKey, Relation};
use qsl_package::{checked_views::ClauseOccurrence, NativeJoinBudget};
use quire_exact::{Charge, ChargePoint, Integer, Meter};
use quire_spec_language::temporal::{
    evaluate_v2, BindingDimension, CheckedSubject, NativeReportError, ObservationRef, Report,
    ReportOutcome, ReportPredecessorLimits, ReportPredecessorResource, ReportRefusal,
};

const AMPLE: ReportPredecessorLimits = ReportPredecessorLimits::new(1 << 30, 1 << 30);

#[trace("TC-144", "FR-052-AC-12", "FR-052-AC-20")]
#[test]
fn original_request_and_lineage_refusals_precede_new_snapshot_work() {
    super::native_owner_report::with_inputs(
        true,
        false,
        |package, occurrence, evidence, request, observation, _| {
            let original = evaluate(
                &CheckedSubject::new(package, occurrence, evidence, request).unwrap(),
                observation,
            );
            let mut correction = after(request, occurrence, &original);
            let no_snapshot_work = ReportPredecessorLimits::new(0, 0);
            let subject = CheckedSubject::new(package, occurrence, evidence, &correction)
                .unwrap()
                .with_predecessor(&original, no_snapshot_work);
            let foreign_revision = ObservationRef {
                revision: "foreign-observation-revision".into(),
                ..observation.clone()
            };
            assert!(matches!(
                evaluate_v2(&subject, &foreign_revision, &mut meter(), &mut budget()),
                Err(NativeReportError::Report(ReportRefusal::BindingMismatch(
                    BindingDimension::Revision
                )))
            ));
            let revised_observation = ObservationRef {
                reference: "observation:lineage-before-snapshot".into(),
                ..observation.clone()
            };
            let revised = evaluate(
                &CheckedSubject::new(package, occurrence, evidence, &correction)
                    .unwrap()
                    .with_predecessor(&original, AMPLE),
                &revised_observation,
            );
            correction.ledger = revised.ledger().cloned();
            let stale = CheckedSubject::new(package, occurrence, evidence, &correction)
                .unwrap()
                .with_predecessor(&original, no_snapshot_work);
            assert!(matches!(
                evaluate_v2(&stale, &revised_observation, &mut meter(), &mut budget()),
                Err(NativeReportError::Report(ReportRefusal::InvalidPredecessor))
            ));
        },
    );
}

#[trace("TC-144", "FR-052-AC-19", "FR-052-AC-20")]
#[test]
fn strict_read_charges_full_prior_comparison_at_exact_limits_and_refuses_legacy_authority() {
    super::native_owner_report::with_inputs(
        true,
        false,
        |package, occurrence, evidence, request, observation, _| {
            let original = evaluate(
                &CheckedSubject::new(package, occurrence, evidence, request).unwrap(),
                observation,
            );
            let mut corrected_request = after(request, occurrence, &original);
            // The same complete-value authority also applies to invalidation.
            let Relation::Superseding(identity) = &corrected_request.relation else {
                panic!("fixture supplies actual predecessor identity");
            };
            corrected_request.relation = Relation::Invalidating(identity.clone());
            let revised_observation = ObservationRef {
                reference: "observation:strict-read-limits".into(),
                ..observation.clone()
            };
            let corrected = evaluate(
                &CheckedSubject::new(package, occurrence, evidence, &corrected_request)
                    .unwrap()
                    .with_predecessor(&original, AMPLE),
                &revised_observation,
            );
            let bytes = corrected.to_owner_bytes();
            assert_eq!(
                corrected.predecessor_owner_bytes(),
                Some(original.to_owner_bytes().as_slice())
            );
            let mut legacy = bytes.clone();
            legacy[4] = 1;
            let ample_subject =
                CheckedSubject::new(package, occurrence, evidence, &corrected_request)
                    .unwrap()
                    .with_predecessor(&original, AMPLE);
            assert_eq!(
                Report::from_owner_bytes(
                    &legacy,
                    &ample_subject,
                    &revised_observation,
                    &mut budget()
                ),
                Err(ReportRefusal::UnsupportedVersion)
            );
            let mut thresholds = [0_u64; 2];
            for (index, resource) in [
                ReportPredecessorResource::Work,
                ReportPredecessorResource::Storage,
            ]
            .into_iter()
            .enumerate()
            {
                let mut bound = 0;
                for _ in 0..64 {
                    let limits = match resource {
                        ReportPredecessorResource::Work => {
                            ReportPredecessorLimits::new(bound, AMPLE.storage)
                        }
                        ReportPredecessorResource::Storage => {
                            ReportPredecessorLimits::new(AMPLE.work, bound)
                        }
                    };
                    let subject =
                        CheckedSubject::new(package, occurrence, evidence, &corrected_request)
                            .unwrap()
                            .with_predecessor(&original, limits);
                    match Report::from_owner_bytes(
                        &bytes,
                        &subject,
                        &revised_observation,
                        &mut budget(),
                    ) {
                        Ok(decoded) => {
                            assert_eq!(decoded, corrected);
                            thresholds[index] = bound;
                            break;
                        }
                        Err(ReportRefusal::PredecessorLimit {
                            resource: denied,
                            limit,
                            actual,
                        }) => {
                            assert_eq!(denied, resource);
                            assert_eq!(limit, bound);
                            assert!(actual > u128::from(bound));
                            bound = u64::try_from(actual).expect("bounded fixture charge");
                        }
                        other => panic!(
                            "only owner snapshot limits may stop this strict read: {other:?}"
                        ),
                    }
                }
                assert!(
                    thresholds[index] > 0,
                    "strict reading must reach an actual admitting threshold"
                );
            }
            let exact = ReportPredecessorLimits::new(thresholds[0], thresholds[1]);
            let exact_subject =
                CheckedSubject::new(package, occurrence, evidence, &corrected_request)
                    .unwrap()
                    .with_predecessor(&original, exact);
            assert_eq!(
                Report::from_owner_bytes(
                    &bytes,
                    &exact_subject,
                    &revised_observation,
                    &mut budget()
                )
                .unwrap(),
                corrected
            );
            for (limits, resource) in [
                (
                    ReportPredecessorLimits::new(exact.work - 1, exact.storage),
                    ReportPredecessorResource::Work,
                ),
                (
                    ReportPredecessorLimits::new(exact.work, exact.storage - 1),
                    ReportPredecessorResource::Storage,
                ),
            ] {
                let subject =
                    CheckedSubject::new(package, occurrence, evidence, &corrected_request)
                        .unwrap()
                        .with_predecessor(&original, limits);
                assert!(
                    matches!(Report::from_owner_bytes(&bytes, &subject, &revised_observation, &mut budget()),
                    Err(ReportRefusal::PredecessorLimit {resource: denied, ..}) if denied == resource)
                );
            }
        },
    );
}

#[trace("TC-144", "FR-052-AC-20")]
#[test]
fn bounded_owner_writer_precharges_every_frame_and_returns_no_bytes_one_short() {
    super::native_owner_report::with_inputs(
        true,
        false,
        |package, occurrence, evidence, request, observation, _| {
            let original = evaluate(
                &CheckedSubject::new(package, occurrence, evidence, request).unwrap(),
                observation,
            );
            let corrected_request = after(request, occurrence, &original);
            let revised_observation = ObservationRef {
                reference: "observation:bounded-owner-writer".into(),
                ..observation.clone()
            };
            let corrected = evaluate(
                &CheckedSubject::new(package, occurrence, evidence, &corrected_request)
                    .unwrap()
                    .with_predecessor(&original, AMPLE),
                &revised_observation,
            );
            let (bytes, usage) = corrected.to_owner_bytes_bounded(AMPLE).unwrap();
            assert_eq!(bytes, corrected.to_owner_bytes());
            assert!(usage.work > u64::try_from(bytes.len()).unwrap());
            assert!(usage.storage > u64::try_from(bytes.len()).unwrap());
            let exact = ReportPredecessorLimits::new(usage.work, usage.storage);
            assert_eq!(
                corrected.to_owner_bytes_bounded(exact).unwrap(),
                (bytes, usage)
            );
            for (limits, resource, actual) in [
                (
                    ReportPredecessorLimits::new(usage.work - 1, usage.storage),
                    ReportPredecessorResource::Work,
                    usage.work,
                ),
                (
                    ReportPredecessorLimits::new(usage.work, usage.storage - 1),
                    ReportPredecessorResource::Storage,
                    usage.storage,
                ),
            ] {
                assert_eq!(
                    corrected.to_owner_bytes_bounded(limits),
                    Err(ReportRefusal::PredecessorLimit {
                        resource,
                        limit: actual - 1,
                        actual: u128::from(actual),
                    })
                );
            }
        },
    );
}

fn budget() -> NativeJoinBudget {
    NativeJoinBudget::new(1 << 30, 1 << 30)
}

fn meter() -> Meter {
    let mut limits = *super::temporal_package_route::unlimited().limits();
    limits.work_units = 1 << 20;
    Meter::new(limits)
}

fn evaluate(subject: &CheckedSubject<'_>, observation: &ObservationRef) -> Report {
    evaluate_v2(subject, observation, &mut meter(), &mut budget())
        .expect("actual checked native evaluator")
}

fn obligation(occurrence: &ClauseOccurrence) -> ObligationKey {
    let selected = qsl_package::CheckedOccurrence {
        role: occurrence.role.clone(),
        ordinal: occurrence.ordinal,
    };
    ObligationKey::new(
        occurrence.node.package.clone(),
        serde_json::to_vec(&(&occurrence.node, selected)).expect("native obligation selection"),
        occurrence.ordinal,
    )
}

fn after(
    request: &EvidenceRequest,
    occurrence: &ClauseOccurrence,
    previous: &Report,
) -> EvidenceRequest {
    let mut corrected = request.clone();
    let ledger = previous
        .ledger()
        .expect("genuine evaluator next-call ledger");
    let latest = ledger
        .result(&obligation(occurrence), request.trigger.as_ref())
        .expect("actual latest obligation result")
        .result_identity()
        .clone();
    assert_eq!(latest.as_bytes(), previous.result_identity());
    corrected.ledger = Some(ledger.clone());
    corrected.relation = Relation::Superseding(latest);
    corrected
}

#[trace("TC-144", "FR-052-AC-19")]
#[test]
fn genuine_correction_retains_the_complete_prior_and_clone_move_and_reader_preserve_it() {
    super::native_owner_report::with_inputs(
        true,
        false,
        |package, occurrence, evidence, request, observation, _| {
            let original_subject =
                CheckedSubject::new(package, occurrence, evidence, request).unwrap();
            let original = evaluate(&original_subject, observation);
            let saved = original.to_owner_bytes();
            let correction = after(request, occurrence, &original);
            let revised_observation = ObservationRef {
                reference: "observation:full-prior:correction-1".into(),
                ..observation.clone()
            };
            let corrected_subject = CheckedSubject::new(package, occurrence, evidence, &correction)
                .unwrap()
                .with_predecessor(&original, AMPLE);
            let corrected = evaluate(&corrected_subject, &revised_observation);
            assert_eq!(corrected.predecessor(), Some(original.result_identity()));
            assert_eq!(corrected.predecessor_owner_bytes(), Some(saved.as_slice()));
            let usage = corrected.validate_predecessor(&original, AMPLE).unwrap();
            assert!(usage.work > 0 && usage.storage > 0);
            let clone = original.clone();
            let moved = clone;
            assert!(corrected.validate_predecessor(&moved, AMPLE).is_ok());
            let decoded_original =
                Report::from_owner_bytes(&saved, &original_subject, observation, &mut budget())
                    .expect("actual strict original reader");
            assert_eq!(decoded_original, original);
            assert!(corrected
                .validate_predecessor(&decoded_original, AMPLE)
                .is_ok());
            let owner_bytes = corrected.to_owner_bytes();
            let decoded = Report::from_owner_bytes(
                &owner_bytes,
                &corrected_subject,
                &revised_observation,
                &mut budget(),
            )
            .expect("strict reader retains complete predecessor");
            assert_eq!(decoded, corrected);
            assert_eq!(decoded.predecessor_owner_bytes(), Some(saved.as_slice()));
            assert!(decoded.validate_predecessor(&moved, AMPLE).is_ok());
            let cloned = decoded.clone();
            assert_eq!(cloned, decoded);
            assert_eq!(
                cloned.predecessor_owner_bytes(),
                decoded.predecessor_owner_bytes()
            );
            assert!(cloned.validate_predecessor(&moved, AMPLE).is_ok());
            assert_eq!(original.to_owner_bytes(), saved);
            assert_eq!(moved.to_owner_bytes(), saved);
            assert_eq!(decoded.to_owner_bytes(), owner_bytes);
            assert_eq!(
                original.validate_predecessor(&moved, AMPLE),
                Err(ReportRefusal::InvalidPredecessor)
            );
        },
    );
}

#[trace("TC-144", "FR-052-AC-19")]
#[test]
fn equal_result_identity_does_not_authorize_a_different_genuinely_produced_prior_outcome() {
    super::native_owner_report::with_inputs(
        true,
        false,
        |package, occurrence, evidence, request, observation, _| {
            let subject = CheckedSubject::new(package, occurrence, evidence, request).unwrap();
            let original = evaluate(&subject, observation);
            assert_eq!(original.outcome(), ReportOutcome::Evaluated);
            let mut exhausted = meter();
            let same_ceiling = exhausted.limits().work_units;
            exhausted
                .charge(Charge::new(ChargePoint::TemporalVisit).work(Integer::from(same_ceiling)))
                .expect("consume the existing finite scalar allowance before the call");
            let different = evaluate_v2(&subject, observation, &mut exhausted, &mut budget())
                .expect("actual evaluator issues its stopped report");
            assert_eq!(different.outcome(), ReportOutcome::Meter);
            assert_eq!(original.result_identity(), different.result_identity(),
            "the original meter ceiling and request are equal, consumed allowance is not identity authority");
            assert_ne!(original.outcome(), different.outcome());
            assert_ne!(original.support(), different.support());
            assert_ne!(original.to_owner_bytes(), different.to_owner_bytes());
            let correction = after(request, occurrence, &original);
            let revised_observation = ObservationRef {
                reference: "observation:full-prior:correction-equal-id".into(),
                ..observation.clone()
            };
            let corrected_subject = CheckedSubject::new(package, occurrence, evidence, &correction)
                .unwrap()
                .with_predecessor(&original, AMPLE);
            let corrected = evaluate(&corrected_subject, &revised_observation);
            assert!(corrected.validate_predecessor(&original, AMPLE).is_ok());
            assert_eq!(
                corrected.validate_predecessor(&different, AMPLE),
                Err(ReportRefusal::InvalidPredecessor)
            );
            assert_eq!(
                corrected.predecessor_owner_bytes(),
                Some(original.to_owner_bytes().as_slice())
            );
            let substituted_subject =
                CheckedSubject::new(package, occurrence, evidence, &correction)
                    .unwrap()
                    .with_predecessor(&different, AMPLE);
            assert_eq!(
                Report::from_owner_bytes(
                    &corrected.to_owner_bytes(),
                    &substituted_subject,
                    &revised_observation,
                    &mut budget()
                ),
                Err(ReportRefusal::InvalidPredecessor)
            );
        },
    );
}

#[trace("TC-144", "FR-052-AC-19")]
#[test]
fn missing_wrong_and_skipped_predecessors_and_changed_reader_context_refuse() {
    super::native_owner_report::with_inputs(
        true,
        false,
        |package, occurrence, evidence, request, observation, _| {
            let original_subject =
                CheckedSubject::new(package, occurrence, evidence, request).unwrap();
            let original = evaluate(&original_subject, observation);
            let first_request = after(request, occurrence, &original);
            let first_observation = ObservationRef {
                reference: "observation:full-prior:first".into(),
                ..observation.clone()
            };
            let missing =
                CheckedSubject::new(package, occurrence, evidence, &first_request).unwrap();
            assert!(matches!(
                evaluate_v2(&missing, &first_observation, &mut meter(), &mut budget()),
                Err(NativeReportError::Report(ReportRefusal::InvalidPredecessor))
            ));
            let first_subject = CheckedSubject::new(package, occurrence, evidence, &first_request)
                .unwrap()
                .with_predecessor(&original, AMPLE);
            let first = evaluate(&first_subject, &first_observation);
            let second_request = after(request, occurrence, &first);
            let second_observation = ObservationRef {
                reference: "observation:full-prior:second".into(),
                ..observation.clone()
            };
            let second_subject =
                CheckedSubject::new(package, occurrence, evidence, &second_request)
                    .unwrap()
                    .with_predecessor(&first, AMPLE);
            let second = evaluate(&second_subject, &second_observation);
            assert!(second.validate_predecessor(&first, AMPLE).is_ok());
            assert_eq!(
                second.validate_predecessor(&original, AMPLE),
                Err(ReportRefusal::InvalidPredecessor)
            );
            let skipped = CheckedSubject::new(package, occurrence, evidence, &second_request)
                .unwrap()
                .with_predecessor(&original, AMPLE);
            assert!(matches!(
                evaluate_v2(&skipped, &second_observation, &mut meter(), &mut budget()),
                Err(NativeReportError::Report(ReportRefusal::InvalidPredecessor))
            ));
            let original_with_prior = CheckedSubject::new(package, occurrence, evidence, request)
                .unwrap()
                .with_predecessor(&original, AMPLE);
            assert!(matches!(
                evaluate_v2(
                    &original_with_prior,
                    observation,
                    &mut meter(),
                    &mut budget()
                ),
                Err(NativeReportError::Report(ReportRefusal::InvalidPredecessor))
            ));
            let wrong_observation = ObservationRef {
                reference: "observation:foreign-context".into(),
                ..second_observation.clone()
            };
            assert_eq!(
                Report::from_owner_bytes(
                    &second.to_owner_bytes(),
                    &second_subject,
                    &wrong_observation,
                    &mut budget()
                ),
                Err(ReportRefusal::BindingMismatch(
                    BindingDimension::Observation
                ))
            );
            let wrong_revision = ObservationRef {
                revision: "revision:foreign-context".into(),
                ..second_observation.clone()
            };
            assert_eq!(
                Report::from_owner_bytes(
                    &second.to_owner_bytes(),
                    &second_subject,
                    &wrong_revision,
                    &mut budget()
                ),
                Err(ReportRefusal::BindingMismatch(BindingDimension::Revision))
            );
            let mut foreign_request = second_request.clone();
            foreign_request.trigger = Some(
                qsl_eval::temporal::evidence::SemanticTriggerIdentity::new(
                    b"foreign-origin".as_slice(),
                )
                .unwrap(),
            );
            let foreign = CheckedSubject::new(package, occurrence, evidence, &foreign_request)
                .unwrap()
                .with_predecessor(&first, AMPLE);
            assert!(matches!(
                evaluate_v2(&foreign, &second_observation, &mut meter(), &mut budget()),
                Err(NativeReportError::Report(ReportRefusal::InvalidPredecessor))
            ));
            assert_eq!(
                second.predecessor_owner_bytes(),
                Some(first.to_owner_bytes().as_slice())
            );
        },
    );
}

#[trace("TC-144", "FR-052-AC-19")]
#[test]
fn a_substitution_inside_the_retained_complete_prior_is_not_authorized_by_its_identity() {
    super::native_owner_report::with_inputs(
        true,
        false,
        |package, occurrence, evidence, request, observation, _| {
            let original_subject =
                CheckedSubject::new(package, occurrence, evidence, request).unwrap();
            let original = evaluate(&original_subject, observation);
            let prior_bytes = original.to_owner_bytes();
            let corrected_request = after(request, occurrence, &original);
            let revised_observation = ObservationRef {
                reference: "observation:full-prior:bytes".into(),
                ..observation.clone()
            };
            let corrected_subject =
                CheckedSubject::new(package, occurrence, evidence, &corrected_request)
                    .unwrap()
                    .with_predecessor(&original, AMPLE);
            let corrected = evaluate(&corrected_subject, &revised_observation);
            let mut changed = corrected.to_owner_bytes();
            let at = changed
                .windows(prior_bytes.len())
                .position(|window| window == prior_bytes)
                .expect("the exact public full predecessor bytes occur in the correction");
            changed[at + prior_bytes.len() - 1] ^= 1;
            assert_eq!(
                Report::from_owner_bytes(
                    &changed,
                    &corrected_subject,
                    &revised_observation,
                    &mut budget()
                ),
                Err(ReportRefusal::InvalidPredecessor)
            );
            assert!(corrected.validate_predecessor(&original, AMPLE).is_ok());
            assert_eq!(original.to_owner_bytes(), prior_bytes);
        },
    );
}

#[trace("TC-144", "FR-052-AC-20")]
#[test]
fn complete_prior_comparison_has_exact_and_one_short_work_and_retained_byte_storage_limits() {
    super::native_owner_report::with_inputs(
        true,
        false,
        |package, occurrence, evidence, request, observation, _| {
            let original = evaluate(
                &CheckedSubject::new(package, occurrence, evidence, request).unwrap(),
                observation,
            );
            let corrected_request = after(request, occurrence, &original);
            let revised_observation = ObservationRef {
                reference: "observation:full-prior:comparison-bounds".into(),
                ..observation.clone()
            };
            let corrected = evaluate(
                &CheckedSubject::new(package, occurrence, evidence, &corrected_request)
                    .unwrap()
                    .with_predecessor(&original, AMPLE),
                &revised_observation,
            );
            let usage = corrected.validate_predecessor(&original, AMPLE).unwrap();
            assert!(usage.work >= u64::try_from(original.to_owner_bytes().len()).unwrap());
            assert!(usage.storage >= u64::try_from(original.to_owner_bytes().len()).unwrap());
            let exact = ReportPredecessorLimits::new(usage.work, usage.storage);
            assert_eq!(
                corrected.validate_predecessor(&original, exact).unwrap(),
                usage
            );
            for (limits, resource, limit) in [
                (
                    ReportPredecessorLimits::new(usage.work - 1, usage.storage),
                    ReportPredecessorResource::Work,
                    usage.work - 1,
                ),
                (
                    ReportPredecessorLimits::new(usage.work, usage.storage - 1),
                    ReportPredecessorResource::Storage,
                    usage.storage - 1,
                ),
            ] {
                let expected_actual = match resource {
                    ReportPredecessorResource::Work => usage.work,
                    ReportPredecessorResource::Storage => usage.storage,
                };
                assert_eq!(
                    corrected.validate_predecessor(&original, limits),
                    Err(ReportRefusal::PredecessorLimit {
                        resource,
                        limit,
                        actual: u128::from(expected_actual)
                    })
                );
            }
            assert_eq!(
                corrected.validate_predecessor(&original, exact).unwrap(),
                usage,
                "each operation starts fresh after a refusal"
            );
        },
    );
}

#[trace("TC-144", "FR-052-AC-20")]
#[test]
fn issuing_the_snapshot_precharges_actual_byte_copies_and_preserves_the_callers_ir_budget_on_refusal(
) {
    super::native_owner_report::with_inputs(
        true,
        false,
        |package, occurrence, evidence, request, observation, _| {
            let original = evaluate(
                &CheckedSubject::new(package, occurrence, evidence, request).unwrap(),
                observation,
            );
            let corrected_request = after(request, occurrence, &original);
            let revised_observation = ObservationRef {
                reference: "observation:full-prior:issuance-bounds".into(),
                ..observation.clone()
            };
            let mut thresholds = [0_u64; 2];
            for (index, resource) in [
                ReportPredecessorResource::Work,
                ReportPredecessorResource::Storage,
            ]
            .into_iter()
            .enumerate()
            {
                let mut bound = 0;
                for _ in 0..64 {
                    let limits = match resource {
                        ReportPredecessorResource::Work => {
                            ReportPredecessorLimits::new(bound, AMPLE.storage)
                        }
                        ReportPredecessorResource::Storage => {
                            ReportPredecessorLimits::new(AMPLE.work, bound)
                        }
                    };
                    let subject =
                        CheckedSubject::new(package, occurrence, evidence, &corrected_request)
                            .unwrap()
                            .with_predecessor(&original, limits);
                    let mut shared = budget();
                    package
                        .project_exact_type(
                            &evidence.rows()[0].operand,
                            &quire_contract_model::SelectedExactIdentities::empty(),
                            &mut shared,
                        )
                        .expect("actual earlier IR projection of the admitted Boolean predicate");
                    let before = shared.clone();
                    assert!(
                        before.consumed().work > 0,
                        "the earlier genuine IR operation must actually spend its caller budget"
                    );
                    match evaluate_v2(&subject, &revised_observation, &mut meter(), &mut shared) {
                        Ok(report) => {
                            assert_eq!(
                                report.predecessor_owner_bytes(),
                                Some(original.to_owner_bytes().as_slice())
                            );
                            assert_eq!(shared.consumed(), before.consumed(), "this capture-free native evaluation adds no IR identity work and preserves all existing charges");
                            thresholds[index] = bound;
                            break;
                        }
                        Err(NativeReportError::Report(ReportRefusal::PredecessorLimit {
                            resource: denied,
                            limit,
                            actual,
                        })) => {
                            assert_eq!(denied, resource);
                            assert_eq!(limit, bound);
                            assert!(actual > u128::from(bound));
                            assert_eq!(shared, before, "owner snapshot denial precedes and does not repurpose the shared IR ledger");
                            bound = u64::try_from(actual)
                                .expect("small actual producer fixture charge");
                        }
                        other => panic!(
                            "only the measured owner limit may stop snapshot issuance: {other:?}"
                        ),
                    }
                }
                assert!(
                    thresholds[index] > 0,
                    "actual bounded issuance reached its exact threshold"
                );
            }
            let exact = ReportPredecessorLimits::new(thresholds[0], thresholds[1]);
            let subject = CheckedSubject::new(package, occurrence, evidence, &corrected_request)
                .unwrap()
                .with_predecessor(&original, exact);
            let corrected = evaluate(&subject, &revised_observation);
            assert!(corrected.validate_predecessor(&original, AMPLE).is_ok());
            for (limits, resource, limit) in [
                (
                    ReportPredecessorLimits::new(exact.work - 1, exact.storage),
                    ReportPredecessorResource::Work,
                    exact.work - 1,
                ),
                (
                    ReportPredecessorLimits::new(exact.work, exact.storage - 1),
                    ReportPredecessorResource::Storage,
                    exact.storage - 1,
                ),
            ] {
                let subject =
                    CheckedSubject::new(package, occurrence, evidence, &corrected_request)
                        .unwrap()
                        .with_predecessor(&original, limits);
                assert!(
                    matches!(evaluate_v2(&subject, &revised_observation, &mut meter(), &mut budget()),
                Err(NativeReportError::Report(ReportRefusal::PredecessorLimit { resource: actual_resource, limit: actual_limit, actual }))
                if actual_resource == resource && actual_limit == limit && actual > u128::from(limit))
                );
            }
        },
    );
}

#[trace("TC-144", "FR-052-AC-21")]
#[test]
fn a_flat_predecessor_chain_clones_moves_and_drops_without_recursive_report_ownership() {
    super::native_owner_report::with_inputs(
        true,
        false,
        |package, occurrence, evidence, request, observation, _| {
            let mut current = evaluate(
                &CheckedSubject::new(package, occurrence, evidence, request).unwrap(),
                observation,
            );
            for revision in 1..=32 {
                let corrected_request = after(request, occurrence, &current);
                let revised_observation = ObservationRef {
                    reference: format!("observation:flat-chain:{revision}"),
                    ..observation.clone()
                };
                let subject =
                    CheckedSubject::new(package, occurrence, evidence, &corrected_request)
                        .unwrap()
                        .with_predecessor(&current, AMPLE);
                let next = evaluate(&subject, &revised_observation);
                assert_eq!(
                    next.predecessor_owner_bytes(),
                    Some(current.to_owner_bytes().as_slice())
                );
                assert!(next.validate_predecessor(&current, AMPLE).is_ok());
                current = next;
            }
            let clone = current.clone();
            assert_eq!(
                clone.predecessor_owner_bytes(),
                current.predecessor_owner_bytes()
            );
            let moved = clone;
            assert_eq!(moved.to_owner_bytes(), current.to_owner_bytes());
            std::thread::Builder::new()
                .name("flat-native-predecessor-drop".into())
                .stack_size(512 * 1024)
                .spawn(move || {
                    let cloned = moved.clone();
                    assert_eq!(
                        cloned.predecessor_owner_bytes(),
                        moved.predecessor_owner_bytes()
                    );
                    assert_eq!(cloned.to_owner_bytes(), moved.to_owner_bytes());
                    drop(cloned);
                    drop(moved);
                })
                .expect("small-stack owner drop thread")
                .join()
                .expect("flat bytes have no recursive Report drop");
            assert!(current.predecessor_owner_bytes().is_some());
        },
    );
}

#[trace("TC-144", "FR-052-AC-19")]
#[test]
fn a_genuinely_emitted_foreign_checked_subject_does_not_read_the_bound_correction() {
    super::native_owner_report::with_inputs(
        true,
        false,
        |package, occurrence, evidence, request, observation, _| {
            let original = evaluate(
                &CheckedSubject::new(package, occurrence, evidence, request).unwrap(),
                observation,
            );
            let corrected_request = after(request, occurrence, &original);
            let revised_observation = ObservationRef {
                reference: "observation:foreign-checked-context".into(),
                ..observation.clone()
            };
            let corrected = evaluate(
                &CheckedSubject::new(package, occurrence, evidence, &corrected_request)
                    .unwrap()
                    .with_predecessor(&original, AMPLE),
                &revised_observation,
            );
            let built = super::temporal_package_route::event_clauses(
            "temporal Foreign using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin { eventually[0,1] holds(c.versionNumber = 2) }",
            &[("Foreign", "foreign-authority")],
        );
            let read = built.read(&built.emission);
            let foreign_package = read.admitted();
            let bare = qsl_package::BarePackage::new(foreign_package);
            let engine =
                qsl_eval::temporal::evidence::EvidencePackage::from_package(&bare, &mut budget())
                    .expect("actual emitted foreign engine");
            let node = &engine
                .rows()
                .names()
                .iter()
                .find(|named| &*named.name == "Foreign")
                .expect("authored foreign clause")
                .declaration;
            let view = qsl_package::checked_views::temporal_subject(foreign_package, node).unwrap();
            let foreign_occurrence = &view.occurrences()[0];
            let foreign_request = super::temporal_package_route::request(
                &engine,
                node,
                Some(foreign_occurrence.ordinal),
                2,
            );
            use quire_spec_language::temporal::{
                ClockBinding, Closure, Completeness, Evidence, Execution, Trace, ValuationEvidence,
                ValuationInstance,
            };
            let trace = Trace {
                clock: ClockBinding {
                    name: "model-steps".into(),
                    profile_identity: quire_spec_language::temporal::EVENT_POSITION.into(),
                    parameters: std::collections::BTreeMap::new(),
                },
                positions: Vec::new(),
                anchor: "obs0".into(),
                triggers: Vec::new(),
                trigger_evidence: Evidence::Admitted,
                trigger_scope: Closure::Closed,
                decision_scope: Closure::Closed,
                surrounding_execution: Closure::Closed,
                execution: Execution::Completed,
                completeness: Completeness::Complete,
                authoritative_origin: true,
                watermark: 100,
                evicted: Vec::new(),
            };
            let empty = ValuationEvidence::admit(
                &view,
                &trace,
                ValuationInstance {
                    anchor: "obs0",
                    capture_instance: "origin:o1",
                },
                &observation.revision,
                Vec::new(),
            )
            .expect("actual empty foreign relation");
            let foreign = CheckedSubject::new(
                foreign_package,
                foreign_occurrence,
                &empty,
                &foreign_request,
            )
            .unwrap();
            assert_ne!(foreign_occurrence.node, occurrence.node);
            assert!(matches!(
                Report::from_owner_bytes(
                    &corrected.to_owner_bytes(),
                    &foreign,
                    &revised_observation,
                    &mut budget()
                ),
                Err(ReportRefusal::BindingMismatch(
                    BindingDimension::PackageIdentity
                        | BindingDimension::Clause
                        | BindingDimension::SourceMap
                ))
            ));
            assert!(corrected.validate_predecessor(&original, AMPLE).is_ok());
        },
    );
}
