// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-430 "Timed refinement" (TC-1170): S3 admits a refinement whose
//! concrete model is timed and whose abstract model is untimed, and refuses
//! a timed abstract model and a clock read in a row; EN-6 decides its
//! safety half on the symbolic product of zones with FR-142's discrete
//! product.

use ix_trace_rs::trace;
use qsl_analyze::zone_check::{
    check_timed_refinement, TimedRefinementRequest, TimedSafetyOutcome, ZoneCheckLimits,
};
use qsl_replay::{RefinementFailure, TemporalCounterexample};
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

/// ADR-026 TV-1: a failing edge refutes only when its pre-state has a
/// time-divergent valuation. `enter` sets `trap` (here `late`) and resets
/// `x`, under the time invariant `when self.late { x <= 0 }`; `undo`, mapped
/// to `stutter`, clears `sent` from there, and every behaviour through its
/// pre-state is time-locked, so the half holds. Without the invariant the
/// same edge refutes.
#[trace("TC-1170", "FR-430-AC-1")]
#[test]
fn a_failing_edge_from_time_locked_valuations_only_does_not_refute() {
    let model = document(
        &["x"],
        &[
            ("send", &["inflight", "x"]),
            ("enter", &["late", "x"]),
            ("undo", &["inflight"]),
        ],
    );
    let abstract_ = renamed_package(&document(&[], &[("send", &["inflight"])]), "test/req");
    let contracts = "post ASend using v on A::Call::send { self.inflight }\n\
         pre MSend using v on M::Call::send { self.x >= 1 ms }\n\
         post MSent using v on M::Call::send { self.inflight and self.x = 0 ms }\n\
         pre MEnter using v on M::Call::enter { self.inflight and self.x >= 1 ms }\n\
         post MEntered using v on M::Call::enter { self.late and self.x = 0 ms }\n\
         pre MUndo using v on M::Call::undo { self.late and self.inflight }\n\
         post MUndone using v on M::Call::undo { not self.inflight }\n";
    // The abstract `late` stays false: the concrete trap is no abstract
    // state.
    let body = refinement("self.inflight")
        .replace("late = self.late", "late = false")
        .replace(
            "step M::Call::send -> A::Call::send(self);",
            "step M::Call::send -> A::Call::send(self);\n\
         step M::Call::enter -> stutter;\nstep M::Call::undo -> stutter;",
        );
    let trapped = "time invariant Trap when { self.late } { self.x <= 0 ms }\n";
    let world = refining(
        &abstract_,
        &model,
        &format!("{DENSE}{contracts}{trapped}{body}"),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    assert!(
        matches!(safety(&world), TimedSafetyOutcome::Holds { .. }),
        "{:?}",
        safety(&world)
    );
    let world = refining(&abstract_, &model, &format!("{DENSE}{contracts}{body}"))
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let TimedSafetyOutcome::Violated { counterexample, .. } = safety(&world) else {
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
