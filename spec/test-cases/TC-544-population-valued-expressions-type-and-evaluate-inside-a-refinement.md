---
id: TC-544
title: "Population-valued expressions type and evaluate inside a refinement only"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-139
    type: verifies
---
# TC-544: Population-valued expressions type and evaluate inside a refinement only

## Description

Verify that a concrete population name and the `count`, `sum`, `exists`,
`all`, `only` and `seq` forms type inside a refinement declaration, are
refused in any other clause, and evaluate over a state, with `only`
undefined for no or several objects.

Scope: FR-139-AC-1 to FR-139-AC-4.

## Test Procedure

Fixture: `RingIsQueue` with universes `rings = {r}`, `slots = {s0, s1}`.

1. Check the `items` row, a `count` form and an `exists` form.
2. Check an invariant of `R::Ring` that uses `count`; check `only` with a
   non-Boolean condition.
3. Evaluate `items`, `count` and `all` at the states of FR-139-AC-3.
4. Evaluate the two `only` variants of FR-139-AC-4.

Tag the tests `#[trace("TC-544", "FR-139-AC-n")]`.

## Expected Results

- Step 1: `R::slots` is `Set<Reference<R::Slot>>[0, 2]`, `seq`'s maximum
  length is 2 and the row conforms; an integer; `Boolean`.
- Step 2: `unsupported_construct`/`expression-form`; `ill_typed`/`type-mismatch` at `s.index`.
- Step 3: `[1, 0]`; `[]`; 2; `false`.
- Step 4: undefined; undefined.
