// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-450 (TC-1221): static and exact replay of hybrid traces against
//! models and claims checked from real source: S1 (`qsl_cst::parse`), S2
//! (`qsl_forms::build_unit`), the assembler, then
//! `PackageDeclarations::check`.

use ix_trace_rs::trace;
use qsl_foundation::SourceIdentity;
use qsl_semantics::check::{
    CheckedGraph, CheckedHybridClaim, HybridTerm, ModeIndex, PackageDeclarations,
};
use quire_exact::Rational;
use quire_semantic_value::checking::CheckingLimits;

use super::expand::Expander;
use super::interval::{IValue, Interval, IntervalEvaluator};
use super::poly::Poly;
use super::*;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"quire.value.complete/v1\";\n\
     profile p = \"quire.temporal.infinite-trace/v1\";\n\
     profile t = \"quire.temporal.timed/v1\";\n\
     dimension Time;\n\
     unit s : Time = rational(1, 1);\n\
     unit ms : Time = rational(1, 1000) * s;\n";

/// `drag` is not recursive, but its body calls the recursive `slow`, whose
/// `decreases` measure is its `Int` parameter: the only way a hybrid term
/// reaches a recursive call, since a hybrid term passes no real to an
/// `Integer` or `Int` parameter (QSpec FR-193).
const FUNCTIONS: &str = "function slow using v(s: Rational[-100, 100; 1, 1], n: Int[0, 9]): \
     Rational[-100, 100; 1, 1] pure decreases(n) \
     { if n > 0 and s > rational(0, 1) then slow(s, n - 1) else s }\n\
     function drag using v(s: Rational[-100, 100; 1, 1]): Rational[-100, 100; 1, 1] pure \
     { slow(s, if s > rational(0, 1) then 1 else 2) }\n";

/// FR-447's `Braking`, with `Moving`'s flow and guard replaceable.
fn braking(flow: &str, guard: &str) -> String {
    format!(
        "hybrid Braking using p {{\n\
           mode Moving invariant {{ v >= 0 and v <= 20 and a >= 1 and a <= 2 }}\n\
             flow {{ {flow} }}\n\
             transition to Stopped when {{ {guard} }} reset {{ v = 0 }};\n\
           mode Stopped invariant {{ v = 0 }}\n\
             flow {{ v' = 0; a' = 0; t' = 1; }}\n\
         }}\n"
    )
}

const FLOW: &str = "v' = -a; a' = 0; t' = 1;";

const CLAIMS: &str =
    "temporal SlowEnough using p over (h: Braking) clock \"model-time\" on origin {\n\
       always [0, 10] holds(h.v < 30)\n\
     }\n\
     temporal Positive using p over (h: Braking) clock \"model-time\" on origin {\n\
       always [0, 12] holds(h.v > 0)\n\
     }\n\
     temporal Defined using p over (h: Braking) clock \"model-time\" on origin {\n\
       always [0, 12] holds(1 / (h.v - 5) != 0)\n\
     }\n\
     temporal Bounded using p over (h: Braking) clock \"model-time\" on origin {\n\
       always [0, 12] holds(1 / (h.v - 5) < 100)\n\
     }\n\
     temporal BoundedTo using t over (h: Braking) clock \"model-time\" on origin {\n\
       always [0 s, 7495 ms] holds(1 / (h.v - 5) < 100)\n\
     }\n\
     temporal BoundedBefore using t over (h: Braking) clock \"model-time\" on origin {\n\
       always [0 s, 7495 ms) holds(1 / (h.v - 5) < 100)\n\
     }\n\
     temporal Dragged using p over (h: Braking) clock \"model-time\" on origin {\n\
       always [0, 12] holds(drag(h.v) < 100)\n\
     }\n";

fn check(declarations: &str) -> CheckedGraph {
    let text = format!("{HEADER}{FUNCTIONS}{declarations}");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("{text}: the unit assembles: {refusal:?}"))
    .with_unwritten_hybrids()
    .check(CheckingLimits::default())
    .unwrap_or_else(|refusals| panic!("{text}: the unit checks: {refusals:?}"))
}

fn braking_unit(flow: &str, guard: &str) -> CheckedGraph {
    check(&format!("{}{CLAIMS}", braking(flow, guard)))
}

