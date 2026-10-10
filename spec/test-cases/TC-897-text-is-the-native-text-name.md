---
id: TC-897
title: "Domain natives retain Text profiles and canonical K2 payloads"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: verifies
---
# TC-897: Domain natives retain Text profiles and canonical K2 payloads

## Description

Verify that model intake admits a scalar type bound to `Text` with each of
QSpec FR-141's six profiles as `Text[min, max; profile]`, refuses bounds
outside `0 <= min <= max`, refuses a profile
FR-141 does not define, refuses a bare `ix://quire/native/Text` that carries
no profile, and refuses an undeclared native name,
`ix://quire/native/UnknownText`. K2's `String` remains a declared native;
it is neither this unknown-name case nor a parameterized Text alias.
Verify the exact native-payload controls of
[FR-056](../functional/FR-056-admit-domain-package-model-declarations.md#fr-056-ac-17).
Scope: FR-056-AC-11 and the canonical-payload part of FR-056-AC-17;
QSpec TC-235 retains K2's model binding and complete business vectors.

## Test Procedure

Start from a domain package whose object type `Note` declares a field
`label` and an operation `rename(to): Boolean` that intake admits.

1. Add a scalar type `Label` bound to `Text` with bounds `0` and `64` and
   profile `nfc`, set `label`'s `typeRef` to `Label` and admit. Repeat with
   each of `unicode-scalars`, `nfd`, `nfkc`, `nfkd` and `binary-utf8`.
2. Set `Label`'s profile to `nfx` and admit. Then restore `nfc` and admit
   with `Label`'s bounds `-1` and `64`, then `65` and `64`.
3. Set `label`'s `typeRef` to `ix://quire/native/Text` and admit.
4. Set `label`'s `typeRef` to `ix://quire/native/UnknownText` and admit.
5. Repeat steps 1 to 4 with `label` restored and the `typeRef` set on
   `rename`'s parameter `to`, then on `rename`'s result.
6. Through the actual shared kernel's public admission APIs, execute every
   FR-056-AC-17 canonical-payload control: UUID bit patterns and invalid
   spellings; Timestamp zero, signs, endpoints, exterior values and invalid
   spellings; malformed closed wire objects at the wire adapter boundary.
   Compare independently written expected payload text and type identity.
7. Compare independently admitted equal UUID payloads, then one differing
   digit; independently admitted Timestamp `0` values, then `0` versus `1`.
   Compare their same-kind canonical keys. Try UUID against Timestamp and
   each against `Int`, retaining the typed refusal and no Boolean.

## Expected Results

- Step 1: admitted each time; `label`'s value type is `Text[0, 64; p]` for
  the profile `p` set.
- Step 2: refused `unsupported_construct`/`declaration-form` at `Label`,
  naming `nfx`; then, for each pair of bounds, refused
  `unsupported_construct`/`declaration-form` at `Label`, naming the bounds.
- Step 3: refused `unsupported_construct`/`declaration-form` at `label`,
  naming `profile`.
- Step 4: refused `invalid_model_binding`/`malformed-declaration` at
  `label`, naming `ix://quire/native/UnknownText`.
- Step 5: the same outcomes at the parameter and at the result.
- Step 6: exactly FR-056-AC-17's admissions and typed payload failures, with
  noncanonical text decided before Timestamp range. Wire shape failures
  produce no typed payload. No rejected spelling is silently repaired.
- Step 7: equal same-kind payloads give true and equal keys; changed payloads
  give false and unequal keys. Mixed kinds refuse under QSpec FR-149, never
  give false or convert. Each native key orders canonical payload content
  unsigned ASCII bytewise with a proper prefix first; it grants no ordering
  operator. Payload/codec success alone is no claim that K2 was admitted.

## Status

🚧 Planned.
