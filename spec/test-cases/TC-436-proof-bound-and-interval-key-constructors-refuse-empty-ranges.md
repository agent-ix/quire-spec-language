---
id: TC-436
title: "Proof-bound and interval-key constructors refuse empty ranges, and domain keys order by node then path"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-097
    type: verifies
---
# TC-436: Proof-bound and interval-key constructors refuse empty ranges, and domain keys order by node then path

## Description

Verify the F `bound` types are non-empty by construction and ordered
deterministically. Scope: FR-097-AC-1.

## Test Procedure

1. Build `FiniteBound::integer_range(5, 4)`, `FiniteBound::depth(0)`, a
   one-point range `[4, 4]`, `cardinality(0)` and `depth(1)`.
2. Order three `DomainKey`s: (n1, []), (n1, [0]), (n2, []).
3. Build `IntervalKey::new(3, 2, …)`, then keys that differ in one component
   each.

## Expected Results

- Step 1: the inverted range and zero depth refuse; each admitted bound
  reports its own `FiniteBoundKind`.
- Step 2: (n1, []) < (n1, [0]) < (n2, []).
- Step 3: the inverted key refuses; keys that differ in lower, upper or clock
  binding are unequal.

## Status

Backed: `qsl-foundation/src/bound.rs`, `tests` module.