fn claim<'g>(graph: &'g CheckedGraph, name: &str) -> &'g CheckedHybridClaim {
    graph
        .hybrid_claims()
        .iter()
        .find(|claim| claim.name.as_str() == name)
        .unwrap_or_else(|| panic!("no claim {name}"))
}

fn q(numerator: i64, denominator: i64) -> Rational {
    Rational::new(numerator.into(), denominator.into()).expect("nonzero")
}

fn n(value: i64) -> Rational {
    q(value, 1)
}

fn point(value: Rational) -> ExactInterval {
    ExactInterval::point(value)
}

fn points(values: &[Rational]) -> Vec<ExactInterval> {
    values.iter().cloned().map(point).collect()
}

fn state(mode: usize, time: Rational, values: &[Rational]) -> TraceState {
    TraceState {
        mode: ModeIndex::new(mode),
        time: point(time),
        enclosure: points(values),
    }
}

fn before(time: Rational, values: &[Rational]) -> TraceBefore {
    TraceBefore {
        time: point(time),
        enclosure: points(values),
    }
}

fn pointer(text: &str) -> JsonPointer {
    text.parse().expect("an RFC 6901 pointer")
}

/// AC-2's point trace over `Braking` (`v`, `a`, `t`): from
/// `(Moving, v = 20, a = 2, t = 0)`, `before` `(time 10, v = 0, a = 2,
/// t = 10)`, and transition 0 to `(Stopped, time 10, v = 0, a = 2, t = 10)`.
fn braking_trace(before_time: i64, before_v: i64) -> HybridTrace {
    HybridTrace {
        start: state(0, n(0), &[n(20), n(2), n(0)]),
        jumps: vec![TraceJump {
            transition: 0,
            before: Some(before(n(before_time), &[n(before_v), n(2), n(10)])),
            state: state(1, n(10), &[n(0), n(2), n(10)]),
        }],
        delta: None,
        loop_to: None,
    }
}

fn reproduced_formula(trace: &HybridTrace) -> ExactReplay {
    ExactReplay::Reproduced(Box::new(HybridCounterexample {
        kind: HybridCounterexampleKind::Formula,
        trace: trace.clone(),
    }))
}

/// FR-450-AC-1 (replay half): static replay over `Braking` reports the
/// first definitely false check in trace order with its member's pointer,
/// widened by `delta`; a negative `delta` is refused at `/delta`.
#[trace("TC-1221", "FR-450-AC-1")]
#[test]
fn static_replay_reports_the_first_definitely_false_check() {
    let graph = braking_unit(FLOW, "v = 0");
    let claim = claim(&graph, "SlowEnough");

    // `start`'s `v` in [25, 26] meets no initial entry: `Moving`'s
    // invariant has `v <= 20`.
    let mut start_outside = braking_trace(10, 0);
    start_outside.start.enclosure[0] = ExactInterval::new(n(25), n(26));
    assert_eq!(
        static_replay(&graph, claim, &start_outside),
        Ok(StaticReplay::Inconsistent {
            at: pointer("/start"),
            check: StaticCheck::Initial,
        })
    );

    // `before`'s `v` in [1/200, 1/100] misses the guard `v = 0` by less
    // than the `delta` 1/100, and by more than none.
    let mut near_guard = braking_trace(10, 0);
    if let Some(before) = &mut near_guard.jumps[0].before {
        before.enclosure[0] = ExactInterval::new(q(1, 200), q(1, 100));
    }
    near_guard.delta = Some(q(1, 100));
    assert_eq!(
        static_replay(&graph, claim, &near_guard),
        Ok(StaticReplay::Consistent)
    );
    near_guard.delta = None;
    assert_eq!(
        static_replay(&graph, claim, &near_guard),
        Ok(StaticReplay::Inconsistent {
            at: pointer("/jumps/0"),
            check: StaticCheck::Jump,
        })
    );

    // The second state's `time.upper` is below the first's `time.lower`.
    let mut backwards = braking_trace(10, 0);
    backwards.jumps[0].before = None;
    backwards.jumps[0].state.time = point(n(-1));
    assert_eq!(
        static_replay(&graph, claim, &backwards),
        Ok(StaticReplay::Inconsistent {
            at: pointer("/jumps/0/state/time"),
            check: StaticCheck::Time,
        })
    );

    let mut negative = braking_trace(10, 0);
    negative.delta = Some(q(-1, 100));
    let refusal = TraceRefusal {
        pointer: pointer("/delta"),
        fault: TraceFault::NegativeDelta,
    };
    assert_eq!(
        (refusal.code(), refusal.cause()),
        ("invalid_result", "invalid-value")
    );
    assert_eq!(
        static_replay(&graph, claim, &negative),
        Err(HybridReplayRefusal::Trace(refusal.clone()))
    );
    assert_eq!(
        exact_replay(&graph, claim, &negative),
        Err(HybridReplayRefusal::Trace(refusal))
    );
}

