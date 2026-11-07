// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-516, FR-522, FR-523, FR-524 (TC-1311, TC-1317, TC-1318, TC-1319): the
//! rulings of the evidence call: an `Incomplete` is an `Ok` result that
//! returns the request's ledgers, a faulted instance is retained, an origin
//! supplies captures, surplus capture facts are refused, silent samples are
//! charged at the meter alone, the occurrence selector pairs a clock row by
//! key, and the identity binds every evidence member.

use ix_trace_rs::trace;
use qsl_eval::temporal::evidence::{
    evaluate_temporal_evidence, ActivationState, ActivationStop, CaptureFact, CapturedValue,
    Completeness, Disposition, Evicted, EvidenceCounter, EvidenceIncomplete, EvidenceLedger,
    EvidenceRequest, ExecutionDisposition, FaultKind, FaultSubject, Indeterminate, InstanceState,
    NonValueKind, Payload, Receipt, Relation, SubjectOccurrence, TemporalEvidenceLimits,
    TriggerEvidence, TriggerRecord, TriggerScope, Truth, UnknownKind,
};
use qsl_eval::temporal::{ProgressClosure, ProgressCompleteness, Valuation};
use qsl_semantics::check::SCALAR_LIMITS_UNLIMITED;
use quire_eval::CallFailure;
use quire_exact::{Meter, ScalarLimits};
use serde_json::json;

use crate::temporal_activation::{
    activated, call, claim, delivery, env, event_env, expect_refusal, fault_of, fixed_env, id,
    input, origin, position, request, run, try_env_under, unlimited, Env, EACH, GUARDED,
    ONE_CAPTURE,
};

fn each() -> (Env, u64) {
    let env = env(EACH, ONE_CAPTURE);
    let p = env.atom("p");
    (env, p)
}

fn asked(env: &Env, p: u64) -> EvidenceRequest {
    let mut asked = request(
        env,
        (0..3)
            .map(|i| position(i, &[(p, Valuation::True)]))
            .collect(),
    );
    asked.deliveries = vec![delivery("t1", "r", "payload", &["5"])];
    asked.trigger = Some(id("t1"));
    asked
}

/// An edit of one request member.
type RequestEdit = Box<dyn Fn(&mut EvidenceRequest)>;

const ORIGIN_CAPTURE: &str = "capture amount: Int[0, 9] = c.level; eventually[0, 2] holds(p)";

#[trace("FR-524-AC-32", "TC-1319")]
#[test]
fn an_origin_establishes_its_captures_from_its_own_facts() {
    let env = env("on origin", ORIGIN_CAPTURE);
    let with = |captures: Vec<CaptureFact>| {
        let mut one = input(Vec::new());
        let mut arrival = origin("o1", "a");
        arrival.captures = captures;
        one.origins = vec![arrival];
        activated(&env, one)
    };
    let value = with(vec![CaptureFact::Value {
        anchor: "obs0".to_owned(),
        value: CapturedValue::new(b"7".as_slice()),
    }]);
    assert_eq!(value.instances.len(), 1);
    let kept = value.instances[0].environment.get("amount").expect("kept");
    assert_eq!(kept.value().as_bytes(), b"7");
    assert_eq!(
        fault_of(&with(Vec::new()), "o1"),
        (
            FaultSubject::Capture("amount".to_owned()),
            FaultKind::Missing
        )
    );
    assert_eq!(
        fault_of(&with(vec![CaptureFact::WrongType]), "o1"),
        (
            FaultSubject::Capture("amount".to_owned()),
            FaultKind::Refused
        )
    );
}

#[trace("FR-524-AC-33", "FR-524-AC-38", "FR-516-AC-28", "TC-1319", "TC-1311")]
#[test]
fn a_surplus_capture_fact_is_refused_after_the_contradiction_check_and_uncharged() {
    let (env, p) = each();
    let mut surplus = delivery("t2", "r", "p", &["5", "6"]);
    surplus.captures.push(CaptureFact::Null);
    let mut request_in = input(vec![delivery("t1", "r", "p", &["5"]), surplus.clone()]);
    match run(&env, request_in.clone()) {
        Err(ActivationStop::Surplus(found)) => {
            assert_eq!(found.surplus_capture_identity, id("t2"));
            assert_eq!(found.surplus_capture_index, 1);
        }
        other => panic!("a surplus refusal, not {other:?}"),
    }
    // A contradiction on t1 comes first.
    request_in
        .deliveries
        .insert(1, delivery("t1", "r2", "other", &["5"]));
    match run(&env, request_in) {
        Err(ActivationStop::Contradiction(found)) => {
            assert_eq!(found.contradicted_identity, id("t1"));
        }
        other => panic!("the contradiction first, not {other:?}"),
    }
    // Through the call: Err(Admission), nothing charged.
    let mut through = asked(&env, p);
    through.deliveries = vec![surplus];
    through.trigger = Some(id("t2"));
    let mut meter = unlimited();
    let refused = evaluate_temporal_evidence(&env.subject(&[]), &through, &mut meter);
    expect_refusal(refused, "surplus-capture");
    assert!(meter.admitted_charges().is_empty());
    // An origin's surplus fact is refused the same way.
    let origins = self::env("on origin", ORIGIN_CAPTURE);
    let mut over = input(Vec::new());
    let mut arrival = origin("o1", "a");
    arrival.captures = vec![CaptureFact::Null, CaptureFact::Null];
    over.origins = vec![arrival];
    assert!(matches!(
        run(&origins, over),
        Err(ActivationStop::Surplus(found)) if found.surplus_capture_identity == id("o1")
    ));
}

#[trace("FR-524-AC-34", "FR-524-AC-11", "FR-516-AC-28", "TC-1319", "TC-1311")]
#[test]
fn a_guard_fault_is_listed_unavailable_and_a_false_guard_is_not_listed() {
    let env = env(GUARDED, ONE_CAPTURE);
    let p = env.atom("p");
    let mut unknown = delivery("t1", "r", "p", &["5"]);
    unknown.guard = None;
    let mut off = delivery("t2", "r", "p", &["5"]);
    off.guard = Some(false);
    let healthy = delivery("t3", "r", "p", &["5"]);
    let activation = activated(
        &env,
        input(vec![unknown.clone(), off.clone(), healthy.clone()]),
    );
    let listed: Vec<_> = activation
        .instances
        .iter()
        .map(|i| i.identity.clone())
        .collect();
    assert_eq!(listed, [id("t1"), id("t3")]);
    assert_eq!(
        fault_of(&activation, "t1"),
        (FaultSubject::Guard, FaultKind::Missing)
    );
    assert!(matches!(
        activation.instances[1].state,
        InstanceState::Active
    ));
    // The same through the call, assessing the healthy one.
    let mut through = request(
        &env,
        (0..3)
            .map(|i| position(i, &[(p, Valuation::True)]))
            .collect(),
    );
    through.deliveries = vec![unknown, off, healthy];
    through.trigger = Some(id("t3"));
    let result = call(&env, &env.subject(&[]), &through).expect("a result");
    assert_eq!(result.truth, Some(Truth::True));
    assert_eq!(result.instances.len(), 2);
    assert!(matches!(
        result.instances[0].state,
        InstanceState::Unavailable(_)
    ));
}

#[trace("FR-524-AC-35", "FR-524-AC-37", "FR-523-AC-20", "TC-1319", "TC-1318")]
#[test]
fn a_faulted_instance_is_retained_and_the_instances_counter_counts_creations_only() {
    let env = env(EACH, ONE_CAPTURE);
    let first = activated(&env, input(vec![delivery("t1", "r", "p", &["5"])]));
    let mut broken = delivery("t3", "r3", "p3", &["5"]);
    broken.captures[0] = CaptureFact::Missing;
    let mut later = input(vec![
        delivery("t1", "r1b", "p", &["9"]),
        delivery("t2", "r2", "p2", &["5"]),
        broken.clone(),
    ]);
    later.ledger = Some(first.ledger.clone());
    let mut meter = unlimited();
    let mut call_meter =
        qsl_eval::temporal::evidence::CallMeter::new(TemporalEvidenceLimits::default(), &mut meter);
    let activation = qsl_eval::temporal::evidence::activate(
        &qsl_eval::temporal::evidence::ClauseShape::of(
            &qsl_eval::temporal::evidence::ClauseFacts::of_checked(&env.fixture.clause),
        ),
        &env.obligation(),
        later,
        &mut call_meter,
    )
    .expect("an activation");
    assert_eq!(call_meter.usage().get(EvidenceCounter::Instances), 1);
    assert_eq!(activation.instances.len(), 3);
    let ledger = activation.ledger;
    assert_eq!(ledger.instance_count(), 3);
    let kept = ledger
        .instance(&env.obligation(), &id("t3"))
        .expect("retained");
    assert!(kept.fault().is_some());
    assert_eq!(kept.environment().entries().len(), 0);
    assert_eq!(kept.payload().as_bytes(), b"p3");
    // A later call: an equal payload adds its receipt and keeps the fault.
    let mut again = input(vec![delivery("t3", "r3b", "p3", &["1"])]);
    again.ledger = Some(ledger.clone());
    let after = activated(&env, again);
    assert_eq!(
        fault_of(&after, "t3"),
        (
            FaultSubject::Capture("amount".to_owned()),
            FaultKind::Missing
        )
    );
    let receipts: Vec<&[u8]> = after.instances[0]
        .receipts
        .iter()
        .map(Receipt::as_bytes)
        .collect();
    assert_eq!(receipts, [b"r3".as_slice(), b"r3b".as_slice()]);
    assert_eq!(after.instances[0].evaluations, 0);
    // A different payload is the contradiction.
    let mut clash = input(vec![delivery("t3", "r3c", "other", &["1"])]);
    clash.ledger = Some(ledger);
    assert!(matches!(
        run(&env, clash),
        Err(ActivationStop::Contradiction(_))
    ));
}

