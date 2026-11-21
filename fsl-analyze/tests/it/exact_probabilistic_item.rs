// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-281-AC-1 and AC-6 (TC-762, TC-763) for EN-5: an unbounded reachability
//! claim compiled from source runs through `analyze_items` on the EN-5
//! adapter, which builds the product, solves it exactly, has the core's
//! checker accept the certificate and only then settles `proved`.
//!
use filament_exact::{Cancel, CancelCause, Integer, Rational};
use fsl_analyze::analyze::{
    analyze_items, exact_probabilistic_item, exact_probabilistic_item_tampered, Engines,
};
use fsl_eval::simulation::model::ModelSystem;
use fsl_eval::simulation::{Limits, TransitionSystem};
use fsl_foundation::RequestIndex;
use fsl_replay::InconclusiveCause;
use fsl_replay::ProbabilityCertificateDocument;
use fsl_replay::{
    claim_binding, Certification, ExactOrBounds, ExactProbabilityCertificateCheck,
    ExactProbabilityGraphError, Extremum, IncompleteCause, ModelCheckLimit, ModelCheckLimits,
    ProbabilityBoundSide, ProbabilityCertificateLocus, ProbabilityCertificateRejection,
    ProbabilityCertificateRule, ProbabilityProductKey, ProofBasis, TerminalRecord, TerminalValue,
};
use fsl_replay::{CertificateLocus, CertificateRule};
use ix_trace_rs::trace;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};

use crate::prob_world::{self, World};

mod certificate_boundary_cases;
mod measure_profile;

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
) -> fsl_analyze::analyze::AnalyzeRecord {
    let engines = Engines::fsl().expect("the linked engines are admitted");
    let item = exact_probabilistic_item(RequestIndex::new(0), world.request(claim, limits));
    let mut records = analyze_items(&engines, vec![item], &Cancel::new());
    assert_eq!(records.len(), 1);
    records.remove(0)
}

/// Equality at the exact reachability threshold proves with the checked
/// EN-5 basis; increasing only that threshold refutes instead of certifying.
#[trace("FR-290-AC-1")]
#[test]
fn exact_threshold_equality_is_certified_but_a_stricter_threshold_is_refuted() {
    let claims =
        "probabilistic AtBound using inf on M over (m: M::Msg) under every scheduler on origin \
        { probability >= 0.96 [ eventually holds(m.delivered) ] }\n\
        probabilistic AboveBound using inf on M over (m: M::Msg) under every scheduler on origin \
        { probability >= 0.9601 [ eventually holds(m.delivered) ] }\n";
    let world = World::new(&prob_world::link(claims));
    let at = settle(&world, "AtBound", ModelCheckLimits::default());
    let terminal = at.terminal().expect("the admitted claim settles");
    assert_eq!(terminal.backend().map(|id| id.as_str()), Some("en-5"));
    let TerminalValue::Proved {
        basis: ProofBasis::ExactValue { value, .. },
        certification: Certification::Certified,
    } = terminal.value()
    else {
        panic!(
            "the exact inclusive bound must prove: {:?}",
            terminal.value()
        );
    };
    assert_eq!(value.value(), &ratio(24, 25));
    assert_eq!(
        at.exact_probabilistic
            .as_ref()
            .expect("checked evidence")
            .value,
        ratio(24, 25)
    );

    let above = settle(&world, "AboveBound", ModelCheckLimits::default());
    assert_eq!(
        above.terminal().expect("a terminal record").value(),
        &TerminalValue::Refuted
    );
    assert!(
        above.exact_probabilistic.is_none(),
        "a refutation is not a certified proof"
    );
    assert!(
        above.probabilistic_counterexample.is_some(),
        "the strict bound needs replayed evidence"
    );
}

#[trace("TC-762", "FR-281-AC-1")]
#[trace("TC-636", "FR-201-AC-1")]
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
    let engines = Engines::fsl().expect("the linked engines are admitted");
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
    let engines = Engines::fsl().expect("the linked engines are admitted");
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
#[trace("TC-636", "FR-201-AC-1")]
#[test]
fn a_rejected_certificate_never_settles_proved() {
    let world = World::new(&prob_world::link(CLAIMS));
    let engines = Engines::fsl().expect("the linked engines are admitted");
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
    let expected_wire = serde_json::to_value(fsl_replay::CertificateLocus::ModelKey(
        fsl_eval::simulation::StateKey::from_canonical_bytes(expected.as_bytes().to_vec()),
    ))
    .unwrap();
    assert_eq!(wire, serde_json::json!({"product_key": expected_wire}));
    let outcome = serde_json::to_value(fsl_replay::OutcomeItem::from_terminal(terminal)).unwrap();
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

#[trace("TC-763", "FR-281-AC-4", "FR-203-AC-3", "FSpec-FR-331-AC-25")]
#[test]
fn a_malformed_probability_certificate_settles_a_typed_rejection() {
    let world = World::new(&prob_world::link(CLAIMS));
    let engines = Engines::fsl().expect("the linked engines are admitted");
    let item = exact_probabilistic_item_tampered(
        RequestIndex::new(0),
        world.request("Likely", ModelCheckLimits::default()),
        |certificate| {
            let omitted = certificate
                .values
                .iter()
                .position(|row| row.value == ratio(4, 5))
                .expect("a noninitial reachable state");
            certificate.values.remove(omitted);
        },
    );
    let record = analyze_items(&engines, vec![item], &Cancel::new()).remove(0);
    assert_eq!(
        record.terminal().expect("a terminal record").value(),
        &TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
            rule: CertificateRule::Malformed,
            at: CertificateLocus::CertificateMember(fsl_replay::CertificateMember::Values),
        })
    );
    assert!(record.exact_probabilistic.is_none());
    assert!(record.probabilistic_counterexample.is_none());
    let terminal = record.terminal().unwrap();
    assert!(terminal.exact_certificate_document().is_none());
    let outcome = serde_json::to_value(fsl_replay::OutcomeItem::from_terminal(terminal)).unwrap();
    assert_eq!(outcome["result"], "inconclusive");
    assert_eq!(
        outcome["cause"],
        serde_json::json!({
            "kind": "certificate-rejected", "rule": "malformed", "at": {"member": "values"}
        })
    );
}