/// FR-450-AC-2 (replay half): AC-2's point trace reproduces with kind
/// `Formula` against `always [0, 12] holds(h.v > 0)`, `v(t) = 20 - 2t`
/// reaching 0 at time 10; against `SlowEnough` it is not reproduced at check
/// `Claim`. Replay reads only the trace, so a trace from a
/// `solver-inconclusive` cause and from a provider's `refuted` entry replay
/// alike.
#[trace("TC-1221", "FR-450-AC-2")]
#[test]
fn the_point_trace_reproduces_a_violated_claim_and_not_a_held_one() {
    let graph = braking_unit(FLOW, "v = 0");
    let trace = braking_trace(10, 0);
    assert_eq!(
        exact_replay(&graph, claim(&graph, "Positive"), &trace),
        Ok(reproduced_formula(&trace))
    );
    assert_eq!(
        exact_replay(&graph, claim(&graph, "SlowEnough"), &trace),
        Ok(ExactReplay::NotReproduced {
            at: JsonPointer::root(),
            check: ExactCheck::Claim,
        })
    );
}

/// FR-450-AC-3 (replay half): the AC-2 trace with `before` `v = 1`, and
/// with `before.time` 9, is not reproduced at check `Dwell`: the solution
/// reaches `v = 0` at time 10 only.
#[trace("TC-1221", "FR-450-AC-3")]
#[test]
fn a_before_the_dwell_does_not_reach_is_not_reproduced_at_dwell() {
    let graph = braking_unit(FLOW, "v = 0");
    let claim = claim(&graph, "Positive");
    let dwell = Ok(ExactReplay::NotReproduced {
        at: pointer("/jumps/0/before"),
        check: ExactCheck::Dwell,
    });
    assert_eq!(exact_replay(&graph, claim, &braking_trace(10, 1)), dwell);
    assert_eq!(exact_replay(&graph, claim, &braking_trace(9, 0)), dwell);
}

/// FR-450-AC-4 (replay half): with `v' = -v` the trace is not exact and
/// replays statically only; with `v' = -a` and `a' = -1 / 10` the solution
/// is `a(t) = 2 - t / 10`, `v(t) = 20 - 2t + t^2 / 20`, and `v(10) = 5`
/// fails check `Dwell`; with `v' = drag(v)`, whose expansion calls the
/// recursive `slow`, the trace is not exact.
#[trace("TC-1221", "FR-450-AC-4")]
#[test]
fn only_a_polynomial_acyclic_flow_replays_exactly() {
    let trace = braking_trace(10, 0);

    let cyclic = braking_unit("v' = -v; a' = 0; t' = 1;", "v = 0");
    assert_eq!(
        exact_replay(&cyclic, claim(&cyclic, "Positive"), &trace),
        Ok(ExactReplay::NotExact)
    );
    assert_eq!(
        static_replay(&cyclic, claim(&cyclic, "Positive"), &trace),
        Ok(StaticReplay::Consistent)
    );

    let falling = braking_unit("v' = -a; a' = -1 / 10; t' = 1;", "v = 0");
    let solution = super::exact::probe::solution(
        &falling.hybrid_models()[0],
        &mut Expander::new(&falling),
        0,
        &[n(20), n(2), n(0)],
    )
    .expect("a polynomial flow");
    assert_eq!(
        solution,
        [
            Poly::new(vec![n(20), n(-2), q(1, 20)]),
            Poly::new(vec![n(2), q(-1, 10)]),
            Poly::new(vec![n(0), n(1)]),
        ]
    );
    assert_eq!(solution[0].eval(&n(10)), n(5));
    assert_eq!(
        exact_replay(&falling, claim(&falling, "Positive"), &trace),
        Ok(ExactReplay::NotReproduced {
            at: pointer("/jumps/0/before"),
            check: ExactCheck::Dwell,
        })
    );

    let dragged = braking_unit("v' = drag(v); a' = 0; t' = 1;", "v = 0");
    assert_eq!(
        exact_replay(&dragged, claim(&dragged, "Positive"), &trace),
        Ok(ExactReplay::NotExact)
    );
}

