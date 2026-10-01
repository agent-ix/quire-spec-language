---
id: TC-832
title: "S6a charges union construction and case selection at QSpec's accounting points"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-322
    type: verifies
---
# TC-832: S6a charges union construction and case selection at QSpec's accounting points

## Description

Scope: FR-322-AC-3.

## Test Procedure

1. Evaluate `Shape::Rect(2, 3)` and `area(Shape::Rect(2, 3))` with a
   recording meter.
2. Repeat with `work_units` one below the recorded total.

Tag each test `#[trace("TC-832", "<AC id>")]`.

## Expected Results

- Step 1: exactly the charge points, sizes and order QSpec's value
  accounting lists for union construction and `case` selection, and no
  other charge for those steps.
- Step 2: stops at the last listed point with
  `incomplete { limit_kind: work_units }`.

## Status

Planned.
