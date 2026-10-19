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

Fixture: a record with `tally: Int[0,3]`, parameters `earlier` and `later` of it, and `n` of `Integer`.

1. Check `earlier.tally < 3 and later.tally = earlier.tally + 1` as a guard, as an invariant and as a function body; then each without the guard.
2. Check `later.tally = earlier.tally`.
3. Check `later.tally = -earlier.tally`, `later.tally = n` and `earlier.tally + 1 <= later.tally`.

## Expected Results

- Step 1: each guarded form checks; each unguarded form refuses `undefined_expression`/`unproved-range` at `earlier.tally + 1`.
- Step 2: checks.
- Step 3: each refuses `ill_typed`/`type-mismatch` at the comparison.
