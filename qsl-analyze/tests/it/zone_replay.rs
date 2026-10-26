// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-237 (TC-692, TC-710): the replay of a counterexample over a timed
//! subject, through `replay_timed_counterexample`, and FR-235 (TC-690,
//! TC-710): the settlement of EN-6's outcome as one terminal record, over
//! the timed fixtures of `crate::zone_search`.
//!
//! The replay is driven directly over the test world's timed subject. The
//! end-to-end path through `replay_model_trace`, which recompiles the unit,
//! waits for S4 to write a timed model's members (FR-230, QSL-580).

use ix_trace_rs::trace;
use qsl_analyze::zone_check::{
    settle_zone_check, ZoneCertificateCheck, ZoneCheckLimits, ZoneCheckOutcome, ZoneCheckRecord,
    ZoneItem, ZoneStats,
};
use qsl_eval::simulation::model::{TimedState, TimedSystem};
use qsl_eval::simulation::Limits;
use qsl_eval::temporal::UndefinedCause;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::digest::DigestRecord;
use qsl_replay::{
    CertificateLocus, CertificateRule, ClosedFormAnalysis, CounterexampleKind, DisagreementCause,
    IncompleteCause, InconclusiveCause, Lasso, ModelCheckLimit, ModelStep, ModelTraceRefusal,
    ModelTraceReplayResult, ModelTraceSettlement, ModelTraceValue, ProofBasis, ReplayRefusal,
    ResultLabel, SettlementBasis, TemporalCounterexample, TerminalValue,
};
use qsl_semantics::family::UnsupportedItem;
use quire_exact::{Cancel, Rational, Undefined};
use quire_semantic_value::catalog::Category;

use crate::timed_world::{document, world, World, DENSE};
use crate::zone_search::{
    claim, delays, exact, lock, operations, rpc, rpc_variant, ticker, violated, windowed, Outcome,
    INFINITE, TIMED,
};

/// Replay `payload` over `world`'s timed subject: as its first clause's
/// counterexample, or as a derived item's when `derived`.
fn replay_on(
    world: &World,
    payload: &TemporalCounterexample,
    derived: bool,
    exploration: &Limits,
) -> Result<ModelTraceSettlement, ReplayRefusal> {
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let node = &world.package.graph().temporal_clauses()[0];
    let letters = system.clause_letters(node).expect("letters");
    qsl_replay::replay_timed_counterexample(
        &timed,
        payload,
        (!derived).then(|| node.clause()),
        &letters,
        system.meter_limits(),
        exploration,
    )
}

fn replay(
    world: &World,
    payload: &TemporalCounterexample,
) -> Result<ModelTraceSettlement, ReplayRefusal> {
    replay_on(world, payload, false, &Limits::default())
}

fn reproduced(settlement: &ModelTraceSettlement) -> bool {
    matches!(settlement, ModelTraceSettlement::Reproduced { .. })
}

fn parity(settlement: &ModelTraceSettlement) -> bool {
    matches!(settlement, ModelTraceSettlement::Inconclusive(_))
}

fn refusal(result: Result<ModelTraceSettlement, ReplayRefusal>) -> (ModelTraceRefusal, Code) {
    match result {
        Err(ReplayRefusal::ModelTrace(refusal)) => {
            let code = ReplayRefusal::ModelTrace(refusal.clone()).code();
            (*refusal, code)
        }
        other => panic!("expected a model-trace refusal, got {other:?}"),
    }
}

/// The steps `moves` take from `world`'s initial state, each after its
/// delay, by the first step of the named operation enabled then.
fn walk(world: &World, moves: &[(&str, Rational)]) -> Vec<ModelStep> {
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let mut state: TimedState = timed.initial().remove(0);
    let mut steps = Vec::new();
    for (operation, delay) in moves {
        state = timed
            .delay(&state, delay)
            .expect("decided")
            .expect("an admissible delay");
        let (step, next) = timed
            .steps(&state)
            .expect("decided")
            .into_iter()
            .find(|(step, _)| step.operation().contains(&format!("Call/{operation}\"")))
            .unwrap_or_else(|| panic!("`{operation}` is enabled"));
        steps.push(ModelStep {
            transition: step.transition().clone(),
            post_state: timed.state_digest(&next).expect("a digest"),
            delay: Some(delay.clone()),
        });
        state = next;
    }
    steps
}

