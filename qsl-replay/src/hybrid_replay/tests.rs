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

/// A checked unit and the declared names of its hybrid claims, in the order
/// `CheckedGraph::hybrid_claims` lists them (a checked claim keeps no name).
struct Unit {
    graph: CheckedGraph,
    claims: Vec<String>,
}

impl std::ops::Deref for Unit {
    type Target = CheckedGraph;

    fn deref(&self) -> &CheckedGraph {
        &self.graph
    }
}

fn check(declarations: &str) -> Unit {
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
    let graph = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("{text}: the unit assembles: {refusal:?}"))
    .check(CheckingLimits::default())
    .unwrap_or_else(|refusals| panic!("{text}: the unit checks: {refusals:?}"));
    let claims = text
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("temporal "))
        .filter_map(|rest| rest.split_whitespace().next())
        .map(str::to_owned)
        .collect();
    Unit { graph, claims }
}

fn braking_unit(flow: &str, guard: &str) -> Unit {
    check(&format!("{}{CLAIMS}", braking(flow, guard)))
}

fn claim<'g>(unit: &'g Unit, name: &str) -> &'g CheckedHybridClaim {
    let position = unit
        .claims
        .iter()
        .position(|claim| claim == name)
        .unwrap_or_else(|| panic!("no claim {name}"));
    &unit.graph.hybrid_claims()[position]
}

fn static_replay(
    graph: &CheckedGraph,
    claim: &CheckedHybridClaim,
    trace: &HybridTrace,
) -> Result<StaticReplay, HybridReplayRefusal> {
    super::static_replay(graph, claim, trace, &mut HybridReplayMeter::new(u64::MAX))
}

fn exact_replay(
    graph: &CheckedGraph,
    claim: &CheckedHybridClaim,
    trace: &HybridTrace,
) -> Result<ExactReplay, HybridReplayRefusal> {
    super::exact_replay(graph, claim, trace, &mut HybridReplayMeter::new(u64::MAX))
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
    let call = expander
        .term(call, &mut HybridReplayMeter::new(u64::MAX))
        .expect("expands");
    let mut meter = HybridReplayMeter::new(u64::MAX);
    let mut evaluator = IntervalEvaluator::new(&mut expander, n(0), &mut meter);
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
        evaluator.evaluate(call.clone(), &over(n(0), q(1, 100))),
        Ok(IValue::Number(Interval {
            lower: Some(n(-100)),
            upper: Some(n(100)),
        }))
    );
    assert_eq!(
        evaluator.evaluate(call, &over(q(1, 2), q(1, 2))),
        Ok(IValue::Number(Interval::point(q(1, 2))))
    );
}

/// FR-450-AC-5: a definite recursive measure still yields a bounded
/// approximation before a long call chain can exhaust the process stack.
#[trace("TC-1221", "FR-450-AC-5")]
#[test]
fn static_replay_bounds_a_deep_definite_recursive_call() {
    let source = format!(
        "function long using v(s: Rational[-100, 100; 1, 1], n: Int[0, 100000]): \
           Rational[-100, 100; 1, 1] pure decreases(n) \
           {{ if n > 0 then long(s, n - 1) else s }}\n\
         function deep using v(s: Rational[-100, 100; 1, 1]): \
           Rational[-100, 100; 1, 1] pure {{ long(s, 300) }}\n\
         {}",
        braking(FLOW, "deep(v) = 0")
    );
    let graph = check(&source);
    let HybridTerm::Compare(_, call, _) = &graph.hybrid_models()[0].modes[0].transitions[0].guard
    else {
        panic!("the guard is a comparison");
    };
    let mut expander = Expander::new(&graph);
    let call = expander
        .term(call, &mut HybridReplayMeter::new(u64::MAX))
        .expect("expands");
    let variables = [
        Interval::point(n(1)),
        Interval::point(n(2)),
        Interval::point(n(0)),
    ];
    let mut limited = HybridReplayMeter::new(100);
    assert_eq!(
        IntervalEvaluator::new(&mut expander, n(0), &mut limited)
            .evaluate(call.clone(), &variables),
        Err(HybridReplayRefusal::ResourceExhausted {
            counter: quire_exact::LimitKind::WorkUnits,
        })
    );
    assert_eq!(limited.consumed(), 100);
    let mut full = HybridReplayMeter::new(u64::MAX);
    assert_eq!(
        IntervalEvaluator::new(&mut expander, n(0), &mut full).evaluate(call, &variables),
        Ok(IValue::Number(Interval::point(n(1))))
    );
}

