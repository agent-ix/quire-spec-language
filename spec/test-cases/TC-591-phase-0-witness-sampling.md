---
id: TC-591
title: "Phase 0 samples seeded witnesses for possible claims, on by default"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-166
    type: verifies
---
# TC-591: Phase 0 samples seeded witnesses for possible claims, on by default

## Description

Verify that EN-1 phase 0 draws `witness_samples` seeded walks per initial
state for each `possible` item, keeps the first walk that reaches the
target, records the seed, is turned off by 0, numbers walks by initial
state, and stops without panicking when the trace index would overflow.

Scope: FR-166-AC-1 to FR-166-AC-4.

## Test Procedure

Fixtures: ADR-022 §7.1's subject; the same unit with two initial snapshots,
`(0, 0)` and `(1, 1)`.

1. `ReachesTwo` with default limits and no seed.
2. `ReachesTwo` with `witness_samples` 0; `ReachesThree` with the default.
3. `ReachesTwo` over the two-snapshot subject; with seed 7; twice with no
   seed.
4. `ReachesTwo` over the two-snapshot subject with `witness_samples`
   `u64::MAX`; with a poll that returns `true`.

Tag the tests `#[trace("TC-591", "FR-166-AC-n")]`.

## Expected Results

- Step 1: each instance's witness ends at the first visited state where the
  bound config's `versionNumber` is 2, source `Sampled`, seed 0
  (`DEFAULT_WITNESS_SEED`), trace index below 64; phase 1 searches for no
  explored witness for the item.
- Step 2: no walk is drawn and witnesses come from phase 1; 64 walks per
  instance, no witness, and the item is left to phase 1.
- Step 3: initial state 1's witness has a trace index in `64..128`; seed 7
  is recorded; the two runs give byte-equal witnesses and record the
  default seed.
- Step 4: `Stopped(ResourceExhausted, WitnessSamples)` before any walk,
  naming the limit and its value; the run stops before the first walk.
