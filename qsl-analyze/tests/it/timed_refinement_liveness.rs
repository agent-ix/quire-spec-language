// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-430 "Timed refinement" (TC-1170), the liveness half: FR-143's
//! second phase on EN-6's symbolic product reads only time-divergent
//! concrete behaviours (ADR-026 TS-4), so only a loop with positive total
//! delay refutes (EZ-6).

use ix_trace_rs::trace;
use qsl_analyze::zone_check::{
    check_timed_refinement, TimedLivenessHalf, TimedLivenessOutcome, TimedRefinementOutcome,
    TimedRefinementUndecided, TimedSafetyOutcome, ZoneUndecided,
};
use qsl_eval::simulation::Limits;
use qsl_replay::{
    open_systems, replay_timed_refinement_trace, Lasso, ModelTraceRefusal, RefinementFailure,
    RefinementTraceSettlement, ReplayRefusal, TemporalCounterexample, UnavailabilityCause,
};
use quire_exact::{Cancel, Integer, Rational};

use crate::timed_refinement::{analyzed, refinement, replay, safety_request};
use crate::timed_world::{document, refining, renamed_package, Refining, DENSE};

/// FR-430-AC-5's `Req`: `send` with the precondition `not self.sent` (here
/// `inflight`), and `ensure fair weak Req::send`.
fn req() -> Vec<u8> {
    renamed_package(&document(&[], &[("send", &["inflight"])]), "test/req")
}

/// FR-430-AC-5's `TReq` refining `Req`: `send` with the precondition
/// `not self.sent` (and, when `guarded`, AC-1's guard `x >= 1`, an extra
/// TD-6 case outside AC-5), and `poll`
/// (precondition `not self.sent`, reset `x`) mapped to a stutter; with
/// `invariant`, the time invariant `when not self.sent { x <= 0 }`.
fn polling(guarded: bool, invariant: bool) -> Refining {
    let model = document(&["x"], &[("send", &["inflight"]), ("poll", &["x"])]);
    let guard = if guarded { " and self.x >= 1 ms" } else { "" };
    let contracts = format!(
        "pre ASendable using v on A::Call::send {{ not self.inflight }}\n\
         post ASend using v on A::Call::send {{ self.inflight }}\n\
         pre MSend using v on M::Call::send {{ not self.inflight{guard} }}\n\
         post MSent using v on M::Call::send {{ self.inflight }}\n\
         pre MPoll using v on M::Call::poll {{ not self.inflight }}\n\
         post MPolled using v on M::Call::poll {{ self.x = 0 ms }}\n"
    );
    let wait = if invariant {
        "time invariant Wait when { not self.inflight } { self.x <= 0 ms }\n"
    } else {
        ""
    };
    let body = refinement("self.inflight").replace(
        "step M::Call::send -> A::Call::send(self);",
        "step M::Call::send -> A::Call::send(self);\n\
         step M::Call::poll -> stutter;\n\
         ensure fair weak A::Call::send;",
    );
    refining(&req(), &model, &format!("{DENSE}{contracts}{wait}{body}"))
        .unwrap_or_else(|refusals| panic!("{refusals:?}"))
}

/// Both halves of `world`'s one refinement.
fn check(world: &Refining) -> TimedRefinementOutcome<&'static str> {
    check_timed_refinement(safety_request(world), &Cancel::new())
        .expect("EN-6 takes the refinement")
}

