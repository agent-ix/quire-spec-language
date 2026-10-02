---
id: TC-625
title: "EN-4 measures a long-run fraction by regeneration with asymptotic coverage"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-190
    type: verifies
---
# TC-625: EN-4 measures a long-run fraction by regeneration with asymptotic coverage

## Description

Verify the regenerative method on `LongRun`, its transient stop and `min_cycles`, weighting, and method refusals.

Scope: FR-190-AC-1 to FR-190-AC-3.

## Test Procedure

Fixtures: `Avail` and `LongRun` (ADR-024 §7.3); its transient-start variant; its `duration`-weighted variant. Seed 7.

1. Run `LongRun` with `Regenerative{min_cycles: 100}` twice; run `long-run fraction holds(v.up) <= 0.9995` with the same parameters; recompute the half-width at each recorded cycle count.
2. Run the transient variant; run `LongRun` with a large `min_cycles`.
3. Run the weighted variant; request `Regenerative` for `P95` and `Okamoto` for `LongRun`.

Tag the tests `#[trace("TC-625", "FR-190-AC-n")]`.

## Expected Results

- Step 1: Accepted, estimate within `1/5000` of `1800/1801`, `coverage: Asymptotic`, equal outcomes, the cycle count the first `n >= 100` with half-width at most `1/5000` and above 100; Undecided, `IndifferenceRegion`.
- Step 2: `Completed` Undecided, `NoRegeneration` naming `max_cycle_steps`; exactly `min_cycles` cycles closed.
- Step 3: down positions weigh 3; both refused `NotStatistical`.
