---
id: FR-450
title: "Replay a hybrid trace against the checked hybrid model"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-447
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-250
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: references
  - target: ix://agent-ix/quire-specification/FR-193
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-146
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-331
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-364
    type: depends_on
---
# FR-450: Replay a hybrid trace against the checked hybrid model

## Description

When QSL reads a hybrid trace, whether a plugin provider's or its own
solver's, it SHALL replay that trace against the claim's checked hybrid model
(FR-447) by QSpec FR-193 "Trace replay" before it settles the item. Static
replay checks each trace in exact rational interval arithmetic within the
trace's tolerance, using the caller's `HybridReplayMeter`, and records its
outcome on the `inconclusive` record. Exact replay decides every eligible
point trace through modes with polynomial flows exactly when its work fits the
caller's meter. A trace it reproduces settles the item
`refuted` with the trace as its counterexample. If the meter is exhausted,
replay settles the item `incomplete` with the exhausted counter and its
accounting setting, without a verdict or counterexample. This is ADR-026 HY-2's
one refutation path for a hybrid item.

## Use case

A plugin δ-complete solver reports `δ-sat` for FR-447's `SlowEnough` with a
trace whose boxes overlap the guard of `Moving`'s transition within its
`delta` of 1/100. QSL replays it statically, finds it consistent, and keeps
it on the `inconclusive` record. A second provider reports the point trace
that starts at `v = 20`, `a = 2`, dwells 10 s and jumps to `Stopped` at
`v = 0`. Against `always [0, 12] holds(h.v > 0)` QSL replays it exactly:
`Moving`'s flow is polynomial, `v(t) = 20 - 2t` reaches 0 at `t = 10`, and the
claim is false there, so the item settles `refuted` with that trace as its
counterexample.

## Inputs

- A `HybridTrace` (FR-250), from a `solver-inconclusive` cause or from a
  `HybridCounterexample` in a provider's `refuted` entry (QSpec FR-193
  "Solver result on the wire"), read through the plugin reader (FR-290) or
  returned by an in-process provider.
- The item's `CheckedHybridClaim` and its `CheckedHybridModel` (FR-447).
- The caller's FR-255 `accounting.work_units` bound, held by the
  `HybridReplayMeter` used by both replay functions.

## Outputs

```rust
pub enum StaticReplay {
    Consistent,
    Inconsistent { at: JsonPointer, check: StaticCheck }, // the first failing check
}
pub enum StaticCheck { Initial, Invariant, Time, Jump, Loop } // QSpec FR-193 static checks 1 to 5
pub struct ReplayRecord {
    pub static_outcome: StaticReplay, // written as `replay.static`
    pub exact: Option<ExactRecord>,   // written as `replay.exact` when present
}
pub enum ExactRecord {
    NotExact,
    NotReproduced { at: JsonPointer, check: ExactCheck },
}
pub enum ExactReplay {
    NotExact,                                   // not a point trace, or a mode without a polynomial flow
    Reproduced(HybridCounterexample),
    NotReproduced { at: JsonPointer, check: ExactCheck },
}
pub enum HybridReplayRefusal {
    // Also carries the existing trace and package admission refusals.
    ResourceExhausted { counter: WorkUnits },
}
pub struct HybridReplayMeter {
    pub work_units: u64, // caller's configured bound
    pub consumed: u64,   // successfully charged work units
}
pub enum ExactCheck { Initial, Dwell, Jump, Loop, Claim } // QSpec FR-193 exact checks 1 to 5
pub struct HybridCounterexample {
    pub kind: HybridCounterexampleKind,         // Formula | UndefinedEvaluation{where, cause}
    pub trace: HybridTrace,
}
pub enum HybridTraceDisagreement {
    NotReproduced { counterexample: HybridCounterexample },
    NotExact { counterexample: HybridCounterexample },
}
```

