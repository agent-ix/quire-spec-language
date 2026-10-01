---
id: TC-539
title: "Replay reproduces an undefined claim evaluation at its position"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: verifies
---
# TC-539: Replay reproduces an undefined claim evaluation at its position

## Description

Verify that `replay_model_trace` reproduces an `UndefinedEvaluation`
counterexample by evaluating the replayed letters and finding the first
undefined one at `where` with the recorded cause, and that any other
position or a missing position is a disagreement.

Scope: FR-128-AC-5.

## Test Procedure

Take TC-537 step 3's first counterexample, with the `Counter` snapshot and
source in the byte provision and universe `{c}` in the request.

1. Replay it.
2. Replay it with `where` set to 1.
3. Replay it with its last step removed, so it ends at value 1.

Tag the test `#[trace("TC-539", "FR-128-AC-5")]`.

## Expected Results

- Step 1: `reproduced-with-evaluated-witness`, `trace_position` 2, value
  `UndefinedEvaluation{where: 2, cause: division-by-zero}`.
- Steps 2 and 3: `inconclusive`, `Verdicts`.
