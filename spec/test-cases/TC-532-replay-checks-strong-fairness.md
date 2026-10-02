---
id: TC-532
title: "Replay checks strong fairness on a model counterexample"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-131
    type: verifies
---
# TC-532: Replay checks strong fairness on a model counterexample

## Description

Verify that `replay_model_trace` checks strong constraints against the loop
with enabledness from the model, accepts a terminal stutter loop, refuses a
weakly fair but strongly unfair lasso, and that such a refusal of an EN-1
counterexample settles `ReplayParity`.

Scope: FR-131-AC-1 to FR-131-AC-3.

## Test Procedure

1. Replay the mutex lasso `0 -acq(2)-> 2 -rel-> 0` in envelopes for the
   `weak each`, `strong` (whole) and `strong each` clauses; then the same
   lasso with its last step removed, for the `weak each` clause.
2. Replay FR-130-AC-2's `Handoff` lasso under its clause; replay FR-126-AC-3's
   stutter lasso in an envelope for the claim with `strong` on `inc`
   added.
3. Settle, as EN-1's, a `Violated` outcome carrying step 1's lasso for the
   `strong each` clause; then one carrying the truncated lasso.

Tag the tests `#[trace("TC-532", "FR-131-AC-n")]`.

## Expected Results

- Step 1: reproduced; reproduced; `ReplayRefusal::UnfairLasso{constraint:
  {Strong, acquire, Each}}`, catalog code `invalid_runtime_input`/
  `invalid-value`, no result; the truncated lasso refuses with FR-128's
  loop-closure refusal, not `UnfairLasso`.
- Step 2: reproduced twice.
- Step 3: `inconclusive`, `ReplayParity`, category inconclusive; then
  `inconclusive`, `ReplayRefused`.
