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
    settle_zone_check, ZoneCheckLimits, ZoneCheckOutcome, ZoneCheckRecord, ZoneItem, ZoneStats,
};
use qsl_eval::simulation::model::{TimedState, TimedSystem};
use qsl_eval::simulation::Limits;
use qsl_eval::temporal::UndefinedCause;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::digest::DigestRecord;
use qsl_replay::{
    CertificateLocus, CertificateRule, ClosedFormAnalysis, CounterexampleKind, DelayBar,
    DisagreementCause, IncompleteCause, InconclusiveCause, Lasso, ModelCheckLimit, ModelStep,
    ModelTraceRefusal, ModelTraceReplayResult, ModelTraceSettlement, ModelTraceValue, ProofBasis,
    ReplayRefusal, ResultLabel, SettlementBasis, StrongFairnessHint, TemporalCounterexample,
    TerminalValue, UnavailabilityCause, ZoneCertificateVerdict,
};
use qsl_semantics::family::UnsupportedItem;
use quire_exact::{Cancel, Rational, Undefined};
use quire_semantic_value::catalog::Category;
use quire_semantic_value::fairness::FairnessKind;

use crate::timed_world::{document, world, World, DENSE};
use crate::zone_search::{
    claim, delays, exact, lock, operations, retried_at, rpc, rpc_variant, short_of_data_claim,
    stall, stall_resetting, ticker, time_lock_item, violated, windowed, Outcome, INFINITE, TIMED,
};

/// Replay `payload` over `world`'s timed subject: as its first clause's
/// counterexample, or as a derived item's when `derived`.
fn replay_on(
    world: &World,
    payload: &TemporalCounterexample,
    derived: bool,
    exploration: &Limits,
) -> Result<ModelTraceSettlement, ReplayRefusal> {
    replay_with_hints(world, payload, derived, exploration).map(|(settlement, _)| settlement)
}

