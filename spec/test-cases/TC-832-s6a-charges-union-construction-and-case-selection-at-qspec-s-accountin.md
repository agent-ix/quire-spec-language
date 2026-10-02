---
id: TC-832
title: "S6a charges union construction at QSpec's accounting point and nothing for case selection"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-322
    type: verifies
---
# TC-832: S6a charges union construction at QSpec's accounting point and nothing for case selection

## Description

Scope: FR-322-AC-3.

## Test Procedure

1. Evaluate `Shape::Rect(2, 3)` and `area(Shape::Rect(2, 3))` with a
   recording meter.
2. Repeat with `work_units` one below the recorded total.

Tag each test `#[trace("TC-832", "<AC id>")]`.

## Expected Results

- Step 1: one `composite.result-retain` for the construction with
  `value_occurrences = 3`; for the `case`, the scrutinee's charges and then
  the `Rect` body's charges, with no charge for arm selection or payload
  binding and none for the unselected arms.
- Step 2: stops at the last listed point with
  `incomplete { limit_kind: work_units }`.

## Status

Planned.
