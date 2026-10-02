---
id: TC-897
title: "A Text scalar type is admitted with its QSpec profile"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: verifies
---
# TC-897: A Text scalar type is admitted with its QSpec profile

## Description

Verify that model intake admits a scalar type bound to `Text` with each of
QSpec FR-141's six profiles as `Text[min, max; profile]`, refuses a profile
FR-141 does not define, refuses a bare `ix://quire/native/Text` that carries
no profile, and refuses an undeclared native name,
`ix://quire/native/String`. Scope: FR-056-AC-11.

## Test Procedure

Start from a domain package whose object type `Note` declares a field
`label` and an operation `rename(to): Boolean` that intake admits.

1. Add a scalar type `Label` bound to `Text` with bounds `0` and `64` and
   profile `nfc`, set `label`'s `typeRef` to `Label` and admit. Repeat with
   each of `unicode-scalars`, `nfd`, `nfkc`, `nfkd` and `binary-utf8`.
2. Set `Label`'s profile to `nfx` and admit.
3. Set `label`'s `typeRef` to `ix://quire/native/Text` and admit.
4. Set `label`'s `typeRef` to `ix://quire/native/String` and admit.
5. Repeat steps 1 to 4 with `label` restored and the `typeRef` set on
   `rename`'s parameter `to`, then on `rename`'s result.

## Expected Results

- Step 1: admitted each time; `label`'s value type is `Text[0, 64; p]` for
  the profile `p` set.
- Step 2: refused `unsupported_construct`/`declaration-form` at `Label`,
  naming `nfx`.
- Step 3: refused `unsupported_construct`/`declaration-form` at `label`,
  naming `profile`.
- Step 4: refused `invalid_model_binding`/`malformed-declaration` at
  `label`, naming `ix://quire/native/String`.
- Step 5: the same outcomes at the parameter and at the result.

## Status

🚧 Planned.