/// FR-450-AC-5 (replay half): `1 / (h.v - 5)` has no value at time 15/2 in
/// `Moving`, where `v = 5`: true up to that instant, the claim reproduces
/// with kind `UndefinedEvaluation` naming the division; false from time
/// 1499/200, before it, the claim reproduces with kind `Formula`, and the
/// same window closed at 1499/200 reproduces as `Formula` while one open
/// there reproduces as `UndefinedEvaluation`, so 1499/200 is the decisive
/// instant. A claim atom calling a recursive function is not exact, and
/// static replay of a guard calling it terminates with the call's result
/// range when the measure is undecided.
#[trace("TC-1221", "FR-450-AC-5")]
#[test]
fn the_decisive_instant_decides_formula_or_undefined_evaluation() {
    let graph = braking_unit(FLOW, "v = 0");
    let trace = braking_trace(10, 0);
    let undefined = Ok(ExactReplay::Reproduced(Box::new(HybridCounterexample {
        kind: HybridCounterexampleKind::UndefinedEvaluation {
            at: Box::new(UndefinedInstant {
                state: 0,
                time: ExactReal::Rational(q(15, 2)),
                locus: ExpressionLocus {
                    site: ExpressionSite::ClaimAtom { atom: 0 },
                    term: TermLocus {
                        term: vec![0],
                        body: Vec::new(),
                    },
                },
            }),
            cause: UndefinedCause::DivisionByZero,
        },
        trace: trace.clone(),
    })));
    assert_eq!(
        exact_replay(&graph, claim(&graph, "Defined"), &trace),
        undefined
    );
    assert_eq!(
        exact_replay(&graph, claim(&graph, "Bounded"), &trace),
        Ok(reproduced_formula(&trace))
    );
    assert_eq!(
        exact_replay(&graph, claim(&graph, "BoundedTo"), &trace),
        Ok(reproduced_formula(&trace))
    );
    assert_eq!(
        exact_replay(&graph, claim(&graph, "BoundedBefore"), &trace),
        undefined
    );
    assert_eq!(
        exact_replay(&graph, claim(&graph, "Dragged"), &trace),
        Ok(ExactReplay::NotExact)
    );

    // A guard `drag(v) = 0`: `drag` passes `slow` the measure `if v > 0
    // then 1 else 2`, undecided over `v` in [0, 1/100], so the call takes
    // `slow`'s whole result range; over the point 1/2 it is evaluated.
    let guarded = braking_unit(FLOW, "drag(v) = 0");
    let mut near = braking_trace(10, 0);
    if let Some(before) = &mut near.jumps[0].before {
        before.enclosure[0] = ExactInterval::new(n(0), q(1, 100));
    }
    assert_eq!(
        static_replay(&guarded, claim(&guarded, "SlowEnough"), &near),
        Ok(StaticReplay::Consistent)
    );
    let HybridTerm::Compare(_, call, _) = &guarded.hybrid_models()[0].modes[0].transitions[0].guard
    else {
        panic!("the guard is a comparison");
    };
    let mut expander = Expander::new(&guarded);
    let call = expander.term(call).expect("expands");
    let mut evaluator = IntervalEvaluator::new(&mut expander, n(0));
    let over = |lower: Rational, upper: Rational| {
        vec![
            Interval {
                lower: Some(lower),
                upper: Some(upper),
            },
            Interval::point(n(2)),
            Interval::point(n(10)),
        ]
    };
    assert_eq!(
        evaluator.evaluate(&call, &over(n(0), q(1, 100))),
        Ok(IValue::Number(Interval {
            lower: Some(n(-100)),
            upper: Some(n(100)),
        }))
    );
    assert_eq!(
        evaluator.evaluate(&call, &over(q(1, 2), q(1, 2))),
        Ok(IValue::Number(Interval::point(q(1, 2))))
    );
}

const BOUNCE: &str = "hybrid Bounce using p {\n\
       mode Up invariant { x >= 0 and x <= 1 } flow { x' = 1; }\n\
         transition to Down when { x = 1 } reset { true };\n\
       mode Down invariant { x >= 0 and x <= 1 } flow { x' = -1; }\n\
         transition to Up when { x = 0 } reset { true };\n\
     }\n\
     temporal Settles using p over (h: Bounce) clock \"model-time\" on origin {\n\
       eventually always holds(h.x < 1)\n\
     }\n\
     temporal FairlySettles using p over (h: Bounce) clock \"model-time\" on origin {\n\
       fair Up;\n\
       eventually always holds(h.x < 1)\n\
     }\n";