/// A counterexample of `kind` with `prefix`, over `world`'s instance `c`.
fn payload(
    world: &World,
    prefix: Vec<ModelStep>,
    kind: CounterexampleKind,
    lasso: Option<Lasso>,
) -> TemporalCounterexample {
    let system = world.system();
    let node = &world.package.graph().temporal_clauses()[0];
    let instance = system
        .instances(&node.clause().over().1)
        .expect("instances")
        .remove(0);
    TemporalCounterexample {
        initial_state: 0,
        prefix,
        over_binding: Some(instance),
        fairness: node.clause().fairness().to_vec(),
        kind,
        lasso,
        blocked: Vec::new(),
        refinement: None,
        final_delay: None,
    }
}

fn whole(value: i64) -> Rational {
    exact(value, 1)
}

/// Part of FR-237-AC-1 (TC-692): FR-236-AC-1's counterexample, `send` at
/// 0, `timeout` after 3 and `reply` after 0 over `Rpc` with `T = 3 ms`,
/// replays to `reproduced-with-evaluated-witness` at trace position 3: the
/// delay 3 is admissible under `x <= 3` and `x <= T`, and the guards
/// `x >= 3` and `x >= 1` hold after it. Not covered: the `Settles` half,
/// a TT-3 claim that EN-6 decides with MightyL (QSL-583).
#[trace("TC-692")]
#[test]
fn a_late_reply_replays_with_its_delays() {
    let three = rpc(3);
    let counterexample = violated(claim(&three, ZoneCheckLimits::default(), &Cancel::new()));
    assert_eq!(operations(&counterexample), ["send", "timeout", "reply"]);
    assert_eq!(delays(&counterexample), [whole(0), whole(3), whole(0)]);
    let settlement = replay(&three, &counterexample).expect("replays");
    assert_eq!(
        settlement,
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Truth(false),
            trace_position: Some(3),
        }
    );
}

/// FR-237-AC-2: refusals settle no result. The `timeout` delay 5/2 leaves
/// its guard `x >= 3` false; 7/2 passes `x <= 3`; an altered post-state
/// digest selects no successor (`stale_dependency`); a lasso whose loop
/// takes no time, and one whose loop does not return to its entry's clock
/// values, are refused.
#[trace("TC-692", "FR-237-AC-2")]
#[test]
fn a_counterexample_that_breaks_time_is_refused() {
    let three = rpc(3);
    let counterexample = violated(claim(&three, ZoneCheckLimits::default(), &Cancel::new()));
    let mut early = counterexample.clone();
    early.prefix[1].delay = Some(exact(5, 2));
    let (cause, code) = refusal(replay(&three, &early));
    assert!(
        matches!(cause, ModelTraceRefusal::NotEnabled { step: 1, .. }),
        "{cause:?}"
    );
    assert_eq!(code, Code::InvalidRuntimeInput);
    let mut late = counterexample.clone();
    late.prefix[1].delay = Some(exact(7, 2));
    let (cause, code) = refusal(replay(&three, &late));
    assert_eq!(
        cause,
        ModelTraceRefusal::DelayNotAdmissible {
            step: Some(1),
            delay: exact(7, 2),
            bar: "time-invariant",
        }
    );
    assert_eq!(code, Code::InvalidRuntimeInput);
    let mut altered = counterexample;
    altered.prefix[2].post_state = altered.prefix[0].post_state;
    let (cause, code) = refusal(replay(&three, &altered));
    assert!(
        matches!(cause, ModelTraceRefusal::PostState { step: 2, .. }),
        "{cause:?}"
    );
    assert_eq!(code, Code::StaleDependency);

    // `Lock`: `go` to B, then `ping`, which changes no field and resets no
    // clock, as the loop.
    let world = lock();
    let loop_of = |delay: Rational| {
        payload(
            &world,
            walk(&world, &[("go", whole(0)), ("ping", delay)]),
            CounterexampleKind::Formula,
            Some(Lasso {
                entry: 1,
                terminal_stutter: false,
            }),
        )
    };
    let (cause, code) = refusal(replay(&world, &loop_of(whole(0))));
    assert_eq!(cause, ModelTraceRefusal::LoopZeroDelay { entry: 1 });
    assert_eq!(code, Code::InvalidRuntimeInput);
    let (cause, _) = refusal(replay(&world, &loop_of(exact(1, 2))));
    assert_eq!(cause, ModelTraceRefusal::LoopOpen { entry: 1, steps: 2 });
}