/// [`replay_on`] with the FR-133 hints of a reproduced lasso.
fn replay_with_hints(
    world: &World,
    payload: &TemporalCounterexample,
    derived: bool,
    exploration: &Limits,
) -> Result<(ModelTraceSettlement, Vec<StrongFairnessHint>), ReplayRefusal> {
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

pub(crate) fn replay(
    world: &World,
    payload: &TemporalCounterexample,
) -> Result<ModelTraceSettlement, ReplayRefusal> {
    replay_on(world, payload, false, &Limits::default())
}

pub(crate) fn reproduced(settlement: &ModelTraceSettlement) -> bool {
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
/// `x >= 3` and `x >= 1` hold after it. The `Settles` half is
/// `crate::zone_mitl::settles_is_proved_and_its_strict_variant_refuted`.
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
/// its guard `x >= 3` false, named by its clause `Timeout`; 7/2 passes
/// `x <= 3`, named by the invariant `ServerDeadline`; an altered post-state
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
        matches!(
            &cause,
            ModelTraceRefusal::GuardFails { step: 1, guards, .. } if guards == &["Timeout"]
        ),
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
            bar: DelayBar::TimeInvariant("ServerDeadline".to_owned()),
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

/// `Stall`'s non-local time-lock counterexample with `final_delay`
/// `delay`: an empty stem.
fn stalled(world: &World, delay: Rational) -> TemporalCounterexample {
    TemporalCounterexample {
        over_binding: None,
        fairness: Vec::new(),
        final_delay: Some(delay),
        ..payload(world, Vec::new(), CounterexampleKind::TimeLock, None)
    }
}

/// Part of FR-237-AC-4 (TC-692): the `Stall` model's non-local time-lock
/// counterexample (stem empty, `final_delay` 1/2) reproduces by fresh
/// exploration finding no quiescent state and no cycle with positive
/// delay; against the variant with `ping` resetting `x` it settles
/// `inconclusive`, `ReplayParity`; an exploration limit stops it (V-7).
/// With `final_delay` 1 the last state `x = 1` admits no positive delay
/// while `ping` is enabled, so it takes the non-local arm and reproduces.
/// EN-6's own `Stall` counterexample (FR-240), whose final delay is the
/// least one, 0 by FR-241, reproduces and settles `refuted`,
/// `closed-scope`. Replaying one envelope twice gives equal results. Not
/// covered: a limit of one state, which `Stall`'s closure, one symbolic
/// state with `ping` looping on it, never exceeds; the transition limit
/// stops it instead (spec gap STD-493).
#[trace("TC-692")]
#[test]
fn a_non_local_time_lock_reproduces_by_fresh_exploration() {
    let world = stall();
    let reproduced_lock = ModelTraceSettlement::Reproduced {
        value: ModelTraceValue::Truth(false),
        trace_position: None,
    };
    let half = stalled(&world, exact(1, 2));
    let settlement = replay_on(&world, &half, true, &Limits::default()).expect("replays");
    assert_eq!(settlement, reproduced_lock);
    assert_eq!(
        replay_on(&world, &half, true, &Limits::default()).expect("replays"),
        settlement
    );

    let resetting = stall_resetting();
    let settlement = replay_on(
        &resetting,
        &stalled(&resetting, exact(1, 2)),
        true,
        &Limits::default(),
    )
    .expect("replays");
    assert!(parity(&settlement), "{settlement:?}");
    let record = settle(
        &ZoneCheckOutcome::Violated {
            counterexample: Box::new(half.clone()),
            stats: ZoneStats::default(),
        },
        ZoneCertificateVerdict::Accepted,
        Ok(settlement),
    );
    assert!(
        matches!(
            record.value,
            TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(_))
        ),
        "{record:?}"
    );

    let one = Limits {
        max_states: 1,
        ..Limits::default()
    };
    assert_eq!(
        replay_on(&world, &half, true, &one).expect("replays"),
        reproduced_lock
    );
    let none = Limits {
        max_transitions: 0,
        ..Limits::default()
    };
    let settlement = replay_on(&world, &half, true, &none).expect("replays");
    assert!(
        matches!(
            settlement,
            ModelTraceSettlement::Stopped {
                reached: qsl_replay::ReachedLimit {
                    limit: ModelCheckLimit::MaxTransitions,
                    value: 0,
                },
                ..
            }
        ),
        "{settlement:?}"
    );
    let record = settle(
        &ZoneCheckOutcome::Violated {
            counterexample: Box::new(half),
            stats: ZoneStats::default(),
        },
        ZoneCertificateVerdict::Accepted,
        Ok(settlement),
    );
    assert!(
        matches!(record.value, TerminalValue::Incomplete(_)),
        "{record:?}"
    );

    let at_bound = stalled(&world, exact(1, 1));
    assert_eq!(
        replay_on(&world, &at_bound, true, &Limits::default()).expect("replays"),
        reproduced_lock
    );

    let outcome = time_lock_item(&world);
    let found = violated(outcome.clone());
    assert_eq!(found, stalled(&world, exact(0, 1)));
    let record = settle_zone_check(
        ZoneItem::Outcome(&outcome),
        &package_id(),
        &Cancel::new(),
        |_| ZoneCertificateVerdict::Accepted,
        |payload| replay_on(&world, payload, true, &Limits::default()).map(replayed),
    );
    assert_eq!(record.value, TerminalValue::Refuted);
    assert_eq!(record.basis, SettlementBasis::ClosedScope);
}

/// `Lock` with `ping` resetting `x` when `resets`.
pub(crate) fn lock_resetting(resets: bool) -> World {
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

pub(crate) fn package_id() -> DigestRecord {
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
    check: ZoneCertificateVerdict,
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
    let record = settle(&proof, ZoneCertificateVerdict::Accepted, Err(fault()));
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
        ZoneCertificateVerdict::Rejected(qsl_replay::CertificateRejection {
            rule,
            at: at.clone(),
        }),
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
    let record = settle(&proof, ZoneCertificateVerdict::Stopped(limit), Err(fault()));
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
        ZoneCertificateVerdict::Accepted,
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
        ZoneCertificateVerdict::Accepted,
        Ok(ModelTraceSettlement::Inconclusive(cause.clone())),
    );
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(cause))
    );
    let refused =
        ReplayRefusal::ModelTrace(Box::new(ModelTraceRefusal::LoopZeroDelay { entry: 0 }));
    let record = settle(&refuted, ZoneCertificateVerdict::Accepted, Err(refused));
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
        |_| ZoneCertificateVerdict::Accepted,
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
        |_| ZoneCertificateVerdict::Accepted,
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