#[trace("FR-524-AC-36", "TC-1319")]
#[test]
fn one_evicted_visit_is_charged_per_hit() {
    let env = env(
        EACH,
        "capture a: Int[0, 9] = q.level; capture b: Int[0, 9] = q.level; eventually[0, 2] holds(p)",
    );
    let usage = |evicted: Vec<Evicted>| {
        let mut meter = unlimited();
        let mut call_meter = qsl_eval::temporal::evidence::CallMeter::new(
            TemporalEvidenceLimits::default(),
            &mut meter,
        );
        let mut one = input(vec![delivery("t1", "r", "p", &["1", "2"])]);
        one.evicted = evicted;
        qsl_eval::temporal::evidence::activate(
            &qsl_eval::temporal::evidence::ClauseShape::of(
                &qsl_eval::temporal::evidence::ClauseFacts::of_checked(&env.fixture.clause),
            ),
            &env.obligation(),
            one,
            &mut call_meter,
        )
        .expect("an activation");
        call_meter.usage()
    };
    let record = |name: &str| Evicted::Capture {
        instance: id("t1"),
        capture: name.to_owned(),
    };
    let none = usage(Vec::new());
    let both = usage(vec![record("a"), record("b")]);
    // The delivery is one visit; the first hit ends the consultation.
    assert_eq!(both.get(EvidenceCounter::EvaluationSteps), 2);
    assert_eq!(both.get(EvidenceCounter::Visited), 1);
    assert_eq!(none.get(EvidenceCounter::EvaluationSteps), 3);
    let elsewhere = usage(vec![Evicted::Capture {
        instance: id("t9"),
        capture: "a".to_owned(),
    }]);
    assert_eq!(elsewhere.get(EvidenceCounter::EvaluationSteps), 3);
    // A valuation lookup that matches one evicted record charges one visit.
    let (e, p) = each();
    let subject = e.subject(&[]);
    let mut hit = asked(&e, p);
    let clean = call(&e, &subject, &hit).expect("a result");
    hit.evicted = vec![Evicted::Valuation {
        instance: None,
        position: 0,
        atom: p,
    }];
    let stopped = call(&e, &subject, &hit).expect("a result");
    assert!(stopped.incomplete.is_some());
    let miss_request = {
        let mut miss = asked(&e, p);
        miss.evicted = vec![Evicted::Valuation {
            instance: None,
            position: 0,
            atom: p + 100,
        }];
        miss
    };
    let missed = call(&e, &subject, &miss_request).expect("a result");
    assert_eq!(missed.usage.entries(), clean.usage.entries());
}

#[trace("FR-524-AC-39", "FR-516-AC-34", "FR-516-AC-35", "TC-1319", "TC-1311")]
#[test]
fn execution_is_carried_into_unknown_and_changes_nothing_else() {
    let (env, p) = each();
    let with = |execution, closure, evidence, record| {
        let mut one = input(Vec::new());
        one.execution = execution;
        one.scope = TriggerScope {
            closure,
            evidence,
            record,
        };
        activated(&env, one).disposition
    };
    for execution in [
        ExecutionDisposition::Completed,
        ExecutionDisposition::Failed,
    ] {
        for (evidence, kind) in [
            (TriggerEvidence::Admitted, UnknownKind::Open),
            (TriggerEvidence::Missing, UnknownKind::Missing),
            (TriggerEvidence::Refused, UnknownKind::Refused),
        ] {
            assert_eq!(
                with(
                    execution,
                    ProgressClosure::Open,
                    evidence,
                    TriggerRecord::Complete
                ),
                Disposition::Unknown {
                    kind,
                    completeness: Completeness::Complete,
                    execution
                }
            );
        }
        assert_eq!(
            with(
                execution,
                ProgressClosure::Closed,
                TriggerEvidence::Admitted,
                TriggerRecord::Complete
            ),
            Disposition::Inactive
        );
    }
    // Through the call: Unknown carries it; an admitted delivery does not
    // change activation or truth.
    let subject = env.subject(&[]);
    let mut open = request(
        &env,
        (0..3)
            .map(|i| position(i, &[(p, Valuation::True)]))
            .collect(),
    );
    open.trigger_scope = ProgressClosure::Open;
    let completed = call(&env, &subject, &open).expect("a result");
    open.execution = ExecutionDisposition::Failed;
    let failed = call(&env, &subject, &open).expect("a result");
    assert_eq!(completed.execution, ExecutionDisposition::Completed);
    assert_eq!(failed.execution, ExecutionDisposition::Failed);
    assert_eq!(completed.activation, ActivationState::Unknown);
    assert_eq!(failed.activation, ActivationState::Unknown);
    let (mut a, mut b) = (asked(&env, p), asked(&env, p));
    b.execution = ExecutionDisposition::Failed;
    a.execution = ExecutionDisposition::Completed;
    let (ra, rb) = (
        call(&env, &subject, &a).expect("a result"),
        call(&env, &subject, &b).expect("a result"),
    );
    assert_eq!((ra.activation, ra.truth), (rb.activation, rb.truth));
    // The trigger record decides Inactive or Unknown(open) (AC-35).
    let mut closed = request(
        &env,
        (0..3)
            .map(|i| position(i, &[(p, Valuation::True)]))
            .collect(),
    );
    assert_eq!(
        call(&env, &subject, &closed).expect("a result").activation,
        ActivationState::Inactive
    );
    closed.completeness = Some(qsl_eval::temporal::evidence::CompletenessRecord {
        reference: qsl_eval::temporal::evidence::EvidenceRef::named("incomplete"),
        state: ProgressCompleteness::Incomplete,
        facts: Vec::new(),
    });
    assert_eq!(
        call(&env, &subject, &closed).expect("a result").activation,
        ActivationState::Unknown
    );
}

#[trace(
    "FR-524-AC-28",
    "FR-524-AC-31",
    "FR-524-AC-12",
    "TC-1319",
    "FR-044-AC-4"
)]
#[test]
fn trigger_evidence_and_capture_faults_reach_the_call() {
    let (env, p) = each();
    let subject = env.subject(&[]);
    for (evidence, code) in [
        (TriggerEvidence::Missing, "trigger-evidence-missing"),
        (TriggerEvidence::Refused, "trigger-evidence-refused"),
    ] {
        let mut one = request(
            &env,
            (0..3)
                .map(|i| position(i, &[(p, Valuation::True)]))
                .collect(),
        );
        one.trigger_evidence = evidence;
        for closure in [ProgressClosure::Closed, ProgressClosure::Open] {
            one.trigger_scope = closure;
            let result = call(&env, &subject, &one).expect("a result");
            assert_eq!(result.activation, ActivationState::Unknown);
            assert_eq!(result.truth, None);
            let named = result.non_value.expect("a non-value");
            assert_eq!(
                (named.kind, named.code),
                (NonValueKind::ActivationUnknown, code)
            );
        }
    }
    for edit in [0, 1] {
        let mut broken = asked(&env, p);
        if edit == 0 {
            broken.deliveries[0].captures[0] = CaptureFact::Null;
        } else {
            broken.evicted = vec![Evicted::Capture {
                instance: id("t1"),
                capture: "amount".to_owned(),
            }];
        }
        let result = call(&env, &subject, &broken).expect("a result");
        assert_eq!(result.activation, ActivationState::Unavailable);
        assert_eq!(result.truth, None);
        assert_eq!(result.usage.get(EvidenceCounter::Positions), 0);
        assert_eq!(result.usage.get(EvidenceCounter::Valuations), 0);
        let named = result.non_value.expect("a non-value");
        assert_eq!(
            (named.kind, named.dimension.as_deref()),
            (NonValueKind::Missing, Some("amount"))
        );
    }
}