/// The strict-guard `Rpc` (ADR-026 §11): `reply` needs `x > 3 ms`,
/// `timeout` `x > T`, `T = 3 ms`.
fn strict(reply: &str) -> World {
    rpc_variant(
        3,
        reply,
        ">",
        INFINITE,
        "temporal NoLateReply using inf over (c: M::Call) clock \"model-steps\" on origin \
         { always holds(not c.late) }\n",
    )
}

/// The strict-guard variant's time-lock counterexample (FR-236-AC-2):
/// `send` after 0, `final_delay` 3.
fn time_lock(world: &World) -> TemporalCounterexample {
    TemporalCounterexample {
        over_binding: None,
        fairness: Vec::new(),
        final_delay: Some(whole(3)),
        ..payload(
            world,
            walk(world, &[("send", whole(0))]),
            CounterexampleKind::TimeLock,
            None,
        )
    }
}

/// FR-237-AC-3: the strict-guard variant's local time-lock reproduces: at
/// `x = 3` no delay is admissible and neither `reply` nor `timeout` is
/// enabled. Replayed against the variant with `reply` guarded by `x >= 3
/// ms`, `reply` is enabled there, so replay takes the non-local arm, whose
/// exploration reaches the quiescent `Replied` state: `inconclusive`.
#[trace("TC-692", "FR-237-AC-3")]
#[test]
fn a_local_time_lock_reproduces_and_a_live_one_does_not() {
    let locked = strict("> 3");
    let settlement =
        replay_on(&locked, &time_lock(&locked), true, &Limits::default()).expect("replays");
    assert_eq!(
        settlement,
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Truth(false),
            trace_position: None,
        }
    );
    let live = strict(">= 3");
    let settlement =
        replay_on(&live, &time_lock(&live), true, &Limits::default()).expect("replays");
    assert!(parity(&settlement), "{settlement:?}");
}

/// FR-237-AC-5 (TC-710): FR-239-AC-5's counterexample over `Ticker`
/// replays to `reproduced-with-evaluated-witness`, the undefined evaluation
/// at position 2, time stamp 2; with its last step removed it settles
/// `inconclusive`.
#[trace("TC-710", "FR-237-AC-5")]
#[test]
fn an_undefined_evaluation_replays_at_its_time() {
    let world = ticker(false);
    let counterexample = violated(claim(&world, ZoneCheckLimits::default(), &Cancel::new()));
    let settlement = replay(&world, &counterexample).expect("replays");
    let ModelTraceSettlement::Reproduced {
        value: ModelTraceValue::Undefined(undefined),
        trace_position,
    } = settlement
    else {
        panic!("expected a reproduction, got {settlement:?}");
    };
    assert_eq!(trace_position, Some(2));
    assert_eq!(undefined.site.position, 2);
    assert_eq!(
        undefined.site.time.as_ref().map(|time| time.stamp.clone()),
        Some(whole(2))
    );
    assert_eq!(
        undefined.cause,
        UndefinedCause::Kernel(Undefined::DivisionByZero)
    );
    let mut short = counterexample;
    short.prefix.pop();
    let settlement = replay(&world, &short).expect("replays");
    assert!(parity(&settlement), "{settlement:?}");
}

