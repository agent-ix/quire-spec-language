---
id: TC-845
title: "A claim over an unbounded declaration settles end to end"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-335
    type: verifies
---
# TC-845: A claim over an unbounded declaration settles end to end

## Description

Verify that a claim over an unbounded collection settles `unsupported`
with a warning, `requires-bound`, a separate bounded item, or
`unbounded-extent` as its registry and domains require, and that S6a
evaluation is a test of one value.

Scope: FR-335-AC-1 to FR-335-AC-5.

## Test Procedure

Compile `f using v(s: Set<Int[0, 9]>): Integer pure { size(s) + 1 }` from
source. In a harness downstream of CG `negotiate_*`, with test
descriptors:

1. Request the `+` claim with an empty registry; read its record.
2. Request it with only a (`value-validity`, `bounded`) descriptor.
3. Request it bounded with `Cardinality{maximum: 8}`, and again with
   `maximum: 9`.
4. Add a quantity-typed root read by the `+` and request it against the
   bounded-only descriptor.
5. Evaluate `f` at S6a on a 1,000-member set, then with a meter too small
   for construction.

Tag the tests `#[trace("TC-845", "FR-335-AC-n")]`.

## Expected Results

- Step 1: `unsupported`, warned, naming `value-validity`; record
  (`value-validity`, `Unbounded`) with one collection domain at `s`.
- Step 2: `finite_bound_available` true; `requires-bound`; no harness.
- Step 3: a new item with its own index, classified `bounded`; three
  distinct obligation identities; the unbounded item still `requires-bound`.
- Step 4: `finite_bound_available` false; `unsupported`, warned,
  `unbounded-extent`.
- Step 5: 1,001 with no cardinality refusal; `Incomplete`; the step 1
  disposition is unchanged.