pub(crate) fn fault() -> ReplayRefusal {
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
            |_| ZoneCertificateVerdict::Accepted,
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
    let outcome = time_lock_item(&locked);
    assert_eq!(violated(outcome.clone()), time_lock(&locked));
    let record = settle_zone_check(
        ZoneItem::Outcome(&outcome),
        &package_id(),
        &Cancel::new(),
        |_| ZoneCertificateVerdict::Accepted,
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
    let record = settle(&outcome, ZoneCertificateVerdict::Accepted, Err(fault()));
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
        |_| ZoneCertificateVerdict::Accepted,
        |_| Err(fault()),
    );
    assert_eq!(record.label, ResultLabel::Unsupported);
    assert_eq!(
        record.value,
        TerminalValue::Unsupported(UnavailabilityCause::PunctualInterval { span })
    );
    assert_eq!(record.value.category(), Category::Unsupported);

    let tight = ZoneCheckLimits {
        max_symbolic_states: 2,
        ..ZoneCheckLimits::default()
    };
    let stopped = claim(&rpc(4), tight, &Cancel::new());
    let record = settle(&stopped, ZoneCertificateVerdict::Accepted, Err(fault()));
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
        |_| ZoneCertificateVerdict::Accepted,
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
    let record = settle(&locked, ZoneCertificateVerdict::Accepted, Err(fault()));
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::NoAdmittedBehaviour)
    );
}

/// Part of FR-235-AC-1 (TC-690): an `Undecided(InsufficientData)` outcome,
/// a window read short of data, settles `inconclusive`, never `proved`,
/// under FR-127's `UndecidedSuccessor` until the spec names a cause (spec
/// gap STD-493). Not covered: an authored aggregate atom, which no TT-2
/// clause has yet.
#[trace("TC-690")]
#[test]
fn a_window_short_of_data_settles_inconclusive() {
    let outcome = short_of_data_claim(&retried_at("rational(5, 2) ms"));
    let record = settle(&outcome, ZoneCertificateVerdict::Accepted, Err(fault()));
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::UndecidedSuccessor)
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

/// A model over `operations` whose one clause is `eventually
/// holds(c.retried)` under infinite-trace, with the fairness set
/// `fairness` and the constraints `model`.
fn retrying(clocks: &[&str], operations: &[(&str, &[&str])], model: &str, fairness: &str) -> World {
    world(
        &document(clocks, operations),
        INFINITE,
        &format!(
            "{DENSE}{model}\
             temporal Retries using inf over (c: M::Call) clock \"model-steps\" on origin \
             {{ {fairness} eventually holds(c.retried) }}\n"
        ),
    )
}

/// `retry`, guarded by `x >= 5 ms`, sets `retried`; nothing else happens,
/// so the initial state at `x = 0` is quiescent and `retry` opens only
/// after a delay of 5.
fn late_retry(fairness: &str) -> World {
    retrying(
        &["x"],
        &[("retry", &["retried"])],
        "pre Retry using v on M::Call::retry { not self.retried and self.x >= 5 ms }\n\
         post Retried using v on M::Call::retry { self.retried }\n",
        fairness,
    )
}

/// The idle-tail counterexample at `world`'s initial state.
fn idle_tail(world: &World) -> TemporalCounterexample {
    payload(
        world,
        Vec::new(),
        CounterexampleKind::Formula,
        Some(Lasso {
            entry: 0,
            terminal_stutter: true,
        }),
    )
}

/// SR-1634001 FND-001 and FND-010: on the idle tail at `x = 0`, `retry`
/// (`x >= 5 ms`) opens at delay 5 and stays enabled, never taken. With no
/// fairness the counterexample to `eventually holds(c.retried)` reproduces
/// with the strong hint for `retry` at loop position 0; under `fair weak
/// retry` and under `fair strong retry` alike it is an unfair lasso, the
/// strong case read over the whole tail, not its first stutter window.
#[trace("TC-692")]
#[test]
fn strong_fairness_reads_the_whole_idle_tail() {
    let free = late_retry("");
    let (settlement, hints) =
        replay_with_hints(&free, &idle_tail(&free), false, &Limits::default()).expect("replays");
    assert!(reproduced(&settlement), "{settlement:?}");
    let [hint] = hints.as_slice() else {
        panic!("one hint: {hints:?}");
    };
    assert_eq!(hint.suggested.kind(), FairnessKind::Strong);
    assert_eq!(hint.suggested.operation().name(), "retry");
    assert_eq!(hint.first_enabled_position, 0);
    for fairness in ["fair weak M::Call::retry;", "fair strong M::Call::retry;"] {
        let world = late_retry(fairness);
        let result = replay(&world, &idle_tail(&world));
        assert!(
            matches!(result, Err(ReplayRefusal::UnfairLasso { .. })),
            "{fairness}: {result:?}"
        );
    }
}

