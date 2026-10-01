---
id: TC-632
title: "Backward induction decides finite-horizon forms exactly, or over dyadic intervals with precision doubling"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-197
    type: verifies
---
# TC-632: Backward induction decides finite-horizon forms exactly, or over dyadic intervals with precision doubling

## Description

Verify exact values for §15.4 and §15.2, the dyadic fallback for §15.1, a mean of a fraction, and the precision budget at equality.

Scope: FR-197-AC-1 to FR-197-AC-4.

## Test Procedure

Fixtures: `Link` and `Deliver`; `P95`; `NoFault`; `Avail` with a mean-of-fraction claim; §15.5's `Deadline` minimum.

1. Compute `Deliver`'s minimum, maximum and workload values and the policy.
2. Compute `Pr(M <= 5 ms)` and `Pr(M <= 2 ms)` under the workload and by XF-2.
3. Compute `NoFault` with `max_rational_bits` 23,254 and with 1,024 at 64 bits.
4. Compute the mean of the fraction; compute `Deadline`'s minimum with `max_rational_bits` 4 and `max_precision_bits` 128.

Tag the tests `#[trace("TC-632", "FR-197-AC-n")]`.

## Expected Results

- Step 1: `24/25`, `4/5`, `send_b`; `99/100`; `391/400`, `17/20`.
- Step 2: `24233/25000` and `4491/5000`; equal verdicts.
- Step 3: the exact rational with denominator `10^7000`; an interval of width at most `2 · 1001 · 2^-64` above `999/1000`.
- Step 4: the exact expectation of FR-197-AC-4; `PrecisionBudget` with an interval containing `99/100`.