#[trace(
    "FR-516-AC-25",
    "FR-516-AC-31",
    "FR-523-AC-22",
    "FR-523-AC-23",
    "FR-523-AC-12",
    "FR-523-AC-13",
    "TC-1311",
    "TC-1318"
)]
#[test]
fn every_stop_is_an_ok_incomplete_result_that_returns_the_requests_ledgers() {
    let (env, p) = each();
    let subject = env.subject(&[]);
    // A request ledger that holds t0 and an earlier result.
    let mut seed = asked(&env, p);
    seed.deliveries = vec![delivery("t0", "r", "seed", &["5"])];
    seed.trigger = Some(id("t0"));
    seed.progress_ledger = Some(qsl_eval::temporal::ProgressLedger::new());
    let seeded = call(&env, &subject, &seed).expect("a result");
    let base = {
        let mut next = asked(&env, p);
        next.ledger = Some(seeded.ledger.clone());
        next.progress_ledger = seeded.progress_ledger.clone();
        next
    };
    let full = call(&env, &subject, &base).expect("a result");
    assert_eq!(full.truth, Some(Truth::True));
    assert_ne!(full.ledger, seeded.ledger, "a completed call creates t1");
    let limited = |counter: EvidenceCounter, ceiling: u64| {
        let mut one = base.clone();
        one.limits = TemporalEvidenceLimits::default().with_ceiling(counter, ceiling);
        one
    };
    let stops: Vec<(&str, EvidenceRequest)> = vec![
        ("positions", limited(EvidenceCounter::Positions, 0)),
        ("valuations", limited(EvidenceCounter::Valuations, 0)),
        ("formula_nodes", limited(EvidenceCounter::FormulaNodes, 0)),
        ("history_span", limited(EvidenceCounter::HistorySpan, 1)),
        ("instances", limited(EvidenceCounter::Instances, 0)),
        ("request_bytes", limited(EvidenceCounter::RequestBytes, 0)),
    ];
    for (name, stopped) in stops {
        let result = call(&env, &subject, &stopped).expect("an Ok result, never Err");
        assert_eq!(result.activation, ActivationState::Unavailable, "{name}");
        assert_eq!(result.truth, None, "{name}");
        assert_eq!(result.settlement, None, "{name}");
        match &result.incomplete {
            Some(EvidenceIncomplete::Limit(reached)) => assert_eq!(reached.counter.name(), name),
            other => panic!("{name}: a counter stop, not {other:?}"),
        }
        assert_eq!(result.ledger, seeded.ledger, "{name}: the request's ledger");
        assert_eq!(
            result.progress_ledger, seeded.progress_ledger,
            "{name}: the request's progress ledger"
        );
    }
    // A meter stop.
    let mut tight = Meter::new(ScalarLimits {
        work_units: 1,
        ..SCALAR_LIMITS_UNLIMITED
    });
    let result = evaluate_temporal_evidence(&subject, &base, &mut tight).expect("an Ok result");
    assert!(matches!(
        result.incomplete,
        Some(EvidenceIncomplete::Meter { .. })
    ));
    assert_eq!(result.activation, ActivationState::Unavailable);
    assert_eq!(result.ledger, seeded.ledger);
    // An evicted required valuation.
    let mut evicted = base.clone();
    evicted.evicted = vec![Evicted::Valuation {
        instance: None,
        position: 0,
        atom: p,
    }];
    let result = call(&env, &subject, &evicted).expect("an Ok result");
    assert!(matches!(
        result.incomplete,
        Some(EvidenceIncomplete::EvictedValuation { .. })
    ));
    assert_eq!(result.ledger, seeded.ledger);
    // A tick sum that does not fit names history_span.
    let mut high = base.clone();
    high.positions[0].position = u64::MAX;
    high.positions[1].position = u64::MAX;
    high.positions[2].position = u64::MAX;
    let result = call(&env, &subject, &high).expect("an Ok result");
    match result.incomplete {
        Some(EvidenceIncomplete::Limit(reached)) => {
            assert_eq!(reached.counter, EvidenceCounter::HistorySpan);
        }
        other => panic!("a history_span stop, not {other:?}"),
    }
    assert_eq!(result.activation, ActivationState::Unavailable);
    // A withdrawn anchor is Unavailable with a Missing non-value and no
    // `incomplete`.
    let mut withdrawn = base.clone();
    withdrawn.anchor = None;
    let result = call(&env, &subject, &withdrawn).expect("an Ok result");
    assert_eq!(result.activation, ActivationState::Unavailable);
    assert!(result.incomplete.is_none());
    assert_eq!(
        result.non_value.expect("a non-value").dimension.as_deref(),
        Some("anchor")
    );
    // The retry under sufficient ceilings equals the call that never stopped.
    let retry = call(&env, &subject, &base).expect("a result");
    assert_eq!(retry.result_identity, full.result_identity);
    assert_eq!(retry.ledger, full.ledger);
    assert_eq!(retry.usage.entries(), full.usage.entries());
}

#[trace("FR-516-AC-26", "TC-1311")]
#[test]
fn a_pending_next_event_verdict_and_a_pending_verdict_are_both_pending() {
    // Event position, open scope: only a further observation resolves it.
    let event = event_env("on origin", "eventually[0, 3] holds(p)");
    let p = event.atom("p");
    let mut asked = request(
        &event,
        (0..2)
            .map(|i| position(i, &[(p, Valuation::False)]))
            .collect(),
    );
    asked.origins = vec![origin("o1", "a")];
    asked.trigger = Some(id("o1"));
    asked.decision_closure = Some(ProgressClosure::Open);
    asked.trigger_scope = ProgressClosure::Open;
    let result = call(&event, &event.subject(&[]), &asked).expect("a result");
    assert_eq!(result.truth, Some(Truth::Pending));
    // Timestamped, window not yet established: `Pending` with a deadline.
    let stamped = env("on origin", "eventually[0, 5] holds(p)");
    let q = stamped.atom("p");
    let mut early = request(&stamped, vec![position(0, &[(q, Valuation::False)])]);
    early.origins = vec![origin("o1", "a")];
    early.trigger = Some(id("o1"));
    early.decision_progress = Some(2);
    early.completeness = Some(qsl_eval::temporal::evidence::CompletenessRecord {
        reference: qsl_eval::temporal::evidence::EvidenceRef::named("c"),
        state: ProgressCompleteness::Complete { through: 2 },
        facts: Vec::new(),
    });
    early.decision_closure = Some(ProgressClosure::Open);
    let result = call(&stamped, &stamped.subject(&[]), &early).expect("a result");
    assert_eq!(result.truth, Some(Truth::Pending));
}

#[trace(
    "FR-516-AC-27",
    "FR-523-AC-21",
    "FR-523-AC-15",
    "FR-516-AC-8",
    "TC-1311",
    "TC-1318"
)]
#[test]
fn silent_samples_are_charged_at_the_meter_and_to_neither_steps_nor_positions() {
    let fixed = fixed_env("on origin", "eventually[0, 3] holds(p)");
    let p = fixed.atom("p");
    let subject = fixed.subject(&[]);
    // Two observed samples; the watermark 3 makes samples 2 and 3 silent.
    let mut asked = request(
        &fixed,
        (0..2)
            .map(|i| position(i, &[(p, Valuation::False)]))
            .collect(),
    );
    asked.origins = vec![origin("o1", "a")];
    asked.trigger = Some(id("o1"));
    asked.decision_progress = Some(3);
    asked.completeness = Some(qsl_eval::temporal::evidence::CompletenessRecord {
        reference: qsl_eval::temporal::evidence::EvidenceRef::named("c"),
        state: ProgressCompleteness::Complete { through: 3 },
        facts: Vec::new(),
    });
    let mut meter = unlimited();
    let result = evaluate_temporal_evidence(&subject, &asked, &mut meter).expect("a result");
    assert_eq!(result.truth, Some(Truth::False));
    let usage = result.usage;
    // Observed: the origin (1), `eventually` at 0 (1), two position scans (2),
    // two lookups (2). Silent: two scans and two lookups, at the meter only.
    assert_eq!(usage.get(EvidenceCounter::EvaluationSteps), 6);
    assert_eq!(usage.get(EvidenceCounter::Positions), 2);
    assert_eq!(usage.get(EvidenceCounter::Valuations), 4);
    // Visited: formula nodes (2), positions (2) and valuations (4).
    assert_eq!(usage.get(EvidenceCounter::Visited), 8);
    let admitted = meter.admitted_charges().len() as u64;
    // The meter point sees every unit: the six, the silent scans (2) and the
    // silent lookups (2), plus the two silent samples' materialization.
    assert_eq!(
        admitted - usage.get(EvidenceCounter::EvaluationSteps),
        4 + 2,
        "silent scans, silent lookups and the two silent samples"
    );
    assert_eq!(usage.entries().len(), 13);
}

