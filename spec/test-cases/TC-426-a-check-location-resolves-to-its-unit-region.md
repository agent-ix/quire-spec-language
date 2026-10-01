---
id: TC-426
title: "A check location resolves to the region of the unit it was read from, or to none"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: verifies
---
# TC-426: A check location resolves to the region of the unit it was read from, or to none

## Description

Verify FR-096's `quire_semantic_value::location::Location` resolution: a body or measure location
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
5. Embed the unit's body in a document whose C-21 map splits one of the
   four locations' spans into more than one document region (a layout
   deletion the map admits), and resolve that location.

Tag the tests `#[trace("TC-426", "FR-096-AC-1")]`.

## Expected Results

- Step 2: the spans of `if a then b else c + d`, `c + d`, `d` and `n`, each
  under `r`, and the same four regions from the checked package.
- Step 3: the same four spans shifted by `k`, each under `d`.
- Step 4: no region for either.
- Step 5: no region -- the position was read from the unit, but its
  embedded span names no single region of the document.

## Status

Passed locally. Steps 2 and 4 are backed by `qsl-semantics` `check::region`
tests. Step 3 (a body embedded in a document, C-21) is backed by
`an_embedded_body_resolves_its_locations_under_the_document_shifted`:
`PackageDeclarations::embedding` carries the body's `SourceMap`,
compared against the map's own body identity (never the caller's) before
it is trusted, and the declarations, the regions taken from them and the
checked package resolve each location under the document's `RawSourceRef`,
shifted by `k`. A span the map splits (layout deletions) resolves to no
single region -- FR-096's fourth no-region case, added and backed directly
against the resolver (`a_span_the_embedding_splits_has_no_region`); no
production caller builds a multi-segment map today. No production caller
sets `embedding` yet either, so a real embedded-document unit still
resolves under its own body reference until `qsl-source`'s document map is
wired to the assembler.