/// `Lock` with `ping` resetting `x` when `resets`.
fn lock_resetting(resets: bool) -> World {
    if !resets {
        return lock();
    }
    world(
        &document(&["x"], &[("go", &["phase"]), ("ping", &["x"])]),
        INFINITE,
        &format!(
            "{DENSE}\
             pre Go using v on M::Call::go {{ self.phase = 0 }}\n\
             post Gone using v on M::Call::go {{ self.phase = 1 }}\n\
             pre Ping using v on M::Call::ping {{ self.phase = 1 }}\n\
             post Pinged using v on M::Call::ping {{ self.x = 0 ms }}\n\
             time invariant Locked when {{ self.phase = 1 }} {{ self.x <= 1 ms }}\n\
             temporal StaysA using inf over (c: M::Call) clock \"model-steps\" on origin \
             {{ always holds(c.phase = 0) }}\n"
        ),
    )
}

/// FR-237-AC-6: over `Lock`, the counterexample `go` after delay 0 to
/// `always holds(c.phase = A)` settles `inconclusive`: the replayed state
/// `(B, x = 0)` has no time-divergent continuation, since `x <= 1` holds
/// in B and `ping` resets nothing. Where `ping` resets `x` the same
/// counterexample reproduces; with an exploration limit of one state the
/// replay stops (V-7).
#[trace("TC-692", "FR-237-AC-6")]
#[test]
fn a_violation_on_time_locked_behaviour_does_not_reproduce() {
    for resets in [false, true] {
        let world = lock_resetting(resets);
        let counterexample = payload(
            &world,
            walk(&world, &[("go", whole(0))]),
            CounterexampleKind::Formula,
            None,
        );
        let settlement = replay(&world, &counterexample).expect("replays");
        if resets {
            assert!(reproduced(&settlement), "{settlement:?}");
            let stopped = replay_on(
                &world,
                &counterexample,
                false,
                &Limits {
                    max_states: 1,
                    max_transitions: 100,
                },
            )
            .expect("replays");
            assert!(
                matches!(
                    stopped,
                    ModelTraceSettlement::Stopped { reached, .. }
                        if reached.limit == ModelCheckLimit::MaxStates && reached.value == 1
                ),
                "{stopped:?}"
            );
        } else {
            assert!(parity(&settlement), "{settlement:?}");
        }
    }
}

/// The package every settled run here checked.
fn package() -> qsl_semantics::library::PackageId {
    qsl_semantics::library::PackageId::of_preimage(b"zone-replay")
}

fn package_id() -> DigestRecord {
    package().record()
}

/// A replay result settled against the run's package.
fn replayed(settlement: ModelTraceSettlement) -> ModelTraceReplayResult {
    ModelTraceReplayResult {
        settlement,
        hints: Vec::new(),
        package_id: package(),
    }
}

/// Settle `outcome` with the certificate check `check` and the replay
/// `result`.
fn settle(
    outcome: &Outcome,
    check: ZoneCertificateCheck,
    result: Result<ModelTraceSettlement, ReplayRefusal>,
) -> ZoneCheckRecord {
    settle_zone_check(
        ZoneItem::Outcome(outcome),
        &package_id(),
        &Cancel::new(),
        |_| check,
        |_| result.map(replayed),
    )
}