#[trace("FR-523-AC-3", "FR-516-AC-8", "TC-1318", "TC-1311")]
#[test]
fn an_origin_clause_over_three_positions_measures_the_stated_usage() {
    let env = env("on origin", "eventually[0, 5] holds(p)");
    let p = env.atom("p");
    let subject = env.subject(&[]);
    let mut asked = request(
        &env,
        (0..3)
            .map(|i| position(i, &[(p, Valuation::False)]))
            .collect(),
    );
    asked.origins = vec![origin("o1", "a")];
    asked.trigger = Some(id("o1"));
    let mut meter = unlimited();
    let result = evaluate_temporal_evidence(&subject, &asked, &mut meter).expect("a result");
    let expected = [
        (EvidenceCounter::FormulaNodes, 2),
        (EvidenceCounter::Instances, 1),
        (EvidenceCounter::Positions, 3),
        (EvidenceCounter::Valuations, 3),
        (EvidenceCounter::Captures, 0),
        (EvidenceCounter::EvaluationSteps, 8),
        (EvidenceCounter::Visited, 8),
        (EvidenceCounter::HistorySpan, 5),
    ];
    for (counter, value) in expected {
        assert_eq!(result.usage.get(counter), value, "{counter:?}");
    }
    // FR-501-AC-12's seven charges plus one for the origin resolved.
    assert_eq!(meter.admitted_charges().len(), 8);
}

#[trace("FR-523-AC-14", "FR-523-AC-19", "TC-1318")]
#[test]
fn a_shared_meter_one_unit_short_stops_the_second_instance_naming_it() {
    let (env, p) = each();
    let subject = env.subject(&[]);
    let mut asked = asked(&env, p);
    asked.deliveries = vec![
        delivery("t1", "r", "p", &["5"]),
        delivery("t2", "r", "p", &["5"]),
    ];
    let mut probe = unlimited();
    let first = evaluate_temporal_evidence(&subject, &asked, &mut probe).expect("a result");
    let used = probe.admitted_charges().len() as u64;
    let mut shared = Meter::new(ScalarLimits {
        work_units: used + 1,
        ..SCALAR_LIMITS_UNLIMITED
    });
    let one = evaluate_temporal_evidence(&subject, &asked, &mut shared).expect("a result");
    assert_eq!(one.truth, first.truth);
    assert_eq!(one.settlement, first.settlement);
    assert_eq!(one.support.len(), first.support.len());
    asked.trigger = Some(id("t2"));
    let two = evaluate_temporal_evidence(&subject, &asked, &mut shared).expect("a result");
    assert_eq!(two.truth, None);
    match two.incomplete {
        Some(EvidenceIncomplete::Meter { instance, .. }) => {
            assert_eq!(instance.as_deref(), Some(b"t2".as_slice()));
        }
        other => panic!("a meter stop naming t2, not {other:?}"),
    }
    // The first call replayed on a fresh meter of the same ceiling equals its
    // first result.
    asked.trigger = Some(id("t1"));
    let mut fresh = Meter::new(ScalarLimits {
        work_units: used + 1,
        ..SCALAR_LIMITS_UNLIMITED
    });
    let replay = evaluate_temporal_evidence(&subject, &asked, &mut fresh).expect("a result");
    assert_eq!(replay.result_identity, one.result_identity);
}

#[trace("FR-516-AC-14", "TC-1311")]
#[test]
fn a_sibling_clause_keeps_its_result_after_a_counter_stop() {
    let (env, p) = each();
    let sibling = self::env("on origin", "eventually[0, 2] holds(q)");
    let q = sibling.atom("q");
    let mut meter = unlimited();
    let mut stopped = asked(&env, p);
    stopped.limits = TemporalEvidenceLimits::default().with_ceiling(EvidenceCounter::Positions, 0);
    let first =
        evaluate_temporal_evidence(&env.subject(&[]), &stopped, &mut meter).expect("an Ok result");
    assert!(first.incomplete.is_some());
    let mut own = request(
        &sibling,
        (0..3)
            .map(|i| position(i, &[(q, Valuation::True)]))
            .collect(),
    );
    own.origins = vec![origin("o1", "a")];
    own.trigger = Some(id("o1"));
    let second =
        evaluate_temporal_evidence(&sibling.subject(&[]), &own, &mut meter).expect("a result");
    assert_eq!(second.truth, Some(Truth::True));
    assert!(second.incomplete.is_none());
}

#[trace("FR-516-AC-5", "TC-1311")]
#[test]
fn each_axis_is_echoed_as_supplied_and_changing_one_changes_only_its_field() {
    let (env, p) = each();
    let subject = env.subject(&[]);
    let base = asked(&env, p);
    let reference = call(&env, &subject, &base).expect("a result");
    let mut axis = base.clone();
    axis.decision_progress = Some(90);
    axis.surrounding_progress = Some(80);
    axis.surrounding_closure = Some(ProgressClosure::Open);
    axis.trigger_scope = ProgressClosure::Open;
    let changed = call(&env, &subject, &axis).expect("a result");
    assert_eq!(changed.decision_progress, Some(90));
    assert_eq!(changed.surrounding_progress, Some(80));
    assert_eq!(changed.surrounding_closure, Some(ProgressClosure::Open));
    assert_eq!(changed.trigger_scope, ProgressClosure::Open);
    // Only the fields changed: the decision closure, the completeness record
    // and the truth are the reference's.
    assert_eq!(changed.decision_closure, reference.decision_closure);
    assert_eq!(
        changed.completeness.as_ref().map(|c| c.state),
        reference.completeness.as_ref().map(|c| c.state)
    );
    assert_eq!(changed.truth, reference.truth);
    // The decision closure alone changes the settlement, not the echoes.
    let mut closure = base.clone();
    closure.decision_closure = Some(ProgressClosure::Open);
    let open = call(&env, &subject, &closure).expect("a result");
    assert_eq!(open.decision_progress, reference.decision_progress);
    assert_eq!(open.decision_closure, Some(ProgressClosure::Open));
    assert_ne!(open.settlement, reference.settlement);
}

#[trace("FR-516-AC-17", "FR-502-AC-2", "TC-1311")]
#[test]
fn a_conflicting_completeness_revision_is_refused_and_the_progress_entry_is_kept() {
    let (env, p) = each();
    let subject = env.subject(&[]);
    let mut first = asked(&env, p);
    first.progress_ledger = Some(qsl_eval::temporal::ProgressLedger::new());
    let accepted = call(&env, &subject, &first).expect("a result");
    let kept = accepted.progress_ledger.clone().expect("a progress ledger");
    let binding = {
        let mut b = subject.progress.clone();
        b.occurrence = 0;
        b
    };
    assert_eq!(kept.entry(&binding).expect("an entry").watermark(), 100);
    let mut retreat = asked(&env, p);
    retreat.progress_ledger = Some(kept.clone());
    retreat.completeness = retreat.completeness.map(|mut record| {
        record.state = ProgressCompleteness::Complete { through: 40 };
        record
    });
    let mut meter = unlimited();
    expect_refusal(
        evaluate_temporal_evidence(&subject, &retreat, &mut meter),
        "progress-contradiction",
    );
    assert!(meter.admitted_charges().is_empty());
    // The entry the refusal left is the retained one: a legal request still
    // finds watermark 100 and completeness through 100.
    let entry = kept.entry(&binding).expect("an entry");
    assert_eq!(entry.watermark(), 100);
    assert!(matches!(
        entry.completeness(),
        ProgressCompleteness::Complete { through: 100 }
    ));
}

#[trace("FR-516-AC-1", "FR-526-AC-17", "TC-1311", "TC-1321")]
#[test]
fn the_request_has_no_supplied_result_member_and_only_the_evaluator_writes_entries() {
    // An exhaustive pattern: a `truth`, `settlement`, `support` or `instance`
    // member added to the request stops this from compiling.
    let (env, p) = each();
    let EvidenceRequest {
        trigger: _,
        correspondence: _,
        positions: _,
        anchor: _,
        deliveries: _,
        origins: _,
        trigger_evidence: _,
        trigger_scope: _,
        decision_progress: _,
        decision_closure: _,
        surrounding_progress: _,
        surrounding_closure: _,
        native_valuations: _,
        execution: _,
        completeness: _,
        authoritative_origin: _,
        evicted: _,
        cover: _,
        progress_ledger: _,
        ledger: _,
        relation: _,
        control: _,
        binding: _,
        occurrence: _,
        limits: _,
    } = asked(&env, p);
    // A ledger starts empty and holds a result entry only after the evaluator
    // wrote it.
    let empty = EvidenceLedger::new();
    assert_eq!((empty.instance_count(), empty.result_count()), (0, 0));
    let result = call(&env, &env.subject(&[]), &asked(&env, p)).expect("a result");
    assert_eq!(
        (result.ledger.instance_count(), result.ledger.result_count()),
        (1, 1)
    );
}

