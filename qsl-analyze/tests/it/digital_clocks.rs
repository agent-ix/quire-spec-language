// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-204 (TC-639, TC-642): EN-5's digital-clock product over ADR-028
//! §15.5's `Retx`, decided by the finite-horizon solver.

use ix_trace_rs::trace;
use qsl_analyze::exact_probabilistic::{
    build_product, decide_bounded_event, ExactProbLimits, FhResult, ProbBuild, ProbProduct,
    ProductAction, ProductKind, ProductRefusal, ProductRequest,
};
use qsl_eval::model_subject::ModelSubject;
use qsl_semantics::check::Bound;
use quire_exact::{Cancel, Integer, Rational};
use serde_json::json;

use crate::timed_world::{self, World};

fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("nonzero")
}

/// `Retx`: `late` is `delivered`, `send(lost)` needs `x >= guard` and resets
/// `x`, the invariant `x <= 2 ms` holds until delivery.
fn retx(guard: &str, reset: bool, declarations: &str, claims: &str) -> World {
    let mut document: serde_json::Value =
        serde_json::from_slice(&timed_world::document(&["x"], &[("send", &["late", "x"])]))
            .expect("the timed fixture is JSON");
    let operation = &mut document["types"][1]["operations"][0];
    operation["params"] = json!([{
        "identity": timed_world::ident("Call/send/lost"),
        "name": "lost",
        "typeRef": "ix://quire/native/Boolean",
        "presence": "required",
        "nullable": false,
        "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": {"source": {"sourceIdentity": timed_world::ident("spec"),
            "path": "spec.qspec", "startLine": 1, "startColumn": 1}},
    }]);
    let reset = if reset { " and self.x = 0 ms" } else { "" };
    let body = format!(
        "{}\
         pre Send using v on M::Call::send {{ not self.late{guard} }}\n\
         post Sent using v on M::Call::send {{ self.late = not lost{reset} }}\n\
         random M::Call::send(lost) ~ {{ true: 1, false: 9 }};\n\
         time invariant Deadline when {{ not self.late }} {{ self.x <= 2 ms }}\n\
         {declarations}{claims}",
        timed_world::DENSE,
    );
    timed_world::world(
        &serde_json::to_vec(&document).expect("serializes"),
        "profile ev = \"quire.temporal.timed/v1\";\n",
        &body,
    )
}

const GUARD: &str = " and self.x >= 1 ms";

fn deadline_claim(name: &str, scheduler: &str, bound: &str, body: &str) -> String {
    format!(
        "probabilistic {name} using ev on M over (c: M::Call) {scheduler} on origin {{ \
         probability {bound} [ {body}[0 ms, 4 ms] holds(c.late) ] }}\n"
    )
}

fn build(world: &World, claim: &str) -> Result<ProbBuild, ProductRefusal> {
    let claim = world
        .package
        .graph()
        .probabilistic_model()
        .claim(claim)
        .expect("the claim checks");
    build_product(
        ProductRequest {
            subject: ModelSubject {
                simulation: world.request(),
                proof_bounds: Vec::new(),
            },
            claim,
            limits: ExactProbLimits::default(),
        },
        &Cancel::new(),
    )
}

fn product(world: &World, claim: &str) -> ProbProduct {
    match build(world, claim) {
        Ok(ProbBuild::Built(product)) => product,
        other => panic!("{claim}: expected a product, got {other:?}"),
    }
}

fn refusal(world: &World, claim: &str) -> ProductRefusal {
    build(world, claim).expect_err("the build is refused")
}

fn initial_value(product: &ProbProduct, bound: Bound) -> Rational {
    let result = decide_bounded_event(product, &bound, &ExactProbLimits::default())
        .expect("the digital product is acyclic");
    let FhResult::Exact { values, .. } = result else {
        panic!("exact values: {result:?}");
    };
    values[product.initial[0].2.index()].clone()
}

fn every(guard: &str, reset: bool) -> World {
    retx(
        guard,
        reset,
        "",
        &format!(
            "{}{}{}",
            deadline_claim("Deadline", "under every scheduler", ">= 0.99", "eventually"),
            deadline_claim("Max", "under every scheduler", "<= 0.999", "eventually"),
            deadline_claim("Always", "under every scheduler", ">= 0.99", "always"),
        ),
    )
}

