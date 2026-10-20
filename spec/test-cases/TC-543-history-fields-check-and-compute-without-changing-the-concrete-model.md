---
id: TC-543
title: "History fields check at S3 and compute along a behaviour without changing the concrete model"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-138
    type: verifies
---
# TC-543: History fields check at S3 and compute along a behaviour without changing the concrete model

## Description

Verify the S3 checks of `history` rows, that history fields stay out of the
concrete model, and that `initial_history` and `step_history` compute
receiver-only updates and report an update that leaves its type as
`MappingUndetermined` without removing the step.

Scope: FR-138-AC-1 to FR-138-AC-4, FR-138-AC-6 and FR-138-AC-7.

## Test Procedure

Fixture: `Register` and `RegisterHistory` (FR-138).

1. Check `RegisterHistory`; compare the concrete model's state nodes, clause
   node identities and initial successors with and without the history row.
2. Check each refusal variant of FR-138-AC-2, including the out-of-range
   literal `5`, the other type's undeclared `last` and the unknown field `zzz`.
3. Compute history along `write(r, 1)`, `write(r, 0)`, `write(r, 1)`.
4. Add the `writes` history field and compute along `write(r, 0)`, `write(r,
   0)`.
5. Add the update `self.last` (no `pre`) and compute along `write(r, 1)`, `write(r, 0)` (FR-138-AC-6).
6. Key the history values over `{r, s}` with two fields (FR-138-AC-7).

Tag the tests `#[trace("TC-543", "FR-138-AC-n")]`.

## Expected Results

- Step 1: one `HistoryRow`; equal nodes, identities and successors.
- Step 2: each variant refuses with FR-138-AC-2's code and subcode, and the
  literal variants (`true`, `5`) refuse at the literal's span.
- Step 3: `last` = 0, 1, 0.
- Step 4: `writes` 1, then `MappingUndetermined` naming `writes`, `r` and
  the second step, whose post-state is the concrete successor.
- Step 5: `last` = 0, 0.
- Step 6: the order `r.last`, `r.writes`, `s.last`, `s.writes`; a one-value difference changes the key.
