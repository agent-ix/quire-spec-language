// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native owner reports over the checked-package temporal engine.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_eval::temporal::evidence::{EvidencePackage, Truth};
use qsl_package::checked_views::temporal_subject;
use quire_spec_language::temporal::{
    evaluate_v2, BindingDimension, CheckedSubject, ClockBinding, Closure, Completeness, Execution,
    ObservationRef, Position, Report, ReportRefusal, Trace, ValuationEvidence, ValuationInstance,
    ValuationRow, EVENT_POSITION,
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
    let engine = EvidencePackage::from_package(&bare).expect("checked engine package");
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

#[trace("FR-052-AC-15", "FR-052-AC-16", "FR-052-AC-17")]
#[test]
fn native_report_owns_truth_and_preserves_duplicate_support_and_roundtrip() {
    with_subject(|subject, observation, rows| {
        let report = evaluate_v2(
            subject,
            observation,
            &mut super::temporal_package_route::unlimited(),
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
        let decoded = Report::from_owner_bytes(&bytes, subject, observation)
            .expect("strict reader admits owner output");
        assert_eq!(decoded, report);
        assert_eq!(decoded.to_owner_bytes(), bytes);
        let repeated = evaluate_v2(
            subject,
            observation,
            &mut super::temporal_package_route::unlimited(),
        )
        .expect("same input reevaluates");
        assert_eq!(repeated.to_owner_bytes(), bytes);
    });
}

#[trace("FR-052-AC-18")]
#[test]
fn every_truncated_prefix_and_extra_byte_is_refused() {
    with_subject(|subject, observation, _| {
        let report = evaluate_v2(
            subject,
            observation,
            &mut super::temporal_package_route::unlimited(),
        )
        .expect("evaluated");
        let bytes = report.to_owner_bytes();
        for end in 0..bytes.len() {
            assert_eq!(
                Report::from_owner_bytes(&bytes[..end], subject, observation),
                Err(ReportRefusal::Truncated),
                "prefix {end}"
            );
        }
        let mut appended = bytes.clone();
        appended.push(0);
        assert_eq!(
            Report::from_owner_bytes(&appended, subject, observation),
            Err(ReportRefusal::TrailingBytes)
        );
        for index in [0, 4] {
            let mut mutated = bytes.clone();
            mutated[index] = 255;
            assert_eq!(
                Report::from_owner_bytes(&mutated, subject, observation),
                Err(ReportRefusal::UnsupportedVersion)
            );
        }
        let mut reordered = bytes.clone();
        reordered[5] = 255;
        assert_eq!(
            Report::from_owner_bytes(&reordered, subject, observation),
            Err(ReportRefusal::OutOfOrder)
        );
    });
}

#[trace("FR-052-AC-18")]
#[test]
fn opaque_observation_and_revision_binding_are_independent() {
    with_subject(|subject, observation, _| {
        let report = evaluate_v2(
            subject,
            observation,
            &mut super::temporal_package_route::unlimited(),
        )
        .expect("evaluated");
        let bytes = report.to_owner_bytes();
        let foreign_observation = ObservationRef {
            reference: "different".into(),
            ..observation.clone()
        };
        assert_eq!(
            Report::from_owner_bytes(&bytes, subject, &foreign_observation),
            Err(ReportRefusal::BindingMismatch(
                BindingDimension::Observation
            ))
        );
        let foreign_revision = ObservationRef {
            revision: "different".into(),
            ..observation.clone()
        };
        assert_eq!(
            Report::from_owner_bytes(&bytes, subject, &foreign_revision),
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

#[trace("FR-052-AC-18")]
#[test]
fn binding_outcome_member_order_and_support_mutations_are_typed_refusals() {
    with_subject(|subject, observation, _| {
        let report = evaluate_v2(
            subject,
            observation,
            &mut super::temporal_package_route::unlimited(),
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
                Report::from_owner_bytes(&changed, subject, observation),
                Err(ReportRefusal::BindingMismatch(dimension))
            );
        }
        let mut outcome = bytes.clone();
        outcome[spans[7].1] = 255;
        assert_eq!(
            Report::from_owner_bytes(&outcome, subject, observation),
            Err(ReportRefusal::UnknownTag)
        );
        let mut swapped = bytes[..5].to_vec();
        swapped.extend_from_slice(&bytes[spans[1].0..spans[1].2]);
        swapped.extend_from_slice(&bytes[spans[0].0..spans[0].2]);
        swapped.extend_from_slice(&bytes[spans[2].0..]);
        assert_eq!(
            Report::from_owner_bytes(&swapped, subject, observation),
            Err(ReportRefusal::OutOfOrder)
        );
        let mut duplicate = bytes[..spans[1].0].to_vec();
        duplicate.extend_from_slice(&bytes[spans[0].0..spans[0].2]);
        duplicate.extend_from_slice(&bytes[spans[1].0..]);
        assert_eq!(
            Report::from_owner_bytes(&duplicate, subject, observation),
            Err(ReportRefusal::OutOfOrder)
        );
        let support_start = spans[8].1;
        let support = fields(&bytes[support_start..spans[8].2], 0);
        let first_row_start = support_start + support[1].1;
        let row = fields(&bytes[first_row_start..support_start + support[1].2], 0);
        let mut position = bytes.clone();
        position[first_row_start + row[0].1] = 200;
        assert_eq!(
            Report::from_owner_bytes(&position, subject, observation),
            Err(ReportRefusal::SupportNotInTrace)
        );
        let key_start = first_row_start + row[1].1;
        let key = fields(&bytes[key_start..first_row_start + row[1].2], 0);
        let mut foreign_key = bytes.clone();
        foreign_key[key_start + key[4].1] ^= 1;
        assert_eq!(
            Report::from_owner_bytes(&foreign_key, subject, observation),
            Err(ReportRefusal::SupportNotInTrace)
        );
    });
}

#[trace("FR-052-AC-15", "FR-052-AC-16")]
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

#[trace("FR-052-AC-12", "FR-052-AC-17", "FR-052-AC-18")]
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
                    &mut super::temporal_package_route::unlimited()
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
            )
            .unwrap();
            assert_eq!(revised.predecessor(), Some(original.result_identity()));
            assert_ne!(revised.result_identity(), original.result_identity());
            assert_eq!(original.to_owner_bytes(), original_bytes);
            assert_eq!(
                Report::from_owner_bytes(
                    &revised.to_owner_bytes(),
                    &revised_subject,
                    &revised_observation
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
                    &mut super::temporal_package_route::unlimited()
                ),
                Err(quire_spec_language::temporal::NativeReportError::Report(
                    ReportRefusal::InvalidPredecessor
                ))
            ));
            assert_eq!(
                Report::from_owner_bytes(&revised.to_owner_bytes(), &stale, &revised_observation),
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
                    &mut super::temporal_package_route::unlimited()
                ),
                Err(quire_spec_language::temporal::NativeReportError::Report(
                    ReportRefusal::InvalidPredecessor
                ))
            ));
            assert_eq!(original.to_owner_bytes(), original_bytes);
        },
    );
}

