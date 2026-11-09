// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-281-AC-1 and AC-6 (TC-762, TC-763) for EN-5: an unbounded reachability
//! claim compiled from source runs through `analyze_items` on the EN-5
//! adapter, which builds the product, solves it exactly, has the core's
//! checker accept the certificate and only then settles `proved`.
//!
use ix_trace_rs::trace;
use qsl_analyze::analyze::{
    analyze_items, exact_probabilistic_item, exact_probabilistic_item_tampered, Engines,
};
use qsl_eval::simulation::model::ModelSystem;
use qsl_eval::simulation::Limits;
use qsl_foundation::RequestIndex;
use qsl_replay::InconclusiveCause;
use qsl_replay::{
    claim_binding, Certification, ExactOrBounds, ExactProbabilityCertificateCheck,
    ExactProbabilityGraphError, Extremum, IncompleteCause, ModelCheckLimit, ModelCheckLimits,
    ProbabilityBoundSide, ProbabilityCertificateLocus, ProbabilityCertificateRejection,
    ProbabilityCertificateRule, ProbabilityProductKey, ProofBasis, TerminalRecord, TerminalValue,
};
use qsl_replay::{CertificateLocus, CertificateRule};
use quire_exact::{Cancel, CancelCause, Integer, Rational};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};

use crate::prob_world::{self, World};

/// `Link` delivers within two attempts: the least delivery probability over
/// every scheduler is 24/25 (send_b twice), the greatest 99/100.
const CLAIMS: &str =
    "probabilistic Likely using inf on M over (m: M::Msg) under every scheduler on origin \
     { probability >= 0.95 [ eventually holds(m.delivered) ] }\n\
     probabilistic Unlikely using inf on M over (m: M::Msg) under every scheduler on origin \
     { probability >= 0.97 [ eventually holds(m.delivered) ] }\n";

fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).unwrap()
}

fn settle(
    world: &World,
    claim: &str,
    limits: ModelCheckLimits,
) -> qsl_analyze::analyze::AnalyzeRecord {
    let engines = Engines::qsl().expect("the linked engines are admitted");
    let item = exact_probabilistic_item(RequestIndex::new(0), world.request(claim, limits));
    let mut records = analyze_items(&engines, vec![item], &Cancel::new());
    assert_eq!(records.len(), 1);
    records.remove(0)
}

#[trace("TC-762", "FR-281-AC-1")]
#[test]
fn a_reachability_bound_that_holds_settles_proved_with_the_checked_certificate() {
    let world = World::new(&prob_world::link(CLAIMS));
    let record = settle(&world, "Likely", ModelCheckLimits::default());
    let terminal = record.terminal().expect("a terminal record");
    assert_eq!(terminal.backend().map(|b| b.as_str()), Some("en-5"));
    let TerminalValue::Proved {
        basis: ProofBasis::ExactValue { value, entries, .. },
        certification,
    } = terminal.value()
    else {
        panic!("expected proved, got {:?}", terminal.value());
    };
    assert_eq!(*certification, Certification::Certified);
    assert_eq!(value.value(), &ratio(24, 25));
    let [entry] = entries.as_slice() else {
        panic!("one entry");
    };
    assert_eq!(entry.extremum, Some(Extremum::Min));
    let binding = entry.binding.as_ref().expect("the claim's one instance");
    assert_eq!(binding.key, "m");
    assert!(binding.population.ends_with("/msgs"), "{binding:?}");
    assert!(
        matches!(&entry.value, ExactOrBounds::Exact { value } if value.value() == &ratio(24, 25))
    );

    let evidence = record
        .exact_probabilistic
        .as_ref()
        .expect("the record carries the accepted certificate");
    assert_eq!(evidence.value, ratio(24, 25));
    assert_eq!(evidence.side, ProbabilityBoundSide::AtLeast);
    // Link has five reachable states: the start, delivered by either send,
    // one loss, two losses.
    assert!(evidence.certificate.values.len() >= 4);
    assert!(evidence
        .certificate
        .values
        .iter()
        .any(|entry| entry.value == ratio(24, 25)));
}

#[trace("TC-763", "FR-281-AC-6")]
#[test]
fn a_state_budget_of_one_settles_incomplete() {
    let world = World::new(&prob_world::link(CLAIMS));
    let limits = ModelCheckLimits {
        limits: Limits::default().with_max_states(1),
        ..ModelCheckLimits::default()
    };
    let record = settle(&world, "Likely", limits);
    let terminal = record.terminal().expect("a terminal record");
    assert_eq!(terminal.backend().map(|b| b.as_str()), Some("en-5"));
    assert_eq!(
        terminal.value(),
        &TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::ExactProbMaxStates,
            value: 1,
        })
    );
    assert!(record.exact_probabilistic.is_none());
}