/// SR-1634001 FND-002: `go` posts `phase = 1` or `phase = 2`, two
/// successors under one transition identity, and `retry` is enabled at
/// phase 2 alone. Under `fair weak retry` the idle tail after the phase-1
/// successor reproduces, and the one after the phase-2 successor, whose
/// `retry` stays enabled and untaken, is an unfair lasso: fairness is read
/// on the recorded successor, not the first one with the identity.
#[trace("TC-692")]
#[test]
fn fairness_is_judged_on_the_recorded_successor() {
    let world = retrying(
        &["x"],
        &[("go", &["phase"]), ("retry", &["retried"])],
        "pre Go using v on M::Call::go { self.phase = 0 }\n\
         post Gone using v on M::Call::go { self.phase = 1 or self.phase = 2 }\n\
         pre Retry using v on M::Call::retry { self.phase = 2 and not self.retried }\n\
         post Retried using v on M::Call::retry { self.retried }\n",
        "fair weak M::Call::retry;",
    );
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let start = timed.initial().remove(0);
    let successors: Vec<_> = timed
        .steps(&start)
        .expect("decided")
        .into_iter()
        .filter(|(step, _)| step.operation().contains("Call/go\""))
        .collect();
    assert_eq!(successors.len(), 2, "two successors of `go`");
    let mut outcomes = Vec::new();
    for (step, next) in &successors {
        let retries = timed
            .guarded_steps(next.discrete())
            .expect("decided")
            .iter()
            .any(|guarded| guarded.operation().contains("Call/retry\""));
        let counterexample = payload(
            &world,
            vec![ModelStep {
                transition: step.transition().clone(),
                post_state: timed.state_digest(next).expect("a digest"),
                delay: Some(whole(0)),
            }],
            CounterexampleKind::Formula,
            Some(Lasso {
                entry: 1,
                terminal_stutter: true,
            }),
        );
        outcomes.push((retries, replay(&world, &counterexample)));
    }
    for (retries, result) in outcomes {
        if retries {
            assert!(
                matches!(result, Err(ReplayRefusal::UnfairLasso { .. })),
                "{result:?}"
            );
        } else {
            assert!(
                matches!(&result, Ok(settlement) if reproduced(settlement)),
                "{result:?}"
            );
        }
    }
}

/// SR-1634002 FND-001: fairness over a loop of steps. `ping` (`x >= 1 ms`)
/// resets `x`, and `retry` is guarded by `x >= 1 ms` too, so on the loop
/// of `ping` after 1 it is enabled at the one instant before each `ping`.
/// Under `fair weak retry` it is not enabled throughout, so the lasso is
/// fair and reproduces; under `fair strong retry` it is enabled infinitely
/// often and never taken, an unfair lasso.
#[trace("TC-692")]
#[test]
fn a_timed_loop_is_weakly_fair_and_strongly_unfair() {
    let looping = |fairness: &str| {
        retrying(
            &["x"],
            &[("ping", &["x"]), ("retry", &["retried"])],
            "pre Ping using v on M::Call::ping { self.x >= 1 ms }\n\
             post Pinged using v on M::Call::ping { self.x = 0 ms }\n\
             pre Retry using v on M::Call::retry { not self.retried and self.x >= 1 ms }\n\
             post Retried using v on M::Call::retry { self.retried }\n",
            fairness,
        )
    };
    let lasso = |world: &World| {
        payload(
            world,
            walk(world, &[("ping", whole(1))]),
            CounterexampleKind::Formula,
            Some(Lasso {
                entry: 0,
                terminal_stutter: false,
            }),
        )
    };
    let weak = looping("fair weak M::Call::retry;");
    let settlement = replay(&weak, &lasso(&weak)).expect("replays");
    assert!(reproduced(&settlement), "{settlement:?}");
    let strong = looping("fair strong M::Call::retry;");
    let result = replay(&strong, &lasso(&strong));
    assert!(
        matches!(result, Err(ReplayRefusal::UnfairLasso { .. })),
        "{result:?}"
    );
}

