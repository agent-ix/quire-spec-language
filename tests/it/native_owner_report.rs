// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native owner reports over the checked-package temporal engine.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_eval::temporal::evidence::{EvidencePackage, Truth};
use qsl_package::checked_views::temporal_subject;
use quire_spec_language::temporal::{
    evaluate_v2, BindingDimension, CheckedSubject, ClockBinding, Closure, Completeness, Execution,
    NativeReportError, ObservationRef, Position, Report, ReportRefusal, Trace, ValuationEvidence,
    ValuationInstance, ValuationRow, EVENT_POSITION,
};

fn with_subject(check: impl FnOnce(&CheckedSubject, &ObservationRef, &[ValuationRow])) {
    with_inputs(
        true,
        false,
        |package, occurrence, evidence, request, observation, rows| {
            let subject = CheckedSubject::new(package, occurrence, evidence, request)
                .expect("checked evaluation subject");
            check(&subject, observation, rows);
        },
    );
}

fn with_inputs(
    value: bool,
    missing: bool,
    check: impl FnOnce(
        &quire_contract_model::CheckedPackageV2,
        &qsl_package::checked_views::ClauseOccurrence,
        &ValuationEvidence,
        &qsl_eval::temporal::evidence::EvidenceRequest,
        &ObservationRef,
        &[ValuationRow],
    ),
) {
    let (package, _) = super::checked_views::recipe_package("event_position");
    let bare = qsl_package::BarePackage::new(&package);
    let engine = EvidencePackage::from_package(
        &bare,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("checked engine package");
    let node = &engine
        .rows()
        .names()
        .iter()
        .find(|name| &*name.name == "Due")
        .expect("origin-activated always clause")
        .declaration;
    let view = temporal_subject(&package, node).expect("checked subject view");
    let occurrence = &view.occurrences()[0];
    let operand = &view.predicate_leaves()[0];
    let request =
        super::temporal_package_route::request(&engine, node, Some(occurrence.ordinal), 2);
    let rows: Vec<_> = [0, 0, 0, 1]
        .into_iter()
        .filter(|position| !missing || *position == 0)
        .map(|position| ValuationRow {
            subject: occurrence.clone(),
            position,
            anchor: "obs0".into(),
            capture_instance: "origin:o1".into(),
            revision: "revision:7".into(),
            operand: operand.clone(),
            value,
        })
        .collect();
    let trace = Trace {
        clock: ClockBinding {
            name: "events".into(),
            profile_identity: EVENT_POSITION.into(),
            parameters: BTreeMap::new(),
        },
        positions: (0..2)
            .map(|coordinate| Position {
                coordinate,
                order: None,
                valuations: BTreeMap::new(),
                instance_valuations: BTreeMap::new(),
            })
            .collect(),
        anchor: "obs0".into(),
        triggers: Vec::new(),
        trigger_evidence: quire_spec_language::temporal::Evidence::Admitted,
        trigger_scope: Closure::Closed,
        decision_scope: Closure::Closed,
        surrounding_execution: Closure::Closed,
        execution: Execution::Completed,
        completeness: Completeness::Complete,
        authoritative_origin: true,
        watermark: 100,
        evicted: Vec::new(),
    };
    let evidence = ValuationEvidence::admit(
        &view,
        &trace,
        ValuationInstance {
            anchor: "obs0",
            capture_instance: "origin:o1",
        },
        "revision:7",
        rows.clone(),
    )
    .expect("all four row occurrences admitted");
    let observation = ObservationRef {
        reference: "observation:opaque".into(),
        revision: "revision:7".into(),
    };
    check(
        &package,
        occurrence,
        &evidence,
        &request,
        &observation,
        &rows,
    );
}

#[trace("TC-144", "FR-052-AC-15", "FR-052-AC-16", "FR-052-AC-17")]
#[test]
fn native_report_owns_truth_and_preserves_duplicate_support_and_roundtrip() {
    with_subject(|subject, observation, rows| {
        let report = evaluate_v2(
            subject,
            observation,
            &mut super::temporal_package_route::unlimited(),
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("native owner evaluates");
        assert_eq!(report.truth(), Some(Truth::True));
        assert_eq!(
            report
                .support()
                .iter()
                .map(|entry| &entry.row)
                .collect::<Vec<_>>(),
            rows.iter().collect::<Vec<_>>()
        );
        assert_eq!(
            report
                .support()
                .iter()
                .map(|entry| entry.position)
                .collect::<Vec<_>>(),
            [0, 0, 0, 1]
        );
        let bytes = report.to_owner_bytes();
        assert_eq!(bytes, report.to_owner_bytes());
        let decoded = Report::from_owner_bytes(
            &bytes,
            subject,
            observation,
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("strict reader admits owner output");
        assert_eq!(decoded, report);
        assert_eq!(decoded.to_owner_bytes(), bytes);
        let repeated = evaluate_v2(
            subject,
            observation,
            &mut super::temporal_package_route::unlimited(),
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("same input reevaluates");
        assert_eq!(repeated.to_owner_bytes(), bytes);
    });
}

#[trace("TC-144", "FR-052-AC-18")]
#[test]
fn every_truncated_prefix_and_extra_byte_is_refused() {
    with_subject(|subject, observation, _| {
        let report = evaluate_v2(
            subject,
            observation,
            &mut super::temporal_package_route::unlimited(),
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("evaluated");
        let bytes = report.to_owner_bytes();
        for end in 0..bytes.len() {
            assert_eq!(
                Report::from_owner_bytes(
                    &bytes[..end],
                    subject,
                    observation,
                    &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
                ),
                Err(ReportRefusal::Truncated),
                "prefix {end}"
            );
        }
        let mut appended = bytes.clone();
        appended.push(0);
        assert_eq!(
            Report::from_owner_bytes(
                &appended,
                subject,
                observation,
                &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
            ),
            Err(ReportRefusal::TrailingBytes)
        );
        for index in [0, 4] {
            let mut mutated = bytes.clone();
            mutated[index] = 255;
            assert_eq!(
                Report::from_owner_bytes(
                    &mutated,
                    subject,
                    observation,
                    &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
                ),
                Err(ReportRefusal::UnsupportedVersion)
            );
        }
        let mut reordered = bytes.clone();
        reordered[5] = 255;
        assert_eq!(
            Report::from_owner_bytes(
                &reordered,
                subject,
                observation,
                &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
            ),
            Err(ReportRefusal::OutOfOrder)
        );
    });
}

#[trace("TC-144", "FR-052-AC-18")]
#[test]
fn opaque_observation_and_revision_binding_are_independent() {
    with_subject(|subject, observation, _| {
        let report = evaluate_v2(
            subject,
            observation,
            &mut super::temporal_package_route::unlimited(),
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("evaluated");
        let bytes = report.to_owner_bytes();
        let foreign_observation = ObservationRef {
            reference: "different".into(),
            ..observation.clone()
        };
        assert_eq!(
            Report::from_owner_bytes(
                &bytes,
                subject,
                &foreign_observation,
                &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
            ),
            Err(ReportRefusal::BindingMismatch(
                BindingDimension::Observation
            ))
        );
        let foreign_revision = ObservationRef {
            revision: "different".into(),
            ..observation.clone()
        };
        assert_eq!(
            Report::from_owner_bytes(
                &bytes,
                subject,
                &foreign_revision,
                &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
            ),
            Err(ReportRefusal::BindingMismatch(BindingDimension::Revision))
        );
    });
}

/// Exact field spans, independent of the production reader, for single mutations.
fn fields(bytes: &[u8], base: usize) -> Vec<(usize, usize, usize)> {
    assert_eq!(&bytes[base..base + 4], b"QSLR");
    let mut cursor = base + 5;
    let mut result = Vec::new();
    while cursor < bytes.len() {
        let length = u64::from_le_bytes(bytes[cursor + 1..cursor + 9].try_into().unwrap());
        let end = cursor + 9 + usize::try_from(length).unwrap();
        result.push((cursor, cursor + 9, end));
        cursor = end;
    }
    result
}

#[trace("TC-144", "FR-052-AC-18")]
#[test]
fn binding_outcome_member_order_and_support_mutations_are_typed_refusals() {
    with_subject(|subject, observation, _| {
        let report = evaluate_v2(
            subject,
            observation,
            &mut super::temporal_package_route::unlimited(),
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .unwrap();
        let bytes = report.to_owner_bytes();
        let spans = fields(&bytes, 0);
        for (index, dimension) in [
            BindingDimension::PackageIdentity,
            BindingDimension::PackageVersion,
            BindingDimension::Clause,
            BindingDimension::Occurrence,
            BindingDimension::SourceMap,
        ]
        .into_iter()
        .enumerate()
        {
            let mut changed = bytes.clone();
            changed[spans[index].1] ^= 1;
            assert_eq!(
                Report::from_owner_bytes(
                    &changed,
                    subject,
                    observation,
                    &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
                ),
                Err(ReportRefusal::BindingMismatch(dimension))
            );
        }
        let mut outcome = bytes.clone();
        outcome[spans[7].1] = 255;
        assert_eq!(
            Report::from_owner_bytes(
                &outcome,
                subject,
                observation,
                &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
            ),
            Err(ReportRefusal::UnknownTag)
        );
        let mut swapped = bytes[..5].to_vec();
        swapped.extend_from_slice(&bytes[spans[1].0..spans[1].2]);
        swapped.extend_from_slice(&bytes[spans[0].0..spans[0].2]);
        swapped.extend_from_slice(&bytes[spans[2].0..]);
        assert_eq!(
            Report::from_owner_bytes(
                &swapped,
                subject,
                observation,
                &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
            ),
            Err(ReportRefusal::OutOfOrder)
        );
        let mut duplicate = bytes[..spans[1].0].to_vec();
        duplicate.extend_from_slice(&bytes[spans[0].0..spans[0].2]);
        duplicate.extend_from_slice(&bytes[spans[1].0..]);
        assert_eq!(
            Report::from_owner_bytes(
                &duplicate,
                subject,
                observation,
                &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
            ),
            Err(ReportRefusal::OutOfOrder)
        );
        let support_start = spans[8].1;
        let support = fields(&bytes[support_start..spans[8].2], 0);
        let first_row_start = support_start + support[1].1;
        let row = fields(&bytes[first_row_start..support_start + support[1].2], 0);
        let mut position = bytes.clone();
        position[first_row_start + row[0].1] = 200;
        assert_eq!(
            Report::from_owner_bytes(
                &position,
                subject,
                observation,
                &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
            ),
            Err(ReportRefusal::SupportNotInTrace)
        );
        let key_start = first_row_start + row[1].1;
        let key = fields(&bytes[key_start..first_row_start + row[1].2], 0);
        let mut foreign_key = bytes.clone();
        foreign_key[key_start + key[4].1] ^= 1;
        assert_eq!(
            Report::from_owner_bytes(
                &foreign_key,
                subject,
                observation,
                &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
            ),
            Err(ReportRefusal::SupportNotInTrace)
        );
    });
}

#[trace("TC-144", "FR-052-AC-15", "FR-052-AC-16")]
#[test]
fn opposite_and_missing_valuations_are_computed_from_admitted_rows() {
    for (value, missing, expected) in [(false, false, Some(Truth::False)), (true, true, None)] {
        with_inputs(
            value,
            missing,
            |package, occurrence, evidence, request, observation, _| {
                // The fixture request still asserts true; the owner must read the admitted rows.
                let subject = CheckedSubject::new(package, occurrence, evidence, request).unwrap();
                let report = evaluate_v2(
                    &subject,
                    observation,
                    &mut super::temporal_package_route::unlimited(),
                    &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
                )
                .unwrap();
                assert_eq!(report.truth(), expected);
                if missing {
                    assert_eq!(
                        report.outcome(),
                        quire_spec_language::temporal::ReportOutcome::Missing
                    );
                }
                assert!(report
                    .support()
                    .iter()
                    .all(|entry| evidence.rows().contains(&entry.row)));
            },
        );
    }
}

#[trace("TC-144", "FR-052-AC-12", "FR-052-AC-17", "FR-052-AC-18")]
#[test]
fn correction_keeps_direct_predecessor_immutable_and_rejects_stale_or_foreign_identity() {
    use qsl_eval::temporal::evidence::{ObligationKey, Relation};
    with_inputs(
        true,
        false,
        |package, occurrence, evidence, request, observation, _| {
            let subject = CheckedSubject::new(package, occurrence, evidence, request).unwrap();
            let original = evaluate_v2(
                &subject,
                observation,
                &mut super::temporal_package_route::unlimited(),
                &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
            )
            .unwrap();
            let original_bytes = original.to_owner_bytes();
            let selected = qsl_package::CheckedOccurrence {
                role: occurrence.role.clone(),
                ordinal: occurrence.ordinal,
            };
            let key = ObligationKey::new(
                occurrence.node.package.clone(),
                serde_json::to_vec(&(&occurrence.node, selected)).unwrap(),
                occurrence.ordinal,
            );
            let original_identity = original
                .ledger()
                .unwrap()
                .result(&key, request.trigger.as_ref())
                .unwrap()
                .result_identity()
                .clone();
            assert_eq!(original_identity.as_bytes(), original.result_identity());
            let mut correction = request.clone();
            correction.ledger = original.ledger().cloned();
            correction.relation = Relation::Superseding(original_identity.clone());
            let revised_subject =
                CheckedSubject::new(package, occurrence, evidence, &correction).unwrap();
            assert!(matches!(
                evaluate_v2(
                    &revised_subject,
                    observation,
                    &mut super::temporal_package_route::unlimited(),
                    &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
                ),
                Err(quire_spec_language::temporal::NativeReportError::Evaluation(_))
            ));
            let revised_observation = ObservationRef {
                reference: "observation:opaque:revised".into(),
                ..observation.clone()
            };
            let revised = evaluate_v2(
                &revised_subject,
                &revised_observation,
                &mut super::temporal_package_route::unlimited(),
                &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
            )
            .unwrap();
            assert_eq!(revised.predecessor(), Some(original.result_identity()));
            assert_ne!(revised.result_identity(), original.result_identity());
            assert_eq!(original.to_owner_bytes(), original_bytes);
            assert_eq!(
                Report::from_owner_bytes(
                    &revised.to_owner_bytes(),
                    &revised_subject,
                    &revised_observation,
                    &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
                )
                .unwrap(),
                revised
            );
            // Once the revision is current, the earlier result is no longer direct.
            correction.ledger = revised.ledger().cloned();
            let stale = CheckedSubject::new(package, occurrence, evidence, &correction).unwrap();
            assert!(matches!(
                evaluate_v2(
                    &stale,
                    observation,
                    &mut super::temporal_package_route::unlimited(),
                    &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
                ),
                Err(quire_spec_language::temporal::NativeReportError::Report(
                    ReportRefusal::InvalidPredecessor
                ))
            ));
            assert_eq!(
                Report::from_owner_bytes(
                    &revised.to_owner_bytes(),
                    &stale,
                    &revised_observation,
                    &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
                ),
                Err(ReportRefusal::InvalidPredecessor)
            );
            // A predecessor known in the ledger under another trigger grants no authority.
            correction.ledger = original.ledger().cloned();
            correction.trigger = Some(
                qsl_eval::temporal::evidence::SemanticTriggerIdentity::new(
                    b"foreign-trigger".as_slice(),
                )
                .unwrap(),
            );
            let foreign = CheckedSubject::new(package, occurrence, evidence, &correction).unwrap();
            assert!(matches!(
                evaluate_v2(
                    &foreign,
                    observation,
                    &mut super::temporal_package_route::unlimited(),
                    &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
                ),
                Err(quire_spec_language::temporal::NativeReportError::Report(
                    ReportRefusal::InvalidPredecessor
                ))
            ));
            assert_eq!(original.to_owner_bytes(), original_bytes);
        },
    );
}

#[trace("TC-144", "FR-052-AC-12", "FR-052-AC-18")]
#[test]
fn reader_rejects_a_correction_self_edge_and_original_with_invented_predecessor() {
    use qsl_eval::temporal::evidence::{ObligationKey, Relation};
    with_inputs(
        true,
        false,
        |package, occurrence, evidence, request, observation, _| {
            let subject = CheckedSubject::new(package, occurrence, evidence, request).unwrap();
            let original = evaluate_v2(
                &subject,
                observation,
                &mut super::temporal_package_route::unlimited(),
                &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
            )
            .unwrap();
            let selected = qsl_package::CheckedOccurrence {
                role: occurrence.role.clone(),
                ordinal: occurrence.ordinal,
            };
            let key = ObligationKey::new(
                occurrence.node.package.clone(),
                serde_json::to_vec(&(&occurrence.node, selected)).unwrap(),
                occurrence.ordinal,
            );
            let identity = original
                .ledger()
                .unwrap()
                .result(&key, request.trigger.as_ref())
                .unwrap()
                .result_identity()
                .clone();
            let mut correction = request.clone();
            correction.ledger = original.ledger().cloned();
            correction.relation = Relation::Superseding(identity);
            let revised_subject =
                CheckedSubject::new(package, occurrence, evidence, &correction).unwrap();
            let revised_observation = ObservationRef {
                reference: "observation:opaque:revised".into(),
                ..observation.clone()
            };
            let revised = evaluate_v2(
                &revised_subject,
                &revised_observation,
                &mut super::temporal_package_route::unlimited(),
                &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
            )
            .unwrap();
            let bytes = revised.to_owner_bytes();
            let spans = fields(&bytes, 0);
            let mut self_edge = bytes[..spans[9].0].to_vec();
            self_edge.push(10);
            self_edge.extend_from_slice(
                &u64::try_from(original.result_identity().len())
                    .unwrap()
                    .to_le_bytes(),
            );
            self_edge.extend_from_slice(original.result_identity());
            self_edge.extend_from_slice(&bytes[spans[9].2..]);
            assert_eq!(
                Report::from_owner_bytes(
                    &self_edge,
                    &revised_subject,
                    &revised_observation,
                    &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
                ),
                Err(ReportRefusal::InvalidPredecessor)
            );
            // Alter only lineage metadata: the result identity differs, but
            // its evaluation input still equals the predecessor's input.
            let mut same_input = original.result_identity().to_vec();
            let mut cursor = 0;
            let mut changed_relation = false;
            while cursor < same_input.len() {
                let tag = same_input[cursor];
                let length = usize::try_from(u64::from_be_bytes(
                    same_input[cursor + 1..cursor + 9].try_into().unwrap(),
                ))
                .unwrap();
                if tag == 11 {
                    same_input[cursor + 9] ^= 1;
                    changed_relation = true;
                    break;
                }
                cursor += 9 + length;
            }
            assert!(changed_relation);
            assert_ne!(same_input, original.result_identity());
            let mut unchanged_input = bytes[..spans[9].0].to_vec();
            unchanged_input.push(10);
            unchanged_input
                .extend_from_slice(&u64::try_from(same_input.len()).unwrap().to_le_bytes());
            unchanged_input.extend_from_slice(&same_input);
            unchanged_input.extend_from_slice(&bytes[spans[9].2..]);
            assert_eq!(
                Report::from_owner_bytes(
                    &unchanged_input,
                    &revised_subject,
                    &revised_observation,
                    &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
                ),
                Err(ReportRefusal::InvalidPredecessor)
            );

            // The same evidence cannot acquire correction authority by
            // replacing its identity with either an incomplete encoding or
            // the valid canonical identity of another observation.
            let original_bytes = original.to_owner_bytes();
            let original_spans = fields(&original_bytes, 0);
            let mut unchanged_correction = original_bytes[..original_spans[9].0].to_vec();
            unchanged_correction.extend_from_slice(&bytes[spans[9].0..]);
            for forged in [
                vec![17, 0, 0, 0, 0, 0, 0, 0, 0],
                revised.result_identity().to_vec(),
            ] {
                let forged = replace_identity(&unchanged_correction, &forged);
                assert_eq!(
                    Report::from_owner_bytes(
                        &forged,
                        &revised_subject,
                        observation,
                        &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
                    ),
                    Err(ReportRefusal::UnknownTag),
                    "an invented changed-input identity must not authorize unchanged evidence"
                );
            }

            let identity = original.result_identity();
            let first_length =
                usize::try_from(u64::from_be_bytes(identity[1..9].try_into().unwrap())).unwrap();
            let missing_clause = identity[9 + first_length..].to_vec();
            let mut empty_clause = vec![1, 0, 0, 0, 0, 0, 0, 0, 0];
            empty_clause.extend_from_slice(&missing_clause);
            // Empty, missing and truncated members all refuse on the actual
            // public owner reader, even for an original with no lineage.
            for malformed in [
                vec![17, 0, 0, 0, 0, 0, 0, 0, 0],
                missing_clause,
                empty_clause,
                identity[..identity.len() - 1].to_vec(),
            ] {
                assert_eq!(
                    Report::from_owner_bytes(
                        &replace_identity(&original_bytes, &malformed),
                        &subject,
                        observation,
                        &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
                    ),
                    Err(ReportRefusal::UnknownTag)
                );
            }

            assert_eq!(
                Report::from_owner_bytes(
                    &bytes,
                    &subject,
                    &revised_observation,
                    &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
                ),
                Err(ReportRefusal::InvalidPredecessor)
            );
        },
    );
}

fn replace_identity(bytes: &[u8], identity: &[u8]) -> Vec<u8> {
    let spans = fields(bytes, 0);
    let mut replaced = bytes[..spans[9].0].to_vec();
    replaced.push(10);
    replaced.extend_from_slice(&u64::try_from(identity.len()).unwrap().to_le_bytes());
    replaced.extend_from_slice(identity);
    replaced.extend_from_slice(&bytes[spans[9].2..]);
    replaced
}

fn refuse_cross_binding_predecessor(
    package: &quire_contract_model::CheckedPackageV2,
    first: &qsl_package::checked_views::ClauseOccurrence,
    second: &qsl_package::checked_views::ClauseOccurrence,
) {
    use qsl_eval::temporal::evidence::{ObligationKey, Relation};
    let bare = qsl_package::BarePackage::new(package);
    let engine = EvidencePackage::from_package(
        &bare,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .unwrap();
    let request =
        super::temporal_package_route::request(&engine, &first.node, Some(first.ordinal), 1);
    let view = temporal_subject(package, &first.node).unwrap();
    let trace = Trace {
        clock: ClockBinding {
            name: "events".into(),
            profile_identity: EVENT_POSITION.into(),
            parameters: BTreeMap::new(),
        },
        positions: vec![Position {
            coordinate: 0,
            order: None,
            valuations: BTreeMap::new(),
            instance_valuations: BTreeMap::new(),
        }],
        anchor: "obs0".into(),
        triggers: Vec::new(),
        trigger_evidence: quire_spec_language::temporal::Evidence::Admitted,
        trigger_scope: Closure::Closed,
        decision_scope: Closure::Closed,
        surrounding_execution: Closure::Closed,
        execution: Execution::Completed,
        completeness: Completeness::Complete,
        authoritative_origin: true,
        watermark: 100,
        evicted: Vec::new(),
    };
    let rows = vec![ValuationRow {
        subject: first.clone(),
        position: 0,
        anchor: "obs0".into(),
        capture_instance: "origin:o1".into(),
        revision: "revision:7".into(),
        operand: view.predicate_leaves()[0].clone(),
        value: true,
    }];
    let evidence = ValuationEvidence::admit(
        &view,
        &trace,
        ValuationInstance {
            anchor: "obs0",
            capture_instance: "origin:o1",
        },
        "revision:7",
        rows,
    )
    .unwrap();
    let subject = CheckedSubject::new(package, first, &evidence, &request).unwrap();
    let observation = ObservationRef {
        reference: "opaque".into(),
        revision: "revision:7".into(),
    };
    let original = evaluate_v2(
        &subject,
        &observation,
        &mut super::temporal_package_route::unlimited(),
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .unwrap();
    let unchanged = original.to_owner_bytes();
    let selected = qsl_package::CheckedOccurrence {
        role: first.role.clone(),
        ordinal: first.ordinal,
    };
    let key = ObligationKey::new(
        first.node.package.clone(),
        serde_json::to_vec(&(&first.node, selected)).unwrap(),
        first.ordinal,
    );
    let identity = original
        .ledger()
        .unwrap()
        .result(&key, request.trigger.as_ref())
        .unwrap()
        .result_identity()
        .clone();
    let second_view = temporal_subject(package, &second.node).unwrap();
    let second_evidence = ValuationEvidence::admit(
        &second_view,
        &trace,
        ValuationInstance {
            anchor: "obs0",
            capture_instance: "origin:o1",
        },
        "revision:7",
        Vec::new(),
    )
    .unwrap();
    let mut correction =
        super::temporal_package_route::request(&engine, &second.node, Some(second.ordinal), 1);
    let second_subject =
        CheckedSubject::new(package, second, &second_evidence, &correction).unwrap();
    let second_report = evaluate_v2(
        &second_subject,
        &observation,
        &mut super::temporal_package_route::unlimited(),
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .unwrap();
    assert_eq!(second_report.truth(), None);
    assert_eq!(
        second_report.outcome(),
        quire_spec_language::temporal::ReportOutcome::Missing
    );
    assert_eq!(second_report.subject().clause, second.node);
    assert_eq!(second_report.subject().occurrence.ordinal, second.ordinal);
    if first.node == second.node {
        assert_eq!(original.subject().clause, second_report.subject().clause);
        assert_ne!(
            original.subject().occurrence,
            second_report.subject().occurrence
        );
    }
    assert_eq!(
        Report::from_owner_bytes(
            &original.to_owner_bytes(),
            &subject,
            &observation,
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
        )
        .unwrap(),
        original
    );
    assert_eq!(
        Report::from_owner_bytes(
            &second_report.to_owner_bytes(),
            &second_subject,
            &observation,
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
        )
        .unwrap(),
        second_report
    );
    correction.ledger = original.ledger().cloned();
    correction.relation = Relation::Superseding(identity);
    let foreign = CheckedSubject::new(package, second, &second_evidence, &correction).unwrap();
    assert!(matches!(
        evaluate_v2(
            &foreign,
            &observation,
            &mut super::temporal_package_route::unlimited(),
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
        ),
        Err(quire_spec_language::temporal::NativeReportError::Report(
            ReportRefusal::InvalidPredecessor
        ))
    ));
    assert_eq!(original.to_owner_bytes(), unchanged);
}

#[trace("TC-144", "FR-052-AC-12", "FR-052-AC-15")]
#[test]
fn another_clause_and_another_occurrence_cannot_borrow_predecessor_authority() {
    let (package, _) = super::checked_views::recipe_package("event_position");
    let bare = qsl_package::BarePackage::new(&package);
    let engine = EvidencePackage::from_package(
        &bare,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .unwrap();
    let first = &engine
        .rows()
        .names()
        .iter()
        .find(|name| &*name.name == "Due")
        .unwrap()
        .declaration;
    let second = &engine
        .rows()
        .names()
        .iter()
        .find(|name| &*name.name == "DueLate")
        .unwrap()
        .declaration;
    let first_view = temporal_subject(&package, first).unwrap();
    let second_view = temporal_subject(&package, second).unwrap();
    assert_ne!(first, second);
    refuse_cross_binding_predecessor(
        &package,
        &first_view.occurrences()[0],
        &second_view.occurrences()[0],
    );

    let twins = super::temporal_package_route::twins();
    let read = twins.read(&twins.emission);
    let package = read.admitted();
    let bare = qsl_package::BarePackage::new(package);
    let engine = EvidencePackage::from_package(
        &bare,
        &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
    )
    .unwrap();
    let node = &engine.rows().names()[0].declaration;
    let view = temporal_subject(package, node).unwrap();
    assert_eq!(view.occurrences().len(), 2);
    assert_eq!(view.occurrences()[0].node, view.occurrences()[1].node);
    assert_ne!(view.occurrences()[0].ordinal, view.occurrences()[1].ordinal);
    refuse_cross_binding_predecessor(package, &view.occurrences()[0], &view.occurrences()[1]);
}

#[trace("TC-144", "FR-052-AC-15", "FR-052-AC-17")]
#[test]
fn native_report_two_build_bytes() {
    let first = std::cell::RefCell::new(Vec::new());
    with_inputs(
        true,
        false,
        |package, occurrence, evidence, request, observation, rows| {
            let subject = CheckedSubject::new(package, occurrence, evidence, request).unwrap();
            let report = evaluate_v2(
                &subject,
                observation,
                &mut super::temporal_package_route::unlimited(),
                &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
            )
            .unwrap();
            assert_eq!(report.truth(), Some(Truth::True));
            assert_eq!(
                report
                    .support()
                    .iter()
                    .map(|entry| &entry.row)
                    .collect::<Vec<_>>(),
                rows.iter().collect::<Vec<_>>()
            );
            assert_eq!(report.subject().package_identity, package.name());
            assert_eq!(report.subject().package_version, "quire.checked-package/v2");
            assert_eq!(report.subject().clause, occurrence.node);
            assert_eq!(report.subject().occurrence.role, occurrence.role);
            assert_eq!(report.subject().occurrence.ordinal, occurrence.ordinal);
            let source = package
                .source_map()
                .iter()
                .find(|entry| {
                    entry.node_id == occurrence.node
                        && entry.role == occurrence.role
                        && entry.ordinal == occurrence.ordinal
                })
                .unwrap();
            assert_eq!(&report.subject().source_map, source);
            assert_eq!(report.observation(), observation);
            let bytes = report.to_owner_bytes();
            assert_eq!(
                Report::from_owner_bytes(
                    &bytes,
                    &subject,
                    observation,
                    &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
                )
                .unwrap(),
                report
            );
            *first.borrow_mut() = bytes;
        },
    );
    // Reconstructing the inputs in this executable must preserve their bytes.
    with_inputs(
        true,
        false,
        |package, occurrence, evidence, request, observation, _| {
            let subject = CheckedSubject::new(package, occurrence, evidence, request).unwrap();
            let report = evaluate_v2(
                &subject,
                observation,
                &mut super::temporal_package_route::unlimited(),
                &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
            )
            .unwrap();
            assert_eq!(report.to_owner_bytes(), *first.borrow());
        },
    );
    if let Some(output) = std::env::var_os("QSL_OWNER_REPORT_OUTPUT") {
        std::fs::write(output, &*first.borrow()).expect("write the compiled fixture's owner bytes");
    }
}

#[trace("TC-144", "FR-052-AC-17")]
#[test]
#[ignore = "local two-build lane: run explicitly under the shared build lock"]
fn native_report_independent_build_bytes() {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"));
    let target = workspace.join("target");
    let holder = tempfile::tempdir().expect("owner report build outputs");
    let mut outputs = Vec::new();
    for metadata in ["qsl_owner_report_one", "qsl_owner_report_two"] {
        // Only this test executable receives the distinct metadata. Dependencies
        // stay cached in the ticket's one target directory.
        let build = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
            .current_dir(workspace)
            .env("CARGO_TARGET_DIR", &target)
            .args([
                "rustc",
                "--locked",
                "-p",
                "quire-spec-language",
                "--features",
                "checked-handoff",
                "--test",
                "it",
                "--message-format=json",
                "--",
                "-C",
                &format!("metadata={metadata}"),
            ])
            .output()
            .expect("compile the independent fixture executable");
        assert!(
            build.status.success(),
            "fixture compilation failed: {}\n{}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );
        let messages: Vec<serde_json::Value> = String::from_utf8(build.stdout)
            .expect("Cargo JSON is UTF-8")
            .lines()
            .map(|line| serde_json::from_str(line).expect("Cargo emits a JSON message"))
            .filter(|message: &serde_json::Value| {
                message["reason"] == "compiler-artifact"
                    && message["target"]["name"] == "it"
                    && message["target"]["src_path"]
                        == workspace.join("tests/it/main.rs").to_str().unwrap()
            })
            .collect();
        assert_eq!(
            messages.len(),
            1,
            "one root integration executable was built"
        );
        assert_eq!(messages[0]["fresh"], false, "each executable is recompiled");
        let executable = PathBuf::from(
            messages[0]["executable"]
                .as_str()
                .expect("Cargo names the compiled test executable"),
        );
        assert!(executable.starts_with(&target));
        let output = holder.path().join(metadata);
        let run = Command::new(&executable)
            .args([
                "native_owner_report::native_report_two_build_bytes",
                "--exact",
            ])
            .env("QSL_OWNER_REPORT_OUTPUT", &output)
            .output()
            .expect("run the independently compiled fixture");
        assert!(
            run.status.success(),
            "compiled fixture failed: {}\n{}",
            String::from_utf8_lossy(&run.stdout),
            String::from_utf8_lossy(&run.stderr)
        );
        let bytes = std::fs::read(output).expect("the exact fixture wrote owner bytes");
        assert!(!bytes.is_empty());
        println!(
            "{metadata}: recompiled fixture produced {} owner bytes",
            bytes.len()
        );
        outputs.push(bytes);
    }
    assert_eq!(
        outputs[0], outputs[1],
        "independent builds preserve owner bytes"
    );
}

#[trace("TC-144", "FR-052-AC-12")]
#[test]
fn reader_refuses_a_result_identity_of_another_clause_or_occurrence() {
    with_subject(|subject, observation, _| {
        let report = evaluate_v2(
            subject,
            observation,
            &mut super::temporal_package_route::unlimited(),
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .unwrap();
        let bytes = report.to_owner_bytes();
        let identity = fields(&bytes, 0)[9];
        // Identity members are tag, eight-byte length, payload; the clause is
        // first and the occurrence is last.
        let mut clause = bytes.clone();
        clause[identity.1 + 9] ^= 1;
        assert_eq!(
            Report::from_owner_bytes(
                &clause,
                subject,
                observation,
                &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
            ),
            Err(ReportRefusal::UnknownTag)
        );
        let mut occurrence = bytes.clone();
        occurrence[identity.2 - 1] ^= 1;
        assert_eq!(
            Report::from_owner_bytes(
                &occurrence,
                subject,
                observation,
                &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
            ),
            Err(ReportRefusal::UnknownTag)
        );
    });
}

#[trace("TC-144", "FR-052-AC-9", "FR-052-AC-15")]
#[test]
fn subject_and_evaluation_refuse_foreign_occurrence_revision_and_anchor() {
    with_inputs(
        true,
        false,
        |package, occurrence, evidence, request, observation, _| {
            let twins = super::temporal_package_route::twins();
            let read = twins.read(&twins.emission);
            let bare = qsl_package::BarePackage::new(read.admitted());
            let engine = EvidencePackage::from_package(
                &bare,
                &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
            )
            .unwrap();
            let view =
                temporal_subject(read.admitted(), &engine.rows().names()[0].declaration).unwrap();
            let foreign_clause = &view.occurrences()[0];
            assert!(temporal_subject(package, &foreign_clause.node).is_err());
            assert!(matches!(
                CheckedSubject::new(package, foreign_clause, evidence, request),
                Err(ReportRefusal::BindingMismatch(BindingDimension::Clause))
            ));
            let mut foreign = occurrence.clone();
            foreign.ordinal += 1000;
            assert!(matches!(
                CheckedSubject::new(package, &foreign, evidence, request),
                Err(ReportRefusal::BindingMismatch(BindingDimension::Occurrence))
            ));
            let mut other = request.clone();
            other.occurrence = Some(qsl_package::CheckedOccurrence {
                role: occurrence.role.clone(),
                ordinal: occurrence.ordinal + 1000,
            });
            assert!(matches!(
                CheckedSubject::new(package, occurrence, evidence, &other),
                Err(ReportRefusal::BindingMismatch(BindingDimension::Occurrence))
            ));
            let mut shorter = request.clone();
            shorter.positions.truncate(1);
            assert!(evidence
                .rows()
                .iter()
                .any(|row| row.position >= shorter.positions.len()));
            assert!(matches!(
                CheckedSubject::new(package, occurrence, evidence, &shorter),
                Err(ReportRefusal::SupportNotInTrace)
            ));
            let subject = CheckedSubject::new(package, occurrence, evidence, request).unwrap();
            let stale = ObservationRef {
                reference: observation.reference.clone(),
                revision: "revision:other".into(),
            };
            assert!(matches!(
                evaluate_v2(
                    &subject,
                    &stale,
                    &mut super::temporal_package_route::unlimited(),
                    &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
                ),
                Err(NativeReportError::Report(ReportRefusal::BindingMismatch(
                    BindingDimension::Revision
                )))
            ));
            let mut moved = request.clone();
            moved.anchor = Some("obs-other".into());
            let subject = CheckedSubject::new(package, occurrence, evidence, &moved).unwrap();
            assert!(matches!(
                evaluate_v2(
                    &subject,
                    observation,
                    &mut super::temporal_package_route::unlimited(),
                    &mut qsl_package::V2ReadLimits::fixture().native_join_budget()
                ),
                Err(NativeReportError::Report(ReportRefusal::SupportNotInTrace))
            ));
        },
    );
}
