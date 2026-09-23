---
id: TC-261
title: "M-2's items are relocated to check and absent from model"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-074
    type: verifies
---
# TC-261: M-2's items are relocated to `check` and absent from `model`

## Description

Verify the inverse of what TC-175 (FR-068-AC-7) asserted before this
requirement: FR-068 required `checked_dispatch_operation` and
`check_field_refinement_obligation` to stay in `model`, unmoved, ahead of
M-2. This requirement is M-2 itself, so the same two symbols now SHALL be
defined under `check` and SHALL be absent from `model` — an implementation
that leaves either one behind in `model` (a partial move), or that adds a
copy under `check` while leaving the original in `model` in place (a
duplicate rather than a move), both fail this test. Scope: FR-074-AC-1,
FR-074-AC-2.

## Test Procedure

1. Scan the whole compiled crate (not only `qsl-semantics/src/check/`) for every defining
   location of `checked_dispatch_operation`.
2. Scan the whole compiled crate for every defining location of
   `check_field_refinement_obligation`.
3. For each of the two names, confirm exactly one defining location exists,
   confirm that location's file path starts with `qsl-semantics/src/check/`, and confirm
   no location's file path starts with `qsl-semantics/src/model/`.

## Expected Results

- Steps 1-2: each of the two names has exactly one defining location in the
  whole crate; more than one location for either name fails this step
  (duplicate definition).
- Step 3: both names' one defining location is under `qsl-semantics/src/check/`; either
  name having any defining location under `qsl-semantics/src/model/` — whether alone or
  alongside a `qsl-semantics/src/check/` copy — fails this step.
