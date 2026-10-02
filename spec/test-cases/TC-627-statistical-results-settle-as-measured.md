---
id: TC-627
title: "Statistical results settle on the measured axis, fail the pipeline when rejected, and never count as proof"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-192
    type: verifies
---
# TC-627: Statistical results settle on the measured axis, fail the pipeline when rejected, and never count as proof

## Description

Verify the outcome-to-record map, the verdict content, the pipeline gate, proof accounting, EN-4's manifest and its refusals.

Scope: FR-192-AC-1 to FR-192-AC-4.

## Test Procedure

Fixtures: synthetic `StatisticalOutcome` values for each row; FR-189-AC-1's and FR-190-AC-1's results; a request with `P95` and the `2 ms` claim.

1. Map each outcome row.
2. Settle FR-189-AC-1's Okamoto and SPRT results and FR-190-AC-1's result.
3. Gate the two-item request, then the request without the rejected item; read the proof summary.
4. Read EN-4's manifest; call `check_statistical` with an every-scheduler claim, `NoFault` without confidence and an `expected accumulate` claim.

Tag the tests `#[trace("TC-627", "FR-192-AC-n")]`.

## Expected Results

- Step 1: each row as FR-192's table states, with no truth and no basis.
- Step 2: the values of FR-192-AC-2.
- Step 3: the gate fails as on one violation, then passes; 0 proved.
- Step 4: exactly (`probabilistic-satisfaction`, `statistical`); `EveryScheduler`, `MissingConfidence`, `ExactOnlyForm`, each `unsupported` with no sample drawn.