#[trace("TC-636", "FSpec-FR-331-AC-25")]
#[test]
fn unsorted_reachable_typed_values_reject_at_the_values_member() {
    let world = World::new(&prob_world::link(CLAIMS));
    let baseline = settle(&world, "Likely", ModelCheckLimits::default());
    let mut certificate = baseline
        .exact_probabilistic
        .as_ref()
        .unwrap()
        .certificate
        .clone();
    assert!(certificate.values.len() > 1);
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = fsl_replay::probabilistic_claim_identity(world.claim("Likely")).unwrap();
    let check = |certificate: &fsl_replay::ExactProbabilityCertificate| {
        fsl_replay::check_claim_exact_reachability(
            &system,
            world.package.graph().probabilistic_model().claims(),
            &item,
            certificate,
            128,
            1024,
            &Cancel::new(),
        )
        .unwrap()
    };
    assert!(matches!(
        check(&certificate),
        ExactProbabilityCertificateCheck::Accepted(_)
    ));
    certificate.values.reverse();
    assert_eq!(
        check(&certificate),
        ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
            rule: ProbabilityCertificateRule::Malformed,
            at: ProbabilityCertificateLocus::Member("values"),
        })
    );
}

#[trace("TC-764", "FR-201-AC-9", "FR-201-AC-10", "FSpec-FR-331-AC-25")]
#[test]
fn missing_document_values_reject_but_identity_and_objective_mismatches_refuse() {
    let world = World::new(&prob_world::link(CLAIMS));
    let baseline = settle(&world, "Likely", ModelCheckLimits::default());
    let bytes = baseline
        .terminal()
        .unwrap()
        .exact_certificate_document()
        .unwrap();
    let original: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = fsl_replay::probabilistic_claim_identity(world.claim("Likely")).unwrap();
    let check = |bytes: &[u8]| {
        fsl_replay::check_probability_certificate_document(
            &system,
            world.package.graph().probabilistic_model().claims(),
            &item,
            bytes,
            fsl_replay::ProbabilityDocumentLimits {
                bytes: 1_000_000,
                storage_bytes: 8_000_000,
                states: 128,
                edges: 1024,
                scalar_bits: 128,
                accounting: fsl_replay::AccountingLimits::default().0,
            },
            &Cancel::new(),
        )
    };
    assert!(matches!(
        check(bytes),
        Ok(ExactProbabilityCertificateCheck::Accepted(_))
    ));
    for duplicate in [false, true] {
        let mut malformed = original.clone();
        let rows = malformed["values"].as_array_mut().unwrap();
        if duplicate {
            rows.insert(1, rows[0].clone());
        } else {
            rows.reverse();
        }
        assert_eq!(
            check(&serde_json::to_vec(&malformed).unwrap()),
            Err(fsl_replay::ProbabilityDocumentError::Malformed),
            "duplicate/out-of-order byte rows remain document admission errors"
        );
    }
    let mut missing = original.clone();
    let rows = missing["values"].as_array_mut().unwrap();
    let omitted = rows
        .iter()
        .position(|row| {
            row["value"]
                == serde_json::json!({
                    "numerator": "4", "denominator": "5"
                })
        })
        .expect("a noninitial reachable state");
    rows.remove(omitted);
    assert_eq!(missing["ranking"], original["ranking"]);
    assert_eq!(
        check(&serde_json::to_vec(&missing).unwrap()).unwrap(),
        ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
            rule: ProbabilityCertificateRule::Malformed,
            at: ProbabilityCertificateLocus::Member("values"),
        })
    );
    let mut identity = original.clone();
    identity["identity"]["ordinal"] = serde_json::json!(1);
    assert_eq!(
        check(&serde_json::to_vec(&identity).unwrap()),
        Err(fsl_replay::ProbabilityDocumentError::IdentityMismatch)
    );
    let mut objective = original;
    objective["objective"]["optimization"] = serde_json::json!("maximum");
    assert_eq!(
        check(&serde_json::to_vec(&objective).unwrap()),
        Err(fsl_replay::ProbabilityDocumentError::ObjectiveMismatch)
    );
}

