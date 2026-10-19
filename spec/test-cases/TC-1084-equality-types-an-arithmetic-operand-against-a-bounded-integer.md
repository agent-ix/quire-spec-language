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

Fixture: `record Counter { tally: Int[0,3]; next: Int[0,3]; }`, an operation `bump` on `Counter`, and `n` of `Integer`.

1. Check `self.tally < 3 and self.next = self.tally + 1` as the `pre` guard of `bump` and as an invariant over `self`; check the function `f(earlier: Counter, later: Counter)` with body `earlier.tally < 3 and later.tally = earlier.tally + 1`. Then each without its `tally < 3 and` guard.
2. Check `later.tally = earlier.tally` in `f`.
3. Check `later.tally = -earlier.tally` and `later.tally = n` in `f`.

## Expected Results

- Step 1: each guarded form checks; each unguarded form refuses `undefined_expression`/`unproved-range` at the sum.
- Step 2: checks.
- Step 3: each refuses `ill_typed`/`type-mismatch` at the comparison.