`static_replay(graph, claim, trace, meter: &mut HybridReplayMeter) ->
Result<StaticReplay, HybridReplayRefusal>` and
`exact_replay(graph, claim, trace, meter: &mut HybridReplayMeter) ->
Result<ExactReplay, HybridReplayRefusal>` take the caller's meter. The
`InconclusiveCause::SolverInconclusive { trace: Option<HybridTrace>, replay:
Option<ReplayRecord> }` carries the static outcome and, when exact replay ran
without reproducing the trace, its exact outcome. The stored FR-193 `replay`
object has a required `static` member and an optional `exact` member; the
`exact` member is `not-exact` or `not-reproduced` with its failing pointer and
check. It is not a bare `StaticReplay`. A reproduced trace
settles as `TerminalValue::Refuted` with its `HybridCounterexample` as the
item's `Counterexample`.

## Behavior

### When replay runs

- QSL SHALL replay every `HybridTrace` it keeps or settles from, after the
  correspondence checks of FR-250 and QSpec FR-193 "Hybrid trace" admit it,
  whether a plugin provider or an in-process provider returned it.
- Replay SHALL read the `CheckedHybridModel` and `CheckedHybridClaim` of the
  checked package, by the variable and mode indices FR-250's `HybridTrace`
  carries, and SHALL read no source.
- A negative `delta` SHALL refuse the result `invalid_result`/`invalid-value`
  at `/delta`, and a `loop_to` past the trace's last state SHALL refuse it
  at `/loop` (QSpec FR-193).

### Static replay

- QSL SHALL run static replay on every trace by QSpec FR-193 "Trace replay"
  checks 1 to 5, in exact rational interval arithmetic over the trace's
  intervals, each comparison widened by `delta` (0 when `None`), and SHALL
  return `Consistent` or `Inconsistent` at the first failing check in trace
  order, with the member's pointer. `static_replay` and `exact_replay` take
  the caller's `HybridReplayMeter` and return a `HybridReplayRefusal` if the
  meter denies a charge.
- The interval evaluator SHALL be stack-safe: it SHALL evaluate nested terms
  and recursive calls iteratively or with an explicit stack, with no recursion
  depth limit. Neither replay function nor a helper it invokes SHALL impose a
  fixed evaluation or iteration cap. One work unit is one static interval
  evaluation step, including each definite recursive call expansion, or one
  exact replay work step. Each exact replay `ClaimSegment` materialization is
  a work step, including segments created by expanding a lasso repetition to
  reach a finite interval. Before each charge, if `consumed` equals the
  configured `work_units` bound, replay SHALL return
  `HybridReplayRefusal::ResourceExhausted { counter: WorkUnits }`; otherwise,
  it SHALL increment `consumed` by one. The bound is FR-255's
  `accounting.work_units` setting.
- Interval evaluation SHALL take every `HybridTerm` form: `Variable` over
  the state's interval, `Constant` as a point, `Arithmetic` and `Negate` by
  exact interval arithmetic, a division whose divisor interval contains 0 as
  the whole real line, `Compare` as definitely true, definitely false or
  undecided, `Logic` and `Not` over those three values, `Conditional` as the
  hull of the branches its condition does not exclude, and `Call` by
  evaluating the function's checked body over the argument intervals. A
  recursive call SHALL be evaluated only while its `decreases` measure
  (FR-062) evaluates to a definite interval value: a recursive call whose
  measure interval is undecided, or whose base-case condition is undecided,
  SHALL take the whole value range of the function's result type (the
  real line for an unbounded numeric type, undecided for `Boolean`). A call
  with a definite measure and base-case condition SHALL continue under the
  supplied meter; its termination comes from FR-062's decreasing measure and
  caller fuel, not a fixed evaluation cap.
- A check SHALL fail only when its value is definitely false.
- Static replay SHALL settle no value: QSL SHALL keep the outcome as
  `SolverInconclusive{trace, replay: Some(outcome)}`, written on the stored
  record as QSpec FR-193's `replay` member (`static`, and `exact` when exact
  replay ran and did not reproduce). QSL SHALL refuse a provider entry whose
  cause carries `replay` with `invalid_result`/`unknown-member` at
  `/cause/replay`.