#[trace("TC-764", "FR-201-AC-11", "FR-282-AC-12")]
#[test]
fn analyzer_document_budgets_preserve_stage_specific_stops_without_certification() {
    use std::sync::atomic::AtomicBool;
    let world = World::new(&prob_world::link(CLAIMS));
    let engines = Engines::fsl().unwrap();
    let baseline = settle(&world, "Likely", ModelCheckLimits::default());
    assert!(!baseline
        .terminal()
        .unwrap()
        .exact_certificate_document()
        .unwrap()
        .is_empty());
    for storage in [false, true] {
        let mut request = world.request("Likely", ModelCheckLimits::default());
        if storage {
            request.limits.document.storage_bytes = 0;
        } else {
            request.limits.document.bytes = 0;
        }
        let reached_candidate = Arc::new(AtomicBool::new(false));
        let observed = Arc::clone(&reached_candidate);
        let item = exact_probabilistic_item_tampered(RequestIndex::new(0), request, move |_| {
            observed.store(true, Ordering::Relaxed);
        });
        let record = analyze_items(&engines, vec![item], &Cancel::new()).remove(0);
        assert_eq!(
            reached_candidate.load(Ordering::Relaxed),
            !storage,
            "zero output bytes must reach a candidate; zero working storage must stop before one exists"
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
        assert!(record.terminal().unwrap().exact_certificate().is_none());
        assert!(record
            .terminal()
            .unwrap()
            .exact_certificate_document()
            .is_none());
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
    let model = system
        .state_key(&system.initial()[0])
        .expect("the owner encodes its initial state")
        .0;
    let bound =
        ProbabilityProductKey::for_model_state(Some(&binding.reference), &model).expect("encodes");
    let unbound = ProbabilityProductKey::for_model_state(None, &model).expect("encodes");
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
        fsl_foundation::BackendId::new("en-5"),
        &rejected,
        None,
        None,
    )
    .is_none());
}

/// FR-203-AC-1 (TC-638): ADR-028 §15.2's `P95` at `5` (the integer
/// `duration` reward) with no confidence parameters settles `proved` with
/// `ExactValue{24233/25000}`, certified, with one entry; §15.4's `Unlikely`
/// settles `refuted` after replay.
#[trace("TC-638", "FR-203-AC-1")]
#[test]
fn p95_settles_proved_with_its_exact_value() {
    let claim = "probabilistic P95 using ev on M over (s: M::Server) under Steady on origin \
         { quantile 0.95 of (accumulate duration from holds(s.phase = 1) \
         until holds(s.phase = 2) within 10) <= 5 }\n";
    let world = World::new(&prob_world::service(claim));
    let record = settle(&world, "P95", ModelCheckLimits::default());
    let terminal = record.terminal().expect("a terminal record");
    let TerminalValue::Proved {
        basis: ProofBasis::ExactValue { value, entries, .. },
        certification,
    } = terminal.value()
    else {
        panic!("expected proved, got {:?}", terminal.value());
    };
    assert_eq!(*certification, Certification::Certified);
    assert_eq!(value.value(), &ratio(24233, 25000));
    assert_eq!(entries.len(), 1);

    let world = World::new(&prob_world::link(CLAIMS));
    let record = settle(&world, "Unlikely", ModelCheckLimits::default());
    assert_eq!(record.terminal().unwrap().value(), &TerminalValue::Refuted);
}

/// FR-203-AC-5 (TC-641): FR-196-AC-5's `Coin` refutation, a claim read
/// undefined (`absent-key`) at a state of positive probability, settles
/// `refuted` with its replayed undefined path, whatever the threshold.
#[trace("TC-641", "FR-203-AC-5", "TC-3153", "FR-202-AC-5", "FR-202-AC-24")]
#[test]
fn an_undefined_evaluation_settles_refuted_whatever_the_threshold() {
    for threshold in ["0.5", "0.01"] {
        let claim = format!(
            "probabilistic Dropped using ev on M over (c: M::Coin) under Even on origin \
             {{ probability >= {threshold} [ always[0,3] holds(c.v = 0) ] }}"
        );
        let world = World::new(&prob_world::coin(&claim));
        let record = settle(&world, "Dropped", ModelCheckLimits::default());
        assert_eq!(
            record.terminal().unwrap().value(),
            &TerminalValue::Refuted,
            "{threshold}"
        );
        let counterexample = record
            .probabilistic_counterexample
            .as_ref()
            .expect("the replayed undefined path");
        let text = format!("{counterexample:?}");
        assert!(
            text.contains("AbsentKey") || text.contains("absent-key"),
            "{text}"
        );
    }
}

