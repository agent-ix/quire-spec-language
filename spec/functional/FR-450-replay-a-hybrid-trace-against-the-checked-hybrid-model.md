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
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: references
  - target: ix://agent-ix/quire-specification/FR-193
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
replay checks every trace in exact rational interval arithmetic within the
trace's tolerance and records the outcome on the `inconclusive` record. Exact
replay decides a point trace through modes with polynomial flows exactly, and
a trace it reproduces settles the item `refuted` with the trace as its
counterexample. This is ADR-026 HY-2's one refutation path for a hybrid item.

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

## Outputs

```rust
pub enum StaticReplay {
    Consistent,
    Inconsistent { at: JsonPointer, check: StaticCheck }, // the first failing check
}
pub enum StaticCheck { Initial, Invariant, Time, Jump, Loop } // QSpec FR-193 static checks 1 to 5
pub enum ExactReplay {
    NotExact,                                   // not a point trace, or a mode without a polynomial flow
    Reproduced(HybridCounterexample),
    NotReproduced { at: JsonPointer, check: ExactCheck },
}
pub enum ExactCheck { Initial, Dwell, Jump, Loop, Claim } // QSpec FR-193 exact checks 1 to 5
pub struct HybridCounterexample {
    pub kind: HybridCounterexampleKind,         // Formula | UndefinedEvaluation{where, cause}
    pub trace: HybridTrace,
}
```

`InconclusiveCause::SolverInconclusive { trace: Option<HybridTrace>, replay:
Option<StaticReplay> }`, and for a reproduced trace
`TerminalValue::Refuted` with its `HybridCounterexample` as the item's
`Counterexample`.

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
  order, with the member's pointer.
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
  real line for an unbounded numeric type, undecided for `Boolean`), so
  static replay always terminates.
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
- A provider's `refuted` entry whose trace gives `NotReproduced` or
  `NotExact` SHALL settle the item `inconclusive`, `ReplayParity` (QSpec
  FR-364), with no `counterexamples` entry.
- No other path SHALL settle a hybrid item `refuted`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-450-AC-1 | Over FR-447's `Braking`, a `solver-inconclusive` trace whose `start` box `v` in `[25, 26]` meets no initial entry replays `Inconsistent { at: /start, check: Initial }`; a trace whose `before` box `v` in `[1/200, 1/100]` misses the guard `v = 0` by less than its `delta` 1/100 replays `Consistent`, and the same trace with no `delta` `Inconsistent` at check `Jump`; a trace whose second state's `time.upper` is below the first's `time.lower` is `Inconsistent` at check `Time`; each item settles `inconclusive`, `SolverInconclusive{trace: Some(..), replay: Some(..)}` with that outcome and no `counterexamples` entry; a `delta` of `-1/100` is refused `invalid_result`/`invalid-value` at `/delta`. | Test (TC-1221) |
| FR-450-AC-2 | Against `always [0, 12] holds(h.v > 0)`, the point trace from `(Moving, v = 20, a = 2, t = 0)` with `before` `(time 10, v = 0, a = 2, t = 10)` and transition 0 to `(Stopped, time 10, v = 0, a = 2, t = 10)` replays `Reproduced` with kind `Formula`, from a `solver-inconclusive` cause and from a provider's `refuted` entry alike, and the item settles `refuted` with that trace as its `Counterexample` and one `counterexamples` entry; against `SlowEnough` the same trace gives `NotReproduced` at check `Claim` and the item stays `inconclusive`. | Test (TC-1221) |
| FR-450-AC-3 | The AC-2 trace with `before` `v = 1` gives `NotReproduced` at check `Dwell`; the same trace with `before.time` 9 gives `NotReproduced` at check `Dwell`; as a provider's `refuted` entry each settles `inconclusive`, `ReplayParity`, with no `counterexamples` entry. | Test (TC-1221) |
| FR-450-AC-4 | With `Moving`'s flow `v' = -v`, the AC-2 trace gives `NotExact` and is replayed statically only, settling `inconclusive`; with `Moving`'s flows `v' = -a` and `a' = -1 / 10` it is replayed exactly, with `a(t) = 2 - t / 10` and `v(t) = 20 - 2t + t^2 / 20`, and gives `NotReproduced` at check `Dwell` because `v(10) = 5`; with a flow `v' = drag(v)` over a recursive `drag`, it gives `NotExact`. | Test (TC-1221) |
| FR-450-AC-5 | The AC-2 trace against `always [0, 12] holds(1 / (h.v - 5) != 0)`, true up to the undefined instant, replays `Reproduced` with kind `UndefinedEvaluation`, naming the instant at time 15/2 in `Moving`, where `v = 5`, and the division's locus, and settles `refuted` with cause `UndefinedEvaluation{where, cause}`; against `always [0, 12] holds(1 / (h.v - 5) < 100)`, false from time 1499/200 before that instant, it replays `Reproduced` with kind `Formula`, the decisive instant 1499/200; against a claim whose atom calls a recursive function it gives `NotExact`; static replay of a guard calling a recursive function whose measure interval is undecided terminates with that call's result range. | Test (TC-1221) |
| FR-450-AC-6 | Over `Bounce` (mode `Up` with `x' = 1` and a transition to `Down` when `x = 1`; mode `Down` with `x' = -1` and a transition to `Up` when `x = 0`; resets `true`), against `eventually always holds(h.x < 1)` under the infinite-trace profile, the point trace from `(Up, x = 0)` through `Down` at time 1 back to `(Up, x = 0)` at time 2 with `loop_to` 0 reproduces as a periodic behaviour, also with `fair Up;`; the same trace with `loop_to` 1, whose last state `(Up, x = 0)` is not the `Down` state it names, gives `NotReproduced` at check `Loop`; the same trace whose last state has `x = 1/2` gives `NotReproduced` at check `Jump`, the reset `true` keeping `x = 0`; and a `loop_to` 2, naming the last state, is refused `invalid_result`/`invalid-value` at `/loop`. | Test (TC-1221) |

## Dependencies

- QSpec FR-193 "Hybrid trace" and "Trace replay" (the trace shape and the
  replay rules); QSpec FR-331 (the `HybridCounterexample`); QSpec FR-364
  (`reproduced-with-evaluated-witness` and `replay-parity`).
- ADR-026 HY-2; ADR-013 O-16.
- [FR-447](FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md)
  (the checked hybrid model and claim),
  [FR-250](FR-250-map-a-hybrid-solver-result-to-a-verdict.md) (the verdict
  map and `HybridTrace`),
  [FR-290](FR-290-settle-plugin-results-as-typed-terminal-records.md) (the plugin reader).