/// FR-450-AC-8: a deep, definite recursive guard charges the caller's
/// meter and completes without using the Rust call stack for its depth.
#[trace("TC-1221", "FR-450-AC-8")]
#[test]
fn static_replay_uses_caller_fuel_for_twenty_thousand_recursive_edges() {
    let source = format!(
        "function descend using v(n: Int[0, 20000]): Boolean pure decreases(n) \
           {{ if n = 0 then true else descend(n - 1) }}\n\
         function descending using v(s: Rational[-100, 100; 1, 1]): Boolean pure \
           {{ descend(20000) }}\n\
         {}{CLAIMS}",
        braking(FLOW, "descending(v) and v = 0")
    );
    let graph = check(&source);
    let trace = braking_trace(10, 0);
    let checked_claim = claim(&graph, "SlowEnough");
    let mut limited = HybridReplayMeter::new(100);
    assert_eq!(
        super::static_replay(&graph, checked_claim, &trace, &mut limited),
        Err(HybridReplayRefusal::ResourceExhausted {
            counter: quire_exact::LimitKind::WorkUnits,
        })
    );
    assert_eq!(limited.consumed(), 100);
    let mut full = HybridReplayMeter::new(u64::MAX);
    assert_eq!(
        super::static_replay(&graph, checked_claim, &trace, &mut full),
        Ok(StaticReplay::Consistent)
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

/// FR-450-AC-7: the same point lasso fits a nearby window within the
/// caller's work budget, while a far finite window stops at that budget.
#[trace("TC-1221", "FR-450-AC-7")]
#[test]
fn a_large_future_window_exhausts_caller_work_units() {
    let graph = check(&format!(
        "{BOUNCE}temporal Far using p over (h: Bounce) clock \"model-time\" on origin {{\n\
           eventually [999999, 1000000] holds(h.x = 2)\n\
         }}\n"
    ));
    let lasso = bounce_trace(n(0), 0);
    let mut nearby_meter = HybridReplayMeter::new(100);
    assert_eq!(
        super::exact_replay(&graph, claim(&graph, "Settles"), &lasso, &mut nearby_meter),
        Ok(reproduced_formula(&lasso))
    );
    let mut far_meter = HybridReplayMeter::new(100_000);
    assert_eq!(
        super::exact_replay(&graph, claim(&graph, "Far"), &lasso, &mut far_meter),
        Err(HybridReplayRefusal::ResourceExhausted {
            counter: quire_exact::LimitKind::WorkUnits,
        })
    );
    assert_eq!(far_meter.consumed(), 100_000);
}

/// FR-450-AC-6: a lasso with an undefined claim atom does not reproduce
/// when a continuously enabled fair transition is never taken.
#[trace("TC-1221", "FR-450-AC-6")]
#[test]
fn an_unfair_lasso_with_an_undefined_atom_is_not_reproduced() {
    let graph = check(
        "hybrid Idle using p {\n\
           mode Staying invariant { x = 0 } flow { x' = 0; }\n\
             transition to Staying when { true } reset { x = 0 };\n\
             transition to Exiting when { true } reset { x = 0 };\n\
           mode Exiting invariant { x = 0 } flow { x' = 0; }\n\
         }\n\
         temporal UndefinedFair using p over (h: Idle) clock \"model-time\" on origin {\n\
           fair Staying::Exiting;\n\
           always holds(1 / h.x != 0)\n\
         }\n",
    );
    let trace = HybridTrace {
        start: state(0, n(0), &[n(0)]),
        jumps: vec![TraceJump {
            transition: 0,
            before: Some(before(n(1), &[n(0)])),
            state: state(0, n(1), &[n(0)]),
        }],
        delta: None,
        loop_to: Some(0),
    };
    assert_eq!(
        exact_replay(&graph, claim(&graph, "UndefinedFair"), &trace),
        Ok(ExactReplay::NotReproduced {
            at: JsonPointer::root(),
            check: ExactCheck::Claim,
        })
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

/// FR-450-AC-8: expression expansion consumes the same caller work budget.
#[trace("TC-1221", "FR-450-AC-8")]
#[test]
fn expanding_a_deep_term_uses_caller_fuel() {
    let graph = check(&braking(FLOW, "v = 0"));
    let mut term = HybridTerm::Boolean(true);
    for _ in 0..1024 {
        term = HybridTerm::Not(Box::new(term));
    }
    let mut expander = Expander::new(&graph);
    let mut limited = HybridReplayMeter::new(1024);
    assert!(matches!(
        expander.term(&term, &mut limited),
        Err(HybridReplayRefusal::ResourceExhausted {
            counter: quire_exact::LimitKind::WorkUnits
        })
    ));
    assert_eq!(limited.consumed(), 1024);
    let mut enough = HybridReplayMeter::new(1025);
    assert!(expander.term(&term, &mut enough).is_ok());
    assert_eq!(enough.consumed(), 1025);
}

// ----------------------------------------------------- settlement (FR-250)

mod settlement {
    use super::*;
    use crate::hybrid::{
        BoundDimension, SolverCoverage, SolverHorizon, SolverMethod, SolverOutcome, SolverResult,
    };
    use crate::model_check::limits::ModelCheckLimit;
    use crate::proof_result::{
        read_first_party_solver_basis, read_plugin_inconclusive_cause_over, AppliedReduction,
        Certification, IncompleteCause, InconclusiveCause, PluginResultRefusal, ProofBasis,
        Proviso, TerminalRecord, TerminalValue, ValueFault,
    };
    use qsl_foundation::{BackendId, RequestIndex};
    use quire_exact::LimitKind;
    use serde_json::{json, Value};
    use std::collections::BTreeSet;

    fn result(outcome: SolverOutcome) -> SolverResult {
        SolverResult {
            method: SolverMethod::FlowpipeOutwardRounded,
            outcome,
            coverage: SolverCoverage::Bounded {
                horizon: SolverHorizon::new(n(10)).expect("non-negative"),
                jumps: 3,
                exhaustive: BTreeSet::from([BoundDimension::Time]),
            },
        }
    }

    fn settle(
        unit: &Unit,
        name: &str,
        result: &SolverResult,
        work_units: u64,
    ) -> Result<crate::hybrid::HybridSettlement, HybridReplayRefusal> {
        result.settle(
            &unit.graph,
            claim(unit, name),
            &mut HybridReplayMeter::new(work_units),
        )
    }

    /// FR-250-AC-4, FR-450-AC-2: a solver's reported trace is kept on the
    /// `inconclusive` record with its static (and not-reproduced exact)
    /// replay outcome and no counterexample; the same trace settles
    /// `refuted` with its counterexample only where exact replay reproduces
    /// it; a result with no trace keeps `trace: None`.
    #[trace("TC-705", "FR-250-AC-4")]
    #[test]
    fn a_reported_trace_is_replayed_and_refutes_only_through_exact_replay() {
        let unit = braking_unit(FLOW, "v = 0");
        let trace = braking_trace(10, 0);
        let meets = result(SolverOutcome::EnclosureMeetsUnsafe(Some(Box::new(
            trace.clone(),
        ))));

        let kept = settle(&unit, "SlowEnough", &meets, u64::MAX).expect("settles");
        assert_eq!(kept.counterexample, None);
        assert_eq!(
            kept.value,
            TerminalValue::Inconclusive(InconclusiveCause::SolverInconclusive {
                trace: Some(Box::new(trace.clone())),
                replay: Some(Box::new(ReplayRecord {
                    static_outcome: StaticReplay::Consistent,
                    exact: Some(ExactRecord::NotReproduced {
                        at: JsonPointer::root(),
                        check: ExactCheck::Claim,
                    }),
                })),
            })
        );

        let refuted = settle(&unit, "Positive", &meets, u64::MAX).expect("settles");
        assert_eq!(refuted.value, TerminalValue::Refuted);
        let Some(counterexample) = refuted.counterexample else {
            panic!("a reproduced trace is the item's counterexample");
        };
        assert_eq!(counterexample.kind, HybridCounterexampleKind::Formula);
        assert_eq!(counterexample.trace, trace);

        let untraced = settle(
            &unit,
            "SlowEnough",
            &result(SolverOutcome::DeltaSat(None)),
            u64::MAX,
        )
        .expect("settles");
        assert_eq!(
            untraced.value,
            TerminalValue::Inconclusive(InconclusiveCause::SolverInconclusive {
                trace: None,
                replay: None,
            })
        );
        assert_eq!(untraced.counterexample, None);
    }

    /// FR-250-AC-4: a trace taking transition 1, one whose enclosure lacks a
    /// variable, and one with a time interval whose lower bound exceeds its
    /// upper bound are each refused at the member; FR-450: a replay that
    /// exhausts the caller's `work_units` settles `incomplete` with no
    /// verdict or counterexample.
    #[trace("TC-705", "FR-250-AC-4")]
    #[test]
    fn a_trace_that_breaks_the_model_is_refused_and_a_spent_meter_is_incomplete() {
        let unit = braking_unit(FLOW, "v = 0");
        let refused = |trace: HybridTrace| {
            settle(
                &unit,
                "SlowEnough",
                &result(SolverOutcome::DeltaSat(Some(Box::new(trace)))),
                u64::MAX,
            )
            .expect_err("a refusal")
        };

        let mut transition = braking_trace(10, 0);
        transition.jumps[0].transition = 1;
        assert_eq!(
            refused(transition),
            HybridReplayRefusal::Trace(TraceRefusal {
                pointer: pointer("/jumps/0/transition"),
                fault: TraceFault::UnknownTransition,
            })
        );
        let mut missing = braking_trace(10, 0);
        missing.start.enclosure.pop();
        assert_eq!(
            refused(missing),
            HybridReplayRefusal::Trace(TraceRefusal {
                pointer: pointer("/start/enclosure"),
                fault: TraceFault::EnclosureArity,
            })
        );
        let mut inverted = braking_trace(10, 0);
        inverted.start.time = ExactInterval::new(n(3), n(2));
        assert_eq!(
            refused(inverted),
            HybridReplayRefusal::Trace(TraceRefusal {
                pointer: pointer("/start/time"),
                fault: TraceFault::InvertedInterval,
            })
        );

        let spent = settle(
            &unit,
            "Positive",
            &result(SolverOutcome::DeltaSat(Some(Box::new(braking_trace(
                10, 0,
            ))))),
            0,
        )
        .expect("settles");
        assert_eq!(
            spent.value,
            TerminalValue::Incomplete(IncompleteCause::LimitReached {
                limit: ModelCheckLimit::EvaluationMeter {
                    counter: LimitKind::WorkUnits
                },
                value: 0,
            })
        );
        assert_eq!(spent.counterexample, None);
    }

    fn rational_json(value: i64) -> Value {
        json!({"numerator": value.to_string(), "denominator": "1"})
    }

    fn interval_json(value: i64) -> Value {
        json!({"lower": rational_json(value), "upper": rational_json(value)})
    }

    fn state_json(mode: &str, time: i64, values: [i64; 3]) -> Value {
        json!({
            "mode": mode,
            "time": interval_json(time),
            "enclosure": {
                "v": interval_json(values[0]),
                "a": interval_json(values[1]),
                "t": interval_json(values[2]),
            },
        })
    }

    fn cause_json(trace: Value) -> Value {
        json!({"type": "solver-inconclusive", "trace": trace})
    }

    fn trace_json() -> Value {
        json!({
            "start": state_json("Moving", 0, [20, 2, 0]),
            "jumps": [{
                "transition": 0,
                "before": {
                    "time": interval_json(10),
                    "enclosure": {
                        "v": interval_json(0),
                        "a": interval_json(2),
                        "t": interval_json(10),
                    },
                },
                "state": state_json("Stopped", 10, [0, 2, 10]),
            }],
        })
    }

    /// FR-250-AC-4, FR-290: through the plugin reader a `solver-inconclusive`
    /// cause carrying a trace in QSpec FR-193's wire form reads
    /// `SolverInconclusive{trace: Some(..)}` with the same trace on a hybrid
    /// claim and is `Malformed` at `/cause/trace` on any other item; a
    /// correspondence failure is `Malformed` at its member and a provider's
    /// `replay` member is an unknown member.
    #[trace("TC-705", "FR-250-AC-4")]
    #[test]
    fn the_plugin_reader_resolves_a_wire_trace_against_the_claims_model() {
        let unit = braking_unit(FLOW, "v = 0");
        let model = &unit.graph.hybrid_models()[0];
        let read = |cause: &Value| read_plugin_inconclusive_cause_over(cause, Some(model));
        let malformed = |pointer: &str| PluginResultRefusal::Malformed {
            pointer: pointer.to_owned(),
            cause: ValueFault::InvalidValue,
        };

        assert_eq!(
            read(&cause_json(trace_json())),
            Ok(InconclusiveCause::SolverInconclusive {
                trace: Some(Box::new(braking_trace(10, 0))),
                replay: None,
            })
        );
        assert_eq!(
            read_plugin_inconclusive_cause_over(
                &json!({"type": "solver-inconclusive"}),
                Some(model)
            ),
            Ok(InconclusiveCause::SolverInconclusive {
                trace: None,
                replay: None,
            })
        );
        assert_eq!(
            read_plugin_inconclusive_cause_over(&cause_json(trace_json()), None),
            Err(malformed("/cause/trace"))
        );

        let mut wrong_transition = trace_json();
        wrong_transition["jumps"][0]["transition"] = json!(1);
        assert_eq!(
            read(&cause_json(wrong_transition)),
            Err(malformed("/cause/trace/jumps/0/transition"))
        );
        let mut missing_variable = trace_json();
        missing_variable["start"]["enclosure"]
            .as_object_mut()
            .expect("an object")
            .remove("t");
        assert_eq!(
            read(&cause_json(missing_variable)),
            Err(malformed("/cause/trace/start/enclosure"))
        );
        let mut inverted = trace_json();
        inverted["start"]["time"] = json!({"lower": rational_json(3), "upper": rational_json(2)});
        assert_eq!(
            read(&cause_json(inverted)),
            Err(malformed("/cause/trace/start/time"))
        );
        let mut unknown_mode = trace_json();
        unknown_mode["start"]["mode"] = json!("Halted");
        assert_eq!(
            read(&cause_json(unknown_mode)),
            Err(malformed("/cause/trace/start/mode"))
        );

        let mut replayed = cause_json(trace_json());
        replayed["replay"] = json!({"static": {"outcome": "consistent"}});
        assert_eq!(
            read(&replayed),
            Err(PluginResultRefusal::UnknownMember {
                pointer: "/cause/replay".to_owned()
            })
        );
    }

    fn proof(method: SolverMethod, seconds: i64) -> SolverResult {
        SolverResult {
            method,
            outcome: SolverOutcome::Unsat,
            coverage: SolverCoverage::Bounded {
                horizon: SolverHorizon::new(n(seconds)).expect("non-negative"),
                jumps: 3,
                exhaustive: BTreeSet::from([BoundDimension::Time]),
            },
        }
    }

    /// FR-250-AC-7: coverage is decided from the checked claim's own clock
    /// and horizon, so the same `Bounded{10, 3, {Time}}` proof settles
    /// `proved` over `SlowEnough` (10 s) and `insufficient-horizon` over
    /// `Positive` (12 s); a plugin's `trusted` proof is checked the same way,
    /// and the plain plugin path admits no `bounded-solver` basis.
    #[trace("TC-705", "FR-250-AC-7")]
    #[test]
    fn coverage_is_decided_from_the_checked_claims_own_horizon() {
        let unit = braking_unit(FLOW, "v = 0");
        let proved = settle(
            &unit,
            "SlowEnough",
            &proof(SolverMethod::DeltaComplete, 10),
            u64::MAX,
        )
        .expect("settles");
        assert!(matches!(proved.value, TerminalValue::Proved { .. }));
        let short = settle(
            &unit,
            "Positive",
            &proof(SolverMethod::DeltaComplete, 10),
            u64::MAX,
        )
        .expect("settles");
        assert!(matches!(
            short.value,
            TerminalValue::Inconclusive(InconclusiveCause::InsufficientHorizon { .. })
        ));
        assert_eq!(
            proof(SolverMethod::DeltaComplete, 10).settlement_basis(claim(&unit, "Positive")),
            crate::proof_result::SettlementBasis::Unsettled
        );

        let basis = |seconds: i64| ProofBasis::BoundedSolver {
            method: SolverMethod::Unclassified,
            horizon: SolverHorizon::new(n(seconds)).expect("non-negative"),
            jumps: 1,
            exhaustive: BTreeSet::from([BoundDimension::Time]),
        };
        let plugin = || BackendId::new("acme");
        let record = |name: &str, basis| {
            TerminalRecord::from_hybrid_plugin_proof(
                RequestIndex::new(0),
                plugin(),
                basis,
                claim(&unit, name),
            )
            .expect("admitted")
        };
        assert!(matches!(
            record("SlowEnough", basis(1)).value(),
            TerminalValue::Inconclusive(InconclusiveCause::InsufficientHorizon { .. })
        ));
        assert!(matches!(
            record("SlowEnough", basis(10)).value(),
            TerminalValue::Proved {
                certification: Certification::Trusted,
                ..
            }
        ));
        assert!(matches!(
            record("Positive", basis(10)).value(),
            TerminalValue::Inconclusive(InconclusiveCause::InsufficientHorizon { .. })
        ));
        assert!(matches!(
            record(
                "SlowEnough",
                ProofBasis::InductiveSolver {
                    method: SolverMethod::InductiveEnclosure
                }
            )
            .value(),
            TerminalValue::Proved { .. }
        ));
        assert_eq!(
            TerminalRecord::from_plugin_proof(RequestIndex::new(0), plugin(), basis(10)).err(),
            Some(PluginResultRefusal::Malformed {
                pointer: "/basis/type".to_owned(),
                cause: ValueFault::InvalidValue,
            })
        );
    }

    /// The whole of FR-290-AC-5 and FR-289-AC-9: every plugin basis spelling
    /// reads as the typed value; a non-hybrid basis settles `proved` and
    /// `trusted` naming the plugin; a `bounded-solver` basis is refused by
    /// the generic entry, and on a hybrid claim settles `proved` when its
    /// coverage satisfies the claim and `inconclusive` when it does not.
    #[trace("TC-774", "FR-290-AC-5")]
    #[trace("TC-773", "FR-289-AC-9")]
    #[test]
    fn plugin_bases_read_and_settle_by_item_kind() {
        use crate::proof_result::read_plugin_basis;
        let plugin = || BackendId::new("acme");
        let solver = |method: &str, horizon: Value| {
            json!({"type": "bounded-solver", "exhaustive": ["time"], "horizon": horizon,
                   "jumps": 3, "method": method})
        };
        let five_halves = json!({"denominator": "2", "numerator": "5"});
        let horizon = |value: Rational| SolverHorizon::new(value).expect("non-negative");
        let bounded = |method, value: Rational| ProofBasis::BoundedSolver {
            method,
            horizon: horizon(value),
            jumps: 3,
            exhaustive: BTreeSet::from([BoundDimension::Time]),
        };
        let read = |value: Value| read_plugin_basis(&value).expect("reads");

        assert_eq!(read(json!({"type": "exhaustive"})), ProofBasis::Exhaustive);
        assert_eq!(
            read(json!({"type": "bounded-complete", "depth": 4})),
            ProofBasis::BoundedComplete { depth: 4 }
        );
        assert_eq!(
            read(json!({"type": "inductive", "depth": 0})),
            ProofBasis::Inductive { depth: 0 }
        );
        let delta = read(solver("delta-complete", five_halves.clone()));
        assert_eq!(delta, bounded(SolverMethod::DeltaComplete, q(5, 2)));
        assert_eq!(
            read(solver("flowpipe-unrounded", five_halves)),
            bounded(SolverMethod::FlowpipeUnrounded, q(5, 2))
        );
        let reduced = read(json!({"type": "reduced", "reductions": [
            {"parameters": {"proviso": "breadth-first-revisit"}, "reduction": "partial-order", "row": "invariant"},
            {"parameters": {"classes": [["a", "b"]]}, "reduction": "copy-swap", "row": "hyper"}
        ]}));
        assert_eq!(
            reduced,
            ProofBasis::Reduced {
                reductions: vec![
                    AppliedReduction::PartialOrder {
                        proviso: Proviso::BreadthFirstRevisit
                    },
                    AppliedReduction::CopySwap {
                        classes: vec![vec!["a".to_owned(), "b".to_owned()]]
                    },
                ]
            }
        );

        for basis in [
            ProofBasis::Exhaustive,
            ProofBasis::BoundedComplete { depth: 4 },
            ProofBasis::Inductive { depth: 0 },
            reduced,
        ] {
            let record =
                TerminalRecord::from_plugin_proof(RequestIndex::new(2), plugin(), basis.clone())
                    .expect("admitted");
            assert_eq!(
                record.value(),
                &TerminalValue::Proved {
                    basis,
                    certification: Certification::Trusted,
                }
            );
            assert_eq!(record.backend(), Some(&plugin()));
        }

        assert_eq!(
            TerminalRecord::from_plugin_proof(RequestIndex::new(2), plugin(), delta.clone()).err(),
            Some(invalid_basis_type())
        );

        let unit = braking_unit(FLOW, "v = 0");
        let hybrid = |name: &str, basis| {
            TerminalRecord::from_hybrid_plugin_proof(
                RequestIndex::new(2),
                plugin(),
                basis,
                claim(&unit, name),
            )
            .expect("admitted")
        };
        let covering = read(solver(
            "delta-complete",
            json!({"denominator": "1", "numerator": "10"}),
        ));
        let proved = hybrid("SlowEnough", covering.clone());
        assert_eq!(
            proved.value(),
            &TerminalValue::Proved {
                basis: covering,
                certification: Certification::Trusted,
            }
        );
        assert_eq!(proved.backend(), Some(&plugin()));
        let short = hybrid("SlowEnough", delta);
        assert_eq!(
            short.value(),
            &TerminalValue::Inconclusive(InconclusiveCause::InsufficientHorizon {
                horizon: horizon(q(5, 2)),
                jumps: 3,
                exhaustive: BTreeSet::from([BoundDimension::Time]),
            })
        );
    }

    fn invalid_basis_type() -> PluginResultRefusal {
        PluginResultRefusal::Malformed {
            pointer: "/basis/type".to_owned(),
            cause: ValueFault::InvalidValue,
        }
    }

    /// FR-250-AC-6: a first-party `uncertified` `bounded-solver` entry whose
    /// method is `flowpipe-unrounded` or `unclassified` is refused
    /// `invalid_result`/`invalid-value` at `/basis/method`; a classified
    /// method and an `inductive-solver` entry read.
    #[trace("TC-705", "FR-250-AC-6")]
    #[test]
    fn a_first_party_entry_with_an_unsound_method_is_refused() {
        let bounded = |method: &str| {
            json!({"type": "bounded-solver", "method": method, "jumps": 3,
                "exhaustive": ["time"],
                "horizon": {"numerator": "10", "denominator": "1"}})
        };
        for method in ["flowpipe-unrounded", "unclassified"] {
            assert_eq!(
                read_first_party_solver_basis(&bounded(method)),
                Err(PluginResultRefusal::Malformed {
                    pointer: "/basis/method".to_owned(),
                    cause: ValueFault::InvalidValue,
                })
            );
        }
        assert!(matches!(
            read_first_party_solver_basis(&bounded("delta-complete")),
            Ok(ProofBasis::BoundedSolver {
                method: SolverMethod::DeltaComplete,
                ..
            })
        ));
        assert!(matches!(
            read_first_party_solver_basis(
                &json!({"type": "inductive-solver", "method": "inductive-enclosure"})
            ),
            Ok(ProofBasis::InductiveSolver { .. })
        ));
        assert!(matches!(
            read_first_party_solver_basis(&json!({"type": "exhaustive"})),
            Err(PluginResultRefusal::Unsupported { .. })
        ));
    }
}
