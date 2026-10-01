---
id: TC-709
title: "Timed runs sample with exact rational delays, measure timed events and replay"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-254
    type: verifies
---
# TC-709: Timed runs sample with exact rational delays, measure timed events and replay

## Description

Verify timed statistical measurement, exact delays on the `2^-q` grid, witness replay with draw recomputation, latency measures, regeneration and the exact route.

Scope: FR-254-AC-1 to FR-254-AC-3.

## Test Procedure

Fixtures: the stochastic `Rpc` variant and its `uniform` reply variant; a model that never regenerates; the strict-guard variant; the variant with no `delay` member on `timeout`.

1. Measure `RareLate` over both variants, twice with one seed.
2. Inspect sampled delays, replay a witness, change one delay and replay; repeat with `q` 8.
3. Measure the latency, run the regenerative method on the non-regenerating model, and request `exact` evidence over the strict-guard variant; measure `RareLate` over the free-`timeout` variant; measure `RareLate` with `max_sample_steps` 1.

Tag the tests `#[trace("TC-709", "FR-254-AC-n")]`.

## Expected Results

- Step 1: `measured`, `Rejected`, `q` 64 in the basis; `measured`, `Accepted`; equal verdicts.
- Step 2: exact rationals; replay and draw recomputation succeed; recomputation fails; basis records 8 and delays lie on the grid's rounded images.
- Step 3: per-sample time-stamp differences; `undecided`, `NoRegeneration`; `unsupported`; `unsupported`, `NotStochastic` naming the state after `send` and `timeout`; `Incomplete(ResourceExhausted)` naming `max_sample_steps` and 1.
