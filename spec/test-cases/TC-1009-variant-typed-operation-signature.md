---
id: TC-1009
title: "An operation parameter and result typed by a variant type are admitted, checked and run"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-360
    type: verifies
---
# TC-1009: An operation parameter and result typed by a variant type are admitted, checked and run

## Description

Verify the signature positions. Scope: FR-360-AC-8.

## Test Procedure

1. Admit an operation with a `Tier` parameter and a `Tier` result and call it with `Config::Tier::Gold`.
2. Call it with a `Config::Level` value.
3. Admit an operation whose parameter and result name an interface-featured object type.

## Expected Results

- Step 1 admits, checks and runs; the checked package emits the `model`/`variant_type` node, an `enum_value` and a `ref` argument, and reads back `Verified` once cross-repo requirement 1 holds; the result compares by case.
- Step 2 refuses `ill_typed`/`type-mismatch` naming the parameter.
- Step 3 refuses `invalid_model_binding`/`malformed-declaration` at the operation node, paths `Operation.params[i].typeRef` and `Operation.returns.typeRef`.
- Assertions compare typed codes, causes, keys and paths.
