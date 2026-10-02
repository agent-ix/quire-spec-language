---
id: FR-197
title: "Decide finite-horizon forms by exact backward induction"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-023
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-028
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: depends_on
---
# FR-197: Decide finite-horizon forms by exact backward induction

## Description

EN-5 SHALL decide a probability bound, a quantile and a mean of a fraction
(ADR-028 XF-1 to XF-3) on FR-196's product by backward induction over its
acyclic undecided states (FH-1): a weighted sum under a workload, the
minimum or maximum over actions over every scheduler. It SHALL compute in
exact rationals first and, when a value passes `max_rational_bits`, over
dyadic intervals in integer arithmetic with outward rounding, doubling the
precision up to `max_precision_bits` (FH-2, AR-1 to AR-3). No floating-point
value SHALL enter the computation.

## Use case

EN-5 decides §15.4's `Deliver` over every scheduler. One pass from the
decided states back to the start gives the minimum `24/25` and the action
that attains it at each state, which is the witness scheduler when the
bound fails.

## Inputs

- FR-196's `ProbProduct` for a claim with a bounded event or measure.
- FR-203's `ExactProbLimits`: `max_rational_bits`, `precision_bits`,
  `max_precision_bits`.

## Outputs

```rust
pub enum FhResult {
    Exact { values: Vec<Rational>, policy: Option<Vec<ActionChoice>> },
    Bounds { lower: Vec<Dyadic>, upper: Vec<Dyadic>, precision_bits: u32, policy: Option<Vec<ActionChoice>> },
    PrecisionBudget { lower: Dyadic, upper: Dyadic },
}
pub struct Dyadic { pub mantissa: BigInt, pub exponent: u32 }   // mantissa · 2^-exponent
```

## Behavior

- A decided state SHALL take its terminal value: 1 or 0 for an event; for a
  quantile over every scheduler, XF-2's terminal reward
  `[activated] · ([M <= c] − q)`, or `[activated] · ([M < c] − q)` for
  `>= c`, maximized; for a mean of a fraction, the fraction computed from
  the accumulated sums.
- EN-5 SHALL visit undecided states in reverse topological order and give
  each the probability-weighted sum over its successors under a workload, or
  the minimum (for a `>= θ` bound) or maximum (for `<= θ`) over its actions
  of that sum over every scheduler. An intermediate state (FR-196) SHALL
  take the minimum or maximum over its post-states.
- Under a workload, EN-5 SHALL decide a quantile claim as the ratio of two
  finite-horizon probabilities, `Pr(M <= c and activated) / Pr(activated)`.
  Over every scheduler EN-5 SHALL decide `<= c` by the sign of XF-2's
  minimum, and `>= c` by XF-2's strict rule: fail when the maximum `v` is
  positive, hold when it is negative, and when `v = 0` fail exactly when
  the maximum activation probability over the actions attaining `v` is
  positive. Under a workload, `>= c` SHALL hold only when `Pr(M < c and activated) / Pr(activated) < q`.
- When the activation probability is 0, EN-5 SHALL settle both quantile
  forms `proved`, as XF-2 does over every scheduler, where a scheduler that
  never activates contributes 0.
- EN-5 SHALL compute in exact rationals in lowest terms. When a value's
  numerator or denominator exceeds `max_rational_bits` bits, it SHALL rerun
  over dyadic intervals at `precision_bits`, rounding each lower end down and
  each upper end up by integer floor and ceiling division. When the interval
  at an initial state still contains the threshold, it SHALL double the
  precision and rerun, up to `max_precision_bits`; an interval that still
  contains it there SHALL return `PrecisionBudget` with that interval.
- A value equal to the threshold SHALL meet a non-strict bound; only the
  exact pass decides it.
- A value equal to the threshold SHALL fail the strict bound of the `>=`
  quantile form.
- Over every scheduler, EN-5 SHALL record the action attaining the minimum
  or maximum at each state, ties broken by canonical transition order, as
  the memoryless deterministic policy (FH-4).
- One pass SHALL visit each product edge once.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-197-AC-1 | §15.4: over every scheduler the value is `24/25` at the start and `4/5` after one loss, with `send_b` chosen at both live states; the maximum is `99/100`; under `Even` the value is `391/400` at the start and `17/20` after one loss. | Test (TC-632) |
| FR-197-AC-2 | §15.2: `Pr(M <= 5 ms) = 24233/25000` and `Pr(M <= 2 ms) = 4491/5000`, exact; the XF-2 transform over every scheduler gives the same verdicts as the ratio under `Steady`, since only one operation is enabled at each state. | Test (TC-632) |
| FR-197-AC-3 | §15.1 with `max_rational_bits` at least 23,254 returns `Exact` with `(9999999/10000000)^1000`, whose denominator is `10^7000`. With `max_rational_bits` 1,024 and `precision_bits` 64 it returns `Bounds` whose interval has width at most `2 · 1001 · 2^-64` and contains the exact value, and lies above `999/1000`. | Test (TC-632) |
| FR-197-AC-4 | A mean-of-fraction claim `mean of fraction holds(v.up) over [0, 2] >= 0.9` over ADR-024 §7.3's `Avail` under `Steady` gives the exact expectation `(1 + 1999/2000 + (1999/2000)^2 + (1/2000) · 9/10) / 3` of the fraction. With `max_rational_bits` 4 and `max_precision_bits` 128, §15.5's minimum `99/100` against threshold `99/100` returns `PrecisionBudget` with an interval containing `99/100`. `quantile 1/2 of M >= 3` over a model whose `M` is 1 or 3 with probability 1/2 each settles `refuted`, over every scheduler (`v = 0` attained with activation probability 1) and under its workload (ratio exactly `1/2`, not below it). A variant whose activation is unreachable settles both `quantile 1/2 of M >= 3` and `quantile 1/2 of M <= 1` `proved` under its workload. | Test (TC-632) |

## Dependencies

- ADR-028 XF-1 to XF-3, XF-8, AR-1 to AR-3, FH-1 to FH-4.
- [FR-196](FR-196-build-the-probabilistic-product.md).

## References

- QSpec half, which owns the quantile transform and the exact values of the
  conformance vectors: QSpec FR-411 and TC-360 (Linear STD-137).
- Owning ticket: Linear QSL-371.