/// SR-1634001 FND-003 (FR-236 "Timed lasso"): `ping` (`x >= 1 ms`) resets
/// `x` and never `y`, which no constraint reads, so the model's largest
/// constant is 1. The loop of one `ping` after 1 entered at `y = 2` closes:
/// `x` returns to 0 and `y` runs free above 1. Entered at `y = 1` it does
/// not, since `y` is not above the constant there.
#[trace("TC-692")]
#[test]
fn a_loop_closes_with_a_free_running_clock() {
    let world = retrying(
        &["x", "y"],
        &[("ping", &["x"])],
        "pre Ping using v on M::Call::ping { self.x >= 1 ms }\n\
         post Pinged using v on M::Call::ping { self.x = 0 ms }\n",
        "",
    );
    let lasso = |pings: usize, entry: usize| {
        payload(
            &world,
            walk(&world, &vec![("ping", whole(1)); pings]),
            CounterexampleKind::Formula,
            Some(Lasso {
                entry,
                terminal_stutter: false,
            }),
        )
    };
    let settlement = replay(&world, &lasso(3, 2)).expect("replays");
    assert!(reproduced(&settlement), "{settlement:?}");
    let (cause, _) = refusal(replay(&world, &lasso(2, 1)));
    assert_eq!(cause, ModelTraceRefusal::LoopOpen { entry: 1, steps: 2 });
}

/// SR-1634001 FND-009 and SR-1634002 FND-005: a `final_delay` on a formula
/// counterexample, and a lasso on a time-lock, are refused, as are a
/// missing step delay and a time-lock with no `final_delay`; `go` after
/// delay 2 into `Lock`'s B breaks its time invariant `Locked` (`x <= 1`).
#[trace("TC-692")]
#[test]
fn malformed_timed_counterexamples_are_refused() {
    let three = rpc(3);
    let counterexample = violated(claim(&three, ZoneCheckLimits::default(), &Cancel::new()));
    let stray = TemporalCounterexample {
        final_delay: Some(whole(1)),
        ..counterexample.clone()
    };
    let (cause, code) = refusal(replay(&three, &stray));
    assert_eq!(cause, ModelTraceRefusal::StrayFinalDelay);
    assert_eq!(code, Code::InvalidRuntimeInput);
    let mut undelayed = counterexample;
    undelayed.prefix[0].delay = None;
    let (cause, _) = refusal(replay(&three, &undelayed));
    assert_eq!(cause, ModelTraceRefusal::MissingDelay { step: Some(0) });

    let locked = strict("> 3");
    let looped = TemporalCounterexample {
        lasso: Some(Lasso {
            entry: 1,
            terminal_stutter: true,
        }),
        ..time_lock(&locked)
    };
    let (cause, _) = refusal(replay_on(&locked, &looped, true, &Limits::default()));
    assert_eq!(cause, ModelTraceRefusal::TimeLockLasso);
    let undelayed = TemporalCounterexample {
        final_delay: None,
        ..time_lock(&locked)
    };
    let (cause, _) = refusal(replay_on(&locked, &undelayed, true, &Limits::default()));
    assert_eq!(cause, ModelTraceRefusal::MissingDelay { step: None });

    // `go` after 2 records the state it reaches, which `Locked` forbids.
    let world = lock();
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let delayed = timed
        .delay(&timed.initial().remove(0), &whole(2))
        .expect("decided")
        .expect("A admits the delay");
    let go = timed
        .guarded_steps(delayed.discrete())
        .expect("decided")
        .into_iter()
        .find(|step| step.operation().contains("Call/go\""))
        .expect("`go` is enabled");
    let reached = TimedState::new(go.post().clone(), go.reset(delayed.clocks()));
    let late = payload(
        &world,
        vec![ModelStep {
            transition: go.transition().clone(),
            post_state: timed.state_digest(&reached).expect("a digest"),
            delay: Some(whole(2)),
        }],
        CounterexampleKind::Formula,
        None,
    );
    let (cause, _) = refusal(replay(&world, &late));
    assert_eq!(
        cause,
        ModelTraceRefusal::TargetInvariant {
            step: 0,
            invariant: "Locked".to_owned(),
        }
    );
}