#[trace("FR-522-AC-24", "FR-522-AC-25", "FR-522-AC-26", "TC-1317")]
#[test]
fn every_evidence_member_changes_the_result_identity() {
    let (env, p) = each();
    let subject = env.subject(&[]);
    let base = asked(&env, p);
    let identity = |edit: &dyn Fn(&mut EvidenceRequest)| {
        let mut one = base.clone();
        edit(&mut one);
        call(&env, &subject, &one)
            .expect("a result")
            .result_identity
    };
    let reference = call(&env, &subject, &base)
        .expect("a result")
        .result_identity;
    let edits: Vec<(&str, RequestEdit)> = vec![
        (
            "identity byte",
            Box::new(|r| {
                r.deliveries[0].identity = id("t\u{3}");
                r.trigger = Some(id("t\u{3}"));
            }),
        ),
        (
            "receipt",
            Box::new(|r| r.deliveries[0].receipt = Receipt::new(b"other".as_slice())),
        ),
        (
            "anchor",
            Box::new(|r| r.deliveries[0].anchor = "obs1".to_owned()),
        ),
        (
            "payload",
            Box::new(|r| r.deliveries[0].payload = Payload::new(b"other".as_slice())),
        ),
        ("guard", Box::new(|r| r.deliveries[0].guard = None)),
        (
            "capture",
            Box::new(|r| r.deliveries[0].captures[0] = CaptureFact::Null),
        ),
        (
            "evicted",
            Box::new(|r| {
                r.evicted = vec![Evicted::Capture {
                    instance: id("t1"),
                    capture: "amount".to_owned(),
                }];
            }),
        ),
        (
            "anchor member",
            Box::new(|r| r.anchor = Some("obs1".to_owned())),
        ),
        (
            "identity",
            Box::new(|r| r.correspondence.identity = "x".to_owned()),
        ),
        (
            "authority_identity",
            Box::new(|r| r.correspondence.authority_identity = "x".to_owned()),
        ),
        (
            "authority_revision",
            Box::new(|r| r.correspondence.authority_revision = "x".to_owned()),
        ),
        (
            "scope",
            Box::new(|r| r.correspondence.scope = "x".to_owned()),
        ),
        (
            "population",
            Box::new(|r| r.correspondence.population = "x".to_owned()),
        ),
        (
            "contract",
            Box::new(|r| r.correspondence.contract = "x".to_owned()),
        ),
        (
            "schema_digest",
            Box::new(|r| r.correspondence.schema_digest = vec![1]),
        ),
        ("digest", Box::new(|r| r.correspondence.digest = vec![1])),
        (
            "authority_digest",
            Box::new(|r| r.correspondence.authority_digest = vec![1]),
        ),
    ];
    for (name, edit) in edits {
        assert_ne!(identity(&*edit), reference, "{name}");
    }
    // The same deliveries in another order.
    let mut two = base.clone();
    two.deliveries = vec![
        delivery("t1", "r", "payload", &["5"]),
        delivery("t2", "r", "payload", &["5"]),
    ];
    let mut swapped = two.clone();
    swapped.deliveries.reverse();
    assert_ne!(
        call(&env, &subject, &two)
            .expect("a result")
            .result_identity,
        call(&env, &subject, &swapped)
            .expect("a result")
            .result_identity
    );
    // Origins.
    let origins = self::env("on origin", "eventually[0, 2] holds(p)");
    let op = origins.atom("p");
    let mut o = request(
        &origins,
        (0..3)
            .map(|i| position(i, &[(op, Valuation::True)]))
            .collect(),
    );
    o.origins = vec![origin("o1", "a")];
    o.trigger = Some(id("o1"));
    let reference = call(&origins, &origins.subject(&[]), &o)
        .expect("a result")
        .result_identity;
    for edit in 0..2 {
        let mut other = o.clone();
        if edit == 0 {
            other.origins[0].payload = Payload::new(b"b".as_slice());
        } else {
            other.origins[0].identity = id("o2");
            other.trigger = Some(id("o2"));
        }
        assert_ne!(
            call(&origins, &origins.subject(&[]), &other)
                .expect("a result")
                .result_identity,
            reference
        );
    }
}

#[trace("FR-516-AC-32", "FR-516-AC-33", "TC-1311")]
#[test]
fn twins_pair_their_clock_rows_by_key_and_an_absent_or_unknown_selector_is_refused() {
    use qsl_eval::temporal::evidence::TraceBinding;
    let base = each().0;
    let row = |ordinal: u64, unit: &str| -> qsl_package::TemporalBinding {
        serde_json::from_value(json!({
            "declaration": {"package": "recipe", "path": ["Due"], "ordinal": 0},
            "occurrence": ordinal,
            "clock_name": "stamp-clock",
            "clock": {"kind": "timestamped_event", "timestamp_unit": unit},
        }))
        .expect("a row")
    };
    // The lock lists the rows ascending (3, 7); the node lists its
    // occurrences 7 then 3.
    let rows = [row(3, "ms"), row(7, "us")];
    let mut subject = base.subject(&[]);
    subject.occurrences = vec![
        SubjectOccurrence {
            occurrence: claim(7),
            clock: Some(&rows[1]),
        },
        SubjectOccurrence {
            occurrence: claim(3),
            clock: Some(&rows[0]),
        },
    ];
    let binding = |unit: &str| TraceBinding {
        profile: base.binding.profile.clone(),
        clock_name: base.binding.clock_name.clone(),
        parameters: serde_json::from_value(
            json!({"kind": "timestamped_event", "timestamp_unit": unit}),
        )
        .expect("a clock"),
    };
    let p = base.atom("p");
    let mut one = asked(&base, p);
    one.occurrence = Some(claim(3));
    one.binding = binding("ms");
    let ok = evaluate_temporal_evidence(&subject, &one, &mut unlimited()).expect("ordinal 3");
    assert_eq!(ok.truth, Some(Truth::True));
    // Position pairing would read the 7-row first and accept this.
    one.binding = binding("us");
    expect_refusal(
        evaluate_temporal_evidence(&subject, &one, &mut unlimited()),
        "binding-mismatch",
    );
    one.occurrence = Some(claim(7));
    assert!(evaluate_temporal_evidence(&subject, &one, &mut unlimited()).is_ok());
    one.binding = binding("ms");
    expect_refusal(
        evaluate_temporal_evidence(&subject, &one, &mut unlimited()),
        "binding-mismatch",
    );
    // The selector is required for twins and must name one.
    one.occurrence = None;
    let mut meter = unlimited();
    expect_refusal(
        evaluate_temporal_evidence(&subject, &one, &mut meter),
        "occurrence-required",
    );
    assert!(meter.admitted_charges().is_empty());
    one.occurrence = Some(claim(5));
    let mut meter = unlimited();
    expect_refusal(
        evaluate_temporal_evidence(&subject, &one, &mut meter),
        "occurrence-unknown",
    );
    assert!(meter.admitted_charges().is_empty());
}

