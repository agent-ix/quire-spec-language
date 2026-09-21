---
id: TC-202
title: "value::ieee and value::division evaluation is unchanged by negotiate_* removal"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-078
    type: verifies
---
# TC-202: value::ieee and value::division evaluation is unchanged by negotiate_* removal

## Description

Verify that removing the `negotiate_*` functions from `value::ieee` and
`value::division` leaves every evaluation function's input/output behavior
unchanged. Scope: FR-078-AC-3.

Catches an over-aggressive removal that deletes or breaks evaluation logic
that happened to sit near the removed negotiation functions (for example,
by deleting a shared helper both the evaluator and the negotiation
predicate called, without noticing the evaluator still needs it).

## Test Procedure

1. Record the evaluated results of a fixture set of IEEE arithmetic value
   expressions (covering normal values, negative zero, NaN and infinities)
   and integer-division value expressions (covering exact division,
   truncation and division-by-zero handling) evaluated through
   `value::ieee` and `value::division` on `main`, before the removal.
2. Apply the `negotiate_*` removal change.
3. Re-run the identical fixture set through `value::ieee` and
   `value::division` after the removal.
4. Compare the before and after results.

## Expected Results

Every fixture's evaluated result after the removal is identical to its
result before the removal, for both modules and across all covered cases.