### Exact replay

- QSL SHALL run exact replay on a trace that QSpec FR-193 calls a point
  trace through modes with polynomial flows: `delta` 0 or `None`, every jump
  carrying `before`, every interval a point, and in every mode the trace
  dwells in each rate built from `Constant`, `Variable`, `Negate`, `+`, `-`
  and `*`, after expanding every `Call` whose function is not recursive, with
  no cycle in the relation "`y` occurs in `x`'s rate", and with no recursive
  call in any invariant, guard, reset, initial condition, activation
  condition, capture or claim atom the replay evaluates. Any other trace
  SHALL give `NotExact`.
- QSL SHALL compute each dwell's solution as polynomials in time with exact
  rational coefficients, integrating the rates in that relation's order from
  the state's point, and SHALL decide QSpec FR-193's exact checks 1 to 5 over
  the reals exactly: an atom along a dwell is a univariate rational function
  of time, and QSL SHALL decide its sign on an interval by exact real root
  isolation, with no floating-point step.
- Exact replay SHALL not substitute an approximate evaluation or a verdict
  when its meter denies a charge.
- Check 5 SHALL evaluate the `CheckedHybridClaim` over the replayed
  behaviour by QSpec FR-193 "Hybrid claims": with its clock, its activation
  and every obligation it creates, its captures, and its fairness set, a
  `loop_to` trace's periodic behaviour admitted only when it satisfies that
  set. A finite prefix SHALL reproduce only as a bad prefix: the claim false
  on every behaviour that extends it, every position past the prefix's end
  read as unknown.