fn undefined_evidence(world: &World, claim: &str) -> fsl_replay::ProbabilityUndefinedWitness {
    let record = settle(world, claim, ModelCheckLimits::default());
    assert_eq!(
        record.terminal().expect("terminal").value(),
        &TerminalValue::Refuted
    );
    let found = record
        .probabilistic_counterexample
        .expect("reproduced evidence");
    assert_eq!(
        found.wire.as_ref().expect("wire")["evidence"]["type"],
        "undefined-path"
    );
    use base64::Engine;
    let witness = found.found.undefined().expect("undefined token");
    assert_eq!(
        found.wire.as_ref().unwrap()["evidence"]["undefined"]["where"]["product_state"]["key"],
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(witness.product_state.as_bytes())
    );
    witness.clone()
}

#[trace("TC-3153", "FR-202-AC-5", "FR-202-AC-24")]
#[test]
fn undefined_replay_checks_first_read_location_control_and_package_ownership() {
    use fsl_replay::{
        replay_probabilistic_undefined, ProbabilityWitnessError, ProbabilityWitnessReplay,
    };
    let claims = "probabilistic Open using ev on M over (c: M::Coin) under Even on origin \
        { probability >= 0.5 [ always[0,3] holds(c.v = 0) ] }\n\
        probabilistic Decided using ev on M over (c: M::Coin) under Even on origin \
        { probability >= 0.5 [ always[0,3] holds(c.v = 1) ] }";
    let world = World::new(&prob_world::coin(claims));
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let witness = undefined_evidence(&world, "Open");
    let limits = world.request("Open", ModelCheckLimits::default()).limits;
    let replay = |witness: &fsl_replay::ProbabilityUndefinedWitness| {
        replay_probabilistic_undefined(
            &world.package,
            &system,
            &witness.path.item,
            witness,
            &limits,
            &Cancel::new(),
        )
    };
    assert!(matches!(
        replay(&witness),
        Ok(ProbabilityWitnessReplay::Reproduced(_))
    ));
    assert_eq!(witness.evaluation.site.position, 1);
    let mut forged = witness.clone();
    forged.evaluation.site.position = 0;
    assert!(matches!(
        replay(&forged),
        Ok(ProbabilityWitnessReplay::Parity)
    ));
    let mut forged = witness.clone();
    forged.evaluation.cause =
        fsl_eval::temporal::UndefinedCause::Kernel(filament_exact::Undefined::DivisionByZero);
    assert!(matches!(
        replay(&forged),
        Ok(ProbabilityWitnessReplay::Parity)
    ));
    let mut forged = witness.clone();
    forged.product_state = forged.path.initial_key.clone();
    assert!(matches!(
        replay(&forged),
        Ok(ProbabilityWitnessReplay::Parity)
    ));
    let mut forged = witness.clone();
    forged.evaluation.site.locus =
        fsl_replay::UndefinedLocus::ClaimPredicate(fsl_replay::ClaimPredicate::Target);
    assert!(matches!(
        replay(&forged),
        Ok(ProbabilityWitnessReplay::Parity)
    ));
    let other = World::new(&prob_world::coin(claims));
    assert!(matches!(
        replay_probabilistic_undefined(
            &other.package,
            &system,
            &witness.path.item,
            &witness,
            &limits,
            &Cancel::new()
        ),
        Err(ProbabilityWitnessError::Refused)
    ));
    let wrong_item = fsl_replay::probabilistic_claim_identity(world.claim("Decided")).unwrap();
    assert!(matches!(
        replay_probabilistic_undefined(
            &world.package,
            &system,
            &wrong_item,
            &witness,
            &limits,
            &Cancel::new()
        ),
        Err(ProbabilityWitnessError::Refused)
    ));
}