/// SR-1634002 FND-002: under a tick source of period 5 ms, `go` moves
/// phase 0 to 1 and `tick` is always enabled. `go` after delay 0 violates
/// `always holds(c.phase = 0)` and reproduces, since its endpoint ticks
/// forever; after delay 1 it is refused, a tick source admitting no
/// positive delay.
#[trace("TC-692")]
#[test]
fn a_tick_source_replays_by_its_ticks() {
    let world = world(
        &document(&[], &[("go", &["phase"]), ("tick", &[])]),
        INFINITE,
        "dimension Time;\nunit s : Time = rational(1, 1);\n\
         unit ms : Time = rational(1, 1000) * s;\n\
         time M tick M::Call::tick period 5 ms unit ms;\n\
         pre Go using v on M::Call::go { self.phase = 0 }\n\
         post Gone using v on M::Call::go { self.phase = 1 }\n\
         temporal StaysA using inf over (c: M::Call) clock \"model-steps\" on origin \
         { always holds(c.phase = 0) }\n",
    );
    let at = |delay: Rational| {
        let mut counterexample = payload(
            &world,
            walk(&world, &[("go", whole(0))]),
            CounterexampleKind::Formula,
            None,
        );
        counterexample.prefix[0].delay = Some(delay);
        counterexample
    };
    let settlement = replay(&world, &at(whole(0))).expect("replays");
    assert!(reproduced(&settlement), "{settlement:?}");
    let (cause, _) = refusal(replay(&world, &at(whole(1))));
    assert_eq!(
        cause,
        ModelTraceRefusal::DelayNotAdmissible {
            step: Some(0),
            delay: whole(1),
            bar: DelayBar::TickSource,
        }
    );
}

/// SR-1634002 FND-003 (FR-235-AC-1's remaining rows): `Undecided(
/// LassoNotConcretized)` settles `inconclusive`, `unsettled`; a violation
/// whose replay a limit stops settles V-7, FR-360 `failed`, execution
/// `resource-incomplete`, basis `unavailable`; a reproduced `Deadlock`
/// counterexample settles `refuted`, `decisive-counterexample`; and a
/// deadlock found only after a delay settles as `LassoNotConcretized`
/// (spec gap STD-493).
#[trace("TC-690", "FR-235-AC-1")]
#[test]
fn the_remaining_timed_rows_settle_as_stated() {
    for cause in [
        qsl_analyze::zone_check::ZoneUndecided::LassoNotConcretized,
        qsl_analyze::zone_check::ZoneUndecided::DeadlockAfterDelay,
    ] {
        let outcome: Outcome = ZoneCheckOutcome::Undecided {
            cause,
            stats: ZoneStats::default(),
        };
        let record = settle(&outcome, ZoneCertificateVerdict::Accepted, Err(fault()));
        assert_eq!(
            record.value,
            TerminalValue::Inconclusive(InconclusiveCause::LassoNotConcretized)
        );
        assert_eq!(
            (record.label, record.basis, record.value.category()),
            (
                ResultLabel::Inconclusive,
                SettlementBasis::Unsettled,
                Category::Inconclusive
            )
        );
    }

    let three = rpc(3);
    let refuted = claim(&three, ZoneCheckLimits::default(), &Cancel::new());
    let reached = qsl_replay::ReachedLimit {
        limit: ModelCheckLimit::MaxStates,
        value: 1,
    };
    let record = settle(
        &refuted,
        ZoneCertificateVerdict::Accepted,
        Ok(ModelTraceSettlement::Stopped {
            reached,
            count: Some(1),
        }),
    );
    assert_eq!(
        record.value,
        TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxStates,
            value: 1,
        })
    );
    assert_eq!(
        (
            record.label,
            record.execution,
            record.basis,
            record.value.category()
        ),
        (
            ResultLabel::Failed,
            qsl_replay::Execution::ResourceIncomplete,
            SettlementBasis::Unavailable,
            Category::Incomplete
        )
    );

    let ZoneCheckOutcome::Violated {
        counterexample,
        stats,
    } = &refuted
    else {
        panic!("a violation");
    };
    let deadlock: Outcome = ZoneCheckOutcome::Violated {
        counterexample: Box::new(TemporalCounterexample {
            kind: CounterexampleKind::Deadlock,
            over_binding: None,
            ..counterexample.as_ref().clone()
        }),
        stats: *stats,
    };
    let record = settle(
        &deadlock,
        ZoneCertificateVerdict::Accepted,
        Ok(ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Truth(false),
            trace_position: None,
        }),
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
}

