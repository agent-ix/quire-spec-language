---
id: TC-1167
title: "A refinement record carries the shared Refinement property form"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-144
    type: verifies
---
# TC-1167: A refinement record carries the shared Refinement property form

## Description

Verify that a refinement item's requirement record carries QSV FR-121's
`PropertyForm::Refinement { liveness }`, that it encodes as
`{"refinement":{"liveness":<bool>}}` and orders after `Liveness`, and that
`RefinementFailure` stays a QSL type.

Scope: FR-144-AC-8.

## Test Procedure

1. Check `CasRefinesCounter` and the same unit with its `ensure` row
   removed; read each requirement record's property form and encode it
   with `quire_canonical::to_vec`.
2. Compare `PropertyForm::Liveness`, `Refinement{liveness: false}` and
   `Refinement{liveness: true}`.
3. Inspect the public items of `quire_semantic_value` for any naming
   `RefinementFailure`.

Tag the tests `#[trace("TC-1167", "FR-144-AC-8")]`.

## Expected Results

- Step 1: `{"refinement":{"liveness":true}}` and
  `{"refinement":{"liveness":false}}`.
- Step 2: `Liveness < Refinement{liveness: false} < Refinement{liveness:
  true}`.
- Step 3: none.