#[trace("TC-763", "FR-201", "FR-203", "FR-282")]
#[test]
fn cancellation_at_the_checker_acceptance_boundary_settles_incomplete_without_evidence() {
    let world = World::new(&prob_world::link(CLAIMS));
    let engines = Engines::qsl().expect("the linked engines are admitted");
    let run = |cancel: &Cancel| {
        let item = exact_probabilistic_item(
            RequestIndex::new(0),
            world.request("Likely", ModelCheckLimits::default()),
        );
        analyze_items(&engines, vec![item], cancel).remove(0)
    };
    let live = Cancel::new();
    let _counting = live.count_charges();
    let completed = run(&live);
    assert!(
        completed.exact_probabilistic.is_some(),
        "the positive control must prove the claim"
    );
    let final_poll = live.charges();
    assert!(final_poll > 0);
    for cause in [CancelCause::Requested, CancelCause::Deadline] {
        let slot = Arc::new(OnceLock::<Cancel>::new());
        let weak = Arc::downgrade(&slot);
        let polls = Arc::new(AtomicU32::new(0));
        let seen = Arc::clone(&polls);
        let cancel = Cancel::observing(move || {
            if seen.fetch_add(1, Ordering::Relaxed) + 1 == final_poll {
                weak.upgrade()
                    .expect("the run owns the slot")
                    .get()
                    .expect("the slot is initialized")
                    .cancel(cause);
            }
        });
        slot.set(cancel.clone())
            .expect("the slot is initialized once");
        let record = run(&cancel);
        assert_eq!(polls.load(Ordering::Relaxed), final_poll);
        assert_eq!(
            record.terminal().expect("a terminal record").value(),
            &TerminalValue::Incomplete(IncompleteCause::Cancelled { source: cause })
        );
        assert!(
            record.exact_probabilistic.is_none(),
            "a cancelled checker cannot provide proof evidence"
        );
    }
}

#[trace("TC-763", "FR-201", "FR-203", "FR-282")]
#[test]
fn cancelling_after_the_solver_finishes_is_observed_by_the_independent_checker() {
    use std::cell::Cell;

    let world = World::new(&prob_world::link(CLAIMS));
    let engines = Engines::qsl().expect("the linked engines are admitted");
    for source in [CancelCause::Requested, CancelCause::Deadline] {
        let cancel = Cancel::new();
        let reached_checker = Cell::new(false);
        let item = exact_probabilistic_item_tampered(
            RequestIndex::new(0),
            world.request("Likely", ModelCheckLimits::default()),
            |_| {
                reached_checker.set(true);
                cancel.cancel(source);
            },
        );
        let record = analyze_items(&engines, vec![item], &cancel).remove(0);
        assert!(
            reached_checker.get(),
            "the solver must finish before cancellation"
        );
        assert_eq!(
            record.terminal().expect("a terminal record").value(),
            &TerminalValue::Incomplete(IncompleteCause::Cancelled { source })
        );
        assert!(
            record.exact_probabilistic.is_none(),
            "checker cancellation cannot certify the solver output"
        );
    }
}

#[trace("TC-762", "FR-281-AC-1")]
#[test]
fn a_missed_bound_is_refuted_only_with_replayed_nonreach_evidence() {
    let world = World::new(&prob_world::link(CLAIMS));
    let record = settle(&world, "Unlikely", ModelCheckLimits::default());
    assert!(record.exact_probabilistic.is_none());
    assert_eq!(record.terminal().unwrap().value(), &TerminalValue::Refuted);
    assert!(record.probabilistic_counterexample.is_some());
}

/// Altering one terminal value retains the checker's rejection and full locus.
#[trace("TC-763", "FR-281-AC-4", "FR-203-AC-3", "FR-201-AC-6")]
#[test]
fn a_rejected_certificate_never_settles_proved() {
    let world = World::new(&prob_world::link(CLAIMS));
    let engines = Engines::qsl().expect("the linked engines are admitted");
    let item = exact_probabilistic_item_tampered(
        RequestIndex::new(0),
        world.request("Likely", ModelCheckLimits::default()),
        |certificate| {
            certificate
                .values
                .iter_mut()
                .find(|entry| entry.value == ratio(0, 1))
                .expect("the two-loss state")
                .value = ratio(1, 1);
        },
    );
    let record = analyze_items(&engines, vec![item], &Cancel::new()).remove(0);
    assert!(record.exact_probabilistic.is_none());
    let terminal = record.terminal().expect("a terminal record");
    let system = ModelSystem::new(world.subject().simulation).expect("the model admits");
    let binding = claim_binding(&system, world.claim("Likely"))
        .unwrap()
        .unwrap();
    let accepted = settle(&world, "Likely", ModelCheckLimits::default());
    let certificate = &accepted.exact_probabilistic.as_ref().unwrap().certificate;
    // The altered terminal row and its one-attempt predecessor both fail
    // their one-step equations; the contract reports the first canonical key.
    let expected = certificate
        .values
        .iter()
        .filter(|entry| entry.value == ratio(0, 1) || entry.value == ratio(4, 5))
        .min_by(|left, right| left.state.cmp(&right.state))
        .expect("the first failing one-step equation")
        .state
        .clone();
    assert_eq!(
        terminal.value(),
        &TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
            rule: CertificateRule::InequalityFails,
            at: CertificateLocus::ProbabilityProductState(expected.clone()),
        })
    );
    let locus = CertificateLocus::ProbabilityProductState(expected.clone());
    let wire = serde_json::to_value(&locus).unwrap();
    // Encode the complete canonical octets independently of the locus serializer.
    let expected_wire = serde_json::to_value(qsl_replay::CertificateLocus::ModelKey(
        qsl_eval::simulation::StateKey::from_canonical_bytes(expected.as_bytes().to_vec()),
    ))
    .unwrap();
    assert_eq!(wire, serde_json::json!({"product_key": expected_wire}));
    let outcome = serde_json::to_value(qsl_replay::OutcomeItem::from_terminal(terminal)).unwrap();
    assert_eq!(outcome["result"], "inconclusive");
    assert_eq!(
        outcome["cause"],
        serde_json::json!({
            "kind": "certificate-rejected", "rule": "inequality-fails", "at": wire
        })
    );
    assert!(String::from_utf8(expected.as_bytes().to_vec())
        .unwrap()
        .contains(&binding.instance.key));
}

