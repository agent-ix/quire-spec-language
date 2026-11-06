---
id: TC-1339
title: "A derived dimension with no terms stays derived"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: verifies
---
# TC-1339: A derived dimension with no terms stays derived

## Description

Verify that a dimension declared as a derived dimension keeps that identity
when its normalized base map is empty: assembly gives it a derived preimage and
a key apart from every base dimension, admission accepts it, lowering builds a
derived `dimension` node with no terms, and replay lists it as not base. Scope:
FR-091-AC-73 and FR-094-AC-14.

## Test Procedure

1. Assemble `dimension L; dimension N = L / L; unit m : L = rational(1, 1);
   unit one : N = rational(1, 1);` and read each unit's dimension.
2. Read `DimensionPreimage::from_json` over a `quire.dimension-node/v2`
   document with `base` false and empty terms, a `v1` document with empty
   terms, a `v1` document carrying `base`, and a `v2` document without
   `base`.
3. Admit a unit graph holding a base dimension and a `v2` derived dimension
   with empty terms.
4. Check and lower a function with a `Quantity` parameter of each unit of
   FR-094-AC-14's `L`, `N` and `M`; read each lowered `dimension` node's
   payload and replay's `AdmittedDimension` for each.

Tag the tests `#[trace("FR-091-AC-73", "TC-1339")]` and
`#[trace("FR-094-AC-14", "TC-1339")]`.

## Expected Results

- Step 1: only `one`'s dimension is dimensionless; `m`'s is not.
- Step 2: the `v2` derived and the `v1` documents read, as derived and base;
  the other two refuse `NonCanonicalPreimage`.
- Step 3: the graph admits, the derived dimension's unit is dimensionless and
  the base dimension's is not.
- Step 4: `L` is base with no terms, `N` is derived with no terms and `M` is
  derived with the one term `L` to the power 2, in both the lowered nodes and
  replay.
