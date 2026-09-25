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

Verify the O-20 request writer (`qsl_route::request`). Scope: FR-097-AC-3, FR-097-AC-4.

## Test Procedure

1. Write items for a bounded claim, a set claim, a set-and-loop claim and an
   infinite-trace claim.
2. Write an unbounded item, then two bounded items with different bounds.
3. Submit, each on a fresh writer: a bounded claim with and without bounds;
   an unknown key; a loop domain with and without a bound for it; a
   mismatched bound kind; an empty set.

## Expected Results

- Step 1: `bounded`; `unbounded` with a finite bound available; and
  `unbounded` with none available for the loop and infinite-trace items.
- Step 2: indices 0, 1, 2; the bounded items are `bounded` and carry their
  proof bounds in key order; the three items are all different.
- Step 3: `NoUnboundedDomain`, `UnknownDomain`, `UnboundableDomain`,
  `KindMismatch` and `MissingDomain` respectively, each
  `invalid_runtime_input`/`invalid-value`, and nothing is written.

## Status

Backed: `qsl-route/src/request.rs`, `tests` module.
