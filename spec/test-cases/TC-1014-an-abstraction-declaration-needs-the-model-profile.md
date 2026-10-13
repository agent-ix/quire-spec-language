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

Verify that an `abstraction-decl` refuses at S3 without the model profile and checks with it. Scope: FR-361-AC-3. TC-811 steps 2 and 3 give the declaration and the malformed forms.

## Test Procedure

1. Compile TC-811 step 1's declaration with the header profile `quire.value.complete/v1`.
2. Compile it with `quire.model.complete/v1`.
3. Compile a declaration with no binding under each profile.

## Expected Results

1. Refused `unsupported_construct`/`declaration-form` at the declaration's span, family `abstraction`.
2. Compiles to the same `CheckedAbstractionRelation` as TC-811 step 1.
3. Refused `invalid_syntax` at the offending token under both profiles.