/// FR-235-AC-1 (TC-690): each row of FR-235's map, from its input to its
/// terminal value, FR-360 label, FR-243 basis and O-16 category; and the
/// zone-certified, closed-form and bounded-solver bases each settle as
/// success.
#[trace("TC-690", "FR-235-AC-1")]
#[test]
fn each_timed_verdict_row_settles_as_stated() {
    let four = rpc(4);
    let proof = claim(&four, ZoneCheckLimits::default(), &Cancel::new());
    // V-1.
    let record = settle(&proof, ZoneCertificateCheck::Accepted, Err(fault()));
    assert_eq!(
        record.value,
        TerminalValue::Proved {
            basis: ProofBasis::ZoneCertified,
            certification: qsl_replay::Certification::Certified,
        }
    );
    assert_eq!(
        (record.label, record.basis, record.value.category()),
        (
            ResultLabel::Proved,
            SettlementBasis::ClosedScope,
            Category::Success
        )
    );
    assert_eq!(record.method, "zone-certified");
    // V-6, a rejected certificate.
    let rule = CertificateRule::BadState;
    let at = CertificateLocus::Entry { index: 0 };
    let record = settle(
        &proof,
        ZoneCertificateCheck::Rejected {
            rule,
            at: at.clone(),
        },
        Err(fault()),
    );
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected { rule, at })
    );
    assert_eq!(
        (record.label, record.basis, record.value.category()),
        (
            ResultLabel::Inconclusive,
            SettlementBasis::Unsettled,
            Category::Inconclusive
        )
    );
    // V-7, a stopped check.
    let limit = IncompleteCause::LimitReached {
        limit: ModelCheckLimit::ZoneMaxSymbolicStates,
        value: 9,
    };
    let record = settle(&proof, ZoneCertificateCheck::Stopped(limit), Err(fault()));
    assert_eq!(record.value, TerminalValue::Incomplete(limit));
    assert_eq!(record.value.category(), Category::Incomplete);

    // V-4: a reproduced formula counterexample.
    let three = rpc(3);
    let refuted = claim(&three, ZoneCheckLimits::default(), &Cancel::new());
    let reproduced = ModelTraceSettlement::Reproduced {
        value: ModelTraceValue::Truth(false),
        trace_position: Some(3),
    };
    let record = settle(
        &refuted,
        ZoneCertificateCheck::Accepted,
        Ok(reproduced.clone()),
    );
    assert_eq!(record.value, TerminalValue::Refuted);
    assert_eq!(
        (record.label, record.basis, record.value.category()),
        (
            ResultLabel::Refuted,
            SettlementBasis::DecisiveCounterexample,
            Category::Violation
        )
    );
    assert!(record.counterexample.is_some());
    // V-6: a replay that settles inconclusive, and one that refuses.
    let cause = DisagreementCause::NoValue {
        proved: qsl_replay::Verdict::from_category(Category::Violation),
        replayed: qsl_replay::Verdict::from_category(Category::Incomplete),
    };
    let record = settle(
        &refuted,
        ZoneCertificateCheck::Accepted,
        Ok(ModelTraceSettlement::Inconclusive(cause.clone())),
    );
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(cause))
    );
    let refused =
        ReplayRefusal::ModelTrace(Box::new(ModelTraceRefusal::LoopZeroDelay { entry: 0 }));
    let record = settle(&refuted, ZoneCertificateCheck::Accepted, Err(refused));
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(Code::InvalidRuntimeInput))
    );
    assert!(record.counterexample.is_none());
    // A replay settled against another package is stale.
    let record = settle_zone_check(
        ZoneItem::Outcome(&refuted),
        &qsl_semantics::library::PackageId::of_preimage(b"another").record(),
        &Cancel::new(),
        |_| ZoneCertificateCheck::Accepted,
        |_| Ok(replayed(reproduced.clone())),
    );
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(Code::StaleDependency))
    );
    // A cancelled run.
    let cancel = Cancel::new();
    cancel.cancel(quire_exact::CancelCause::Requested);
    let cancelled = claim(&four, ZoneCheckLimits::default(), &cancel);
    let record = settle_zone_check(
        ZoneItem::Outcome(&cancelled),
        &package_id(),
        &cancel,
        |_| ZoneCertificateCheck::Accepted,
        |_| Err(fault()),
    );
    assert!(
        matches!(
            record.value,
            TerminalValue::Incomplete(IncompleteCause::Cancelled { .. })
        ),
        "{:?}",
        record.value
    );

    // The success bases.
    for basis in [
        ProofBasis::ZoneCertified,
        ProofBasis::ClosedForm {
            analysis: ClosedFormAnalysis::FixedPriorityRta,
        },
        ProofBasis::BoundedSolver {
            method: qsl_replay::SolverMethod::DeltaComplete,
            horizon: qsl_replay::SolverHorizon::new(whole(1)).expect("a horizon"),
            jumps: 1,
            exhaustive: std::collections::BTreeSet::new(),
        },
    ] {
        let value = TerminalValue::Proved {
            basis,
            certification: qsl_replay::Certification::Certified,
        };
        assert_eq!(value.category(), Category::Success);
    }
}

