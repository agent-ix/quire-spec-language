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
   Run the concrete adapter vectors below with their retained input origins;
   compare the complete code, cause, location kind, document and path steps,
   the typed kernel construction cause where reached, and absence of outputs.
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
  noncanonical text decided before Timestamp range. Every adapter vector below
  has exactly its stated `invalid_runtime_input` cause and typed JSON input
  path. Envelope failures precede kernel construction; kernel failures retain
  their native-specific construction class. No failure produces an admitted
  native value, repaired payload or Boolean. Generic `invalid-value` for a
  missing/duplicate/unknown member, or a lost path, fails this oracle.
- Step 7: equal same-kind payloads give true and equal keys; changed payloads
  give false and unequal keys. Mixed kinds refuse under QSpec FR-149, never
  give false or convert. Each native key orders canonical payload content
  unsigned ASCII bytewise with a proper prefix first; it grants no ordering
  operator. Payload/codec success alone is no claim that K2 was admitted.

### Step 6 adapter vectors

Use the QSL canonical-native adapter boundary, not a fabricated new document
format or a direct kernel call pretending to decode JSON. The nested origins
come from the existing
[FR-106](../functional/FR-106-admit-snapshots-and-invocations.md)
snapshot shape: `populations[0].objects[1].fields.order_id` for a declared
UUID field and `populations[0].objects[1].fields.occurred_at` for a declared
Timestamp field. Supply the actual selected snapshot's document identity `D`
unchanged. This isolates the newly specified native envelopes at those field
slots; it does not assert that the current whole-document reader or its schema
already admits them.

The expected locations use
[FR-274](ix://agent-ix/quire-specification/FR-274)'s existing `json_path`
kind, document identity and tagged field/index steps. Define fixed expected
step sequences independently of the adapter result:

- `U` = `[field populations, index 0, field objects, index 1, field fields,
  field order_id]`.
- `T` = `[field populations, index 0, field objects, index 1, field fields,
  field occurred_at]`.
- `U.text`, `U.kind`, `U.extra` and `T.nanoseconds` append exactly one
  `field` step with that member's spelling. These abbreviations describe
  typed steps; they are not display strings or RFC 6901 pointers.

For example, `U.text` means a complete typed location whose kind is
`json_path`, document is exactly `D`, and steps are exactly
`[field populations, index 0, field objects, index 1, field fields,
field order_id, field text]`, using the published FR-274 discriminators,
not a newly defined location encoding. An empty path, a path lacking its enclosing
array indices, an absent document, a runtime-value path or a human display
path is not equal to this expected location.

The `Input` column contains exact envelope bytes presented at the indicated
origin. In rows that omit the unchanged UUID payload, `u` denotes exactly
the string `00112233-4455-6677-8899-aabbccddeeff`; expand it before execution.
All rows expect code `invalid_runtime_input` and the exact cause and location
listed. Kernel classes below refer to quire-exact's `ConstructionCause`
classes at `Component::Value`, not new diagnostic-catalog causes.

| Vector | Origin / expected native | Input | Exact cause | Exact JSON path | Deciding boundary / kernel construction result |
| --- | --- | --- | --- | --- | --- |
| A01 | `U` / UUID | `{"kind":"uuid","text":` (truncated at the payload) | `malformed-json` | `U` | Envelope read; kernel admission not called. The independently supplied origin remains known even though the envelope has no parsed tree. |
| A02 | `U` / UUID | `{"kind":"uuid"}` | `missing-member` | `U.text` | Envelope member validation; kernel admission not called. |
| A03 | `U` / UUID | `{"kind":"uuid","text":"u","text":"u"}` | `duplicate-member` | `U.text` | Original-byte envelope read detects the second `text`; kernel admission not called. No last-wins parse. |
| A04 | `U` / UUID | `{"kind":"uuid","text":"u","extra":true}` | `unknown-member` | `U.extra` | Envelope member validation; kernel admission not called. |
| A05 | `U` / UUID | `{"kind":"unknown","text":"u"}` | `invalid-value` | `U.kind` | Envelope tag validation; kernel admission not called. |
| A06 | `U` / UUID | `{"kind":"uuid","text":0}` | `wrong-value-kind` | `U.text` | Envelope payload-kind validation; kernel admission not called, even though zero fits a number domain. |
| A07 | `U` / UUID | `{"kind":"timestamp","nanoseconds":"0"}` | `wrong-value-kind` | `U.kind` | The native kind differs from the selected field's UUID type; no UUID value, retagging or coercion. |
| A08 | `U` / UUID | `{"kind":"uuid","text":"00112233-4455-6677-8899-AABBCCDDEEFF"}` | `invalid-value` | `U.text` | Kernel admission returns the noncanonical UUID construction class at `Component::Value`. |
| A09 | `T` / Timestamp | `{"kind":"timestamp","nanoseconds":"-0"}` | `invalid-value` | `T.nanoseconds` | Kernel admission returns the noncanonical Timestamp construction class at `Component::Value`. |
| A10 | `T` / Timestamp | `{"kind":"timestamp","nanoseconds":"170141183460469231731687303715884105728"}` | `invalid-value` | `T.nanoseconds` | Kernel admission returns the Timestamp out-of-domain construction class at `Component::Value`. |
| A11 | `T` / Timestamp | `{"kind":"timestamp","nanoseconds":"0170141183460469231731687303715884105728"}` | `invalid-value` | `T.nanoseconds` | Kernel admission returns the noncanonical Timestamp class, not out-of-domain; text admission precedes range. |
| A12 | `U` / UUID | `{"kind":"uuid","text":"INVALID","extra":true}` | `unknown-member` | `U.extra` | Envelope failure precedes the invalid payload; kernel admission not called. |

Each failed row exposes no constructed native value, normalized replacement,
effective model view or truth value. Retain the selected expected native kind
and original envelope bytes as inputs; neither is overwritten with a fallback
kind or a repaired spelling. A07's wrong-kind result is decided before any
attempt to treat the Timestamp payload as UUID. A08–A11 preserve the three
typed kernel failure classes while projecting their existing QSL code/cause
and retaining the full nested origin. The original positive and negative
canonical-payload controls remain required; this table adds diagnostic and
boundary assertions, not weaker admission alternatives.

## Status

🚧 Planned.
