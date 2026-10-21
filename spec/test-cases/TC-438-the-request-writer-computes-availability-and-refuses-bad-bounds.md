---
id: TC-438
title: "The request writer computes the available finite bound, writes a bounded request as its own item, and refuses bad bounds before writing"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-097
    type: verifies
---
# TC-438: The request writer computes the available finite bound, writes a bounded request as its own item, and refuses bad bounds before writing

## Description

Verify the O-20 request writer (`qsl_route::request`). Scope: FR-097-AC-3, FR-097-AC-4
FR-097-AC-9 (kind emission) and FR-097-AC-10 (temporal subject proof bounds).

## Test Procedure

1. Write items for a bounded claim, a set claim, a set-and-loop claim and an
   infinite-trace claim.
2. Write an unbounded item, then two bounded items with different bounds.
3. Submit, each on a fresh writer: a bounded claim with and without bounds;
   an unknown key; a loop domain with and without a bound for it; a
   mismatched bound kind; an empty set.
4. Write a bounded item over a `population`, an `integer` and a `recursive`
   domain, with a cardinality, an integer range and a depth bound.
5. Supply subject proof bounds to a temporal item over a model subject, over
   a supplied trace, and to a non-temporal item; supply a non-population key
   and a non-cardinality bound.

## Expected Results

- Step 1: `bounded`; `unbounded` with a finite bound available; and
  `unbounded` with none available for the loop and infinite-trace items.
- Step 2: indices 0, 1, 2; the bounded items are `bounded` and carry their
  proof bounds in key order; the three items are all different.
- Step 3: `NoUnboundedDomain`, `UnknownDomain`, `UnboundableDomain`,
  `KindMismatch` and `MissingDomain` respectively, each
  `invalid_runtime_input`/`invalid-value`, and nothing is written.
- Step 4: each emitted proof bound carries a kind, `population`, `integer`
  and `recursive` respectively, and each kind's `finite_kind` equals its
  bound's kind.
- Step 5: over a model subject the bounds read back in key order with the
  `population` kind, and the extent, index, occurrence and `proof_bounds()`
  are unchanged; a supplied-trace item and a non-temporal item have none
  (`None`) and refuse; the non-population key and the non-cardinality bound
  refuse; each refusal is `invalid_runtime_input`/`invalid-value`.
