---
id: TC-658
title: "Interval operators over a protocol measure distance in counted steps"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-213
    type: verifies
---
# TC-658: Interval operators over a protocol measure distance in counted steps

## Description

Verify protocol anchors, counted and uncounted steps, interval distance in the evaluator, and the model checker's interval expansion over counted steps.

Scope: FR-213-AC-1 to FR-213-AC-3.

## Test Procedure

1. Evaluate FR-213-AC-1's claims on the `B`-first behaviour of the
   repaired `Fill`, reading each position's anchor.
2. Evaluate `eventually[0,2] holds(k.bal = 0)` at each position of
   ADR-027 §7.1's failure and success paths.
3. Model-check FR-213-AC-3's two claims over the repaired `Fill`, and the
   same claim forms over a model subject.

Tag the tests `#[trace("TC-658", "FR-213-AC-n")]`.

## Expected Results

- Step 1: `protocol` anchors at positions 1, 4 and 5; `eventually[0,2]`
  holds at 0, `eventually[0,1]` does not.
- Step 2: holds at position 2, fails at 1; fails from 1 on the success path.
- Step 3: `proved` and `refuted` as FR-213-AC-3 states; FR-126's verdicts
  over the model subject.
