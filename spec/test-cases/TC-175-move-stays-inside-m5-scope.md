---
id: TC-175
title: "The move stays inside M-5: no early M-2 work, no edge widening"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-068
    type: verifies
---
# TC-175: The move stays inside M-5: no early M-2 work, no edge widening

## Description

Verify two scope boundaries at once, both easy for an otherwise-correct
module move to cross without anyone noticing at review time, because neither
shows up as a missing feature: (1) `check`'s only new edge into a `value::`
submodule other than through its own siblings is the ruling's declared
interim edge — `EnumDeclaration`, `EnumValue` from `value::enumeration` and
`check_comparable`, `result_unit`, `UnitOperation` from `value::quantity` —
and no other `value::` submodule (`model`, `library`, `package`, or any
sibling M-2 has not yet relabeled) gains a new consumer in `check`; and (2)
`model::checked_dispatch` and
`model::conformance::check_field_refinement_obligation` — M-2's items — stay
in `model`, not moved into `check` ahead of the corrected order (M-5 before
M-2). An implementation that folds in either piece of M-2's work early, or
that widens the interim edge by importing a whole `value::` prelude instead
of the five named items, would pass every other criterion in this
requirement while doing work QSL-7 (M-2) is supposed to do, sequenced ahead
of when the ADR's own layering argument for it (§6.1's intra-layer-3 order)
was decided. Scope: FR-068-AC-6, FR-068-AC-7.

## Test Procedure

1. Enumerate every `use` statement under `src/check/` that resolves into any
   `value::` submodule.
2. Filter to the five named items (`EnumDeclaration`, `EnumValue`,
   `check_comparable`, `result_unit`, `UnitOperation`) and confirm every
   remaining `value::`-resolving import outside this list is empty.
3. Search `check` for any definition or re-export of
   `model::checked_dispatch` or
   `model::conformance::check_field_refinement_obligation`.
4. Search `model` and confirm both symbols remain defined there, unchanged
   from the pre-move baseline.
5. Confirm `model` itself has not been moved below `check` in `src/lib.rs`'s
   module order or in any layering annotation this repository's tooling
   reads (there is no automated §6.1 layer-order gate yet — ADR-011 §6.1
   notes #226 owns that gate and names an interim manual-inspection rule
   until it lands — so this step is inspection, not a tool run).

## Expected Results

- Step 2: the only `value::`-resolving imports under `src/check/` are the
  five named items; any additional import into `model`, `library`,
  `package`, or any other `value::` submodule fails this step, naming the
  file and the unexpected import.
- Step 3: neither `model::checked_dispatch` nor
  `model::conformance::check_field_refinement_obligation` appears anywhere
  under `check`; either one appearing fails this step.
- Step 4: both symbols remain defined in `model`, byte-identical to the
  pre-move baseline; any diff fails this step.
- Step 5: `model` is not repositioned below `check`; any change to the
  module order that implies otherwise fails this step by inspection.
