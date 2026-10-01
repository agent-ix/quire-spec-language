---
id: TC-610
title: "An HP-1 relation whose first refuting tuple is undefined settles refuted with UndefinedEvaluation"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-179
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-182
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-183
    type: verifies
---
# TC-610: An HP-1 relation whose first refuting tuple is undefined settles refuted with UndefinedEvaluation

## Description

Verify that a tuple on which a step relation's body is undefined refutes the item with cause `UndefinedEvaluation` naming the tuple, that the tuple is the first refuting evidence in canonical order, and that replay reproduces the undefined value at that tuple.

Scope: FR-179-AC-4, FR-182-AC-5, FR-183-AC-5.

## Test Procedure

Fixture: the `Cell` model of FR-179-AC-4 and the relation `R` (`1 / x.k >= y.k`).

1. Run `R` through EN-1.
2. Replay its counterexample, then the same payload with its cause changed to `precondition-false`.
3. Settle the outcome with step 2's first replay result.

Tag the tests `#[trace("TC-610", "<AC id>")]`.

## Expected Results

- Step 1: `Violated` with a `StepTuple` on the first tuple in canonical order, which has `x.k = 0`, and `undefined` set to `UndefinedEvaluation{where: that tuple, cause: division-by-zero}`; never `Holds`.
- Step 2: `reproduced-with-evaluated-witness` with the `UndefinedEvaluation` as its value; then `inconclusive`, `Verdicts`.
- Step 3: `refuted`, `decisive-counterexample`, category violation, with a record carrying the `UndefinedEvaluation`, the tuple's two executions and the cause; never `proved`.
