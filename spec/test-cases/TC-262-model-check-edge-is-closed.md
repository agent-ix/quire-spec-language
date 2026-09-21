---
id: TC-262
title: "The model -> check edge is fully closed after M-2"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-074
    type: verifies
---
# TC-262: The `model` → `check` edge is fully closed after M-2

## Description

Verify the inverse of what TC-176 (FR-068-AC-9, FR-068-CON-5) asserted
before this requirement: FR-068 required `model` → `check` to be bounded to
exactly two files and thirteen names, both imported directly from
`crate::check`. This requirement (M-2) moves the code that made those two
files need the edge at all, so after this requirement, `model` → `check`
SHALL be bounded to zero files and zero names — not narrower, empty. An
implementation that narrows the edge (for example, leaving one of the
thirteen names imported by one of the two files, having moved only part of
the code) fails this test the same way a wider-than-bounded edge would have
failed TC-176. Scope: FR-074-AC-3.

## Test Procedure

1. Resolve the real import graph at the post-macro-expansion level (not only
   a textual scan) for every `use` statement under `src/model/`, and collect
   every edge that resolves into `crate::check`, whether written directly
   or reached through `crate::value`'s aggregate re-export.
2. Confirm the collected edge set from step 1 is empty.
3. Separately, run a textual scan: `grep -rn "use crate::check" src/model/`.
4. Confirm the textual scan from step 3 finds nothing.

## Expected Results

- Step 2: the resolved `model` → `check` edge set is empty; any edge found,
  from any file under `src/model/` (not only the two files FR-068 bounded
  the interim edge to), fails this step and names the offending file and
  import.
- Step 4: the textual scan finds no line; any match fails this step.
