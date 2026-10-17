---
id: TC-1027
title: "No label-only match and no composed parser reachable from the producer"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-362
    type: verifies
---
# TC-1027: No label-only match and no composed parser reachable from the producer

## Description

Verify FR-362-AC-5 and FR-362-AC-6 by absence, so that a second route cannot return unnoticed.

## Test Procedure

1. Search the workspace (`src`, `qsl-*`, `tests`, `xtask`) for the symbols `UnitLabel`, `UnitLabelMismatch` and `checking::source_inventory::CheckedUnit`, and for any public type named `CheckedUnit` other than `spine::CheckedUnit`.
2. Search the producer and the type phase it runs for the symbols `parse_native`, `parse_native_source`, `ComposedUnit`, `syntax::composed`, `linking::composed` and `checking::composed`.
3. Search `src` for the same symbols and for the paths `src/parser/composed`, `src/syntax/composed`, `src/linking/composed` and `src/checking/composed`.
4. Read the checked-unit constructor and list every comparison it makes on a source reference.

## Expected Results

- Step 1 finds the one `spine::CheckedUnit` and none of the other symbols. It closes when the label-only types and the root-crate inventory `CheckedUnit` are gone.
- Step 2 finds none of them.
- Step 3 finds none of them and none of the paths. Steps 2 and 3 close when the composed lane is gone.
- Step 4 finds only whole-reference comparisons (authority, identity and digest).
