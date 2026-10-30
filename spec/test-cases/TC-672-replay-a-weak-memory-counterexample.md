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

Verify memory components in counterexamples, depth checks, race replay by vector clocks, and lasso fairness under the memory constraints.

Scope: FR-227-AC-1 to FR-227-AC-4, FR-227-AC-7.

## Test Procedure

1. Replay `SB`'s `tso` counterexample.
2. Replay `SB`'s `ra` counterexample; replay it with `Lx`'s depth 2.
3. Replay FR-224-AC-2's race counterexample, the forged-`earlier` envelope
   and the forged race over the `release`/`acquire` shape.
4. Replay FR-228-AC-2's unflushed `tso` lasso and an FR-228-AC-3 `ra`
   lasso whose loop repeatedly loads location `ℓ` by thread `b` from
   non-mo-last messages, with no mo-last load by `b` in the loop.
5. Run FR-227-AC-7's `SB` with `Edge` resolved to `tso` and replay its
   counterexample, then with its `flush` step removed; run it resolved to
   `ra` and replay its counterexample.

Tag the tests `#[trace("TC-672", "FR-227-AC-n")]`.

## Expected Results

- Step 1: buffers match §10 and `reproduced-with-evaluated-witness`.
- Step 2: `reproduced-with-evaluated-witness`; `invalid_runtime_input`/
  `invalid-value`.
- Step 3: `reproduced-with-evaluated-witness`; `inconclusive`,
  `ReplayParity` twice.
- Step 4: `invalid_runtime_input`/`invalid-value` as unfair under
  `flush(w)` for `tso` and under the strong visibility constraint for the
  loaded `(thread, location)` pair for `ra`.
- Step 5: a two-step counterexample, a store and its thread's `flush`,
  `where.locus` naming `Edge`, and `reproduced-with-evaluated-witness`;
  `inconclusive`, `ReplayParity`; a one-step counterexample under `ra`, and
  `reproduced-with-evaluated-witness`.
