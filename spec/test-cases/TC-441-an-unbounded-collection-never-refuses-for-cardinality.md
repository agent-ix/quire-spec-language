---
id: TC-441
title: "An unbounded collection never refuses for cardinality and stops only on the caller's meter"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-097
    type: verifies
---
# TC-441: An unbounded collection never refuses for cardinality and stops only on the caller's meter

## Description

Verify the optional kernel bounds (ADR-014 N-3) end to end. Scope: FR-097-AC-7, FR-097-AC-8.

## Test Procedure

1. Form an unbounded `Sequence<Integer>` of 1000 elements, then the same
   elements under `[0, 1]`, then under a meter with 999 value occurrences.
2. Key `Sequence<Int[0,9]>` unbounded and at `[0, u64::MAX]`.
3. Check `map`, `flatMap`, `filter` and `flatten` (outer, inner and both
   unbounded) over unbounded sources, and a `sum` over one.
4. Check and evaluate `allInstances` over `Population(None)` with an
   unbounded binding; admit mismatched maxima.
5. Lower `Population(None)`.

## Expected Results

- Step 1: completes; `AboveMaximum`; `Incomplete` at `collection.bound`.
- Step 2: two different nodes, the unbounded one a bare `sequence`.
- Step 3: unbounded result types; the `sum` range is unproved with no upper
  bound.
- Step 4: an unbounded `Set` with the selected members; each mismatch is an
  input refusal.
- Step 5 (interim, until QSL-42 gives an unbounded population its own
  node): `UnrepresentableBound`, with no node written. The target is
  FR-097-AC-8's own node, distinct from an unbounded `Set<Reference<T>>`.

## Status

Backed: `quire-exact/src/collection.rs`,
`qsl-semantics/src/check/lowering/tests.rs`,
`qsl-eval/tests/it/collection_queries.rs` and
`qsl-eval/tests/it/model_reference_queries.rs`, each test tagged `#[trace("TC-441", "FR-097-AC-7")]` or, for the
population steps, `#[trace("TC-441", "FR-097-AC-8")]`.
