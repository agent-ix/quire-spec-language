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
prefix, malformed-lasso refusals, fairness on a model-step lasso, and
metering.

Scope: FR-329-AC-1 to FR-329-AC-5 and FR-329-AC-8 to FR-329-AC-10.

## Test Procedure

1. On the observed lasso (empty prefix, loop 0, 1, 2) evaluate the three
   clauses of FR-329-AC-1.
2. On the observed lasso (prefix 5, loop 0, 1, 2) evaluate the two clauses
   of FR-329-AC-2.
3. Evaluate any clause over a lasso with an empty loop, and over a
   `Lasso::Model` whose last loop post-state is not its entry state.
4. Over ADR-018 §6's `upd(a)` lasso as `Lasso::Model`, evaluate
   `always eventually holds(b.versionNumber = 2)` under `fair weak
   attemptUpdate` and under `fair weak each attemptUpdate`.
5. Evaluate step 1's first clause with a meter one unit short of its visit
   count.
6. Under a bounded profile, evaluate `eventually[1,1] holds(p)` and
   `eventually[0,1] holds(p)` over a fair zero-counted `Lasso::Model` on a
   protocol subject (loop of `fork` and `join` steps, `p` false throughout
   the loop, true at position 0).
7. Under a bounded profile, evaluate a clause over a `Lasso::Observed`, over
   a `Lasso::Model` whose loop has one counted step, and over a zero-counted
   `Lasso::Model` that is unfair for a scheduler constraint.

Tag the tests `#[trace("TC-839", "FR-329-AC-n")]`.

## Expected Results

- Step 1: `true`; `true`; `false` at 0.
- Step 2: `true`; `false` at 0.
- Step 3: `invalid_runtime_input`/`invalid-value` for each.
- Step 4: `false` at 0; refusal `invalid_runtime_input`/`invalid-value`
  naming the `each` constraint.
- Step 5: `Incomplete` with no truth value.
- Step 6: `false` at 0; `true`.
- Step 7: `invalid_runtime_input`/`invalid-value` for each, the unfair one
  naming the scheduler constraint, and no truth value.