/// FR-430-AC-5: without the time invariant, the `poll` loop at
/// `sent = false` admits a positive delay and never takes `send`, which
/// is enabled throughout: the liveness half returns `Violated` with an
/// empty stem, a loop of one `poll` step whose delay is 1 (its delay set
/// is every positive rational, unbounded, so ADR-026 EZ-8 picks its lower
/// end 0 plus one unit) and failure `Divergence{constraint: send}`, beside
/// safety `Holds`; the lasso replays through FR-241's delays.
#[trace("TC-1170", "FR-430-AC-5")]
#[test]
fn a_positive_delay_loop_that_never_takes_an_enabled_abstract_class_refutes() {
    let world = polling(false, false);
    let outcome = check(&world);
    assert!(matches!(outcome.safety, TimedSafetyOutcome::Holds { .. }));
    let Some(TimedLivenessHalf::Checked(TimedLivenessOutcome::Violated { counterexample, .. })) =
        outcome.liveness
    else {
        panic!("the liveness half is violated: {:?}", outcome.liveness);
    };
    assert_eq!(counterexample.initial_state, 0);
    assert_eq!(
        counterexample.lasso,
        Some(Lasso {
            entry: 0,
            terminal_stutter: false,
        })
    );
    let loop_steps: Vec<(bool, Option<Rational>)> = counterexample
        .prefix
        .iter()
        .map(|step| {
            (
                step.transition.operation().contains("/Call/poll\""),
                step.delay.clone(),
            )
        })
        .collect();
    assert_eq!(
        loop_steps,
        [(true, Some(Rational::from_integer(Integer::one())))]
    );
    let [refinement] = world.package.graph().refinements() else {
        panic!("one refinement");
    };
    let constraint = refinement
        .abstract_operation_fairness()
        .next()
        .expect("the ensure row")
        .clone();
    assert_eq!(
        counterexample.refinement,
        Some(RefinementFailure::Divergence {
            constraint: Some(constraint.clone()),
            protocol_class: None,
        })
    );
    assert!(matches!(
        replay(&world, &counterexample),
        RefinementTraceSettlement::Reproduced {
            failure: RefinementFailure::Divergence { .. },
            ..
        }
    ));
}

/// FR-430-AC-5 at replay (FR-237 "Lasso" over time): the same lasso with
/// its loop's delay set to 0 is refused `LoopZeroDelay`, since a loop with
/// no total delay is no time-divergent behaviour.
#[trace("TC-1170", "FR-430-AC-5")]
#[test]
fn a_refinement_lasso_with_no_loop_delay_is_refused_at_replay() {
    let world = polling(false, false);
    let Some(TimedLivenessHalf::Checked(TimedLivenessOutcome::Violated { counterexample, .. })) =
        check(&world).liveness
    else {
        panic!("the liveness half is violated");
    };
    let mut forged = *counterexample;
    forged.prefix[0].delay = Some(Rational::from_integer(Integer::zero()));
    let [refinement] = world.package.graph().refinements() else {
        panic!("one refinement");
    };
    let systems = open_systems(
        refinement,
        &world.concrete_request(),
        &world.package,
        vec![world.abstract_initial.clone()],
    )
    .expect("the subjects open");
    let refusal = replay_timed_refinement_trace(refinement, &systems, &forged, &Limits::default())
        .expect_err("a zero-delay loop is refused");
    assert!(
        matches!(
            &refusal,
            ReplayRefusal::ModelTrace(trace)
                if matches!(**trace, ModelTraceRefusal::LoopZeroDelay { entry: 0 })
        ),
        "{refusal:?}"
    );
}

/// FR-430-AC-5: with the time invariant `when not self.sent { x <= 0 }`,
/// the `poll` loop at `sent = false` has total delay 0, so it does not
/// refute; the subject still admits a time-divergent behaviour (`send` at
/// once, then the idle tail), and the liveness half returns
/// `Checked(Holds)` beside safety `Holds`.
#[trace("TC-1170", "FR-430-AC-5")]
#[test]
fn a_zero_delay_loop_does_not_refute() {
    let outcome = check(&polling(false, true));
    assert!(matches!(outcome.safety, TimedSafetyOutcome::Holds { .. }));
    assert!(
        matches!(
            outcome.liveness,
            Some(TimedLivenessHalf::Checked(
                TimedLivenessOutcome::Holds { .. }
            ))
        ),
        "the liveness half holds: {:?}",
        outcome.liveness
    );
}

