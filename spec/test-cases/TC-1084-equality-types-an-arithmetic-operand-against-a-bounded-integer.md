---
id: TC-1084
title: "An equality types an arithmetic operand against a bounded integer wherever it is written"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: verifies
---
# TC-1084: An equality types an arithmetic operand against a bounded integer wherever it is written

## Description

Verify QSpec FR-149's expected-type rule in a guard, an invariant and a function body, and the forms it does not coerce.

Scope: FR-104-AC-12.

## Test Procedure

Fixture: a model `M` with the object type `Counter { tally: Int[0,3]; next: Int[0,3]; }` and an operation `bump` declared on it; the header of FR-108's unit; a function `f(earlier: Counter, later: Counter, n: Integer)`.

1. Check `pre G using v on M::Counter::bump { self.tally < 3 and self.next = self.tally + 1 }` and `invariant I using v on M::Counter at current { self.tally < 3 and self.next = self.tally + 1 }`; check `f` with body `earlier.tally < 3 and later.tally = earlier.tally + 1`. Then each without its `tally < 3 and` guard.
2. Check `later.tally = earlier.tally` as the body of `f`.
3. Check `later.tally = -earlier.tally` and `later.tally = n` as the body of `f`.

## Expected Results

- Step 1: each guarded form checks; each unguarded form refuses `undefined_expression`/`unproved-range` at the sum.
- Step 2: checks.
- Step 3: each refuses `ill_typed`/`type-mismatch` at the comparison.
