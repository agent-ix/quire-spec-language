---
id: FR-190
title: "Measure a long-run fraction by regeneration"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-022
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-024
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-188
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-189
    type: depends_on
---
# FR-190: Measure a long-run fraction by regeneration

## Description

When the request names `Regenerative{min_cycles}`, EN-4 SHALL measure a
`long-run fraction` claim (ADR-024 PF-6) by sampling one long behaviour per
test from its initial state, cutting it into regeneration cycles at each
return to that state, and deciding from the ratio estimator and its
asymptotic confidence interval (ADR-024 ST-7). The result SHALL state
coverage `Asymptotic`.

## Use case

A verification operator requests ADR-024 §7.3's `LongRun` claim. EN-4 runs
`Avail` under `Steady`, counts up-positions per cycle between returns to
`up`, and stops when the interval half-width is at most the claim's `ι`. The
result says "measured: accepted" and labels its coverage asymptotic, so
the reader sees that the guarantee is a limit, not a finite-sample bound.

## Inputs

- FR-189's `StatisticalRequest` with `method: Regenerative{min_cycles}` and
  a `LongRunFraction` claim.

## Outputs

FR-189's `StatisticalOutcome`, whose verdict basis records
`Regenerative{min_cycles}`, the cycle count as `samples`, the steps taken as
`draws`, and `coverage: Asymptotic`.

## Behavior

- Test `j` SHALL sample one behaviour at trace index `j` from its initial
  state `s0` with FR-188's sampler, step by step, and close a cycle at each
  later position whose state key equals `s0`'s.
- Each cycle `k` SHALL contribute `Y_k`, the weighted count of its positions
  where `P` holds, and `L_k`, its total weight; unweighted, each position
  has weight 1, and `weighted by R` weights each position by the reward of
  the step that leaves it.
- After each closed cycle, with `n` cycles closed and `n >= min_cycles`,
  EN-4 SHALL compute the estimate `Â = Σ Y_k / Σ L_k` and the half-width
  `z · √(S² + 1/n) / (L̄ · √n)`, with `S` the sample standard deviation of
  `Y_k − Â · L_k`, `L̄` the mean cycle weight and `z` the one-sided
  `1 − min(α', β')` normal quantile. The square root and the quantile SHALL
  be QSpec's rational bounds, rounded so the half-width rounds up.
- The test SHALL stop when the half-width is at most `ι`, and decide as
  Okamoto does (FR-189) over the interval `[Â − w, Â + w]`.
- When a cycle reaches `max_cycle_steps` steps without returning to `s0`,
  EN-4 SHALL end the test Undecided with cause
  `NoRegeneration{max_cycle_steps}`, naming the setting, its value and
  `statistical.max_cycle_steps`, so a transient initial state, which never
  regenerates, ends there (FR-191).
- A cycle whose total weight is 0 SHALL be kept; a run whose closed cycles
  all have weight 0 has no estimate yet and continues.
- A request naming the regenerative method for a claim that is not a
  long-run fraction, or the Okamoto or SPRT method for a long-run fraction,
  EN-4 SHALL refuse as `NotStatistical`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-190-AC-1 | `LongRun` (ADR-024 §7.3) with `Regenerative{min_cycles: 100}` settles `Completed` Accepted with an estimate within `1/5000` of `1800/1801`, an interval of half-width at most `1/5000` whose lower end is at least `999/1000`, a cycle count equal to the first `n >= 100` at which the half-width recomputed from the recorded cycles is at most `1/5000`, and `coverage: Asymptotic`. `long-run fraction holds(v.up) <= 0.9995` with the same parameters settles `Completed` Undecided, `IndifferenceRegion`, and a run whose first 100 cycles have no failure does not stop at 100 cycles. Two runs with the same seed give equal outcomes. | Test (TC-625) |
| FR-190-AC-2 | A variant of `Avail` whose initial state is a start state that no step re-enters settles `Completed` Undecided with cause `NoRegeneration` naming `max_cycle_steps`, its value and `statistical.max_cycle_steps`. With `min_cycles` above the number of cycles the half-width rule would need, the run closes exactly `min_cycles` cycles before deciding. | Test (TC-625) |
| FR-190-AC-3 | A `weighted by duration` long-run claim over a variant of `Avail` with a `duration` reward of 1 on `tick` and 3 on `repair` reads each position's weight from the step that leaves it, so a down position counts 3 in `L_k`. `Regenerative` for `P95` and `Okamoto` for `LongRun` are refused `NotStatistical`. | Test (TC-625) |

## Dependencies

- ADR-024 PF-6, ST-7, ST-8, ST-9, SV-2, RU-1.
- [FR-188](FR-188-draw-weighted-choices-with-the-revised-sampler.md),
  [FR-189](FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md).

## References

- QSpec half, which owns the regenerative estimator, its interval and
  `NoRegeneration`: QSpec FR-408 (Linear STD-137).
- Owning ticket: Linear QSL-371.
