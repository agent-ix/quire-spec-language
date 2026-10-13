---
id: TC-1012
title: "An operation body refuses without the model profile and compiles with it"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-361
    type: verifies
---
# TC-1012: An operation body refuses without the model profile and compiles with it

## Description

Verify that a `body` declaration refuses `unsupported_construct` in a unit that does not select `quire.model.complete/v1`, and refuses once. Scope: FR-361-AC-1.

This catches a gate that leaves `body` accepted under the value profile and a gate that also refuses it under the model profile.

## Test Procedure

1. Compile TC-1011's unit shape (its `model` declaration, object type `A` and operation `A.add`) with the header profile `quire.value.complete/v1` and the declaration `body ok using V on M::A::add(amount) { amount }`.
2. Compile the same unit with the header profile `quire.model.complete/v1`.

## Expected Results

1. One refusal, `unsupported_construct`/`declaration-form` at the body declaration's span, retaining the family `operation-body`, the selected layers and the required layer; no package. The `model` declaration raises no refusal.
2. The compile succeeds and the operation's `has_body` is set.
