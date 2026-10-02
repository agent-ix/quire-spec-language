---
id: FR-238
title: "Represent zones as difference-bound matrices in exact integer arithmetic"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-231
    type: depends_on
---
# FR-238: Represent zones as difference-bound matrices in exact integer arithmetic

## Description

The native zone engine EN-6 SHALL represent a set of clock valuations as a
zone held in a difference-bound matrix (DBM) whose bounds are
arbitrary-precision integers or `∞`, after scaling every constant of the
model and the claim once to an integer (ADR-026 EZ-2). The DBM operations
below are the only operations the engine's symbolic search applies to
valuations.

## Use case

A verification operator models a controller with constants such as
`1/3 ms`, `2.5 ms` and `10^30 ns`. The engine checks it exactly: no
constant is too large, no bound overflows, and a strict and a non-strict
bound on the same difference stay distinct.

## Inputs

- The clocks of a timed subject (FR-231) and every constant of its model
  and claim, as exact rationals in the model's unit.

## Outputs

```rust
pub struct Dbm { dim: usize, bounds: Vec<Bound> }  // dim = clocks + 1 (reference clock)
pub enum Bound { Finite { value: BigInt, strict: bool }, Infinite }

impl Dbm {
    pub fn zero(dim: usize) -> Dbm;
    pub fn close(&mut self);                        // canonical form
    pub fn is_empty(&self) -> bool;
    pub fn constrain(&mut self, i: usize, j: usize, b: Bound) -> Result<(), DbmError>; // x_i − x_j ≺ b
    pub fn reset(&mut self, clock: usize, value: &BigInt) -> Result<(), DbmError>;
    pub fn up(&mut self);                           // time elapse
    pub fn includes(&self, other: &Dbm) -> Result<bool, DbmError>;
}

pub enum DbmError {
    ClockOutOfRange { clock: usize, dim: usize },   // an index at or above dim
    ResetReferenceClock,                            // the reference clock is fixed at 0
    DimensionMismatch { left: usize, right: usize },
}
```

and the scale factor, the least common multiple of every constant's
denominator.

## Behavior

- The engine SHALL compute one scale factor per check, the least common
  multiple of the denominators of every constant in the model and the
  claim automaton, and multiply each constant by it, so every DBM bound is
  an integer.
- A bound SHALL be `(c, <)`, `(c, <=)` or `∞`, with `c` an arbitrary
  precision integer and every addition checked; `(c, <)` SHALL be tighter
  than `(c, <=)`.
- `close` SHALL put the DBM in canonical form by shortest-path closure.
- `is_empty` SHALL hold exactly when the closed DBM has a negative cycle,
  that is, some diagonal entry tighter than `(0, <=)`.
- `constrain`, `reset`, `up` and `includes` SHALL compute intersection with
  a constraint, reset of one clock to a constant, time elapse (every upper
  bound against the reference clock set to `∞`) and zone inclusion on
  canonical DBMs.
- Every operation SHALL return a canonical DBM, or empty.
- `constrain` and `reset` with a clock index at or above `dim`, `reset` of
  the reference clock, and `includes` over DBMs of different dimensions
  SHALL return `DbmError` and leave the DBM unchanged, never panic.
- The engine SHALL fix no bound on the number of clocks, on a constant or on
  a DBM entry; memory held by stored DBMs is charged to FR-239's budget.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-238-AC-1 | With constants `1/3`, `5/2` and `1` the scale factor is 6 and the scaled constants are 2, 15 and 6. With a constant `10^30` and another `1/7`, every bound is computed with no overflow and the scale factor is 7. | Test (TC-693) |
| FR-238-AC-2 | Over clocks `x, y`: `zero` then `up` then `constrain(x <= 3)` gives the zone `0 <= x = y <= 3`; `reset(x, 0)` on it gives `x = 0, 0 <= y <= 3`; adding `y > 3` makes it empty; adding `y >= 3` does not. `includes` holds for `x <= 3` over `x < 3` and not the reverse. `constrain` naming clock 2 of a one-clock DBM returns `ClockOutOfRange`, `reset` of clock 0 returns `ResetReferenceClock` and `includes` across dimensions 2 and 3 returns `DimensionMismatch`, each leaving the DBM unchanged. | Test (TC-693) |
| FR-238-AC-3 | On 10,000 seeded random DBMs of dimension at most 5 with bounds in `[-20, 20]`, `close`, `constrain`, `reset`, `up`, `is_empty` and `includes` agree with a reference implementation that reads a zone as its unclosed constraints (time elapse and reset by Fourier-Motzkin elimination) on every point of the `1/dim` grid in one-unit windows: at a seeded anchor, at the least corner of each zone involved, and, for a refused inclusion, past the entry where the included zone is looser. A non-empty zone's least-corner window holds one of its points, so emptiness is checked in both directions. Every result is canonical: closing it again changes nothing. | Test (TC-693) |

## Dependencies

- ADR-026 §8 EZ-2.
- [FR-230](FR-230-check-time-declarations-clocks-and-clock-constraints.md),
  [FR-231](FR-231-read-a-timed-subject-s-behaviours-as-timed-traces.md).

## References

- J. Bengtsson and W. Yi, "Timed automata: semantics, algorithms and
  tools", 2004 (ADR-026 References).