#[trace("FR-526-AC-12", "TC-1321")]
#[test]
fn a_lineage_stop_is_an_ok_result_with_the_requests_ledger_and_the_ceiling_completes() {
    let (env, p) = each();
    let subject = env.subject(&[]);
    let at = |value: Valuation,
              previous: Option<&qsl_eval::temporal::evidence::EvidenceResult>,
              ceiling: u64| {
        let mut one = request(
            &env,
            vec![
                position(0, &[(p, value)]),
                position(1, &[(p, Valuation::False)]),
            ],
        );
        one.deliveries = vec![delivery("t1", "r", "payload", &["5"])];
        one.trigger = Some(id("t1"));
        if let Some(earlier) = previous {
            one.relation = Relation::Superseding(earlier.result_identity.clone());
            one.ledger = Some(earlier.ledger.clone());
        }
        one.limits =
            TemporalEvidenceLimits::default().with_ceiling(EvidenceCounter::Lineage, ceiling);
        one
    };
    let values = [
        Valuation::True,
        Valuation::False,
        Valuation::Missing,
        Valuation::True,
    ];
    let mut chain: Vec<qsl_eval::temporal::evidence::EvidenceResult> = Vec::new();
    for (index, value) in values.iter().take(3).enumerate() {
        let next = call(&env, &subject, &at(value.clone(), chain.last(), 3)).expect("a result");
        assert_eq!(next.lineage, index as u64);
        chain.push(next);
    }
    let request_ledger = chain[2].ledger.clone();
    let stopped =
        call(&env, &subject, &at(values[3].clone(), chain.last(), 2)).expect("an Ok result");
    assert_eq!(stopped.activation, ActivationState::Unavailable);
    assert_eq!(stopped.truth, None);
    assert_eq!(stopped.lineage, 3);
    assert_eq!(stopped.ledger, request_ledger);
    let latest = stopped
        .ledger
        .result(&env.obligation(), Some(&id("t1")))
        .expect("latest");
    assert_eq!(
        latest.lineage(),
        2,
        "no entry was recorded for the stopped call"
    );
    match stopped.incomplete {
        Some(EvidenceIncomplete::Limit(reached)) => {
            assert_eq!(reached.counter, EvidenceCounter::Lineage);
        }
        other => panic!("a lineage stop, not {other:?}"),
    }
    let done = call(&env, &subject, &at(values[3].clone(), chain.last(), 3)).expect("a result");
    assert!(done.incomplete.is_none());
    assert!(done.truth.is_some());
    assert_eq!(done.lineage, 3);
    let entry = done
        .ledger
        .result(&env.obligation(), Some(&id("t1")))
        .expect("recorded");
    assert_eq!(entry.lineage(), 3);
}

#[trace("FR-516-AC-3", "TC-1311")]
#[test]
fn a_withdrawn_completeness_record_or_valuation_is_a_missing_non_value_beside_the_truth() {
    let (env, p) = each();
    let subject = env.subject(&[]);
    let reference = call(&env, &subject, &asked(&env, p)).expect("a result");
    assert_eq!(reference.truth, Some(Truth::True));
    let mut no_record = asked(&env, p);
    no_record.completeness = None;
    let result = call(&env, &subject, &no_record).expect("a result");
    assert_eq!(result.activation, ActivationState::Active);
    assert_eq!(result.truth, Some(Truth::True));
    assert_eq!(result.support.len(), reference.support.len());
    let named = result.non_value.expect("a non-value");
    assert_eq!(
        (named.kind, named.dimension.as_deref()),
        (NonValueKind::Missing, Some("completeness"))
    );
    assert_ne!(
        result.settlement,
        Some(qsl_eval::temporal::evidence::Settlement::ClosedScope)
    );
    // A support fact: the valuation at a position the verdict does not rest
    // on is withdrawn, the verdict is the remaining evidence's.
    let mut positions: Vec<_> = (0..3)
        .map(|i| position(i, &[(p, Valuation::True)]))
        .collect();
    positions[1].valuations.clear();
    let mut gap = request(&env, positions);
    gap.deliveries = vec![delivery("t1", "r", "payload", &["5"])];
    gap.trigger = Some(id("t1"));
    let result = call(&env, &subject, &gap).expect("a result");
    assert_eq!(result.truth, Some(Truth::True));
    let named = result.non_value.expect("a non-value");
    assert_eq!(named.kind, NonValueKind::Missing);
    assert_eq!(named.dimension, Some(format!("valuation:{p}@1")));
    let _ = (Indeterminate::IncompleteInput, CallFailure::Cancelled);
}

#[trace("FR-516-AC-9", "FR-516-AC-1", "TC-1311")]
#[test]
fn every_opaque_evidence_member_is_echoed_byte_for_byte_and_decides_nothing() {
    let (env, p) = each();
    let subject = env.subject(&[]);
    let base = asked(&env, p);
    // An open decision scope with a decisive witness gives a support.
    let mut base = base;
    base.decision_closure = Some(ProgressClosure::Open);
    // A unique witness at position 1, so a single valuation supports it.
    base.positions = (0..3)
        .map(|i| {
            let value = if i == 1 {
                Valuation::True
            } else {
                Valuation::False
            };
            position(i, &[(p, value)])
        })
        .collect();
    let reference = call(&env, &subject, &base).expect("a result");
    let mut changed = base.clone();
    let opaque = |r: &mut qsl_eval::temporal::evidence::EvidenceRef| {
        r.contract = "contract-x".to_owned();
        r.schema_digest = vec![1, 2, 3];
        r.digest = vec![4, 5];
        r.authority_digest = vec![6];
    };
    opaque(&mut changed.correspondence);
    for position in &mut changed.positions {
        opaque(&mut position.observation);
    }
    let result = call(&env, &subject, &changed).expect("a result");
    assert_eq!(
        (result.truth, result.settlement, result.activation),
        (reference.truth, reference.settlement, reference.activation)
    );
    for echoed in std::iter::once(&result.correspondence)
        .chain(result.support.iter().map(|support| &support.observation))
    {
        assert_eq!(echoed.contract, "contract-x");
        assert_eq!(echoed.schema_digest, vec![1, 2, 3]);
        assert_eq!(echoed.digest, vec![4, 5]);
        assert_eq!(echoed.authority_digest, vec![6]);
    }
    assert!(!result.support.is_empty());
}

#[trace("FR-524-AC-24", "FR-524-AC-25", "TC-1319")]
#[test]
fn the_evicted_valuation_names_its_instance_and_atom_and_other_records_change_nothing() {
    let (env, p) = each();
    let subject = env.subject(&[p]);
    let mut positions: Vec<_> = (0..3).map(|i| position(i, &[])).collect();
    for position in &mut positions {
        position
            .instance_valuations
            .insert(id("t1"), [(p, Valuation::True)].into_iter().collect());
    }
    let mut one = request(&env, positions);
    one.deliveries = vec![delivery("t1", "r", "payload", &["5"])];
    one.trigger = Some(id("t1"));
    let clean = call(&env, &subject, &one).expect("a result");
    assert_eq!(clean.truth, Some(Truth::True));
    // Another atom, another instance, another position: no effect.
    for other in [
        Evicted::Valuation {
            instance: Some(id("t1")),
            position: 0,
            atom: p + 50,
        },
        Evicted::Valuation {
            instance: Some(id("t2")),
            position: 0,
            atom: p,
        },
        Evicted::Valuation {
            instance: Some(id("t1")),
            position: 9,
            atom: p,
        },
    ] {
        let mut miss = one.clone();
        miss.evicted = vec![other];
        let result = call(&env, &subject, &miss).expect("a result");
        assert_eq!(result.truth, Some(Truth::True));
        assert!(result.incomplete.is_none());
    }
    let mut hit = one.clone();
    hit.evicted = vec![Evicted::Valuation {
        instance: Some(id("t1")),
        position: 0,
        atom: p,
    }];
    let result = call(&env, &subject, &hit).expect("a result");
    assert_eq!(result.truth, None);
    match result.incomplete {
        Some(EvidenceIncomplete::EvictedValuation {
            instance,
            position,
            atom,
        }) => {
            assert_eq!((instance, position, atom), (Some(id("t1")), 0, p));
        }
        other => panic!("the evicted subject, not {other:?}"),
    }
}

#[trace("FR-523-AC-6", "FR-523-AC-3", "TC-1318")]
#[test]
fn a_timed_interval_has_no_tick_reach_and_a_finite_window_interval_is_one() {
    use qsl_eval::temporal::evidence::{ClauseWork, ReachFault};
    let reach = |body: &str| {
        let fixture = crate::timed_trace::timed(body);
        ClauseWork::of(fixture.clause.formula())
    };
    // A timed interval counts time, closed and integral or not, and has no
    // tick reach (FR-326: only the finite-window profile counts ticks).
    for body in [
        "eventually[1 ms, 5 ms] holds(c.level = 2)",
        "eventually[0 ms, 3 ms) holds(c.level = 2)",
        "eventually(0 ms, 3 ms] holds(c.level = 2)",
    ] {
        assert_eq!(reach(body), Err(ReachFault::Timed), "{body}");
    }
    // A finite-window clause measures ticks.
    let window = env("on origin", "eventually[2, 5] holds(p)");
    let work = ClauseWork::of(window.fixture.clause.formula()).expect("a tick reach");
    assert_eq!(
        (work.history_span, work.future_reach, work.formula_nodes),
        (5, 5, 2)
    );
}