/// ADR-026 TD-6 beside FR-430-AC-5: with `send` guarded `x >= 1` and the
/// time invariant `when not self.sent { x <= 0 }`, `send` is never enabled
/// and the only behaviour is the zero-delay `poll` loop, so the subject
/// admits no time-divergent behaviour: the safety half settles
/// `NoAdmittedBehaviour`, a vacuous `Holds` never proving, and the
/// liveness half does not run.
#[trace("TC-1170", "FR-430-AC-5")]
#[test]
fn a_subject_with_only_a_zero_delay_loop_admits_no_behaviour() {
    let outcome = check(&polling(true, true));
    assert!(
        matches!(
            outcome.safety,
            TimedSafetyOutcome::Undecided {
                cause: TimedRefinementUndecided::Zone(ZoneUndecided::NoAdmittedBehaviour),
                ..
            }
        ),
        "{:?}",
        outcome.safety
    );
    assert!(outcome.liveness.is_none());
}

/// The abstract `send` contracts of FR-430-AC-5's `Req`.
const ASEND: &str = "pre ASendable using v on A::Call::send { not self.inflight }\n\
                     post ASend using v on A::Call::send { self.inflight }\n";

/// A refinement of `A` (operations `abstract_ops`) by `M` (clocks
/// `clocks`, operations `ops`) under the time declaration `time`, with
/// `contracts` and, after the `send` step row, `rows`; `late` and
/// `retried` map to `false`.
fn world_of(
    clocks: &[&str],
    ops: &[(&str, &[&str])],
    abstract_ops: &[(&str, &[&str])],
    time: &str,
    contracts: &str,
    rows: &str,
) -> Refining {
    let abstract_model = renamed_package(&document(&[], abstract_ops), "test/req");
    let model = document(clocks, ops);
    let body = refinement("self.inflight")
        .replace("late = self.late", "late = false")
        .replace("retried = self.retried", "retried = false")
        .replace(
            "step M::Call::send -> A::Call::send(self);",
            &format!("step M::Call::send -> A::Call::send(self);\n{rows}"),
        );
    refining(
        &abstract_model,
        &model,
        &format!("{time}{ASEND}{contracts}{body}"),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"))
}

/// The liveness half's violation of `outcome`, beside safety `Holds`.
fn violation(outcome: TimedRefinementOutcome<&'static str>) -> TemporalCounterexample {
    assert!(
        matches!(outcome.safety, TimedSafetyOutcome::Holds { .. }),
        "{:?}",
        outcome.safety
    );
    match outcome.liveness {
        Some(TimedLivenessHalf::Checked(TimedLivenessOutcome::Violated {
            counterexample, ..
        })) => *counterexample,
        other => panic!("the liveness half is violated: {other:?}"),
    }
}

/// The operation names of `counterexample`'s steps.
fn names(counterexample: &TemporalCounterexample) -> Vec<String> {
    counterexample
        .prefix
        .iter()
        .map(|step| {
            let operation = step.transition.operation();
            let name = operation.rsplit("/Call/").next().unwrap_or_default();
            name.split('"').next().unwrap_or_default().to_owned()
        })
        .collect()
}

const POLL: &str = "pre MSend using v on M::Call::send { not self.inflight }\n\
                    post MSent using v on M::Call::send { self.inflight }\n\
                    pre MPoll using v on M::Call::poll { not self.inflight }\n\
                    post MPolled using v on M::Call::poll { self.x = 0 ms }\n";

/// SR-1702001 FND-001: the liveness half's counts are its own product's:
/// the AC-5 violation's product, with the divergence clock and its delay
/// edges, stores more symbolic states than the safety half's.
#[trace("TC-1170", "FR-430-AC-5")]
#[test]
fn the_liveness_half_counts_its_own_product() {
    let outcome = check(&polling(false, false));
    let TimedSafetyOutcome::Holds { stats: safety, .. } = outcome.safety else {
        panic!("the safety half holds");
    };
    let Some(TimedLivenessHalf::Checked(TimedLivenessOutcome::Violated { stats, .. })) =
        outcome.liveness
    else {
        panic!("the liveness half is violated");
    };
    assert!(
        stats.symbolic_states > safety.symbolic_states,
        "{stats:?} {safety:?}"
    );
    assert!(stats.retained_edges > 0);
}

/// FR-143 over `F_C` (FR-232 over time): with `assume fair weak
/// M::Call::send`, the `poll` loop leaves the unguarded `send` enabled
/// throughout and never takes it, so it is unfair and does not refute: the
/// liveness half holds.
#[trace("TC-1170", "FR-430-AC-5")]
#[test]
fn a_loop_unfair_to_a_concrete_constraint_does_not_refute() {
    let world = world_of(
        &["x"],
        &[("send", &["inflight"]), ("poll", &["x"])],
        &[("send", &["inflight"])],
        DENSE,
        POLL,
        "step M::Call::poll -> stutter;\nassume fair weak M::Call::send;\n\
         ensure fair weak A::Call::send;",
    );
    let outcome = check(&world);
    assert!(
        matches!(
            outcome.liveness,
            Some(TimedLivenessHalf::Checked(
                TimedLivenessOutcome::Holds { .. }
            ))
        ),
        "{:?}",
        outcome.liveness
    );
}

/// FR-143 over `F_C`: `tick` (guard `x <= 0`) under `assume fair weak
/// M::Call::tick` is enabled only at the instant after each `poll`, so a
/// fair loop takes it; the loop of `poll` and `tick` with positive delay
/// still never takes `send`: `Divergence`, replayed.
#[trace("TC-1170", "FR-430-AC-5")]
#[test]
fn a_loop_fair_to_a_concrete_constraint_refutes() {
    let world = world_of(
        &["x"],
        &[("send", &["inflight"]), ("poll", &["x"]), ("tick", &[])],
        &[("send", &["inflight"])],
        DENSE,
        &format!(
            "{POLL}pre MTick using v on M::Call::tick {{ not self.inflight and self.x <= 0 ms }}\n"
        ),
        "step M::Call::poll -> stutter;\nstep M::Call::tick -> stutter;\n\
         assume fair weak M::Call::tick;\nensure fair weak A::Call::send;",
    );
    let counterexample = violation(check(&world));
    let entry = counterexample.lasso.expect("a lasso").entry;
    assert!(names(&counterexample)[entry..].contains(&"tick".to_owned()));
    assert!(matches!(
        counterexample.refinement,
        Some(RefinementFailure::Divergence { .. })
    ));
    assert!(matches!(
        replay(&world, &counterexample),
        RefinementTraceSettlement::Reproduced { .. }
    ));
}

/// FR-143: a loop of `ping`, which maps to the abstract `ping`, takes an
/// abstract step on every pass but never `send`, which is enabled
/// throughout: `AbstractUnfair{constraint: send}`, replayed.
#[trace("TC-1170", "FR-430-AC-5")]
#[test]
fn a_loop_of_other_abstract_steps_is_abstract_unfair() {
    let world = world_of(
        &["x"],
        &[("send", &["inflight"]), ("ping", &["x"])],
        &[("send", &["inflight"]), ("ping", &[])],
        DENSE,
        "pre MSend using v on M::Call::send { not self.inflight }\n\
         post MSent using v on M::Call::send { self.inflight }\n\
         pre MPing using v on M::Call::ping { not self.inflight }\n\
         post MPinged using v on M::Call::ping { self.x = 0 ms }\n",
        "step M::Call::ping -> A::Call::ping(self);\nensure fair weak A::Call::send;",
    );
    let counterexample = violation(check(&world));
    assert!(names(&counterexample).iter().all(|name| name == "ping"));
    assert!(
        matches!(
            counterexample.refinement,
            Some(RefinementFailure::AbstractUnfair { .. })
        ),
        "{:?}",
        counterexample.refinement
    );
    assert!(matches!(
        replay(&world, &counterexample),
        RefinementTraceSettlement::Reproduced {
            failure: RefinementFailure::AbstractUnfair { .. },
            ..
        }
    ));
}

/// The shortest stem: `start` (to `late`) then the `poll` loop, which
/// `late` enables. The search's stem to the loop ends with a `poll` that
/// the loop repeats; rolled back, the lasso is `start` then a loop of one
/// `poll`, entered at position 1, and it replays.
#[trace("TC-1170", "FR-430-AC-5")]
#[test]
fn a_lasso_is_rolled_back_to_its_shortest_stem() {
    let world = world_of(
        &["x"],
        &[
            ("send", &["inflight"]),
            ("poll", &["x"]),
            ("start", &["late"]),
        ],
        &[("send", &["inflight"])],
        DENSE,
        "pre MSend using v on M::Call::send { not self.inflight }\n\
         post MSent using v on M::Call::send { self.inflight }\n\
         pre MPoll using v on M::Call::poll { not self.inflight and self.late }\n\
         post MPolled using v on M::Call::poll { self.x = 0 ms }\n\
         pre MStart using v on M::Call::start { not self.late }\n\
         post MStarted using v on M::Call::start { self.late }\n",
        "step M::Call::poll -> stutter;\nstep M::Call::start -> stutter;\n\
         ensure fair weak A::Call::send;",
    );
    let counterexample = violation(check(&world));
    assert_eq!(names(&counterexample), ["start", "poll"]);
    assert_eq!(
        counterexample.lasso,
        Some(Lasso {
            entry: 1,
            terminal_stutter: false,
        })
    );
    assert!(matches!(
        replay(&world, &counterexample),
        RefinementTraceSettlement::Reproduced {
            failure: RefinementFailure::Divergence { .. },
            ..
        }
    ));
}

/// FR-231's idle tail: a quiescent state (no time invariant, not urgent)
/// may idle forever with time diverging. The initial state is one, and
/// idling there never takes `send`, which is enabled throughout: a lasso of
/// the terminal stutter at position 0, `Divergence`, replayed.
#[trace("TC-1170", "FR-430-AC-5")]
#[test]
fn an_idle_tail_that_never_takes_an_enabled_class_refutes() {
    let world = world_of(
        &["x"],
        &[("send", &["inflight"])],
        &[("send", &["inflight"])],
        DENSE,
        "pre MSend using v on M::Call::send { not self.inflight }\n\
         post MSent using v on M::Call::send { self.inflight }\n",
        "ensure fair weak A::Call::send;",
    );
    let counterexample = violation(check(&world));
    assert!(counterexample.prefix.is_empty());
    assert_eq!(
        counterexample.lasso,
        Some(Lasso {
            entry: 0,
            terminal_stutter: true,
        })
    );
    assert!(matches!(
        replay(&world, &counterexample),
        RefinementTraceSettlement::Reproduced {
            failure: RefinementFailure::Divergence { .. },
            ..
        }
    ));
}

/// FR-143 on the timed path: a strong `F_A` constraint leaves the liveness
/// half `Unsupported(StrongAbstractFairness)`, and a hidden abstract field
/// `Unsupported(HiddenFieldLiveness)`, whatever the safety half is.
#[trace("TC-1170", "FR-430-AC-5")]
#[test]
fn the_timed_liveness_half_is_unsupported_as_for_an_untimed_subject() {
    let rows = "step M::Call::poll -> stutter;\nensure fair strong A::Call::send;";
    let strong = world_of(
        &["x"],
        &[("send", &["inflight"]), ("poll", &["x"])],
        &[("send", &["inflight"])],
        DENSE,
        POLL,
        rows,
    );
    assert_eq!(
        format!("{:?}", check(&strong).liveness),
        format!(
            "{:?}",
            Some(TimedLivenessHalf::Unsupported(
                UnavailabilityCause::StrongAbstractFairness
            ))
        )
    );
    let model = document(&["x"], &[("send", &["inflight"]), ("poll", &["x"])]);
    let body = refinement("self.inflight")
        .replace(" late = self.late;", "")
        .replace("retried = self.retried", "retried = false")
        .replace(
            "step M::Call::send -> A::Call::send(self);",
            "step M::Call::send -> A::Call::send(self);\nstep M::Call::poll -> stutter;\n\
             ensure fair weak A::Call::send;",
        );
    let hidden = refining(&req(), &model, &format!("{DENSE}{ASEND}{POLL}{body}"))
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    assert_eq!(
        format!("{:?}", check(&hidden).liveness),
        format!(
            "{:?}",
            Some(TimedLivenessHalf::Unsupported(
                UnavailabilityCause::HiddenFieldLiveness
            ))
        )
    );
}

/// FR-241-AC-4's drift under refinement: `a` (to `late`, guard `x >= 1`,
/// resets `x`) and `b` (back, guard `y > 1`, resets `y`) under `x <= 1`
/// loop with time diverging and never take `send`, yet no loop repeats
/// with equal delays, so the violating component concretises to no lasso:
/// `Undecided(LassoNotConcretized)`.
#[trace("TC-1170", "FR-430-AC-5")]
#[test]
fn a_drifting_violation_is_undecided() {
    let world = world_of(
        &["x", "y"],
        &[
            ("send", &["inflight"]),
            ("a", &["late", "x"]),
            ("b", &["late", "y"]),
        ],
        &[("send", &["inflight"])],
        DENSE,
        "pre MSend using v on M::Call::send { not self.inflight and self.retried }\n\
         post MSent using v on M::Call::send { self.inflight }\n\
         pre MA using v on M::Call::a { not self.late and self.x >= 1 ms }\n\
         post MAed using v on M::Call::a { self.late and self.x = 0 ms }\n\
         pre MB using v on M::Call::b { self.late and self.y > 1 ms }\n\
         post MBed using v on M::Call::b { not self.late and self.y = 0 ms }\n\
         time invariant HoldIdle when { not self.late } { self.x <= 1 ms }\n\
         time invariant HoldBusy when { self.late } { self.x <= 1 ms }\n",
        "step M::Call::a -> stutter;\nstep M::Call::b -> stutter;\n\
         ensure fair weak A::Call::send;",
    );
    let outcome = check(&world);
    assert!(
        matches!(
            outcome.liveness,
            Some(TimedLivenessHalf::Checked(
                TimedLivenessOutcome::Undecided {
                    cause: TimedRefinementUndecided::Zone(ZoneUndecided::LassoNotConcretized),
                    ..
                }
            ))
        ),
        "{:?}",
        outcome.liveness
    );
}

/// Under a tick source time advances by ticks: ticking forever never takes
/// `send`, which is enabled throughout, so the liveness half is violated by
/// a lasso of ticks, `Divergence`, replayed.
#[trace("TC-1170", "FR-430-AC-5")]
#[test]
fn a_tick_source_refutes_by_a_loop_of_ticks() {
    let world = world_of(
        &[],
        &[("send", &["inflight"]), ("tick", &[])],
        &[("send", &["inflight"])],
        "dimension Time;\nunit s : Time = rational(1, 1);\n\
         unit ms : Time = rational(1, 1000) * s;\n\
         time M tick M::Call::tick period 1 ms unit ms;\n",
        "pre MSend using v on M::Call::send { not self.inflight and self.retried }\n\
         post MSent using v on M::Call::send { self.inflight }\n\
         pre MTick using v on M::Call::tick { not self.inflight }\n",
        "step M::Call::tick -> stutter;\nensure fair weak A::Call::send;",
    );
    let counterexample = violation(check(&world));
    let entry = counterexample.lasso.expect("a lasso").entry;
    assert!(names(&counterexample)[entry..]
        .iter()
        .all(|name| name == "tick"));
    assert!(matches!(
        counterexample.refinement,
        Some(RefinementFailure::Divergence { .. })
    ));
    assert!(matches!(
        replay(&world, &counterexample),
        RefinementTraceSettlement::Reproduced { .. }
    ));
}

/// FR-144 over EN-6 (TC-1170): the item's value is the liveness half's
/// once the safety half is proved. With safety `Holds` and its certificate
/// accepted, the `poll` loop's violation settles the item `refuted` through
/// the lasso's replay, carrying the lasso; with the time invariant the
/// liveness half holds and the item is `Proved{Exhaustive, Uncertified}`,
/// since no core checker covers that half (ADR-020 CT-4).
#[trace("TC-1170", "FR-430-AC-5")]
#[test]
fn the_item_settles_from_its_liveness_half_once_safety_is_proved() {
    use qsl_replay::{Certification, ProofBasis, TerminalValue};

    let violated = polling(false, false);
    assert!(matches!(
        check(&violated).safety,
        TimedSafetyOutcome::Holds { .. }
    ));
    let record = analyzed(&violated);
    let terminal = record.terminal().expect("a terminal record");
    assert_eq!(*terminal.value(), TerminalValue::Refuted);
    let counterexample = record.counterexample().expect("the replayed lasso");
    assert!(counterexample.lasso.is_some(), "{counterexample:?}");
    assert!(matches!(
        counterexample.refinement,
        Some(RefinementFailure::Divergence { .. })
    ));

    let holds = analyzed(&polling(false, true));
    assert_eq!(
        *holds.terminal().expect("a terminal record").value(),
        TerminalValue::Proved {
            basis: ProofBasis::Exhaustive,
            certification: Certification::Uncertified,
        }
    );
}

/// FR-144's settlement of the item from both halves (TC-1170), over the
/// `poll` world's outcome with its halves replaced: an `Unsupported`
/// liveness half after a proved safety half settles V-8 with its cause
/// (ADR-020 AX-5); a safety certificate the checker rejects keeps the
/// safety half's `CertificateRejected` record whatever the liveness half
/// is, and no lasso is replayed.
#[trace("TC-1170", "FR-430-AC-5")]
#[test]
fn the_item_keeps_the_safety_record_unless_safety_is_proved() {
    use qsl_analyze::zone_check::settle_timed_refinement;
    use qsl_replay::{
        CertificateLocus, CertificateRejection, CertificateRule, InconclusiveCause, TerminalValue,
        ZoneCertificateVerdict,
    };

    let outcome = check(&polling(false, false));
    assert!(matches!(
        outcome.liveness,
        Some(TimedLivenessHalf::Checked(
            TimedLivenessOutcome::Violated { .. }
        ))
    ));
    let no_replay =
        |_: &TemporalCounterexample| -> Result<RefinementTraceSettlement, ReplayRefusal> {
            panic!("no lasso is replayed")
        };

    let unsupported = TimedRefinementOutcome {
        safety: outcome.safety.clone(),
        liveness: Some(TimedLivenessHalf::Unsupported(
            UnavailabilityCause::StrongAbstractFairness,
        )),
    };
    let record = settle_timed_refinement(
        &unsupported,
        &Cancel::new(),
        |_| Ok(ZoneCertificateVerdict::Accepted),
        no_replay,
    );
    assert_eq!(
        record.value,
        TerminalValue::Unsupported(UnavailabilityCause::StrongAbstractFairness)
    );

    let rejection = CertificateRejection {
        rule: CertificateRule::BadState,
        at: CertificateLocus::ZoneNode { node: 0 },
    };
    let record = settle_timed_refinement(
        &outcome,
        &Cancel::new(),
        |_| Ok(ZoneCertificateVerdict::Rejected(rejection.clone())),
        no_replay,
    );
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
            rule: rejection.rule,
            at: rejection.at.clone(),
        })
    );
}
