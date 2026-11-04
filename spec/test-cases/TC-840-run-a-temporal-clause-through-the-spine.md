---
id: TC-840
title: "run_clause runs a selected temporal clause over a supplied trace"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-330
    type: verifies
---
# TC-840: run_clause runs a selected temporal clause over a supplied trace

## Description

Verify the `Temporal` selection of `run_clause` from source: selection,
admission, evaluation dispatch by profile and trace shape, categories, exit
codes.

Scope: FR-330-AC-1 to FR-330-AC-4 and FR-330-AC-12.

## Test Procedure

Use a `Counter` unit with clauses `Bounded` (`eventually[0,1] holds(c.value
= 2)`, event-position false-extension) and `Reaches` (`eventually
holds(c.value = 2)`, infinite-trace), and snapshots with `c.value` 0, 1, 2.

1. Run `Bounded` over the `Finite` trace 0, 1, 2.
2. Run `Reaches` over that `Finite` trace, over `Lasso` loop 0, 1 and over
   `Lasso` loop 0, 1, 2.
3. Run a selection naming `Absent`; `Bounded` over a `Lasso`; `Reaches` with
   `over` key `ghost`; `Reaches` over a `Lasso` with an empty loop.
4. Run `Reaches` over `Lasso` loop 0, 1, 2 with a work budget of zero; run
   every request of steps 1 to 3 twice.

5. Read the `decisive` of step 1's violation, of `always holds(c.value <= 1)` `on origin`, of the `forall` clause over `items` 1, 2, 3 at positions 0 and 1 and 3, 0, 5 at position 2, and of a pending, a successful and an `Undefined` report.

Tag the tests `#[trace("TC-840", "FR-330-AC-n")]`.

## Expected Results

- Step 1: stage `evaluate`, `violation`, position 0, exit 10.
- Step 2: `pending`, exit 0, no `truth`; `violation` at 0; `success`,
  exit 0.
- Step 3: `select` `missing_declaration`/`missing-name`; `admit`
  `invalid_runtime_input`/`invalid-value`; `admit`
  `invalid_runtime_input`/`wrong-role-mapping`; `evaluate` `refusal`
  `invalid_runtime_input`/`invalid-value`.
- Step 4: `incomplete`, exit 22; equal reports on each repeat.
- Step 5: step 1's `decisive` has `position` 0, no quantifier or index, element `false` and the root value path; the `always` clause's has `position` 2 with the violation at position 0; the `forall` clause's has `position` 2 and the `forall` occurrence key, element 0, index 1 and the value path to `items`; the pending, successful and `Undefined` reports carry none.
