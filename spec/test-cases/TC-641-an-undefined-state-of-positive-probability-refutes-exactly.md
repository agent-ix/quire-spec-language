---
id: TC-641
title: "An undefined state reached with positive probability refutes an exact claim"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-202
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-203
    type: verifies
---
# TC-641: An undefined state reached with positive probability refutes an exact claim

## Description

Verify that EN-5 refutes a probabilistic claim that evaluates undefined at a
product state of positive probability, with an `Undefined` path as its
evidence, that the evidence replays, and that the item settles `refuted`
with cause `UndefinedEvaluation` whatever the threshold.

Scope: FR-196-AC-5, FR-202-AC-5, FR-203-AC-5.

## Test Procedure

Fixture: the `Coin` model and claim of FR-196-AC-5.

1. Build the product and run EN-5.
2. Replay the evidence, then the same evidence with `where` set to 0.
3. Settle the outcome with step 2's first replay result.

Tag the tests `#[trace("TC-641", "<AC id>")]`.

## Expected Results

- Step 1: the build stops at `v = 1`, position 1; evidence `Undefined` path
  `flip` with `b = 1`, probability `1/2`, `UndefinedEvaluation{where: 1,
  cause: division-by-zero}`.
- Step 2: `reproduced-with-evaluated-witness`; then `inconclusive`,
  `ReplayParity`.
- Step 3: `refuted`, `decisive-counterexample`, category violation, cause
  `UndefinedEvaluation`.
