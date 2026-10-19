---
id: TC-659
title: "Every protocol step class needs an explicit refinement row"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-214
    type: verifies
---
# TC-659: Every protocol step class needs an explicit refinement row

## Description

Verify that S3 checks a refinement step map over a protocol subject with one explicit row per step class and refuses missing and duplicate rows.

Scope: FR-214-AC-1 to FR-214-AC-4.

## Test Procedure

Write every refinement declaration as source text, with its population row and
the object row mapping `Acc` from `Cell` that FR-214-AC-1 names, and check it
through the real local Rust boundary (parse, then check), so a row the grammar
refuses cannot pass.

1. Check FR-214-AC-1's refinement of `add()` by `Fill`; read the checked
   map.
2. Remove the `step join M::Fill::Both` row, then the `step M::Fill::Done`
   row; add a second `step M::Cell::inc` row.
3. Add the node row `step M::Fill::A -> stutter`; check ADR-027 §7.1 without the `cend` row and
   `Chatter` without its `duplicate` row, then without its `lose` row.
4. Check an `-> any` row, and an abstract protocol whose internal list names
   `fork` as internal.

Tag the tests `#[trace("TC-659", "FR-214-AC-n")]`.

## Expected Results

- Step 1: one row per class; attempts `Visible`, control steps `Internal`.
- Step 2: `missing_declaration`/`missing-name` naming the join and the
  finish node; `invalid_model_binding`/`conflicting-binding`.
- Step 3: `A` maps by its node row; refusals naming `Refund` and, for either omission, `ch`.
- Step 4: `Either`; the abstract `fork` internal, other abstract steps
  visible.