#[trace("FR-052-AC-12", "FR-052-AC-18")]
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
                Report::from_owner_bytes(&self_edge, &revised_subject, &revised_observation),
                Err(ReportRefusal::InvalidPredecessor)
            );
            assert_eq!(
                Report::from_owner_bytes(&bytes, &subject, &revised_observation),
                Err(ReportRefusal::InvalidPredecessor)
            );
        },
    );
}

fn refuse_cross_binding_predecessor(
    package: &quire_contract_model::CheckedPackageV2,
    first: &qsl_package::checked_views::ClauseOccurrence,
    second: &qsl_package::checked_views::ClauseOccurrence,
) {
    use qsl_eval::temporal::evidence::{ObligationKey, Relation};
    let bare = qsl_package::BarePackage::new(package);
    let engine = EvidencePackage::from_package(&bare).unwrap();
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
        Report::from_owner_bytes(&original.to_owner_bytes(), &subject, &observation).unwrap(),
        original
    );
    assert_eq!(
        Report::from_owner_bytes(
            &second_report.to_owner_bytes(),
            &second_subject,
            &observation
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
            &mut super::temporal_package_route::unlimited()
        ),
        Err(quire_spec_language::temporal::NativeReportError::Report(
            ReportRefusal::InvalidPredecessor
        ))
    ));
    assert_eq!(original.to_owner_bytes(), unchanged);
}

#[trace("FR-052-AC-12", "FR-052-AC-15")]
#[test]
fn another_clause_and_another_occurrence_cannot_borrow_predecessor_authority() {
    let (package, _) = super::checked_views::recipe_package("event_position");
    let bare = qsl_package::BarePackage::new(&package);
    let engine = EvidencePackage::from_package(&bare).unwrap();
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
    let engine = EvidencePackage::from_package(&bare).unwrap();
    let node = &engine.rows().names()[0].declaration;
    let view = temporal_subject(package, node).unwrap();
    assert_eq!(view.occurrences().len(), 2);
    assert_eq!(view.occurrences()[0].node, view.occurrences()[1].node);
    assert_ne!(view.occurrences()[0].ordinal, view.occurrences()[1].ordinal);
    refuse_cross_binding_predecessor(package, &view.occurrences()[0], &view.occurrences()[1]);
}

#[trace("FR-052-AC-15", "FR-052-AC-17")]
#[test]
fn native_report_two_build_bytes() {
    with_inputs(
        true,
        false,
        |package, occurrence, evidence, request, observation, rows| {
            let subject = CheckedSubject::new(package, occurrence, evidence, request).unwrap();
            let report = evaluate_v2(
                &subject,
                observation,
                &mut super::temporal_package_route::unlimited(),
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
                Report::from_owner_bytes(&bytes, &subject, observation).unwrap(),
                report
            );
            if let Some(previous) = std::env::var_os("QSL_OWNER_REPORT_PREVIOUS") {
                assert_eq!(
                    bytes,
                    std::fs::read(previous).expect("first build's owner bytes"),
                    "independently built evaluators must emit identical bytes"
                );
            }
            if let Some(output) = std::env::var_os("QSL_OWNER_REPORT_OUTPUT") {
                std::fs::write(output, &bytes).expect("write this build's owner bytes");
            }
        },
    );
}
