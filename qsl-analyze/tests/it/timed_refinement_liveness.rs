// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-430 "Timed refinement" (TC-1170), the liveness half: FR-143's
//! second phase on EN-6's symbolic product reads only time-divergent
//! concrete behaviours (ADR-026 TS-4), so only a loop with positive total
//! delay refutes (EZ-6).

use ix_trace_rs::trace;
use qsl_analyze::zone_check::{
    check_timed_refinement, TimedLivenessHalf, TimedLivenessOutcome, TimedRefinementOutcome,
    TimedRefinementRequest, TimedRefinementUndecided, TimedSafetyOutcome, ZoneCheckLimits,
    ZoneUndecided,
};
use qsl_eval::simulation::Limits;
use qsl_replay::{
    open_systems, replay_timed_refinement_trace, Lasso, ModelTraceRefusal, RefinementFailure,
    RefinementTraceSettlement, ReplayRefusal,
};
use quire_exact::{Cancel, Integer, Rational};

use crate::timed_refinement::{refinement, replay};
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
fn check(world: &Refining) -> TimedRefinementOutcome<String> {
    let [refinement] = world.package.graph().refinements() else {
        panic!("one refinement");
    };
    check_timed_refinement(
        TimedRefinementRequest {
            refinement,
            obligation: refinement.name().to_owned(),
            concrete: world.concrete_request(),
            abstract_package: &world.package,
            abstract_initial: vec![world.abstract_initial.clone()],
            limits: ZoneCheckLimits::default(),
        },
        &Cancel::new(),
    )
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
            constraint: constraint.clone()
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
/// `Checked(HoldsReduced{reductions: []})` beside safety `Holds`.
#[trace("TC-1170", "FR-430-AC-5")]
#[test]
fn a_zero_delay_loop_does_not_refute() {
    let outcome = check(&polling(false, true));
    assert!(matches!(outcome.safety, TimedSafetyOutcome::Holds { .. }));
    let Some(TimedLivenessHalf::Checked(TimedLivenessOutcome::HoldsReduced { reductions, .. })) =
        outcome.liveness
    else {
        panic!("the liveness half holds: {:?}", outcome.liveness);
    };
    assert!(reductions.is_empty());
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