#[trace("FR-516-AC-2", "TC-1311")]
#[test]
fn a_clause_with_no_tick_reach_is_refused_before_anything_is_charged() {
    let fixture = crate::timed_trace::timed("eventually[0 ms, 3 ms) holds(c.level = 2)");
    let env = crate::temporal_activation::rows_for(
        crate::temporal_trace::Fixture {
            text: String::new(),
            clause: fixture.clause,
        },
        "quire.temporal.timestamped-event.finite-window/v1",
        json!({"kind": "timestamped_event", "timestamp_unit": "ms"}),
    );
    let mut asked = request(&env, vec![position(0, &[])]);
    asked.origins = vec![origin("o1", "a")];
    asked.trigger = Some(id("o1"));
    let mut meter = unlimited();
    expect_refusal(
        evaluate_temporal_evidence(&env.subject(&[]), &asked, &mut meter),
        "clause-unreadable",
    );
    assert!(meter.admitted_charges().is_empty());
}

fn exact_silent_request(env: &Env, p: u64) -> EvidenceRequest {
    let mut asked = request(env, vec![position(0, &[(p, Valuation::False)])]);
    asked.origins = vec![origin("o1", "a")];
    asked.trigger = Some(id("o1"));
    asked.decision_progress = Some(3);
    asked.completeness = Some(qsl_eval::temporal::evidence::CompletenessRecord {
        reference: qsl_eval::temporal::evidence::EvidenceRef::named("c"),
        state: ProgressCompleteness::Complete { through: 3 },
        facts: Vec::new(),
    });
    asked
}

/// The spec vector of FR-523-AC-21: a fixed-sample clause `on origin`,
/// `eventually[0, 3] holds(p)`, one observed position 0 with `p` false,
/// watermark 3 and `Complete { through: 3 }` over a closed scope.
#[trace("FR-516-AC-27", "FR-523-AC-21", "TC-1311", "TC-1318")]
#[test]
fn the_spec_vector_of_silent_samples_measures_its_exact_usage() {
    let fixed = fixed_env("on origin", "eventually[0, 3] holds(p)");
    let p = fixed.atom("p");
    let mut meter = unlimited();
    let result = evaluate_temporal_evidence(
        &fixed.subject(&[]),
        &exact_silent_request(&fixed, p),
        &mut meter,
    )
    .expect("a result");
    assert_eq!(result.truth, Some(Truth::False));
    let usage = result.usage;
    assert_eq!(usage.get(EvidenceCounter::FormulaNodes), 2);
    assert_eq!(usage.get(EvidenceCounter::Positions), 1);
    assert_eq!(usage.get(EvidenceCounter::Valuations), 4);
    assert_eq!(usage.get(EvidenceCounter::EvaluationSteps), 4);
    assert_eq!(usage.get(EvidenceCounter::Visited), 7);
    // The meter at `temporal.visit`: the four evaluated charges, six
    // charges at the three silent samples (a scan, a node visit and a lookup
    // each), and three materialization charges, one per silent sample, that
    // `admit_progressed` makes before any position is evaluated.
    assert_eq!(meter.admitted_charges().len(), 13);
}

/// FR-043-AC-1 over both false-extension profiles (event-position and
/// fixed-sample): with the scope closed and complete through 3, a `holds` atom
/// that is false is false past closure bare, grouped and under a connective,
/// settled by the closed scope, while a temporal constant keeps its authored
/// value in the same three forms: the constant and the atom are not folded.
#[trace("FR-327-AC-8", "FR-523-AC-21", "FR-043-AC-1", "TC-1311")]
#[test]
fn a_closed_scope_separates_an_atom_from_a_constant_in_both_false_extension_profiles() {
    use qsl_eval::temporal::evidence::Settlement;
    type Build = fn(&str, &str) -> Env;
    let profiles: [(&str, Build); 2] = [("event-position", event_env), ("fixed-sample", fixed_env)];
    for (profile, build) in profiles {
        for (body, atom_form) in [
            ("always[0, 3] holds(p)", true),
            ("always[0, 3] (holds(p))", true),
            ("true and always[0, 3] holds(p)", true),
            ("always[0, 3] true", false),
            ("always[0, 3] (true)", false),
            ("true and always[0, 3] true", false),
        ] {
            let env = build("on origin", body);
            let p = if atom_form { env.atom("p") } else { 0 };
            let mut asked = exact_silent_request(&env, p);
            if !atom_form {
                asked.positions[0].valuations.clear();
            }
            let result = call(&env, &env.subject(&[]), &asked)
                .unwrap_or_else(|_| panic!("{profile} {body}: a result"));
            let expected = if atom_form { Truth::False } else { Truth::True };
            assert_eq!(result.truth, Some(expected), "{profile} {body}");
            assert_eq!(
                result.settlement,
                Some(Settlement::ClosedScope),
                "{profile} {body}: settled by the closed scope"
            );
        }
    }
}

/// NFR-008-AC-1, sweep: over the declared population of eight operators, three
/// profiles, nesting depths one through four and four interval bound pairs
/// (384 authored clauses), no case wraps, saturates or narrows: every call is an
/// `Ok` result that either carries a truth or reports a typed limit stop. The
/// sweep asserts no per-case value, because which composition a nested formula
/// performs depends on the offsets its trace reaches.
#[trace("FR-523-AC-16", "NFR-008-AC-1", "TC-1318")]
#[test]
fn the_declared_population_never_wraps_or_saturates() {
    const OPERATORS: [(&str, &str); 8] = [
        ("eventually", "eventually{i} {a}"),
        ("always", "always{i} {a}"),
        ("once", "once{i} {a}"),
        ("historically", "historically{i} {a}"),
        ("until", "({a} until{i} {a})"),
        ("release", "({a} release{i} {a})"),
        ("since", "({a} since{i} {a})"),
        ("triggered", "({a} triggered{i} {a})"),
    ];
    let bounds = [
        (0, i64::MAX),
        (1, i64::MAX),
        (i64::MAX - 1, i64::MAX),
        (0, i64::MAX / 2),
    ];
    let (mut cases, mut valued, mut stopped) = (0usize, 0usize, 0usize);
    for (name, template) in OPERATORS {
        for profile in ["event-position", "fixed-sample", "finite-window"] {
            for depth in 1..=4usize {
                for (lower, upper) in bounds {
                    cases += 1;
                    let interval = format!("[{lower}, {upper}]");
                    let mut body = "holds(p)".to_owned();
                    for _ in 0..depth {
                        body = template.replace("{i}", &interval).replace("{a}", &body);
                    }
                    let label = format!("{name} depth {depth} {interval} under {profile}");
                    // A limit reached by the S3 check is a typed stop before any
                    // position is visited.
                    let Ok(env) = try_env_under(profile, "on origin", &body) else {
                        stopped += 1;
                        continue;
                    };
                    let p = env.atom("p");
                    let mut asked = exact_silent_request(&env, p);
                    asked.positions[0].valuations.insert(p, Valuation::True);
                    match call(&env, &env.subject(&[]), &asked) {
                        Ok(result) if result.truth.is_some() => valued += 1,
                        Ok(result) => {
                            assert!(
                                matches!(
                                    result.incomplete,
                                    Some(EvidenceIncomplete::Limit(_))
                                        | Some(EvidenceIncomplete::Meter { .. })
                                ) || result.non_value.is_some()
                                    || result.indeterminate.is_some(),
                                "{label}: neither a truth nor a typed stop: {result:?}"
                            );
                            stopped += 1;
                        }
                        Err(_) => panic!("{label}: a refusal, not a result"),
                    }
                }
            }
        }
    }
    assert_eq!(cases, 8 * 3 * 4 * 4, "the declared population is 384 cases");
    assert!(
        valued > 0 && stopped > 0,
        "the population reaches both the valued and the stopped regime: {valued} / {stopped}"
    );
}

#[trace("FR-516-AC-27", "FR-516-AC-44", "TC-1311")]
#[test]
fn a_support_pair_at_a_silent_sample_names_no_observed_fact_and_is_skipped() {
    // `eventually[0, 3] holds(p)` false everywhere: flipping any one of the
    // four atoms changes the verdict, so the decision support has four pairs,
    // one at the observation and three at silent samples; only the first
    // names an observed fact.
    let fixed = fixed_env("on origin", "eventually[0, 3] holds(p)");
    let p = fixed.atom("p");
    let asked = exact_silent_request(&fixed, p);
    let result = call(&fixed, &fixed.subject(&[]), &asked).expect("a result, not a fault");
    assert_eq!(result.truth, Some(Truth::False));
    assert_eq!(result.support.len(), 1);
    assert_eq!(result.support[0].observation.identity, "obs0");
    assert_eq!(result.support[0].atom, p);
}

