---
id: TC-1016
title: "A model declaration refuses without the model profile"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-361
    type: verifies
---
# TC-1016: A model declaration refuses without the model profile

## Description

Verify that a `model` declaration refuses without the model profile and that a value-only state clause does not. Scope: FR-361-AC-5.

## Test Procedure

1. Compile a unit with the header profile `quire.value.complete/v1`, a `model` declaration and a supplied domain package.
2. Compile a unit with the same header and a `pre` clause over only value-complete operands.
3. Compile the unit of step 1 under `quire.model.complete/v1`.

## Expected Results

1. Refused `unsupported_construct`/`declaration-form` at the `model` declaration, family `model-import`; no package.
2. Compiles.
3. The `model` declaration is admitted as FR-056 states.