/// A one-object model `model` over `clocks` and `operations`, with the
/// claim `always holds(true)`, which replay of a derived item reads past.
fn trap_world(time: &str, clocks: &[&str], operations: &[(&str, &[&str])], model: &str) -> World {
    world(
        &document(clocks, operations),
        INFINITE,
        &format!(
            "{time}{model}temporal Any using inf over (c: M::Call) clock \"model-steps\" on \
             origin {{ always holds(true) }}\n"
        ),
    )
}

/// EN-6's time-lock counterexample over `world`, which replay reproduces.
fn reproduced_time_lock(world: &World) -> TemporalCounterexample {
    let counterexample = violated(time_lock_item(world));
    assert_eq!(counterexample.kind, CounterexampleKind::TimeLock);
    assert_eq!(
        replay_on(world, &counterexample, true, &Limits::default()).expect("replays"),
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Truth(false),
            trace_position: None,
        }
    );
    counterexample
}

/// FR-240's search over trapped valuations of every shape, each reported
/// lock replayed (FR-237):
/// - a diagonal trap reached by a step: `r` resets `x`, and `go` needs
///   `x >= 2` before `y` reaches 3, so after `r` at `y > 1` only `r`
///   loops;
/// - an open boundary: `go` leaves while `x < 1`, so from `x = 1` on, held
///   to `x <= 2`, only `ping` loops: the least delay into the trap is 1;
/// - a Zeno reset loop: `ping` resets `x` under `x <= 0`, so time never
///   passes;
/// - a tick source: after `work`, only `spin` loops and no tick comes.
#[trace("TC-695")]
#[test]
fn trapped_valuations_of_every_shape_are_found() {
    let diagonal = trap_world(
        DENSE,
        &["x", "y"],
        &[("go", &["phase"]), ("r", &["x"])],
        "pre Go using v on M::Call::go { self.phase = 0 and self.x >= 2 ms }\n\
         post Gone using v on M::Call::go { self.phase = 1 }\n\
         pre R using v on M::Call::r { self.phase = 0 }\n\
         post Reset using v on M::Call::r { self.x = 0 ms }\n\
         time invariant Hold when { self.phase = 0 } { self.y <= 3 ms }\n",
    );
    let found = reproduced_time_lock(&diagonal);
    assert_eq!(found.prefix.len(), 1);
    assert!(found.prefix[0].transition.operation().contains("Call/r\""));

    let open = trap_world(
        DENSE,
        &["x"],
        &[("go", &["phase"]), ("ping", &[])],
        "pre Go using v on M::Call::go { self.phase = 0 and self.x < 1 ms }\n\
         post Gone using v on M::Call::go { self.phase = 1 }\n\
         pre Ping using v on M::Call::ping { self.phase = 0 }\n\
         time invariant Hold when { self.phase = 0 } { self.x <= 2 ms }\n",
    );
    let found = reproduced_time_lock(&open);
    assert!(found.prefix.is_empty());
    assert_eq!(found.final_delay, Some(whole(1)));

    let zeno = trap_world(
        DENSE,
        &["x"],
        &[("ping", &["x"])],
        "post Pinged using v on M::Call::ping { self.x = 0 ms }\n\
         time invariant Frozen when { true } { self.x <= 0 ms }\n",
    );
    let found = reproduced_time_lock(&zeno);
    assert!(found.prefix.is_empty());
    assert_eq!(found.final_delay, Some(whole(0)));

    let ticks = trap_world(
        "dimension Time;\nunit s : Time = rational(1, 1);\n\
         unit ms : Time = rational(1, 1000) * s;\n\
         time M tick M::Call::tick period 1 ms unit ms;\n",
        &[],
        &[("work", &["phase"]), ("tick", &[]), ("spin", &[])],
        "pre Tick using v on M::Call::tick { self.phase = 0 }\n\
         pre Work using v on M::Call::work { self.phase = 0 }\n\
         post Worked using v on M::Call::work { self.phase = 1 }\n\
         pre Spin using v on M::Call::spin { self.phase = 1 }\n",
    );
    let found = reproduced_time_lock(&ticks);
    assert_eq!(found.prefix.len(), 1);
    assert!(found.prefix[0]
        .transition
        .operation()
        .contains("Call/work\""));
}

