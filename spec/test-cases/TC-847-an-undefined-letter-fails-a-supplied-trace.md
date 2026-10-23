---
id: TC-847
title: "An undefined letter fails a clause over a supplied trace or lasso, and replays"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-327
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-328
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-329
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-330
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-331
    type: verifies
---
# TC-847: An undefined letter fails a clause over a supplied trace or lasso, and replays

## Description

Verify that the temporal evaluator returns `Undefined` at the first position
whose letter is undefined, over a finite trace, a finite prefix and a lasso;
that it maps to violation with cause `UndefinedEvaluation`; that a bounded
`on origin` clause reads only its horizon; and that replay reproduces the
undefined value at its position.

Scope: FR-327-AC-5, FR-327-AC-7, FR-328-AC-5, FR-329-AC-6, FR-330-AC-5,
FR-330-AC-8, FR-331-AC-4.

A window aggregate short of data is the separate `InsufficientData`
outcome, category `undefined`, which never hides a false activation, an
undefined atom or a refused condition (FR-327-AC-7, FR-330-AC-8).

## Test Procedure

Fixture: the `Counter` unit, the closed trace and the prefix with `c.value`
0, 1, 2, and the observed lasso with an empty prefix and loop 0, 1, 2.

1. Evaluate the two clauses of FR-327-AC-5 over the closed trace.
2. Evaluate FR-328-AC-5's clause over the prefix.
3. Evaluate FR-329-AC-6's clause over the lasso.
4. Run FR-330-AC-5's request from source.
5. Replay FR-331-AC-4's two packets.
6. Evaluate each clause of FR-327-AC-7 over the closed trace, and the two
   clauses over the one-position trace with `c.value` 2.
7. Run FR-330-AC-8's request from source.

Tag the tests `#[trace("TC-847", "<AC id>")]`.

## Expected Results

- Step 1: `Undefined{where: 2, cause: division-by-zero}`, category
  violation; `Completed(true)`.
- Step 2: `Undefined{where: 2, cause: division-by-zero}`.
- Step 3: `Undefined{where: 2, cause: division-by-zero}`.
- Step 4: `violation`, exit 10, position 2, cause `UndefinedEvaluation`.
- Step 5: `reproduced-with-evaluated-witness`; `inconclusive`,
  `ReplayParity`.
- Step 6: `InsufficientData{where: 2, count: 0, min_count: 1}`, category
  `undefined`; `Completed(false)` at 1; `Completed(false)` at 2;
  `UndefinedEvaluation{where: 1, cause: division-by-zero}`;
  `UndefinedEvaluation` at 2; `InsufficientData{where: 0, count: 1,
  min_count: 2}`; `InsufficientData` at 0 for the `on origin` clause; over
  the one-position trace, `UndefinedEvaluation` at 0 for both orders.
- Step 7: stage `evaluate`, `undefined`, exit 10, position 2, `count` 0,
  `min_count` 1, no `truth`, no counterexample.
