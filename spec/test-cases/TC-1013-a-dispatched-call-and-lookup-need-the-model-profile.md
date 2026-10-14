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

Verify that the `dispatch-call` and `lookup` expression families refuse `unsupported_construct` without the model profile and are admitted with it. Scope: FR-361-AC-2.

## Test Procedure

1. Unit A: a `model` declaration with object type `A` and operation `A.add(amount: Integer): Integer`, and a function whose body calls `a.add(1)` on a parameter `a: M::A`. Compile it with the header profile `quire.value.complete/v1`.
2. Unit B: a `model` declaration with object type `A`, a population binding `p` of type `Population<M::A>` and a function whose body is `lookup<M::A>(p, r) absent undefined`. Compile it with the header profile `quire.value.complete/v1`.
3. Compile units A and B with the header profile `quire.model.complete/v1`.

## Expected Results

1. One refusal, `unsupported_construct`/`expression-form` at the call's span, family `dispatch-call`; no package.
2. One refusal, `unsupported_construct`/`expression-form` at the lookup's span, family `lookup`; no package.
3. Both compile: the call resolves to `A.add` and the lookup reads `p`.