#[trace("TC-3153", "FR-202-AC-25", "FR-202-AC-26")]
#[test]
fn undefined_replay_distinguishes_workload_repeated_keys_and_scheduler_choices() {
    use fsl_eval::simulation::model::TransitionId;
    use fsl_replay::{
        replay_probabilistic_undefined, ProbabilityWitnessError, ProbabilityWitnessReplay,
    };
    let claims = "probabilistic Work using inf on M over (c: M::Coin) under Even on origin \
        { probability >= 0.5 [ eventually holds(c.v = 1) ] }\n\
        probabilistic Every using inf on M over (c: M::Coin) under every scheduler on origin \
        { probability >= 0.5 [ eventually holds(c.v = 1) ] }\n\
        probabilistic Bounded using ev on M over (c: M::Coin) under every scheduler on origin \
        { probability >= 0.5 [ always[0,3] holds(c.v = 0) ] }";
    let mut model = prob_world::coin(claims);
    model.operations.push(prob_world::Operation {
        name: "keep",
        parameters: vec![],
        modifies: vec![],
        deletes: false,
    });
    // Zero weight admits keep without changing the workload's drop probability.
    model.body = model.body.replace(
        "weight Coin::drop = 1;",
        "weight Coin::drop = 1; weight Coin::keep = 0;",
    );
    let world = World::new(&model);
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let mut workload = undefined_evidence(&world, "Work");
    let every = undefined_evidence(&world, "Every");
    let bounded = undefined_evidence(&world, "Bounded");
    let pre_read: serde_json::Value =
        serde_json::from_slice(every.product_state.as_bytes()).unwrap();
    assert!(pre_read[5].is_null(), "unbounded monitor is null");
    assert!(pre_read[6].is_null(), "unbounded accumulator is null");
    let limits = world.request("Work", ModelCheckLimits::default()).limits;
    let initial = system.initial().remove(0);
    let workload_decl = system.input().probabilistic.workload("Even").unwrap();
    let returning = system.weighted_steps_with_cancel(&initial, workload_decl, &Cancel::new()).unwrap().unwrap().into_iter().find(|step| matches!(step.random.as_slice(), [filament_exact::Value::Integer(value)] if value == &Integer::from(1_i64))).unwrap();
    let TransitionId::Operation(transition) = returning.transition else {
        panic!("operation");
    };
    let action = returning.identity.unwrap();
    let first = fsl_replay::ProbabilityWitnessStep {
        choice: fsl_replay::ProductAction::Identity(
            fsl_replay::Schedule::of(&system, &action).unwrap(),
        ),
        action: Some(action),
        transition: Some(transition),
        draw: Some(returning.random),
        post: returning.post,
        post_key: workload.path.initial_key.clone(),
        elapsed: ratio(0, 1),
    };
    workload.path.paths[0].steps.insert(0, first);
    workload.evaluation.site.position = 2;
    let replay = |witness: &fsl_replay::ProbabilityUndefinedWitness| {
        replay_probabilistic_undefined(
            &world.package,
            &system,
            &witness.path.item,
            witness,
            &limits,
            &Cancel::new(),
        )
    };
    let Ok(ProbabilityWitnessReplay::Reproduced(token)) = replay(&workload) else {
        panic!("workload permits the returning key");
    };
    assert_eq!(token.mass(), &ratio(1, 4));
    let mut repeated = workload;
    repeated.path.item = every.path.item.clone();
    repeated.path.scheduler = every.path.scheduler.clone();
    assert!(matches!(
        replay(&repeated),
        Ok(ProbabilityWitnessReplay::Parity)
    ));
    let keep = system
        .actions_with_cancel(&initial, &Cancel::new())
        .unwrap()
        .unwrap()
        .into_iter()
        .find(|action| action.identity.operation.contains("keep"))
        .unwrap();
    for name in ["Every", "Bounded"] {
        let choice = fsl_replay::ProductAction::Identity(
            fsl_replay::Schedule::of(&system, &keep.identity).unwrap(),
        );
        let item = fsl_analyze::analyze::exact_probabilistic_item_witness_tampered(
            RequestIndex::new(0),
            world.request(name, ModelCheckLimits::default()),
            move |path| {
                path.scheduler[0].1 = choice.clone();
            },
        );
        let record = analyze_items(&Engines::fsl().unwrap(), vec![item], &Cancel::new()).remove(0);
        assert_eq!(
            record.terminal().unwrap().value(),
            &TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(
                fsl_foundation::diagnostic::Code::InvalidRuntimeInput
            ))
        );
        assert!(record.probabilistic_counterexample.is_none());
    }
    for mut wrong in [every.clone(), bounded] {
        wrong.path.scheduler[0].1 = fsl_replay::ProductAction::Identity(
            fsl_replay::Schedule::of(&system, &keep.identity).unwrap(),
        );
        assert!(matches!(
            replay(&wrong),
            Err(ProbabilityWitnessError::Refused)
        ));
    }
    let own_item = fsl_replay::probabilistic_claim_identity(world.claim("Work")).unwrap();
    assert!(TerminalRecord::refuted_by_probability_witness(
        RequestIndex::new(0),
        fsl_foundation::BackendId::new("en-5"),
        &own_item,
        &token
    )
    .is_some());
    let other_item = fsl_replay::probabilistic_claim_identity(world.claim("Every")).unwrap();
    assert!(TerminalRecord::refuted_by_probability_witness(
        RequestIndex::new(0),
        fsl_foundation::BackendId::new("en-5"),
        &other_item,
        &token
    )
    .is_none());
}

