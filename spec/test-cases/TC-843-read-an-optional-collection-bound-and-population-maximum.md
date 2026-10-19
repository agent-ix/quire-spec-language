---
id: TC-843
title: "Collection and population types resolve with an optional bound"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-333
    type: verifies
---
# TC-843: Collection and population types resolve with an optional bound

## Description

Verify that collection and population types parse and resolve with or
without a bound, an absent bound resolving to the unbounded kernel type.

Scope: FR-333-AC-1 to FR-333-AC-3.

## Test Procedure

1. Compile `function f using v(s: Set<Integer>): Integer pure { 0 }`, the same
   with `Set<Integer>[0, 8]`, and with `Sequence<Integer>`, `Bag<Integer>` and
   `OrderedSet<Integer>`; read each parameter's resolved type.
2. Resolve `Population<Account>` and `Population<Account>[4]`.
3. Compile `Set<Integer>[3, 2]` and `Set<Integer>[0, x]`.

Tag the tests `#[trace("TC-843", "FR-333-AC-n")]`.

## Expected Results

- Step 1: `bound: None` for each unbounded form; `Some{0, 8}`; the bounded
  and unbounded `Set<Integer>` types are unequal.
- Step 2: `Population(None)`; `Population(Some(4))`.
- Step 3: `ill_typed`/`type-mismatch` at the type form; an S1 parse
  diagnostic at `x`.
