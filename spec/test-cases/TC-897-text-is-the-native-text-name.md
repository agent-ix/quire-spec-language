---
id: TC-897
title: "Text is the native text name and an undeclared native name is refused"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: verifies
---
# TC-897: Text is the native text name and an undeclared native name is refused

## Description

Verify that model intake reads `ix://quire/native/Text` as QSL's native
`Text`, refuses it for its unexpressible `profile` parameter, and refuses an
undeclared native name, `ix://quire/native/String`. Scope:
FR-056-AC-11.

## Test Procedure

Start from a domain package whose object type `Note` declares a field
`label` and an operation `rename(to): Boolean` that intake admits.

1. Set `label`'s `typeRef` to `ix://quire/native/String` and admit.
2. Set `label`'s `typeRef` to `ix://quire/native/Text` and admit.
3. Repeat steps 1 and 2 with `label` restored and the `typeRef` set on
   `rename`'s parameter `to`.
4. Repeat steps 1 and 2 with the `typeRef` set on `rename`'s result.

## Expected Results

- Step 1: refused `invalid_model_binding`/`malformed-declaration` at
  `label`, naming `ix://quire/native/String`.
- Step 2: refused `unsupported_construct`/`declaration-form` at `label`,
  naming the parameter `profile`.
- Steps 3 and 4: the same two refusals, at the parameter and at the result.

## Status

🚧 Planned.