#[trace("TC-3153", "FR-202-AC-26")]
#[test]
fn undefined_replay_preserves_limits_and_cancellation_at_the_final_poll() {
    use fsl_replay::{
        replay_probabilistic_undefined, ProbabilityWitnessError, ProbabilityWitnessReplay,
    };
    let claim = "probabilistic Dropped using ev on M over (c: M::Coin) under Even on origin \
        { probability >= 0.5 [ always[0,3] holds(c.v = 0) ] }";
    let world = World::new(&prob_world::coin(claim));
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let witness = undefined_evidence(&world, "Dropped");
    let limits = world.request("Dropped", ModelCheckLimits::default()).limits;
    let mut key_meter = filament_exact::Meter::new(filament_exact::ScalarLimits {
        work_units: 0,
        ..prob_world::UNLIMITED
    });
    let exhausted = key_meter
        .charge(filament_exact::Charge::new(
            filament_exact::ChargePoint::CollectionMemberTest,
        ))
        .unwrap_err();
    assert!(matches!(
        ProbabilityWitnessError::from(fsl_replay::ProbabilityDocumentError::Accounting(Box::new(
            exhausted
        ))),
        ProbabilityWitnessError::Limit {
            limit: ModelCheckLimit::EvaluationMeter { .. },
            value: 0
        }
    ));
    for budget in ["work", "expansion", "storage", "paths"] {
        let mut request = world.request("Dropped", ModelCheckLimits::default());
        match budget {
            "work" => request.subject.simulation.meter.work_units = 0,
            "expansion" => request.subject.simulation.expansion.max_candidates = 0,
            "storage" => request.limits.document.storage_bytes = 0,
            "paths" => request.limits.max_witness_paths = 0,
            _ => unreachable!(),
        }
        let record = analyze_items(
            &Engines::fsl().unwrap(),
            vec![exact_probabilistic_item(RequestIndex::new(0), request)],
            &Cancel::new(),
        )
        .remove(0);
        assert!(
            matches!(
                record.terminal().expect("stopped terminal").value(),
                TerminalValue::Incomplete(IncompleteCause::LimitReached { .. })
            ),
            "{budget}: {record:?}"
        );
        assert!(record.probabilistic_counterexample.is_none(), "{budget}");
        assert!(record.exact_probabilistic.is_none(), "{budget}");
    }
    for budget in ["work", "expansion"] {
        let mut simulation = world.subject().simulation;
        match budget {
            "work" => simulation.meter.work_units = 0,
            "expansion" => simulation.expansion.max_candidates = 0,
            _ => unreachable!(),
        }
        let meter = simulation.meter;
        let expansion = simulation.expansion;
        let replay_system = ModelSystem::new(simulation)
            .expect("limited replay subject admits without rebuilding evidence");
        let error = replay_probabilistic_undefined(
            &world.package,
            &replay_system,
            &witness.path.item,
            &witness,
            &limits,
            &Cancel::new(),
        )
        .expect_err("native replay must exhaust its own budget");
        let ProbabilityWitnessError::Build(issue) = error else {
            panic!("expected typed replay exhaustion, got {error:?}");
        };
        let settled = fsl_replay::settle_exact_prob_build_issue(
            &issue,
            fsl_replay::statistical::SubjectLimits {
                expansion,
                meter: &meter,
            },
        );
        assert!(
            matches!(
                (budget, settled.value()),
                (
                    "work",
                    TerminalValue::Incomplete(IncompleteCause::LimitReached {
                        limit: ModelCheckLimit::EvaluationMeter { .. },
                        value: 0
                    })
                ) | (
                    "expansion",
                    TerminalValue::Incomplete(IncompleteCause::LimitReached {
                        limit: ModelCheckLimit::MaxCandidates,
                        value: 0
                    })
                )
            ),
            "{budget}: {settled:?}"
        );
    }
    let run = |limits: &fsl_analyze::exact_probabilistic::ExactProbLimits, cancel: &Cancel| {
        replay_probabilistic_undefined(
            &world.package,
            &system,
            &witness.path.item,
            &witness,
            limits,
            cancel,
        )
    };
    let mut stopped = limits;
    stopped.model.limits.max_states = 0;
    assert!(matches!(
        run(&stopped, &Cancel::new()),
        Err(ProbabilityWitnessError::Graph(
            ExactProbabilityGraphError::LimitReached
        ))
    ));
    let mut stopped = limits;
    stopped.model.limits.max_transitions = 0;
    assert!(matches!(
        run(&stopped, &Cancel::new()),
        Err(ProbabilityWitnessError::Graph(
            ExactProbabilityGraphError::EdgeLimitReached
        ))
    ));
    let mut stopped = limits;
    stopped.document.storage_bytes = 0;
    assert!(matches!(
        run(&stopped, &Cancel::new()),
        Err(ProbabilityWitnessError::Limit {
            limit: ModelCheckLimit::CertificateMaxWorkingBytes,
            value: 0
        })
    ));
    let polls = Arc::new(AtomicU32::new(0));
    let count = Arc::clone(&polls);
    let counting = Cancel::observing(move || {
        count.fetch_add(1, Ordering::Relaxed);
    });
    assert!(matches!(
        run(&limits, &counting),
        Ok(ProbabilityWitnessReplay::Reproduced(_))
    ));
    let final_poll = polls.load(Ordering::Relaxed);
    assert!(final_poll > 0);
    for cause in [CancelCause::Requested, CancelCause::Deadline] {
        let cancelled = Cancel::new();
        cancelled.cancel(cause);
        assert!(
            matches!(run(&limits, &cancelled), Err(ProbabilityWitnessError::Graph(ExactProbabilityGraphError::Cancelled(found))) if found == cause)
        );
        let slot = Arc::new(OnceLock::<Cancel>::new());
        let weak = Arc::downgrade(&slot);
        let seen = Arc::new(AtomicU32::new(0));
        let count = Arc::clone(&seen);
        let cancel = Cancel::observing(move || {
            if count.fetch_add(1, Ordering::Relaxed) + 1 == final_poll {
                weak.upgrade().unwrap().get().unwrap().cancel(cause);
            }
        });
        slot.set(cancel.clone()).unwrap();
        assert!(
            matches!(run(&limits, &cancel), Err(ProbabilityWitnessError::Graph(ExactProbabilityGraphError::Cancelled(found))) if found == cause)
        );
    }
}