fn fault() -> ReplayRefusal {
    ReplayRefusal::Fault(quire_semantic_value::catalog::InternalFault::new(
        "test",
        "not-called",
    ))
}

/// FR-235-AC-3 (TC-690): `NoLateReply` with `T = 3 ms` settles `refuted`
/// only after FR-237's replay reproduces it; with the `timeout` delay
/// changed to 5/2 the replay refuses and the item settles `inconclusive`.
/// The strict-guard variant's time-lock-freedom item, with the time-lock
/// counterexample, settles `refuted`, basis `closed-scope`, kind
/// `TimeLock`.
#[trace("TC-690", "FR-235-AC-3")]
#[test]
fn a_timed_refutation_settles_through_its_replay() {
    let three = rpc(3);
    let refuted = claim(&three, ZoneCheckLimits::default(), &Cancel::new());
    let settle_with = |outcome: &Outcome| {
        settle_zone_check(
            ZoneItem::Outcome(outcome),
            &package_id(),
            &Cancel::new(),
            |_| ZoneCertificateCheck::Accepted,
            |payload| replay(&three, payload).map(replayed),
        )
    };
    let record = settle_with(&refuted);
    assert_eq!(record.value, TerminalValue::Refuted);
    assert_eq!(record.basis, SettlementBasis::DecisiveCounterexample);
    let ZoneCheckOutcome::Violated {
        counterexample,
        stats,
    } = &refuted
    else {
        panic!("a violation");
    };
    let mut early = counterexample.as_ref().clone();
    early.prefix[1].delay = Some(exact(5, 2));
    let tampered = ZoneCheckOutcome::Violated {
        counterexample: Box::new(early),
        stats: *stats,
    };
    let record = settle_with(&tampered);
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(Code::InvalidRuntimeInput))
    );
    assert_eq!(record.label, ResultLabel::Inconclusive);

    let locked = strict("> 3");
    let outcome: Outcome = ZoneCheckOutcome::Violated {
        counterexample: Box::new(time_lock(&locked)),
        stats: ZoneStats::default(),
    };
    let record = settle_zone_check(
        ZoneItem::Outcome(&outcome),
        &package_id(),
        &Cancel::new(),
        |_| ZoneCertificateCheck::Accepted,
        |payload| replay_on(&locked, payload, true, &Limits::default()).map(replayed),
    );
    assert_eq!(record.value, TerminalValue::Refuted);
    assert_eq!(record.basis, SettlementBasis::ClosedScope);
    assert_eq!(
        record
            .counterexample
            .map(|counterexample| counterexample.kind),
        Some(CounterexampleKind::TimeLock)
    );
}

/// FR-235-AC-4 (TC-690): FR-232-AC-3's `Stall` claim settles
/// `inconclusive`, `NoAdmittedBehaviour`; FR-234-AC-3's punctual liveness
/// claim settles `unsupported`, `PunctualInterval`; a zone search stopped
/// by `max_symbolic_states` settles `incomplete`, `limit-reached`, naming
/// the limit.
#[trace("TC-690", "FR-235-AC-4")]
#[test]
fn vacuity_punctuality_and_budgets_settle_as_stated() {
    let stall = windowed(
        &["x"],
        &[("ping", &[])],
        "time invariant Stall when { true } { self.x <= 1 ms }\n",
        "always holds(true)",
    );
    let outcome = claim(&stall, ZoneCheckLimits::default(), &Cancel::new());
    let record = settle(&outcome, ZoneCertificateCheck::Accepted, Err(fault()));
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::NoAdmittedBehaviour)
    );

    let punctual = crate::timed_world::world(
        &document(&["x"], &[("go", &["phase"])]),
        TIMED,
        &format!(
            "{DENSE}temporal P using tt over (c: M::Call) clock \"model-time\" on origin \
             {{ always (holds(c.late) implies eventually[2 ms, 2 ms] holds(c.retried)) }}\n"
        ),
    );
    let clause = punctual.package.graph().temporal_clauses()[0].clause();
    let span = clause.unsupported_punctual().expect("a punctual interval");
    let disposed = UnsupportedItem::PunctualInterval { span };
    let record = settle_zone_check::<&'static str>(
        ZoneItem::Unsupported(&disposed),
        &package_id(),
        &Cancel::new(),
        |_| ZoneCertificateCheck::Accepted,
        |_| Err(fault()),
    );
    assert_eq!(record.label, ResultLabel::Unsupported);
    assert_eq!(record.value.category(), Category::Unsupported);

    let tight = ZoneCheckLimits {
        max_symbolic_states: 2,
        ..ZoneCheckLimits::default()
    };
    let stopped = claim(&rpc(4), tight, &Cancel::new());
    let record = settle(&stopped, ZoneCertificateCheck::Accepted, Err(fault()));
    let limit = IncompleteCause::LimitReached {
        limit: ModelCheckLimit::ZoneMaxSymbolicStates,
        value: 2,
    };
    assert_eq!(record.value, TerminalValue::Incomplete(limit));
    assert_eq!(limit.as_str(), "limit-reached");
    assert_eq!(
        ModelCheckLimit::ZoneMaxSymbolicStates.setting(),
        "zone_check.max_symbolic_states"
    );
}

