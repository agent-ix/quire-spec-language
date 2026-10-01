---
id: TC-839
title: "S6a evaluates an infinite-trace clause exactly over a fair lasso"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-329
    type: verifies
---
# TC-839: S6a evaluates an infinite-trace clause exactly over a fair lasso

## Description

Verify ADR-018 SM-8 lasso evaluation with past operators reaching into the
prefix, malformed-lasso refusals, fairness on model-step and observed
lassos, and metering.

Scope: FR-329-AC-1 to FR-329-AC-5.

## Test Procedure

1. On the observed lasso (empty prefix, loop 0, 1, 2) evaluate the three
   clauses of FR-329-AC-1.
2. On the observed lasso (prefix 5, loop 0, 1, 2) evaluate the two clauses
   of FR-329-AC-2.
3. Evaluate any clause over a lasso with an empty loop, and over a
   `Lasso::Model` whose last loop post-state is not its entry state.
4. Over ADR-018 §6's `upd(a)` lasso as `Lasso::Model`, evaluate
   `always eventually holds(b.versionNumber = 2)` under `fair weak
   attemptUpdate` and under `fair weak each attemptUpdate`; then under
   `fair weak attemptUpdate` over the same lasso as `Lasso::Observed`.
5. Evaluate step 1's first clause with a meter one unit short of its visit
   count.

Tag the tests `#[trace("TC-839", "FR-329-AC-n")]`.

## Expected Results

- Step 1: `true`; `true`; `false` at 0.
- Step 2: `true`; `false` at 0.
- Step 3: `invalid_runtime_input`/`invalid-value` for each.
- Step 4: `false` at 0; refusal `invalid_runtime_input`/`invalid-value`
  naming the `each` constraint; refusal `invalid_runtime_input`/
  `invalid-value` for the observed lasso.
- Step 5: `Incomplete` with no truth value.
