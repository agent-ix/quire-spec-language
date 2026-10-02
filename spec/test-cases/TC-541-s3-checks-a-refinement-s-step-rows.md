---
id: TC-541
title: "S3 checks a refinement's step rows, over a model subject and a protocol subject"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: verifies
---
# TC-541: S3 checks a refinement's step rows, over a model subject and a protocol subject

## Description

Verify that every concrete operation, and over a protocol subject every
concrete step class, needs exactly one written-out step row, and that
receivers, arguments and results are typed against the abstract operation.

Scope: FR-136-AC-1 to FR-136-AC-4.

## Test Procedure

Fixtures: `CasRefinesCounter` and `RingIsQueue` (ADR-020 §8); FR-136-AC-4's
protocol subject over a model with operations `incA` and `incB`.

1. Check `CasRefinesCounter` and `RingIsQueue`.
2. Remove the `peek` row; write it twice; add a row for
   `Impl::Counter::nope`.
3. Check each right-side variant of FR-136-AC-3.
4. Check the protocol subject with no control-node rows, then with `fork`,
   `join` and `finish` rows, then with an extra attempt-node row, then with
   two `join` rows.

Tag the tests `#[trace("TC-541", "FR-136-AC-n")]`.

## Expected Results

- Step 1: the step rows of FR-136-AC-1.
- Step 2: `missing_declaration`/`missing-name` naming `peek`;
  `invalid_model_binding`/`conflicting-binding` naming both rows;
  `missing_declaration`/`missing-name`.
- Step 3: `ill_typed`/`operator-ineligible`;
  `ill_typed`/`type-mismatch` at the argument; `arguments [None]`;
  `ill_typed`/`type-mismatch`.
- Step 4: `missing_declaration`/`missing-name` naming the `fork` node;
  checks; checks with the node row recorded;
  `invalid_model_binding`/`conflicting-binding`.
