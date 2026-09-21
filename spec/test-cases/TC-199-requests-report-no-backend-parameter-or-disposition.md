---
id: TC-199
title: "requests::report takes no Backend parameter and has no capability/family disposition"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-077
    type: verifies
---
# TC-199: requests::report takes no Backend parameter and has no capability/family disposition

## Description

Verify, at the type level, that `linking::composed::requests::report`'s
signature takes no `Backend`-shaped parameter, and that its returned
disposition type (or any type reachable from it) contains no
`UnsupportedCapability` variant and no `UnsupportedFamily` variant. Scope:
FR-077-AC-1, FR-077-AC-2.

Catches an implementation that keeps an optional `backend: Option<Backend>`
parameter "for compatibility" (a parameter of the forbidden shape, even if
always passed `None`), and an implementation that renames
`UnsupportedCapability`/`UnsupportedFamily` to a differently spelled
variant (e.g. `CapabilityUnavailable`) that still exists and is still
produced from backend-support state, which would pass a naive grep for the
old names but not the intent of the removal.

## Test Procedure

1. Call `requests::report` with a representative set of admitted requested
   pairs, supplying no backend argument, and confirm the call compiles and
   returns a complete report.
2. Attempt to construct a call to `requests::report` that supplies backend
   information of any kind (a `Backend` value, a registry reference, or an
   `Option` wrapping either) and confirm no such call compiles.
3. Enumerate every variant of the disposition type `requests::report`
   returns (and of every type nested inside it) and compare the full
   variant name list against the forbidden names `UnsupportedCapability`
   and `UnsupportedFamily`, and confirm no variant's documented semantics
   is "this pair's capability/family is unsupported because no backend
   covers it" regardless of name.

## Expected Results

Step 1 succeeds. Step 2 fails to compile for every backend-shaped
parameter attempted. Step 3 finds no variant named `UnsupportedCapability`
or `UnsupportedFamily`, and no variant whose semantics depend on backend
registration state.