/// FR-204-AC-1 (TC-639 step 1): over every scheduler the minimum is `99/100`
/// and the maximum `9999/10000`.
#[trace("TC-639", "FR-204-AC-1")]
#[test]
fn deadline_extrema_over_every_scheduler() {
    let world = every(GUARD, true);
    let built = product(&world, "Deadline");
    assert_eq!(built.kind, ProductKind::Mdp);
    assert_eq!(
        initial_value(&built, Bound::AtLeast(ratio(99, 100))),
        ratio(99, 100)
    );
    assert_eq!(
        initial_value(&built, Bound::AtMost(ratio(999, 1000))),
        ratio(9999, 10000)
    );
    // The scheduler's delays are named unit delays, not an operation.
    assert!(built
        .edges
        .iter()
        .any(|edge| matches!(edge.choice, ProductAction::UnitDelay) && edge.action.is_none()));
}

/// FR-204-AC-4 (TC-639 step 4): a `send` without guard or reset is a
/// zero-delay cycle, and an `always` event has no digital reading.
#[trace("TC-639", "FR-204-AC-4")]
#[test]
fn zero_delay_cycle_and_form_shape_are_unsupported() {
    let cyclic = every("", false);
    let ProductRefusal::ZeroDelayCycle { states } = refusal(&cyclic, "Deadline") else {
        panic!("a zero-delay cycle");
    };
    assert!(!states.is_empty());
    let world = every(GUARD, true);
    assert!(matches!(
        refusal(&world, "Always"),
        ProductRefusal::TimedFormShape { .. }
    ));
}

/// FR-204-AC-2 (TC-639 step 2): a strict guard has no digital reading.
#[trace("TC-639", "FR-204-AC-2")]
#[test]
fn a_strict_guard_is_unsupported() {
    let world = every(" and self.x > 1 ms", true);
    assert!(matches!(
        refusal(&world, "Deadline"),
        ProductRefusal::StrictClockConstraint { .. }
    ));
}

fn one_workload(declarations: &str) -> World {
    retx(
        GUARD,
        true,
        &format!("{declarations}workload One on M {{ weight Call::send = 1; }}\n"),
        &format!(
            "{}{}",
            deadline_claim("Under", "under One", ">= 0.99", "eventually"),
            deadline_claim("UnderMax", "under One", "<= 0.999", "eventually"),
        ),
    )
}

/// FR-204-AC-3 (TC-639 step 3): a `discrete` delay under a workload is a DTMC
/// of exact value `159129/160000`; the same declaration over every scheduler
/// and an `exponential` one under the workload are unsupported.
#[trace("TC-639", "FR-204-AC-3")]
#[test]
fn discrete_delays_under_a_workload_are_a_chain() {
    let world = one_workload("delay M::Call::send ~ discrete { 1 ms: 1, 2 ms: 1 };\n");
    let built = product(&world, "Under");
    assert_eq!(built.kind, ProductKind::Dtmc);
    assert_eq!(
        initial_value(&built, Bound::AtLeast(ratio(99, 100))),
        ratio(159_129, 160_000)
    );

    let scheduler = retx(
        GUARD,
        true,
        "delay M::Call::send ~ discrete { 1 ms: 1, 2 ms: 1 };\n",
        &deadline_claim("Deadline", "under every scheduler", ">= 0.99", "eventually"),
    );
    let ProductRefusal::DelayDistribution { operation } = refusal(&scheduler, "Deadline") else {
        panic!("a delay distribution");
    };
    assert!(operation.contains("send"), "{operation}");

    let exponential = one_workload("delay M::Call::send ~ exponential(1 per ms);\n");
    assert!(matches!(
        refusal(&exponential, "Under"),
        ProductRefusal::DelayDistribution { .. }
    ));
}

/// FR-204-AC-5 (TC-642): under a workload that gives no delay distribution the
/// delay stays the scheduler's: minimum `99/100`, maximum `9999/10000`.
#[trace("TC-642", "FR-204-AC-5")]
#[test]
fn free_delays_under_a_workload_are_decided_by_minimum_and_maximum() {
    let world = one_workload("");
    let built = product(&world, "Under");
    assert_eq!(built.kind, ProductKind::Mdp);
    assert_eq!(
        initial_value(&built, Bound::AtLeast(ratio(99, 100))),
        ratio(99, 100)
    );
    assert_eq!(
        initial_value(&built, Bound::AtMost(ratio(999, 1000))),
        ratio(9999, 10000)
    );
}
