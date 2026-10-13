---
id: TC-1013
title: "A dispatched call and a lookup refuse without the model profile"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-361
    type: verifies
---
# TC-1013: A dispatched call and a lookup refuse without the model profile

## Description

Verify that the `dispatch-call` and `lookup` expression families refuse `unsupported_construct` without the model profile. Scope: FR-361-AC-2.

## Test Procedure

1. Compile a unit with the header profile `quire.value.complete/v1` whose function body calls `x.op(1)`.
2. Compile a unit with the same header whose function body is `lookup<T>(p, r) absent undefined`.
3. Compile both units with the header profile `quire.model.complete/v1`.

## Expected Results

1. Refused `unsupported_construct`/`expression-form` at the call's span, family `dispatch-call`.
2. Refused `unsupported_construct`/`expression-form` at the lookup's span, family `lookup`.
3. Neither unit gives this refusal.
