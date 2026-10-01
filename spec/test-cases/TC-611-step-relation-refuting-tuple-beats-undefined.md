---
id: TC-611
title: "An HP-1 relation whose first refuting tuple is false settles refuted ahead of later undefined tuples"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-179
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-182
    type: verifies
---
# TC-611: An HP-1 relation whose first refuting tuple is false settles refuted ahead of later undefined tuples

## Description

Verify that the first refuting evidence in canonical order decides a step relation: a false tuple that precedes every undefined tuple refutes the item as an ordinary refutation.

Scope: FR-179-AC-5, FR-182-AC-6.

## Test Procedure

Fixture: the `Cell` model of FR-179-AC-4 and the relation `R2` (`1 / (1 - x.k) > y.k`).

1. Run `R2` through EN-1.
2. Replay its counterexample and settle the outcome.

Tag the tests `#[trace("TC-611", "<AC id>")]`.

## Expected Results

- Step 1: `Violated` with a `StepTuple` on the tuple with `x.k = 0` and `y.k = 1`, with no `undefined` member.
- Step 2: the counterexample reproduces, and the item settles `refuted`, `decisive-counterexample`, with no `UndefinedEvaluation`.
