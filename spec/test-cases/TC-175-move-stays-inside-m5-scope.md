---
id: TC-175
title: "The move stays inside M-5: no early M-2 work, no edge widening"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-068
    type: verifies
---
# TC-175: The move stays inside M-5: no early M-2 work, no edge widening

**Retired in part by TC-253 (FR-074, ADR-011 §7.3 M-2, QSL-7, 2026-09-21).**
This test case's Scope named two acceptance criteria: FR-068-AC-6 (the
two-tier `value::` import bound, live) and FR-068-AC-7 (M-2's items stay in
`model`, ahead of M-2 landing). FR-074 is M-2: it moved
`model::checked_dispatch` and `model::conformance::check_field_refinement_obligation`
into `check`, the exact inverse of what FR-068-AC-7 (now retired, see FR-068)
required. Steps 3-5 below and their Expected Results, which verified the
pre-M-2 "stays in `model`" shape, are retired along with FR-068-AC-7;
TC-253 verifies the post-M-2 inverse (both items now in `check`, absent from
`model`) in the corresponding xtask test
(`m2_items_moved_to_check_and_are_absent_from_model`, `#[trace("TC-253", ...)]`).
Steps 1-2 (FR-068-AC-6) remain live and unaffected — this document's Test
Procedure and Expected Results below are otherwise unchanged from the
pre-M-2 baseline, and are not renumbered, so the retired steps stay visible
as what they were rather than disappearing from the file's history.

## Description

Verify two scope boundaries at once, both easy for an otherwise-correct
module move to cross without anyone noticing at review time, because neither
shows up as a missing feature: (1) `check`'s only new *shipped* (non-
`#[cfg(test)]`) edges into `value::` submodules are FR-068-AC-6's two
permitted tiers — unbounded imports from the nine K-designated siblings X-1
has not yet relocated, and exactly the ruling's declared interim edge
(`EnumDeclaration`, `EnumValue` from `value::enumeration`;
`check_comparable`, `result_unit`, `UnitOperation`, `QuantityUnit` from
`value::quantity`; `TextProfile` from `value::text` —
**amended, PR #282 review F3: seven items across three modules, not five
across two; `family.rs`'s pre-existing `encode_value_type` needs
`QuantityUnit`/`TextProfile` and this criterion could not pass as originally
written against any conforming implementation. Scope clarified, owner
ruling, PR #282 review, post-rebase: this tier bounds `check`'s *shipped*
dependency graph. `family.rs`'s own moved golden-digest test fixture, inside
its own `#[cfg(test)] mod tests` block, needs `TextType` from `value::text`;
widening tier 2 to admit it was tried and reverted, since a test-only import
is not part of the shipped graph this criterion bounds, and admitting it
would have permanently licensed production code to the same import with no
way for this criterion's own verification to catch that back. Tier 2 stays
seven items; this criterion's scan now excludes `#[cfg(test)]`-gated
imports on that stated basis**) — and no
other `value::` submodule (`model`, `library`, `package`, `definition`,
`unit`, `key`, `reference`, `containment`, `model_query`,
`package_identity`, or any sibling M-2 has not yet relabeled) gains a new
shipped consumer in `check`; and (2) `model::checked_dispatch` and
`model::conformance::check_field_refinement_obligation` — M-2's items — stay
in `model`, not moved into `check` ahead of the corrected order (M-5 before
M-2). An implementation that folds in either piece of M-2's work early, or
that widens the interim edge by importing a whole `value::` prelude instead
of the seven named items, would pass every other criterion in this
requirement while doing work QSL-7 (M-2) is supposed to do, sequenced ahead
of when the ADR's own layering argument for it (§6.1's intra-layer-3 order)
was decided. Scope: FR-068-AC-6, FR-068-AC-7.

## Test Procedure

1. Enumerate every *shipped* (non-`#[cfg(test)]`) `use` statement under
   `src/check/` that resolves into any `value::` submodule.
2. Partition step 1's imports into three groups, matching FR-068-AC-6's
   two permitted tiers plus everything else: (a) imports resolving into the
   nine K-designated siblings (`collection`, `comparison`, `composite`,
   `decimal`, `equality`, `ieee`, `node`, `numeric`, `rational`) —
   unbounded in which items they import, since X-1, not this requirement,
   owns closing them; (b) imports resolving into `value::enumeration`,
   `value::quantity` or `value::text`, filtered to the seven named items
   (`EnumDeclaration`, `EnumValue`, `check_comparable`, `result_unit`,
   `UnitOperation`, `QuantityUnit`, `TextProfile`); and (c)
   everything else.
   Confirm group (b) contains no item beyond the seven named, and confirm
   group (c) is empty. Also confirm every group-(a)/(b) import is written in
   crate-absolute, submodule-qualified form (`crate::value::<submodule>::
   Name`), never through `value`'s flat aggregate re-export.
3. Search `check` for any definition or re-export of
   `model::checked_dispatch` or
   `model::conformance::check_field_refinement_obligation`.
4. Search `model` and confirm `model::checked_dispatch`'s and
   `model::conformance::check_field_refinement_obligation`'s own bodies and
   signatures are unchanged from the pre-move baseline. This step allows
   exactly one class of diff in `model/checked_dispatch.rs` and
   `model/conformance.rs`: the `use crate::value::{...}` line FR-068-CON-5
   requires repointing at `use crate::check::{...}` for the thirteen
   relocated names (see FR-068 Behavior, "The interim `model` → `check`
   edge"). It does not allow any other diff, to either file's definitions,
   signatures, or any import CON-5 does not name.
5. Confirm `model` itself has not been moved below `check` in `src/lib.rs`'s
   module order or in any layering annotation this repository's tooling
   reads (there is no automated §6.1 layer-order gate yet — ADR-011 §6.1
   notes #226 owns that gate and names an interim manual-inspection rule
   until it lands — so this step is inspection, not a tool run).

## Expected Results

- Step 2: group (a) may hold any number of items from the nine K-designated
  siblings; group (b) holds only the seven named items — any additional item
  resolving from `enumeration`, `quantity` or `text` fails this step; group
  (c) is empty — any import into `model`, `library`, `package`, `definition`,
  `unit`, `key`, `reference`, `containment`, `model_query`,
  `package_identity`, or any other `value::` submodule outside groups (a)/(b)
  fails this step, naming the file and the unexpected import. Any group-(a)/
  (b) import written through `value`'s flat aggregate rather than its owning
  submodule also fails this step.
- Step 3: neither `model::checked_dispatch` nor
  `model::conformance::check_field_refinement_obligation` appears anywhere
  under `check`; either one appearing fails this step.
- Step 4: `model::checked_dispatch`'s and
  `model::conformance::check_field_refinement_obligation`'s own bodies and
  signatures are byte-identical to the pre-move baseline; the only permitted
  diff in either file is the `use` line FR-068-CON-5 requires repointing at
  `crate::check`. Any other diff — to either definition's body or signature,
  to any import CON-5 does not name, or to a third `model` file — fails this
  step.
- Step 5: `model` is not repositioned below `check`; any change to the
  module order that implies otherwise fails this step by inspection.
