---
id: TC-175
title: "The move stays inside M-5: no early M-2 work, no edge widening"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-068
    type: verifies
---
# TC-175: The move stays inside M-5: no early M-2 work, no edge widening

**Retired in part by TC-261 (FR-074, ADR-011 §7.3 M-2, QSL-7, 2026-09-21).**
This test case's Scope named two acceptance criteria: FR-068-AC-6 (`check`'s
import bound, live) and FR-068-AC-7 (M-2's items stay in `model`, ahead of
M-2 landing). FR-074 is M-2: it moved `model::checked_dispatch` and
`model::conformance::check_field_refinement_obligation` into `check`, the
exact inverse of what FR-068-AC-7 (now retired, see FR-068) required. Steps
3-5 below and their Expected Results, which verified the pre-M-2 "stays in
`model`" shape, are retired along with FR-068-AC-7; TC-261 verifies the
post-M-2 inverse in the corresponding xtask test
(`m2_items_moved_to_check_and_are_absent_from_model`, `#[trace("TC-261", ...)]`).
The retired steps are not renumbered, so they stay visible as what they
were.

**Amended by the layer-rule ruling (2026-09-22).** Steps 1-2 follow
FR-068-AC-6 as amended: `check` is bounded by a module-level layer rule, not
by the former tier (b) list of exactly seven `value` items or FR-087's tier
(c) list of exactly two `library` items. Both item lists are retired. They
capped edges ADR-011 §6.1 already permits, went red on legitimate refactors,
and were the M-5 ratchet, which is done. FR-068's Description gives the
reasons in full.

## Description

Verify that `check`'s shipped (non-`#[cfg(test)]`) imports stay inside
FR-068-AC-6's module-level layer rule. `check` may import any item of a
permitted module: `quire_exact`, `qsl_foundation`, `forms`, `check` and
`family`, `model` and `value::model_query`, `library`, the §6.2
`semantic_value` modules (`value::definition`, `value::enumeration`,
`value::unit`, `value::quantity`, `value::key`, `value::reference`,
`value::containment`, `value::semantic_node`) and `value::declaration`. The
`value` K-copy list is now empty (`value::outcome` left it under QSL-131
O2, which deleted the module; `value::decimal`, `value::ieee`,
`value::numeric` and `value::text` left it under QSL-131 O3, which deleted
the four modules; `value::collection`, `value::composite`,
`value::equality` and `value::rational` left it under QSL-131 V5b, which
deleted the four modules). `check` imports nothing from a later layer:
`checked_package`, `package`, `value::expression`, `route`, `replay` or
`lowering`. Every `value` import names its submodule.

The failure this catches is a later-layer edge into `check`, the direction
ADR-011 §6.1 forbids. An extra item from an already-permitted module is not a
failure. Scope: FR-068-AC-6, FR-068-AC-7 (retired).

## Test Procedure

1. Enumerate every shipped (non-`#[cfg(test)]`) import under `src/check/`:
   every `use` line, and every `crate::`-rooted path written inline in code.
   Resolve each to the module it names, resolving `super::` and `self::`
   paths relative to their file. Imports from `std` and third-party crates
   are outside the rule and are not classified. Resolve a flat `crate::value::Name`
   through `value/mod.rs`'s re-export table only to report which module it
   reaches; the flat form itself fails in step 2.
2. Classify each import as permitted (its module is on FR-068-AC-6's
   permitted list), forbidden (its module is `checked_package`, `package`,
   `value::expression`, `route`, `replay` or `lowering`, or a descendant of
   one), or unlisted (anything else). Confirm there are no forbidden and no
   unlisted imports, and that every `value` import is written
   `crate::value::<submodule>::Name`.
3. *(Retired with FR-068-AC-7; see the header.)* Search `check` for any
   definition or re-export of `model::checked_dispatch` or
   `model::conformance::check_field_refinement_obligation`.
4. *(Retired with FR-068-AC-7.)* Search `model` and confirm
   `model::checked_dispatch`'s and
   `model::conformance::check_field_refinement_obligation`'s own bodies and
   signatures are unchanged from the pre-move baseline, apart from the `use`
   line FR-068-CON-5 required repointing at `crate::check`.
5. *(Retired with FR-068-AC-7.)* Confirm `model` has not been moved below
   `check` in `src/lib.rs`'s module order or in any layering annotation this
   repository's tooling reads.
6. Adverse checks, each run against a fixture tree rather than the real
   crate:
   - a shipped `use crate::checked_package::CheckedPackage;` under
     `src/check/` fails step 2 and names the file, line and module;
   - a shipped inline path `crate::value::expression::Evaluation` with no
     `use` line fails step 2;
   - a shipped `use crate::value::Rational;` (flat aggregate) fails step 2;
   - a shipped inline flat path `crate::value::Presence::Optional` fails
     step 2;
   - a shipped import from a module on no list (for example
     `crate::value::member`) fails step 2;
   - the same forbidden import inside a `#[cfg(test)]` item does not fail
     step 2;
   - an additional item from a permitted module (for example a new name from
     `crate::value::quantity`) does not fail step 2.

## Expected Results

- Steps 1-2: every shipped import under `src/check/` is permitted and
  submodule-qualified. A forbidden or unlisted import, or a flat `value`
  import, fails this step and names the file, line and resolved module. The
  number of items imported from a permitted module is not checked.
- Steps 3-5: retired with FR-068-AC-7; TC-261 verifies the post-M-2 shape.
- Step 6: each adverse fixture produces the stated result. A forbidden edge
  that passes, or a permitted item that fails, fails this test.
