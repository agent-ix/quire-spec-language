---
id: TC-631
title: "EN-5 builds the DTMC or MDP product with monitors, accumulators and intermediate states"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: verifies
---
# TC-631: EN-5 builds the DTMC or MDP product with monitors, accumulators and intermediate states

## Description

Verify the product for §15.1, §15.2 and §15.4, intermediate states for residual nondeterminism, `NotMarkov`, and determinism.

Scope: FR-196-AC-1 to FR-196-AC-4.

## Test Procedure

Fixtures: ADR-024 §7.1 and §7.2 models and claims; `Link` with `Deliver`; the two `Health` variants of FR-187-AC-3.

1. Build §15.1's product and check layering and probability sums.
2. Build `P95`'s product at `5 ms` and read the accumulator values and an edge probability.
3. Build `Deliver` over every scheduler and under `Even`.
4. Build the `Health` variants over every scheduler and under a workload; build one request twice.

Tag the tests `#[trace("TC-631", "FR-196-AC-n")]`.

## Expected Results

- Step 1: at most 1,001 undecided states plus two decided states; sums 1; every path decided within 1,001 positions.
- Step 2: values `0`, `1 ms` to `4 ms` and saturated `6 ms`; `343/5000`.
- Step 3: an MDP with two actions per live state; a DTMC with FR-187-AC-2's probabilities.
- Step 4: intermediate states the monitor does not read; `NotMarkov` under a workload and for the empty variant; equal builds.
