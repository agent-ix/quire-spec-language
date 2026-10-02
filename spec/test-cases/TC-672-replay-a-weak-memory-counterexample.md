---
id: TC-672
title: "Weak-memory counterexamples carry memory components and replay"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-227
    type: verifies
---
# TC-672: Weak-memory counterexamples carry memory components and replay

## Description

Verify memory components in counterexamples, their recomputation in replay, depth checks, race replay by vector clocks, and lasso fairness under the memory constraints.

Scope: FR-227-AC-1 to FR-227-AC-4.

## Test Procedure

1. Replay `SB`'s `tso` counterexample; replay it with the resolved model
   changed to `sc`.
2. Replay `SB`'s `ra` counterexample; replay it with `Lx`'s depth 2 and
   with a changed message view.
3. Replay FR-224-AC-2's race counterexample, the forged-`earlier` envelope
   and the forged race over the `release`/`acquire` shape.
4. Replay FR-228-AC-2's unflushed lasso.

Tag the tests `#[trace("TC-672", "FR-227-AC-n")]`.

## Expected Results

- Step 1: buffers match §10 and `reproduced-with-evaluated-witness`;
  `stale_dependency`/`revision-mismatch`.
- Step 2: `reproduced-with-evaluated-witness`; `invalid_runtime_input`/
  `invalid-value`; `stale_dependency`/`revision-mismatch`.
- Step 3: `reproduced-with-evaluated-witness`; `inconclusive`,
  `ReplayParity` twice.
- Step 4: `invalid_runtime_input`/`invalid-value` as unfair.
