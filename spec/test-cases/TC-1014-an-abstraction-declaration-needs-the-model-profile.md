---
id: TC-1014
title: "An abstraction declaration refuses without the model profile"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-361
    type: verifies
---
# TC-1014: An abstraction declaration refuses without the model profile

## Description

Verify that an `abstraction-decl` refuses at S3 without the model profile and checks with it. Scope: FR-361-AC-3. TC-811 step 1 gives the declaration; its steps 3 and 4 give the malformed forms.

## Test Procedure

1. Compile TC-811 step 1's source (its `model` declaration and `abstraction-decl`) with the header profile `quire.value.complete/v1`.
2. Compile it with `quire.model.complete/v1`.
3. Compile TC-811 step 3's declaration (no binding) under each profile.

## Expected Results

1. One refusal, `unsupported_construct`/`declaration-form` at the declaration's span, family `abstraction`.
2. Compiles to the same `CheckedAbstractionRelation` as TC-811 step 1.
3. Refused `invalid_syntax` at the offending token under both profiles.
