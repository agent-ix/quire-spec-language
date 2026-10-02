---
id: TC-782
title: "inspect builds each typed view from the source map"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-297
    type: verifies
---
# TC-782: inspect builds each typed view from the source map

## Description

Verify the six inspection views.

Scope: FR-297-AC-1 to FR-297-AC-4.

## Test Procedure

1. Build the package view of FR-027-AC-10's library request.
2. Build the node view selected by `seven`'s occurrence key over the spine fixture; then select a node key the package lacks.
3. Build the counterexample views of FR-281-AC-2's record and FR-290-AC-1's record.
4. Build the trace view of FR-283-AC-2's lasso, the state-graph view of an FR-120 simulation run, and the outcome view of FR-286-AC-2's refusal.

Tag the tests `#[trace("TC-782", "<AC id>")]`.

## Expected Results

- Step 1: it shows the `package_id`, the `test/geometry` dependency with its `package_id`, the exported declarations and the requirement records, each equal to the package's.
- Step 2: kind function, the result type and a source region whose bytes equal the declaration's text; the unknown selector refuses naming the selector and the `package_id`.
- Step 3: each assignment under its parameter name and the function's `QualifiedName`; the second shows the `trusted` label.
- Step 4: four positions' states, loop start at position 2 and the deciding position; exactly the retained states and transitions; the cause, catalog code and `Locus`.