- When checks 1 to 4 hold, exact replay SHALL decide by the decisive
  instant of QSpec FR-193 "Trace replay": the earliest instant at which the
  prefix up to it is a bad prefix of the claim or an expression the replay
  evaluates has no value (a division by zero, an argument outside its
  parameter's range). If an expression has no value at that instant, it
  SHALL return `Reproduced` with kind `UndefinedEvaluation{where, cause}`,
  `where` naming that instant's state, its time stamp and the expression's
  locus; otherwise `Reproduced` with kind `Formula`. A `loop_to` trace
  reproduces with kind `UndefinedEvaluation` at the earliest instant of its
  periodic behaviour with no value, else with kind `Formula` when check 5
  holds. Otherwise it SHALL return `NotReproduced` at the first failing
  check.

### Settlement

- A `Reproduced` trace SHALL settle the item `refuted` (ADR-018 V-4) with
  its `HybridCounterexample` as the item's `Counterexample` and one
  `counterexamples` entry, with the cause `UndefinedEvaluation{where, cause}`
  for that kind, whether the provider reported the trace as
  `solver-inconclusive` or as `refuted`.
- A `NotReproduced` or `NotExact` trace from a `solver-inconclusive` cause
  SHALL leave the item `inconclusive`, `SolverInconclusive{trace, replay}`.
- A `HybridReplayRefusal::ResourceExhausted { counter: WorkUnits }` from either replay
  function SHALL settle the item
  `incomplete`, `Incomplete(LimitReached{limit, value, setting})`, whether
  the trace came from a `solver-inconclusive` cause or a provider's `refuted`
  entry. `limit` SHALL be `work_units`, `value` the meter's configured
  `work_units` bound, and `setting` `accounting.work_units`; the caller can
  raise the bound through that
  setting. The item SHALL carry no counterexample and no verdict.
- A provider's `refuted` entry whose trace gives `NotReproduced` or
  `NotExact` SHALL settle the item `inconclusive` with
  `ReplayParity(DisagreementCause::HybridTrace(disagreement))` (QSpec FR-364).
  `disagreement` SHALL be `HybridTraceDisagreement::NotReproduced` when exact
  replay returned `NotReproduced`, and
  `HybridTraceDisagreement::NotExact` when exact replay returned `NotExact`.
  Each variant SHALL carry the provider's complete `HybridCounterexample`
  (its kind and trace), and SHALL not be converted to a
  `SeparatingWitnessRecord`, `Verdicts` or `NoValue`. The serialized
  disagreement cause SHALL use `{"cause":"hybrid-trace","disagreement":{"type":"not-reproduced","counterexample":{"type":"hybrid-counterexample","kind":...,"trace":...}}}` or the same object with `type` `not-exact`; `trace` uses QSpec FR-193's hybrid trace wire form. The enclosing `replay-parity` cause SHALL preserve this distinct variant and the complete counterexample. The
  resulting record SHALL have no `counterexamples` entry.
- No other path SHALL settle a hybrid item `refuted`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-450-AC-1 | Over FR-447's `Braking`, a `solver-inconclusive` trace whose `start` box `v` in `[25, 26]` meets no initial entry replays `Inconsistent { at: /start, check: Initial }`; a trace whose `before` box `v` in `[1/200, 1/100]` misses the guard `v = 0` by less than its `delta` 1/100 replays `Consistent`, and the same trace with no `delta` `Inconsistent` at check `Jump`; a trace whose second state's `time.upper` is below the first's `time.lower` is `Inconsistent` at check `Time`; each item settles `inconclusive`, `SolverInconclusive{trace: Some(..), replay: Some(..)}` with that outcome and no `counterexamples` entry; a `delta` of `-1/100` is refused `invalid_result`/`invalid-value` at `/delta`. | Test (TC-1221) |
| FR-450-AC-2 | Against `always [0, 12] holds(h.v > 0)`, the point trace from `(Moving, v = 20, a = 2, t = 0)` with `before` `(time 10, v = 0, a = 2, t = 10)` and transition 0 to `(Stopped, time 10, v = 0, a = 2, t = 10)` replays `Reproduced` with kind `Formula`, from a `solver-inconclusive` cause and from a provider's `refuted` entry alike, and the item settles `refuted` with that trace as its `Counterexample` and one `counterexamples` entry; against `SlowEnough` the same trace gives `NotReproduced` at check `Claim` and the item stays `inconclusive`. | Test (TC-1221) |
| FR-450-AC-3 | The AC-2 trace with `before` `v = 1` gives `NotReproduced` at check `Dwell`; as a provider's `refuted` entry it settles `inconclusive`, `ReplayParity(DisagreementCause::HybridTrace(NotReproduced{counterexample}))`, preserving the complete input counterexample, and has no `counterexamples` entry. The same input with `before.time` 9 gives `NotReproduced` at check `Dwell` and the same distinct `NotReproduced` cause with its complete counterexample. | Test (TC-1221) |
| FR-450-AC-9 | On FR-447's `Braking` with `Moving`'s flow changed to `v' = -v`, a provider's `refuted` entry carrying the otherwise valid AC-2 point trace returns `NotExact`; the item settles `inconclusive`, `ReplayParity(DisagreementCause::HybridTrace(NotExact{counterexample}))`, retaining the complete counterexample and distinguishing it from `NotReproduced`; no `counterexamples` entry is written. | Test (TC-1221) |
| FR-450-AC-4 | With `Moving`'s flow `v' = -v`, the AC-2 trace gives `NotExact` and is replayed statically only, settling `inconclusive`; with `Moving`'s flows `v' = -a` and `a' = -1 / 10` it is replayed exactly, with `a(t) = 2 - t / 10` and `v(t) = 20 - 2t + t^2 / 20`, and gives `NotReproduced` at check `Dwell` because `v(10) = 5`; with a flow `v' = drag(v)` over a recursive `drag`, it gives `NotExact`. | Test (TC-1221) |
| FR-450-AC-5 | The AC-2 trace against `always [0, 12] holds(1 / (h.v - 5) != 0)`, true up to the undefined instant, replays `Reproduced` with kind `UndefinedEvaluation`, naming the instant at time 15/2 in `Moving`, where `v = 5`, and the division's locus, and settles `refuted` with cause `UndefinedEvaluation{where, cause}`; against `always [0, 12] holds(1 / (h.v - 5) < 100)`, false from time 1499/200 before that instant, it replays `Reproduced` with kind `Formula`, the decisive instant 1499/200; against a claim whose atom calls a recursive function it gives `NotExact`; static replay of a guard calling a recursive function whose measure interval is undecided terminates with that call's result range. | Test (TC-1221) |
| FR-450-AC-6 | Over `Bounce`, `hybrid Bounce using p { mode Up invariant { x >= 0 and x <= 1 } flow { x' = 1; } transition to Down when { x = 1 } reset { true }; mode Down invariant { x >= 0 and x <= 1 } flow { x' = -1; } transition to Up when { x = 0 } reset { true }; }`, whose initial states are `Up` with `x` in `[0, 1]` at time 0, against `eventually always holds(h.x < 1)` under the infinite-trace profile, the point trace from `(Up, x = 0)` through `Down` at time 1 back to `(Up, x = 0)` at time 2 with `loop_to` 0 reproduces as a periodic behaviour, also with `fair Up;`; the same trace with `loop_to` 1, whose last state `(Up, x = 0)` is not the `Down` state it names, gives `NotReproduced` at check `Loop`; the same trace whose last state has `x = 1/2` gives `NotReproduced` at check `Jump`, the reset `true` keeping `x = 0`; and a `loop_to` 2, naming the last state, is refused `invalid_result`/`invalid-value` at `/loop`. | Test (TC-1221) |
| FR-450-AC-7 | Over `Bounce` and its `Up`/`Down` point lasso from FR-450-AC-6 with `loop_to` 0, a provider's `refuted` trace against `eventually [999999, 1000000] holds(h.x = 2)` needs at least 500,000 `ClaimSegment`s to evaluate the finite interval because the lasso repeats every 2 seconds and replay reaches the 1,000,000-second upper bound; with `work_units` 100,000, the meter cannot cover all required segment charges and exhausts deterministically before deciding the formula, so replay returns `HybridReplayRefusal::ResourceExhausted{counter: WorkUnits}` and settles `incomplete`, `LimitReached{limit: work_units, value: 100000, setting: accounting.work_units}`, with no verdict or `counterexamples` entry. | Test (TC-1221) |
| FR-450-AC-8 | In a `Bounce` variant whose `Up` transition guard is `descending(x) and x = 1`, define the nonrecursive pure Boolean wrapper `descending(s: Rational[0, 20000]): Boolean pure { descend(20000) }` and the recursive pure Boolean function `descend(n: Int[0, 20000]): Boolean pure decreases(n) { if n = 0 then true else descend(n - 1) }`. S3 admits the guard call to `descending` because its argument and parameter are rational, while the `Int` recursion stays in the wrapper body outside the hybrid-term call boundary; QSpec FR-146 admits `n` as an integer measure. Static replay evaluates 20,000 recursive edges, making 20,001 recursive `descend` calls plus the one `descending` wrapper call, on an explicit stack; with `work_units` 100, the meter exhausts while evaluating the guard and replay returns `HybridReplayRefusal::ResourceExhausted{counter: WorkUnits}`, settling `incomplete`, `LimitReached{limit: work_units, value: 100, setting: accounting.work_units}`, with no verdict or counterexample; with `work_units` `u64::MAX`, it returns `Consistent` without stack overflow. | Test (TC-1221) |

## Dependencies

- QSpec FR-193 "Hybrid trace" and "Trace replay" (the trace shape and the
  replay rules); QSpec FR-331 (the `HybridCounterexample`); QSpec FR-364
  (`reproduced-with-evaluated-witness` and `replay-parity`).
- QSpec FR-146 (recursive function measures).
- ADR-026 HY-2; ADR-013 O-16.
- FR-255 (accounting limits and the `accounting.work_units` setting).
- [FR-447](FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md)
  (the checked hybrid model and claim),
  [FR-250](FR-250-map-a-hybrid-solver-result-to-a-verdict.md) (the verdict
  map and `HybridTrace`),
  [FR-290](FR-290-settle-plugin-results-as-typed-terminal-records.md) (the plugin reader).