/// FR-240's order: at one node a local time-lock comes before a non-local
/// one, and the first lock in canonical order is reported. Where `go`
/// leaves while `x < 1` and nothing else is enabled under `x <= 2`, the
/// valuations from 1 are trapped and `x = 2` is a local time-lock: the
/// local one is reported, after delay 2. Over the time-locked `Ticker`,
/// whose last state holds a local time-lock, the initial node's trap comes
/// first.
#[trace("TC-695")]
#[test]
fn a_local_lock_comes_first_at_a_node_and_the_first_node_first() {
    let both = trap_world(
        DENSE,
        &["x"],
        &[("go", &["phase"])],
        "pre Go using v on M::Call::go { self.phase = 0 and self.x < 1 ms }\n\
         post Gone using v on M::Call::go { self.phase = 1 }\n\
         time invariant Hold when { self.phase = 0 } { self.x <= 2 ms }\n",
    );
    let found = reproduced_time_lock(&both);
    assert!(found.prefix.is_empty());
    assert_eq!(found.final_delay, Some(whole(2)));

    let first = violated(time_lock_item(&ticker(true)));
    assert!(first.prefix.is_empty());
    assert_eq!(first.final_delay, Some(whole(0)));
}

/// Part of FR-415-AC-13 (TC-367), SR-1676002 FND-002: EN-6 over a timed
/// subject whose bound `max_clocks` is 1, below the two live clocks `x` and
/// `y` of `Call`, stops `LimitReached(MaxClocks(1))` when it expands the
/// initial state, and the outcome settles `incomplete` `limit-reached`
/// naming `max_clocks`, its value 1 and the setting
/// `model_check.max_clocks`. Under a bound of 2 the claim is decided. Not
/// covered: the request's `limits.max_clocks` member, whose request schema
/// (FR-368) this crate does not read.
#[trace("TC-367")]
#[test]
fn live_clocks_beyond_max_clocks_settle_limit_reached() {
    let world = crate::zone_search::rpc_world(
        &document(
            &["x", "y"],
            &[
                ("send", &["phase", "inflight", "late", "x"]),
                ("reply", &["phase", "inflight", "late"]),
                ("timeout", &["phase"]),
                ("reset", &["phase"]),
            ],
        ),
        4,
        INFINITE,
        "temporal NoLateReply using inf over (c: M::Call) clock \"model-steps\" on origin \
         { always holds(not c.late) }\n",
    );
    let decided = |max_clocks: u64| {
        let system = world.system();
        let timed = TimedSystem::with_max_clocks(
            &system,
            "M",
            std::num::NonZeroU64::new(max_clocks).expect("positive"),
        )
        .expect("a timed subject");
        let node = &world.package.graph().temporal_clauses()[0];
        let letters = system.clause_letters(node).expect("letters");
        qsl_analyze::zone_check::check_timed_model(
            qsl_analyze::zone_check::ZoneCheckRequest {
                subject: &timed,
                item: qsl_analyze::zone_check::ZoneCheckItem::Clause {
                    clause: node.clause(),
                    letters: &letters,
                },
                obligation: qsl_route::deadlock::ItemObligation {
                    subject: crate::zone_search::SUBJECT,
                    kind: qsl_route::deadlock::ObligationKind::Authored(
                        qsl_replay::node_occurrence(node),
                    ),
                },
                limits: ZoneCheckLimits::default(),
            },
            &Cancel::new(),
        )
        .expect("EN-6 takes the claim")
    };
    let stopped = decided(1);
    assert!(
        matches!(
            stopped,
            ZoneCheckOutcome::Stopped {
                cause: qsl_analyze::zone_check::ZoneStop::LimitReached(
                    qsl_analyze::zone_check::ZoneCheckLimit::MaxClocks(1)
                ),
                ..
            }
        ),
        "{stopped:?}"
    );
    let record = settle(&stopped, ZoneCertificateVerdict::Accepted, Err(fault()));
    assert_eq!(
        record.value,
        TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxClocks,
            value: 1,
        })
    );
    assert_eq!(ModelCheckLimit::MaxClocks.as_str(), "max_clocks");
    assert_eq!(
        ModelCheckLimit::MaxClocks.setting(),
        "model_check.max_clocks"
    );
    assert!(
        matches!(decided(2), ZoneCheckOutcome::Holds { .. }),
        "{:?}",
        decided(2)
    );
}
