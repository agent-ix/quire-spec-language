---
id: TC-696
title: "Symbolic counterexamples concretize canonically to exact rational delays"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-241
    type: verifies
---
# TC-696: Symbolic counterexamples concretize canonically to exact rational delays

## Description

Verify canonical backward concretization of paths and lassos and the no-concretization outcome.

Scope: FR-241-AC-1 to FR-241-AC-4.

## Test Procedure

Fixtures: ADR-026 §11's `Rpc` unit with `T = 3 ms` and `T = 4 ms`, universe `{c}`; the retry model; the `Serve` model; a hand-built symbolic lasso with contradictory loop constraints.

1. Concretize `NoLateReply`'s path with `T = 3 ms`.
2. Concretize the retry refutation twice and replay it.
3. Concretize FR-240-AC-2's lasso.
4. Concretize the contradictory lasso with no other accepting cycle.

Tag the tests `#[trace("TC-696", "FR-241-AC-n")]`.

## Expected Results

- Step 1: delays `0, 3, 0`.
- Step 2: `0 < d1 < 1`, `d1 + d2 > 1`, `d2 < 1`, exact; byte-equal runs; replay reproduces.
- Step 3: positive loop delay and `x > 1` at loop entry.
- Step 4: `Undecided(LassoNotConcretized)`.