/// Rebuild every offered key under the actual checked claim, deliberately
/// continuing even after its monitor decides to forge an adversarial path.
fn rekey_undefined(
    world: &World,
    name: &str,
    witness: &mut fsl_replay::ProbabilityUndefinedWitness,
) {
    use fsl_eval::automaton::QualifiedBoundedMonitor;
    use fsl_eval::simulation::model::PredicateReading;
    use fsl_semantics::check::{Event, ProbForm};
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let claim = world.claim(name);
    witness.path.item = fsl_replay::probabilistic_claim_identity(claim).unwrap();
    let binding = claim_binding(&system, claim).unwrap();
    let reference = binding.as_ref().map(|b| &b.reference);
    let context = fsl_replay::probabilistic_formula_monitor_context(claim, reference).unwrap();
    let ProbForm::Probability {
        event: Event::Formula { formula, .. },
        ..
    } = &claim.form
    else {
        panic!("bounded formula");
    };
    let mut monitor =
        QualifiedBoundedMonitor::for_observations(formula.formula(), &context, 100).unwrap();
    let predicate = formula.condition(monitor.atom().index()).unwrap();
    witness.evaluation.site.locus = fsl_replay::UndefinedLocus::Atom(monitor.atom());
    let mut control = None;
    for position in 0..=witness.path.paths[0].steps.len() {
        let state = if position == 0 {
            &witness.path.initial
        } else {
            &witness.path.paths[0].steps[position - 1].post
        };
        let readings = system
            .state_predicates_with_cancel(state, &[(predicate, reference)], &Cancel::new())
            .unwrap();
        match readings.as_slice() {
            [PredicateReading::True] | [PredicateReading::False] => {
                let truth = matches!(readings[0], PredicateReading::True);
                control = Some(
                    match control {
                        None => monitor.start(truth),
                        Some(before) => monitor.step(before, truth),
                    }
                    .unwrap(),
                );
            }
            [PredicateReading::Undefined { cause, .. }] => {
                witness.evaluation.cause = cause.clone();
            }
            other => panic!("unexpected letter: {other:?}"),
        }
        let monitor_key = match control {
            Some(control) => monitor.key(control),
            None => monitor.unread_key(),
        }
        .unwrap();
        let raw = system.state_key(state).unwrap().0;
        let key = ProbabilityProductKey::for_formula_state(
            reference,
            &raw,
            &monitor_key,
            fsl_replay::ProbabilityDocumentIoLimits::default(),
            &Cancel::new(),
        )
        .unwrap();
        if position == 0 {
            witness.path.initial_key = key.clone();
        } else {
            witness.path.paths[0].steps[position - 1].post_key = key.clone();
        }
        witness.product_state = key;
    }
}

#[trace("TC-3153", "FR-202-AC-24")]
#[test]
fn undefined_replay_stops_at_decisions_with_matching_claim_keys() {
    use fsl_replay::{replay_probabilistic_undefined, ProbabilityWitnessReplay};
    let claims = "probabilistic Open using ev on M over (c: M::Coin) under Even on origin { probability >= 0.5 [ always[0,3] holds(c.armed or not c.armed) ] }\n\
        probabilistic Initial using ev on M over (c: M::Coin) under Even on origin { probability >= 0.5 [ always[0,3] holds(c.armed) ] }\n\
        probabilistic Later using ev on M over (c: M::Coin) under Even on origin { probability >= 0.5 [ always[0,3] holds(not c.armed) ] }";
    let world = World::new(&prob_world::armed_coin(claims));
    let witness = undefined_evidence(&world, "Open");
    assert_eq!(witness.evaluation.site.position, 2);
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let mut decisions = Vec::new();
    for name in ["Initial", "Later"] {
        let mut forged = witness.clone();
        rekey_undefined(&world, name, &mut forged);
        let limits = world.request(name, ModelCheckLimits::default()).limits;
        decisions.push((
            name,
            matches!(
                replay_probabilistic_undefined(
                    &world.package,
                    &system,
                    &forged.path.item,
                    &forged,
                    &limits,
                    &Cancel::new()
                ),
                Ok(ProbabilityWitnessReplay::Parity)
            ),
        ));
    }
    assert!(
        decisions.iter().all(|(_, parity)| *parity),
        "decided controls must reject matching-key continuations: {decisions:?}"
    );
}

