---
id: TC-425
title: "A check location resolves to the region of the unit it was read from, or to none"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: verifies
---
# TC-425: A check location resolves to the region of the unit it was read from, or to none

## Description

Verify FR-096's `check::Location` resolution: a body or measure location
follows its child path to the node's span under the unit's `RawSourceRef`,
the checked package resolves the same way, and a position in a tree not
read from the unit has no region. This catches a resolver that returns the
declaration's span for every path, and one that invents a region for a
synthesized tree.

Scope: FR-096-AC-1.

## Test Procedure

1. Under unit reference `r`, check declarations whose function `0` has body
   `if a then b else c + d` and measure `n`, each form carrying its spans.
2. Resolve (`Body{0}`, `[]`), (`Body{0}`, `[2]`), (`Body{0}`, `[2, 1]`) and
   (`Measure{0}`, `[]`) against the declarations and against the checked
   package.
3. Embed the same unit at byte offset `k` of a document with reference
   `d`, with no layout deletions, and resolve the four locations again.
4. Resolve a location in an FR-151 synthesized function and a location with
   `Origin::Expression`.

Tag the tests `#[trace("TC-425", "FR-096-AC-1")]`.

## Expected Results

- Step 2: the spans of `if a then b else c + d`, `c + d`, `d` and `n`, each
  under `r`, and the same four regions from the checked package.
- Step 3: the same four spans shifted by `k`, each under `d`.
- Step 4: no region for either.

## Status

Planned. ADR-013 §7 slice S-5b (QSL-160), after S-4b and FR-091-AC-10.