#[trace("FR-516-AC-3", "FR-516-AC-43", "TC-1311")]
#[test]
fn a_withdrawn_valuation_names_its_atom_and_position_in_one_exact_text() {
    let (env, p) = each();
    let mut positions: Vec<_> = (0..3)
        .map(|i| position(i, &[(p, Valuation::True)]))
        .collect();
    positions[1].valuations.clear();
    let mut gap = request(&env, positions);
    gap.deliveries = vec![delivery("t1", "r", "payload", &["5"])];
    gap.trigger = Some(id("t1"));
    let result = call(&env, &env.subject(&[]), &gap).expect("a result");
    let named = result.non_value.expect("a non-value");
    assert_eq!(named.code, "fact-withdrawn");
    assert_eq!(named.dimension.as_deref(), Some("valuation:1@1"));
    // The first absent valuation the evaluator read, whether or not the
    // decision rests on it: only position 2 is absent here.
    let mut late: Vec<_> = (0..3)
        .map(|i| position(i, &[(p, Valuation::True)]))
        .collect();
    late[2].valuations.clear();
    let mut later = request(&env, late);
    later.deliveries = vec![delivery("t1", "r", "payload", &["5"])];
    later.trigger = Some(id("t1"));
    let result = call(&env, &env.subject(&[]), &later).expect("a result");
    assert_eq!(
        result.non_value.expect("a non-value").dimension.as_deref(),
        Some("valuation:1@2")
    );
}

#[trace("FR-516-AC-42", "TC-1311")]
#[test]
fn a_refusal_of_the_evaluation_after_charges_is_an_ok_active_refused_result() {
    // Two observations of one stamp with no usable order.
    let tied = |body: &str, reads_q: bool, keyed: bool| {
        let env = env("on origin", body);
        let p = env.atom("p");
        let q = if reads_q { env.atom("q") } else { p };
        let mut positions = vec![
            position(
                0,
                &[
                    (p, Valuation::True),
                    (
                        q,
                        if reads_q {
                            Valuation::False
                        } else {
                            Valuation::True
                        },
                    ),
                ],
            ),
            {
                // Keyed, the pair reads alike (one instant, no conflict).
                let second = if keyed {
                    Valuation::False
                } else {
                    Valuation::True
                };
                let mut tie = position(0, &[(p, Valuation::True), (q, second)]);
                tie.observation = qsl_eval::temporal::evidence::EvidenceRef::named("obs0b");
                tie
            },
        ];
        positions.truncate(2);
        if keyed {
            for (index, one) in positions.iter_mut().enumerate() {
                one.order_key = Some(qsl_eval::temporal::OrderKey {
                    authority: "seq".to_owned(),
                    key: index as i64,
                });
            }
        }
        let mut asked = request(&env, positions);
        asked.origins = vec![origin("o1", "a")];
        asked.trigger = Some(id("o1"));
        call(&env, &env.subject(&[]), &asked).expect("an Ok result")
    };
    let result = tied("holds(p) until[0, 5] holds(q)", true, false);
    assert_eq!(result.activation, ActivationState::Active);
    assert_eq!(result.truth, None);
    assert_eq!(result.settlement, None);
    let named = result.non_value.expect("a non-value");
    assert_eq!(named.kind, NonValueKind::Refused);
    assert_eq!(named.code, "temporal-refusal");
    assert_eq!(named.dimension.as_deref(), Some("unordered-tie"));
    assert_eq!(named.detail, [("position".to_owned(), "1".to_owned())]);
    // The same tied pair under `eventually` has no order-sensitive operator.
    assert!(tied("eventually[0, 1] holds(p)", false, false)
        .truth
        .is_some());
    // Distinct order keys of one authority order the pair: a truth.
    let ordered = tied("holds(p) until[0, 5] holds(q)", true, true);
    assert!(ordered.truth.is_some());
    assert!(
        ordered.non_value.is_none()
            || ordered.non_value.as_ref().map(|n| n.code) != Some("temporal-refusal")
    );
}

fn silent_request(env: &Env, body_atom: u64) -> EvidenceRequest {
    exact_silent_request(env, body_atom)
}

#[trace("FR-523-AC-24", "FR-516-AC-27", "TC-1318", "TC-1311")]
#[test]
fn the_reach_of_the_clause_bounds_the_silent_samples_that_are_materialized() {
    // `eventually[0, 1]` reaches one sample past the observation, so one
    // silent sample is materialized although the watermark is 3.
    let fixed = fixed_env("on origin", "eventually[0, 1] holds(p)");
    let p = fixed.atom("p");
    let mut meter = unlimited();
    let result =
        evaluate_temporal_evidence(&fixed.subject(&[]), &silent_request(&fixed, p), &mut meter)
            .expect("a result");
    let usage = result.usage;
    assert_eq!(usage.get(EvidenceCounter::EvaluationSteps), 4);
    assert_eq!(usage.get(EvidenceCounter::Positions), 1);
    assert_eq!(usage.get(EvidenceCounter::Valuations), 2);
    assert_eq!(usage.get(EvidenceCounter::Visited), 5);
    assert_eq!(meter.admitted_charges().len(), 7);
}

#[trace("FR-523-AC-10", "FR-516-AC-27", "TC-1318", "TC-1311")]
#[test]
fn a_meter_ceiling_of_nine_stops_at_the_tenth_charge_the_observed_lookup() {
    let fixed = fixed_env("on origin", "eventually[0, 3] holds(p)");
    let p = fixed.atom("p");
    let subject = fixed.subject(&[]);
    let asked = silent_request(&fixed, p);
    let mut full = unlimited();
    evaluate_temporal_evidence(&subject, &asked, &mut full).expect("a result");
    assert_eq!(full.admitted_charges().len(), 13);
    let run = |work_units: u64| {
        let mut meter = Meter::new(ScalarLimits {
            work_units,
            ..SCALAR_LIMITS_UNLIMITED
        });
        let result =
            evaluate_temporal_evidence(&subject, &asked, &mut meter).expect("an Ok result");
        (result, meter)
    };
    let (nine, nine_meter) = run(9);
    assert!(matches!(
        nine.incomplete,
        Some(EvidenceIncomplete::Meter { .. })
    ));
    // The charge log of the stopped call is the successful prefix of the
    // full call's log, in order.
    assert_eq!(nine_meter.admitted_charges(), &full.admitted_charges()[..9]);
    // Three materializations, the origin, `eventually` at 0, the scan at the
    // observed position and the scans at the three silent samples are the
    // nine; the tenth is the lookup at the observed position 0.
    assert_eq!(nine.usage.get(EvidenceCounter::EvaluationSteps), 3);
    assert_eq!(nine.usage.get(EvidenceCounter::Positions), 1);
    assert_eq!(nine.usage.get(EvidenceCounter::Valuations), 0);
    let (ten, ten_meter) = run(10);
    assert!(matches!(
        ten.incomplete,
        Some(EvidenceIncomplete::Meter { .. })
    ));
    assert_eq!(ten_meter.admitted_charges(), &full.admitted_charges()[..10]);
    // The tenth charge spent `valuations` and `evaluation_steps`: a lookup
    // at an observed position, not at a silent sample.
    assert_eq!(ten.usage.get(EvidenceCounter::Valuations), 1);
    assert_eq!(ten.usage.get(EvidenceCounter::EvaluationSteps), 4);
}

#[trace("FR-516-AC-38", "FR-502-AC-8", "TC-1311", "TC-1296")]
#[test]
fn the_covered_positions_limit_is_an_ok_incomplete_result_with_the_requests_ledgers() {
    let (env, p) = each();
    let mut subject = env.subject(&[]);
    let mut asked = asked(&env, p);
    asked.progress_ledger = Some(qsl_eval::temporal::ProgressLedger::new());
    // The system setting is 2 positions; the request covers 3.
    subject.progress_capacity = 2;
    let mut meter = unlimited();
    let result = evaluate_temporal_evidence(&subject, &asked, &mut meter).expect("an Ok result");
    assert_eq!(result.activation, ActivationState::Unavailable);
    assert_eq!(result.truth, None);
    match result.incomplete {
        Some(EvidenceIncomplete::CoveredPositions(limit)) => {
            assert_eq!(limit.limit, "node count");
            assert_eq!(limit.value, 2);
            assert_eq!(limit.setting, "progress.covered_positions");
        }
        other => panic!("a covered-positions stop, not {other:?}"),
    }
    assert_eq!(result.progress_ledger, asked.progress_ledger);
    assert_eq!(result.ledger, EvidenceLedger::new());
    assert!(meter.admitted_charges().is_empty());
    // Exactly at the setting the call completes and retains the positions.
    subject.progress_capacity = 3;
    let done = evaluate_temporal_evidence(&subject, &asked, &mut unlimited()).expect("a result");
    assert_eq!(done.truth, Some(Truth::True));
    assert_eq!(
        done.progress_ledger.as_ref().map(|ledger| ledger.len()),
        Some(1)
    );
}
