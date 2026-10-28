// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-430 "Timed refinement" (TC-1170): S3 admits a refinement whose
//! concrete model is timed and whose abstract model is untimed, and refuses
//! a timed abstract model and a clock read in a row; EN-6 decides its
//! safety half on the symbolic product of zones with FR-142's discrete
//! product.

use ix_trace_rs::trace;
use qsl_analyze::zone_check::{
    check_timed_model, check_timed_refinement, TimedRefinementRequest, TimedSafetyOutcome,
    ZoneCheckItem, ZoneCheckLimits, ZoneCheckOutcome, ZoneCheckRequest,
};
use qsl_eval::simulation::model::{ModelSystem, TimedSystem};
use qsl_eval::simulation::Limits;
use qsl_replay::{
    open_systems, replay_timed_refinement_trace, RefinementFailure, RefinementTraceSettlement,
    TemporalCounterexample,
};
use qsl_route::deadlock::{ItemObligation, ObligationKind};
use qsl_route::reductions::{ObligationReductions, Reductions};
use quire_exact::{Cancel, Rational};

use crate::timed_world::{document, refining, renamed_package, Refining, DENSE};

/// FR-430-AC-1's untimed `Req`: the object `c` (here `Call`) with `sent`
/// (here `inflight`), and `send` setting it.
fn req() -> Vec<u8> {
    renamed_package(&document(&[], &[("send", &["inflight"])]), "test/req")
}

/// FR-430-AC-1's timed `TReq`: `Req` with the clock `x`, `send` guarded
/// `x >= 1`.
fn treq() -> Vec<u8> {
    document(&["x"], &[("send", &["inflight"])])
}

/// The concrete model's steps of `counterexample`, each with its delay.
fn steps(counterexample: &TemporalCounterexample) -> Vec<(String, Rational)> {
    counterexample
        .prefix
        .iter()
        .map(|step| {
            let operation = step.transition.operation();
            let name = ["send", "enter", "undo"]
                .into_iter()
                .find(|name| operation.contains(&format!("/Call/{name}\"")))
                .unwrap_or_else(|| panic!("an operation of the fixture: {operation}"));
            (
                name.to_owned(),
                step.delay.clone().expect("a timed step has a delay"),
            )
        })
        .collect()
}