/// FR-235-AC-5 (TC-710): FR-239-AC-5's outcome settles `refuted`,
/// `decisive-counterexample`, category violation, with the undefined
/// evaluation `division-by-zero` at position 2, time stamp 2, after
/// FR-237's replay reproduces it; the time-locked variant's
/// `Undecided(NoAdmittedBehaviour)` settles `inconclusive`,
/// `NoAdmittedBehaviour`.
#[trace("TC-710", "FR-235-AC-5")]
#[test]
fn an_undefined_timed_evaluation_settles_refuted() {
    let world = ticker(false);
    let outcome = claim(&world, ZoneCheckLimits::default(), &Cancel::new());
    let record = settle_zone_check(
        ZoneItem::Outcome(&outcome),
        &package_id(),
        &Cancel::new(),
        |_| ZoneCertificateCheck::Accepted,
        |payload| replay(&world, payload).map(replayed),
    );
    assert_eq!(record.value, TerminalValue::Refuted);
    assert_eq!(
        (record.basis, record.value.category()),
        (SettlementBasis::DecisiveCounterexample, Category::Violation)
    );
    let Some(CounterexampleKind::UndefinedEvaluation(undefined)) = record
        .counterexample
        .map(|counterexample| counterexample.kind)
    else {
        panic!("an undefined evaluation");
    };
    assert_eq!(undefined.site.position, 2);
    assert_eq!(undefined.site.time.map(|time| time.stamp), Some(whole(2)));
    assert_eq!(
        undefined.cause,
        UndefinedCause::Kernel(Undefined::DivisionByZero)
    );

    let locked = claim(&ticker(true), ZoneCheckLimits::default(), &Cancel::new());
    let record = settle(&locked, ZoneCertificateCheck::Accepted, Err(fault()));
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::NoAdmittedBehaviour)
    );
}

/// FR-237 "Evaluation" under the timed profile: a TT-2 counterexample
/// replays by evaluating the formula over the timed trace. `go` at 5/2 ms
/// passes `eventually[2 ms, 2 ms] holds(c.retried)` and reproduces; over
/// the model that never steps, the terminal-stutter lasso of
/// `eventually[0 ms, 3 ms] holds(c.retried)` reproduces on the idle tail.
#[trace("TC-692")]
#[test]
fn a_bounded_window_counterexample_replays_over_its_timed_trace() {
    for world in [
        crate::zone_search::retried_at("rational(5, 2) ms"),
        crate::zone_search::idle("eventually[0 ms, 3 ms] holds(c.retried)"),
    ] {
        let counterexample = violated(claim(&world, ZoneCheckLimits::default(), &Cancel::new()));
        let settlement = replay(&world, &counterexample).expect("replays");
        assert!(reproduced(&settlement), "{settlement:?}");
    }
}