#[trace("FR-201")]
#[test]
fn defined_always_items_keep_their_prior_unsupported_disposition() {
    let claims = "probabilistic True using ev on M over (c: M::Coin) under Even on origin { probability >= 0.5 [ always[0,0] holds(c.v = 0) ] }\n\
        probabilistic False using ev on M over (c: M::Coin) under Even on origin { probability >= 0.5 [ always[0,0] holds(c.v = 1) ] }";
    let mut model = prob_world::coin(claims);
    model
        .body
        .push_str("\npre Never using v on M::Coin::drop { false }\n");
    let world = World::new(&model);
    for name in ["True", "False"] {
        let record = settle(&world, name, ModelCheckLimits::default());
        assert!(
            record.terminal().is_none(),
            "defined always must retain its existing unsupported disposition: {record:?}"
        );
        assert!(record.exact_probabilistic.is_none());
        assert!(record.probabilistic_counterexample.is_none());
    }
}

fn external_always_world(bound: &str) -> World {
    let claim = format!("probabilistic External using ev on M over (c: M::Coin) under Even on origin {{ probability {bound} [ always[0,0] holds(c.v = 0) ] }}");
    let mut model = prob_world::coin(&claim);
    model
        .body
        .push_str("\npre Never using v on M::Coin::drop { false }\n");
    World::new(&model)
}

fn external_always_initial_key(world: &World, system: &ModelSystem<'_>) -> ProbabilityProductKey {
    use fsl_eval::automaton::QualifiedBoundedMonitor;
    use fsl_semantics::check::{Event, ProbForm};
    let claim = world.claim("External");
    let binding = claim_binding(system, claim).unwrap();
    let reference = binding.as_ref().map(|binding| &binding.reference);
    let context = fsl_replay::probabilistic_formula_monitor_context(claim, reference).unwrap();
    let ProbForm::Probability {
        event: Event::Formula { formula, .. },
        ..
    } = &claim.form
    else {
        panic!("bounded formula");
    };
    let mut monitor =
        QualifiedBoundedMonitor::for_observations(formula.formula(), &context, 100).unwrap();
    let control = monitor.start(true).unwrap();
    let monitor_key = monitor.key(control).unwrap();
    let raw = system.state_key(&system.initial()[0]).unwrap().0;
    ProbabilityProductKey::for_formula_state(
        reference,
        &raw,
        &monitor_key,
        fsl_replay::ProbabilityDocumentIoLimits::default(),
        &Cancel::new(),
    )
    .unwrap()
}

#[trace("FR-201")]
#[test]
fn external_always_probability_certificate_remains_unsupported() {
    let world = external_always_world(">= 0.5");
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let key = external_always_initial_key(&world, &system);
    let identity = fsl_replay::probabilistic_claim_identity(world.claim("External")).unwrap();
    let document = ProbabilityCertificateDocument {
        cyclic_dtmc: false,
        identity: identity.clone(),
        targets: vec![key.clone()],
        event: None,
        failures: Vec::new(),
        reward: None,
        optimization: fsl_replay::ProbabilityObjective::Workload,
        timed: None,
        values: vec![fsl_replay::ExactProbabilityValue {
            state: key.clone(),
            value: ratio(1, 1),
        }],
        ranking: vec![(key, 0)],
    };
    let limits = fsl_replay::ProbabilityDocumentLimits {
        bytes: 1 << 20,
        storage_bytes: 1 << 24,
        states: 100,
        edges: 100,
        scalar_bits: 256,
        accounting: prob_world::UNLIMITED,
    };
    let bytes = fsl_replay::write_probability_certificate(&document, limits, &Cancel::new())
        .expect("well-formed externally offered certificate");
    let checked = fsl_replay::check_probability_certificate_document(
        &system,
        world.package.graph().probabilistic_model().claims(),
        &identity,
        &bytes,
        limits,
        &Cancel::new(),
    );
    assert!(
        matches!(
            checked,
            Err(fsl_replay::ProbabilityDocumentError::Unsupported)
        ),
        "defined always must refuse admission: {checked:?}"
    );
}

#[trace("FR-202")]
#[test]
fn external_always_path_set_remains_unsupported() {
    let world = external_always_world("<= 0.5");
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let key = external_always_initial_key(&world, &system);
    let identity = fsl_replay::probabilistic_claim_identity(world.claim("External")).unwrap();
    let binding = claim_binding(&system, world.claim("External")).unwrap();
    let witness = fsl_replay::ProbabilisticCounterexample {
        item: identity.clone(),
        binding: binding.map(|binding| binding.reference),
        initial: system.initial().remove(0),
        initial_key: key,
        scheduler: Vec::new(),
        paths: vec![fsl_replay::ProbabilityWitnessPath { steps: Vec::new() }],
        subsystem: None,
    };
    let limits = world
        .request("External", ModelCheckLimits::default())
        .limits;
    assert!(matches!(
        fsl_replay::replay_probabilistic_witness(
            &world.package,
            &system,
            &identity,
            &witness,
            &limits,
            &Cancel::new()
        ),
        Err(fsl_replay::ProbabilityWitnessError::Graph(
            ExactProbabilityGraphError::UnsupportedClaimForm
        ))
    ));
}
