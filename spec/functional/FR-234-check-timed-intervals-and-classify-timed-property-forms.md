---
id: FR-234
title: "Check timed intervals and classify timed property forms"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-233
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-090
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-250
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-255
    type: depends_on
---
# FR-234: Check timed intervals and classify timed property forms

## Description

QSL's S3 checker SHALL admit the timed profile over a `model-time` claim
and over a timed trace, check timed intervals with open or closed ends on
every interval-capable operator, and record each claim's timed property
form, TT-1 to TT-4 (ADR-026 DF-1 to DF-7). The form decides which engine
arm decides the claim (FR-239, FR-242) and, with the decidability boundary
of DF-5, whether a punctual interval is admitted.

## Use case

A verification operator writes `eventually[0 ms, 3 ms)` with a strict
upper end, an `always` over data that must hold at every instant, and a
bounded deadline with an exact punctual interval. The checker admits each
in the form whose engine decides it, and tells them in advance when a
punctual interval inside an unbounded liveness claim cannot be decided.

## Inputs

- A checked clause bound `ModelTime` (FR-233), or a clause over a timed
  trace, with its formula and activation.

## Outputs

```rust
pub struct TimedInterval {
    pub lower: ExactRational, pub lower_closed: bool,
    pub upper: ExactRational, pub upper_closed: bool,
}

pub enum TimedPropertyForm {
    TimedInvariant,                          // TT-1
    BoundedWindow { horizon: ExactRational },// TT-2
    TimedSafety,                             // TT-3
    TimedLiveness,                           // TT-4
}
```

on the checked clause, and the `Unsupported(PunctualInterval)` disposition
for a clause DF-5 does not admit over a timed subject.

## Behavior

- A `ModelTime` clause SHALL check under the timed profile. The timed
  profile SHALL admit every unbounded operator infinite-trace admits.
- Every interval-capable operator (`eventually`, `always`, `until`,
  `release`, `once`, `historically`, `since`, `triggered`) SHALL admit a
  timed interval `[a, b]`, `(a, b]`, `[a, b)` or `(a, b)` with exact
  rational bounds `0 <= a <= b`.
- If an interval has `a > b`, or has an open end with `a = b`, then the
  checker SHALL refuse `invalid_timed_model`/`invalid-interval` at
  the interval's span. An interval `[a,*]` SHALL refuse as ADR-014 TR-3
  states.
- The checker SHALL mark an interval with `a = b` and both ends closed as
  punctual.
- **Forms.** The checker SHALL classify each timed clause, in this order:
  `TimedInvariant` for `always holds(I)` with `I` a state predicate;
  `BoundedWindow{horizon}` for activation `on origin` with only interval
  operators, `horizon` the sum of the formula's upper bounds;
  `TimedSafety` for a formula in ADR-014 A-4's safety fragment;
  `TimedLiveness` for any other admitted formula.
- **Punctuality.** When a `TimedInvariant`, `TimedSafety` or
  `TimedLiveness` clause over a timed subject holds a punctual interval,
  the request writer SHALL dispose of its item as `Unsupported`,
  `PunctualInterval`, naming the interval's span, and route it to no
  engine. A `BoundedWindow` clause SHALL admit punctual intervals.
- Over a single timed trace every admitted formula SHALL evaluate,
  punctual intervals included.
- **Pointwise meaning.** The layer-5 evaluator SHALL implement QSpec
  FR-416's pointwise meaning of every timed operator and its stutter
  invariance, deciding each time-stamp difference's membership in an
  interval in exact arithmetic with each end's openness.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-234-AC-1 | Over `Rpc`, `Settles` classifies as `TimedSafety`; `always holds(not c.late)` under the timed profile as `TimedInvariant`; `eventually[0 ms, 3 ms] holds(c.phase = Replied)` on origin as `BoundedWindow{horizon: 3}`; `always eventually[0 ms, 5 ms] holds(c.phase = Idle)` as `TimedLiveness`. | Test (TC-689) |
| FR-234-AC-2 | `[0 ms, 3 ms)` and `(1 ms, 3 ms]` check with their ends' openness recorded and different FR-255 keys from `[0 ms, 3 ms]`; `(2 ms, 2 ms]` and `[3 ms, 2 ms]` refuse `invalid_timed_model`/`invalid-interval` at the interval's span. | Test (TC-689) |
| FR-234-AC-3 | `always (holds(p) implies eventually[2 ms, 2 ms] holds(q))` over a timed subject is disposed `Unsupported`, `PunctualInterval`, naming the interval; the bounded `eventually[2 ms, 2 ms] holds(q)` on origin is admitted as `BoundedWindow{horizon: 2}`; the unbounded form evaluates on a supplied timed trace. | Test (TC-689) |
| FR-234-AC-4 | On the timed trace with stamps `0, 1, 3` and `q` true only at position 2, `eventually[0 ms, 3 ms] holds(q)` at 0 is `true`, `eventually[0 ms, 3 ms) holds(q)` at 0 is `false`, and `once(1 ms, 2 ms] holds(p)` at 2 with `p` only at position 1 is `true`. Inserting a step that repeats position 1's discrete state at stamp 1 leaves each value unchanged. | Test (TC-689) |

## Dependencies

- ADR-026 §6 DF-1 to DF-7; ADR-014 TR-3 and A-4; ADR-018 TP-1 to TP-4.
- [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md),
  [FR-233](FR-233-bind-a-model-claim-to-model-time-or-model-steps.md).
- QSpec FR-416 (pointwise meaning of timed operators), QSpec FR-090 and
  FR-250 (the profile row), QSpec FR-255 (interval keys
  with end openness).

## References

- QSpec half: QSpec FR-416 (Linear STD-139) owns the timed profile row,
  its pointwise semantics and interval openness, with QSpec FR-250 and
  FR-255 (ADR-026 OV-1, OV-4).
