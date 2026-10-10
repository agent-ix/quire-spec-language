---
id: TC-824
title: "A case checks under each clause kind and adds no requirement record"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-318
    type: verifies
---
# TC-824: A case checks under each clause kind and adds no requirement record

## Description

Scope: FR-318-AC-5.

## Test Procedure

1. Check the `case` of `area` (FR-318-AC-1) inside a state clause body (as a Boolean
   comparison `... = 0`) and inside a `decreases` measure.
2. Collect requirement records for the state clause, and for a unit that
   holds only `area`.

Tag each test `#[trace("TC-824", "<AC id>")]`.

## Expected Results

- Step 1: both check under their clause kinds.
- Step 2: the state clause yields exactly one `value-validity` record, its
  own; the `area` unit yields no record for the `case` or its
  exhaustiveness obligation.