#[trace("TC-763", "FR-281-AC-4")]
#[test]
fn a_malformed_probability_certificate_settles_a_typed_rejection() {
    let world = World::new(&prob_world::link(CLAIMS));
    let engines = Engines::qsl().expect("the linked engines are admitted");
    let item = exact_probabilistic_item_tampered(
        RequestIndex::new(0),
        world.request("Likely", ModelCheckLimits::default()),
        |certificate| certificate.values.clear(),
    );
    let record = analyze_items(&engines, vec![item], &Cancel::new()).remove(0);
    assert!(matches!(
        record.terminal().expect("a terminal record").value(),
        TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
            rule: CertificateRule::Malformed,
            ..
        })
    ));
    assert!(record.exact_probabilistic.is_none());
}

#[trace("TC-764", "FR-201-AC-11", "FR-282-AC-12")]
#[test]
fn analyzer_document_budgets_stop_after_solving_without_certification() {
    use std::sync::atomic::AtomicBool;
    let world = World::new(&prob_world::link(CLAIMS));
    let engines = Engines::qsl().unwrap();
    let baseline = settle(&world, "Likely", ModelCheckLimits::default());
    assert!(!baseline
        .exact_probabilistic
        .as_ref()
        .unwrap()
        .document
        .is_empty());
    for storage in [false, true] {
        let mut request = world.request("Likely", ModelCheckLimits::default());
        if storage {
            request.limits.document.storage_bytes = 0;
        } else {
            request.limits.document.bytes = 0;
        }
        let reached_emission = Arc::new(AtomicBool::new(false));
        let observed = Arc::clone(&reached_emission);
        let item = exact_probabilistic_item_tampered(RequestIndex::new(0), request, move |_| {
            observed.store(true, Ordering::Relaxed);
        });
        let record = analyze_items(&engines, vec![item], &Cancel::new()).remove(0);
        assert!(
            reached_emission.load(Ordering::Relaxed),
            "graph enumeration and solve must have succeeded"
        );
        let expected = if storage {
            ModelCheckLimit::CertificateMaxWorkingBytes
        } else {
            ModelCheckLimit::CertificateMaxBytes
        };
        assert_eq!(
            record.terminal().unwrap().value(),
            &TerminalValue::Incomplete(IncompleteCause::LimitReached {
                limit: expected,
                value: 0
            })
        );
        assert_eq!(
            expected.setting(),
            if storage {
                "certificate_check.max_working_bytes"
            } else {
                "certificate_check.max_certificate_bytes"
            }
        );
        assert!(record.exact_probabilistic.is_none());
    }
}

#[test]
fn the_bound_reference_is_part_of_the_product_key() {
    let world = World::new(&prob_world::link(CLAIMS));
    let system = ModelSystem::new(world.subject().simulation).expect("the model admits");
    let binding = claim_binding(&system, world.claim("Likely"))
        .expect("one instance")
        .expect("the claim has an over");
    let bound = ProbabilityProductKey::for_model_state(Some(&binding.reference), b"state")
        .expect("encodes");
    let unbound = ProbabilityProductKey::for_model_state(None, b"state").expect("encodes");
    assert_ne!(bound, unbound);
    let text = String::from_utf8(bound.as_bytes().to_vec()).expect("JSON text");
    assert!(
        text.contains("object_type") && text.contains("universe"),
        "{text}"
    );
}

#[test]
fn several_instances_are_an_unsupported_binding() {
    let world = World::with_objects(&prob_world::link(CLAIMS), &["m", "n"]);
    let system = ModelSystem::new(world.subject().simulation).expect("the model admits");
    assert!(matches!(
        claim_binding(&system, world.claim("Likely")),
        Err(ExactProbabilityGraphError::UnsupportedBinding)
    ));
}

#[test]
fn a_rejected_check_builds_no_proved_record() {
    let rejected = ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
        rule: ProbabilityCertificateRule::InequalityFails,
        at: ProbabilityCertificateLocus::Member("values"),
    });
    assert!(TerminalRecord::proved_by_exact_certificate(
        RequestIndex::new(0),
        qsl_foundation::BackendId::new("en-5"),
        &rejected,
        None,
        None,
    )
    .is_none());
}
