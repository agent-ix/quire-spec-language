---
id: TC-1001
title: "A model-operation argument and a function argument admit a child at a parent-typed parameter"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-359
    type: verifies
---
# TC-1001: A model-operation argument and a function argument admit a child at a parent-typed parameter

## Description

Verify that a model operation, like a function, accepts a child record at a parameter naming the ancestor. Scope: FR-359-AC-8.

Catches an implementation whose operation argument path accepts only an identical type, so the same child checks at a function and refuses at an operation.

## Test Procedure

1. Declare `Money`, `TaxedMoney` and an object operation `Order.pay(m: Money): Money` whose body returns its argument, and a function `g(m: Money): Money` doing the same. Call both with a `TaxedMoney`, emit the package and read it back through `emit_and_read_v2`.
2. Call `pay` from an operation that goes through an `Upcast` of the operation, and through a dispatch guard and body of a subtype that redeclares it.
3. Call `pay` with a `Money` where the parameter is `TaxedMoney`, and with an unrelated `Fee`.
4. Call `g` with the same two values.

## Expected Results

- Step 1 checks, emits, reads back `Verified` and runs both calls; the result of each is the `TaxedMoney` argument with its most-specific type.
- Step 2 checks and runs, with the same result.
- Step 3 refuses `ill_typed`/`type-mismatch` at the argument, naming parameter `m`.
- Step 4 refuses the same way with the same cause.
- Assertions compare typed values, causes and parameter names, never message text.