/// EN-6's safety half of `world`'s one refinement.
fn safety(world: &Refining) -> TimedSafetyOutcome {
    let [refinement] = world.package.graph().refinements() else {
        panic!("one refinement");
    };
    check_timed_refinement(
        TimedRefinementRequest {
            refinement,
            concrete: world.concrete_request(),
            abstract_package: &world.package,
            abstract_initial: vec![world.abstract_initial.clone()],
            limits: ZoneCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect("EN-6 takes the refinement")
}

const CONTRACTS: &str = "post ASend using v on A::Call::send { self.inflight }\n\
     pre MSend using v on M::Call::send { self.x >= 1 ms }\n\
     post MSent using v on M::Call::send { self.inflight }\n";

/// The refinement of `A` by `M` with the field row `inflight = {row}`.
fn refinement(row: &str) -> String {
    format!(
        "refinement Timed using inf abstract A concrete M {{\n\
         population A::calls from M::calls;\n\
         object A::Call from M::Call {{ phase = self.phase; inflight = {row}; late = self.late; retried = self.retried; }}\n\
         step M::Call::send -> A::Call::send(self);\n\
         }}\n"
    )
}

fn unit(row: &str) -> Result<Refining, Vec<(String, String, String)>> {
    refining(
        &req(),
        &treq(),
        &format!("{DENSE}{CONTRACTS}{}", refinement(row)),
    )
}

#[trace("TC-1170", "FR-430-AC-1")]
#[test]
fn s3_admits_a_timed_concrete_model_under_an_untimed_abstract_one() {
    let world = unit("self.inflight").unwrap_or_else(|refusals| panic!("{refusals:?}"));
    assert_eq!(world.package.graph().refinements().len(), 1);
}

#[trace("TC-1170", "FR-430-AC-1")]
#[test]
fn a_clock_read_in_the_mapping_is_refused_at_the_read() {
    let Err(refusals) = unit("self.x >= 1 ms") else {
        panic!("a clock read in the mapping is refused");
    };
    assert_eq!(
        refusals,
        [(
            "unsupported_construct".to_owned(),
            "expression-form".to_owned(),
            "self.x".to_owned()
        )]
    );
}

/// FR-430-AC-1: a refinement whose abstract model is `TReq` and whose
/// concrete model is a copy `TReq2` of it.
#[trace("TC-1170", "FR-430-AC-1")]
#[test]
fn a_timed_abstract_model_is_refused_at_its_reference() {
    let treq2 = renamed_package(&treq(), "test/treq2");
    let Err(refusals) = refining(
        &treq2,
        &treq(),
        &format!(
            "{DENSE}time A dense unit ms;\n{CONTRACTS}\
             pre ASend2 using v on A::Call::send {{ self.x >= 1 ms }}\n{}",
            refinement("self.inflight")
        ),
    ) else {
        panic!("a timed abstract model is refused");
    };
    assert_eq!(
        refusals,
        [(
            "unsupported_construct".to_owned(),
            "declaration-form".to_owned(),
            "A".to_owned()
        )]
    );
}

/// FR-430-AC-1: `TReq` refines `Req`, and EN-6's product holds. Its edges
/// are the discrete steps only: from `sent = false` and `x >= 0`, `send` at
/// `x >= 1` reaches `sent = true` and `x >= 1`, and `send` again is covered
/// by that state; no delay is an edge, so `check_step` runs for `send`
/// alone.
#[trace("TC-1170", "FR-430-AC-1")]
#[test]
fn the_safety_half_of_a_timed_refinement_holds_on_the_zone_product() {
    let world = unit("self.inflight").unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let TimedSafetyOutcome::Holds { stats } = safety(&world) else {
        panic!("the safety half holds");
    };
    assert_eq!((stats.symbolic_states, stats.retained_edges), (2, 2));
}

/// FR-430 "Timed refinement": `check_step` runs on each discrete edge. With
/// `send` mapped to `stutter`, its edge changes the mapped `sent`, so the
/// half is violated at position 1, after the least admissible delay 1.
#[trace("TC-1170", "FR-430-AC-1")]
#[test]
fn a_discrete_edge_that_fails_check_step_refutes_after_its_delay() {
    let body = refinement("self.inflight").replace("-> A::Call::send(self)", "-> stutter");
    let world = refining(&req(), &treq(), &format!("{DENSE}{CONTRACTS}{body}"))
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let TimedSafetyOutcome::Violated { counterexample, .. } = safety(&world) else {
        panic!("the safety half is violated");
    };
    assert_eq!(
        steps(&counterexample),
        [("send".to_owned(), Rational::from_integer(1_i64.into()))]
    );
    assert!(
        matches!(
            counterexample.refinement,
            Some(RefinementFailure::StutterChanged { position: 1 })
        ),
        "{:?}",
        counterexample.refinement
    );
}

/// FR-430-AC-2's `TReq` with `trap` (here `late`): `send` resets `x`;
/// `enter` (precondition `sent`, guard `x >= 1`) sets `trap` and resets
/// `x`; `undo` (precondition `trap and sent`) clears `sent`. `trap` maps to
/// the abstract `late`, which stays false: the trap is no abstract state.
/// With `trapped`, the time invariant `when self.late { x <= 0 }`.
fn trap_undo(trapped: bool) -> Refining {
    let model = document(
        &["x"],
        &[
            ("send", &["inflight", "x"]),
            ("enter", &["late", "x"]),
            ("undo", &["inflight"]),
        ],
    );
    let contracts = "post ASend using v on A::Call::send { self.inflight }\n\
         pre MSend using v on M::Call::send { self.x >= 1 ms }\n\
         post MSent using v on M::Call::send { self.inflight and self.x = 0 ms }\n\
         pre MEnter using v on M::Call::enter { self.inflight and self.x >= 1 ms }\n\
         post MEntered using v on M::Call::enter { self.late and self.x = 0 ms }\n\
         pre MUndo using v on M::Call::undo { self.late and self.inflight }\n\
         post MUndone using v on M::Call::undo { not self.inflight }\n";
    let body = refinement("self.inflight")
        .replace("late = self.late", "late = false")
        .replace(
            "step M::Call::send -> A::Call::send(self);",
            "step M::Call::send -> A::Call::send(self);\n\
             step M::Call::enter -> stutter;\nstep M::Call::undo -> stutter;",
        );
    let trap = if trapped {
        "time invariant Trap when { self.late } { self.x <= 0 ms }\n"
    } else {
        ""
    };
    refining(&req(), &model, &format!("{DENSE}{contracts}{trap}{body}"))
        .unwrap_or_else(|refusals| panic!("{refusals:?}"))
}

/// FR-145 over time (QSpec FR-430): `world`'s refinement counterexample
/// `payload` replayed through FR-241's exact delays.
fn replay(world: &Refining, payload: &TemporalCounterexample) -> RefinementTraceSettlement {
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
    replay_timed_refinement_trace(refinement, &systems, payload, &Limits::default())
        .expect("the counterexample replays")
}

/// FR-430-AC-2: without the time invariant, the safety half is violated by
/// `send` after delay 1, `enter` after delay 1 and `undo` after delay 0,
/// failing at position 3, and the counterexample replays through FR-241's
/// delays. With it, the only pre-state points of `undo` (trap set, `x = 0`)
/// reach no time-divergent continuation, so the failing edge does not
/// refute (ADR-026 TV-1) and the half holds, while the time-lock-freedom
/// item of that model is refuted. The same counterexample replayed over the
/// trapped model reproduces the failing step but settles inconclusive,
/// since its pre-state point is time-locked.
#[trace("TC-1170", "FR-430-AC-2")]
#[test]
fn a_failing_undo_refutes_only_where_time_can_diverge() {
    let free = trap_undo(false);
    let TimedSafetyOutcome::Violated { counterexample, .. } = safety(&free) else {
        panic!("without the invariant the half is violated");
    };
    let one = Rational::from_integer(1_i64.into());
    assert_eq!(
        steps(&counterexample),
        [
            ("send".to_owned(), one.clone()),
            ("enter".to_owned(), one),
            ("undo".to_owned(), Rational::from_integer(0_i64.into())),
        ]
    );
    assert_eq!(
        counterexample.refinement,
        Some(RefinementFailure::StutterChanged { position: 3 })
    );
    assert!(
        matches!(
            replay(&free, &counterexample),
            RefinementTraceSettlement::Reproduced {
                failure: RefinementFailure::StutterChanged { position: 3 },
                ..
            }
        ),
        "{:?}",
        replay(&free, &counterexample)
    );

    let trapped = trap_undo(true);
    let outcome = safety(&trapped);
    assert!(
        matches!(outcome, TimedSafetyOutcome::Holds { .. }),
        "{outcome:?}"
    );
    let system = ModelSystem::new(trapped.concrete_request()).expect("the system admits");
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let time_lock = check_timed_model(
        ZoneCheckRequest {
            subject: &timed,
            item: ZoneCheckItem::TimeLockFreedom,
            obligation: ItemObligation {
                subject: "trap-undo",
                kind: ObligationKind::TimeLockFreedom,
                reductions: ObligationReductions::default(),
            },
            limits: ZoneCheckLimits::default(),
            reductions: &Reductions::default(),
        },
        &Cancel::new(),
    )
    .expect("EN-6 takes the item");
    assert!(
        matches!(time_lock, ZoneCheckOutcome::Violated { .. }),
        "{time_lock:?}"
    );
    assert!(
        matches!(
            replay(&trapped, &counterexample),
            RefinementTraceSettlement::Inconclusive(_)
        ),
        "{:?}",
        replay(&trapped, &counterexample)
    );
}

/// FR-145 over time: a counterexample whose delay breaks a guard is
/// refused, as FR-237 refuses it, before any refinement check runs.
#[trace("TC-1170", "FR-430-AC-2")]
#[test]
fn a_timed_refinement_counterexample_with_an_early_step_is_refused() {
    let free = trap_undo(false);
    let TimedSafetyOutcome::Violated {
        mut counterexample, ..
    } = safety(&free)
    else {
        panic!("the half is violated");
    };
    counterexample.prefix[0].delay = Some(Rational::from_integer(0_i64.into()));
    let [refinement] = free.package.graph().refinements() else {
        panic!("one refinement");
    };
    let systems = open_systems(
        refinement,
        &free.concrete_request(),
        &free.package,
        vec![free.abstract_initial.clone()],
    )
    .expect("the subjects open");
    assert!(replay_timed_refinement_trace(
        refinement,
        &systems,
        &counterexample,
        &Limits::default()
    )
    .is_err());
}

/// FR-430-AC-1: a clock read in a step row is refused at the read.
#[trace("TC-1170", "FR-430-AC-1")]
#[test]
fn a_clock_read_in_a_step_row_is_refused_at_the_read() {
    let body = refinement("self.inflight").replace("A::Call::send(self)", "A::Call::send(self.x)");
    let Err(refusals) = refining(&req(), &treq(), &format!("{DENSE}{CONTRACTS}{body}")) else {
        panic!("a clock read in a step row is refused");
    };
    assert_eq!(
        refusals,
        [(
            "unsupported_construct".to_owned(),
            "expression-form".to_owned(),
            "self.x".to_owned()
        )]
    );
}
