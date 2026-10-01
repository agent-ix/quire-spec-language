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

Scope: FR-327-AC-5, FR-328-AC-5, FR-329-AC-6, FR-330-AC-5, FR-331-AC-4.

## Test Procedure

Fixture: the `Counter` unit, the closed trace and the prefix with `c.value`
0, 1, 2, and the observed lasso with an empty prefix and loop 0, 1, 2.

1. Evaluate the two clauses of FR-327-AC-5 over the closed trace.
2. Evaluate FR-328-AC-5's clause over the prefix.
3. Evaluate FR-329-AC-6's clause over the lasso.
4. Run FR-330-AC-5's request from source.
5. Replay FR-331-AC-4's two packets.

Tag the tests `#[trace("TC-847", "<AC id>")]`.

## Expected Results

- Step 1: `Undefined{where: 2, cause: division-by-zero}`, category
  violation; `Completed(true)`.
- Step 2: `Undefined{where: 2, cause: division-by-zero}`.
- Step 3: `Undefined{where: 2, cause: division-by-zero}`.
- Step 4: `violation`, exit 10, position 2, cause `UndefinedEvaluation`.
- Step 5: `reproduced-with-evaluated-witness`; `inconclusive`,
  `ReplayParity`.
