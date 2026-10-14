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
deterministically, and `DomainKind`'s FR-331 wire spellings exact. Scope:
FR-097-AC-1 and FR-097-AC-9 (wire spellings).

## Test Procedure

1. Build `FiniteBound::integer_range(5, 4)`, `FiniteBound::depth(0)`, a
   one-point range `[4, 4]`, `cardinality(0)` and `depth(1)`.
2. Order three `Node` `DomainKey`s: (n1, []), (n1, [0]), (n2, []); then a
   `Population` key (m1, 0) against them, and `Population` keys (m1, 1),
   (m2, 0).
3. Build `IntervalKey::new(3, 2, …)`, then keys that differ in one component
   each.
4. Write each of the seven `DomainKind`s to its wire label and read it back;
   read `Collection` and `infinite_trace`.

## Expected Results

- Step 1: the inverted range and zero depth refuse; each admitted bound
  reports its own `FiniteBoundKind`.
- Step 2: (n1, []) < (n1, [0]) < (n2, []); every `Node` key sorts before
  every `Population` key, even when the member type is below the node;
  population (m1, 0) < (m1, 1) < (m2, 0).
- Step 3: the inverted key refuses; keys that differ in lower, upper or clock
  binding are unequal.
- Step 4: each kind writes as `collection`, `population`, `integer`,
  `recursive`, `loop`, `infinite-trace` or `quantity` and reads back to
  itself; `Collection` and `infinite_trace` read as no kind.