/// The `Bounce` lasso from `(Up, x = 0)` through `Down` at time 1 back to
/// `Up` at time 2, its last `x` given.
fn bounce_trace(last_x: Rational, loop_to: u64) -> HybridTrace {
    HybridTrace {
        start: state(0, n(0), &[n(0)]),
        jumps: vec![
            TraceJump {
                transition: 0,
                before: Some(before(n(1), &[n(1)])),
                state: state(1, n(1), &[n(1)]),
            },
            TraceJump {
                transition: 0,
                before: Some(before(n(2), &[n(0)])),
                state: state(0, n(2), &[last_x]),
            },
        ],
        delta: None,
        loop_to: Some(loop_to),
    }
}

/// FR-450-AC-6 (replay half): the `Bounce` lasso with `loop_to` 0
/// reproduces `eventually always holds(h.x < 1)` as a periodic behaviour,
/// with and without `fair Up;`; with `loop_to` 1 it is not reproduced at
/// check `Loop`; with a last `x = 1/2` at check `Jump`, the reset `true`
/// keeping `x = 0`; and a `loop_to` 2, naming the last state, is refused at
/// `/loop`.
#[trace("TC-1221", "FR-450-AC-6")]
#[test]
fn a_lasso_reproduces_as_a_periodic_behaviour() {
    let graph = check(BOUNCE);
    let lasso = bounce_trace(n(0), 0);
    for name in ["Settles", "FairlySettles"] {
        assert_eq!(
            exact_replay(&graph, claim(&graph, name), &lasso),
            Ok(reproduced_formula(&lasso)),
            "{name}"
        );
    }
    let settles = claim(&graph, "Settles");
    assert_eq!(
        exact_replay(&graph, settles, &bounce_trace(n(0), 1)),
        Ok(ExactReplay::NotReproduced {
            at: pointer("/loop"),
            check: ExactCheck::Loop,
        })
    );
    assert_eq!(
        exact_replay(&graph, settles, &bounce_trace(q(1, 2), 0)),
        Ok(ExactReplay::NotReproduced {
            at: pointer("/jumps/1"),
            check: ExactCheck::Jump,
        })
    );
    let past_last = Err(HybridReplayRefusal::Trace(TraceRefusal {
        pointer: pointer("/loop"),
        fault: TraceFault::LoopNotBeforeLast,
    }));
    assert_eq!(
        exact_replay(&graph, settles, &bounce_trace(n(0), 2)),
        past_last
    );
    assert_eq!(
        static_replay(&graph, settles, &bounce_trace(n(0), 2)).map(|_| ()),
        past_last.map(|_: ExactReplay| ())
    );
}

/// Exact real arithmetic under exact replay's sign decisions: the
/// irrational roots of `t^2 - 2` order against rationals and each other,
/// and a polynomial's sign at one is exact.
#[trace("TC-1221")]
#[test]
fn irrational_roots_compare_and_sign_exactly() {
    let square = Poly::new(vec![n(-2), n(0), n(1)]);
    let roots = square.roots_in(&n(-2), &n(2));
    let [negative, positive] = roots.as_slice() else {
        panic!("two roots: {roots:?}");
    };
    assert!(negative < positive);
    assert!(*positive > ExactReal::Rational(q(7, 5)));
    assert!(*positive < ExactReal::Rational(q(3, 2)));
    // t^3 - 2t vanishes at sqrt(2); t - 1 is positive there.
    assert_eq!(
        positive.sign_of(&Poly::new(vec![n(0), n(-2), n(0), n(1)])),
        std::cmp::Ordering::Equal
    );
    assert_eq!(
        positive.sign_of(&Poly::new(vec![n(-1), n(1)])),
        std::cmp::Ordering::Greater
    );
    // sqrt(2) + 1 is the root of t^2 - 2t - 1 above 2.
    let other = Poly::new(vec![n(-1), n(-2), n(1)]).roots_in(&n(2), &n(3));
    assert_eq!(other.len(), 1);
    assert_eq!(positive.shifted(&n(1)), other[0]);
}
