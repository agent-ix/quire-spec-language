---
id: TC-992
title: "Keep native String and UUID as distinct checked value leaves"
type: TC
relationships:
  - target: "ix://agent-ix/quire-spec-language/FR-056"
    type: verifies
---
# TC-992: Keep native String and UUID as distinct checked value leaves

## Description

Verify that the real QSL pipeline maps model-native `String` and `UUID` to
distinct checked value leaves, preserves their value and equality semantics,
and keeps profile-bounded `Text` separate. Exercise QSpec TC-235 K2's
`Money.currency`, `Order.order_id`, `OrderPlaced.order_id` and
`Fulfilment.correlation_id` meanings without rewriting the selected source
package. `OrderPlaced.order_id` is an ordinary event payload field; the
identity slots are `Order.order_id` and `Fulfilment.correlation_id`.

## Test Procedure

1. Read and normalize the shared TC-235 K2 package through the actual FCD
   reader and QSL intake. Use the original `String` and `UUID` native names.
   Assemble the checked model and inspect the checked type of
   `Money.currency`, the UUID identity semantics of `Order.order_id` and
   `Fulfilment.correlation_id`, and the ordinary UUID event field
   `OrderPlaced.order_id`. Also inspect a profile-bounded `Text` control.
2. Construct two native String values containing the same scalar sequence and
   compare them. Compare NFC `é` (U+00E9) with `e` followed by U+0301 in the
   native String type, then compare those values under `Text[0, 8; nfc]`.
   Inspect the checked type and value displays for String and Text.
3. Construct UUID values from the canonical payloads
   `00112233-4455-6677-8899-aabbccddeeff` and
   `00112233-4455-6677-8899-aabbccddeefe`. Compare equal copies of the first,
   then compare the two different payloads. Compare a UUID with a native
   String containing the first UUID's canonical text. This cross-type equality
   is a static type-check operation: retain `ill_typed` and verify that it
   produces no Boolean result. Inspect the UUID type and value displays.
4. At runtime, supply a String value to a checked UUID-typed member and a UUID
   value to a checked String-typed member. Retain each
   `invalid_runtime_input`/`wrong-value-kind` refusal at the member input path;
   neither refusal produces a value.

## Expected Results

- Step 1: the original 21 K2 declaration records are read and normalized.
  `Money.currency` has checked type `String`; `Order.order_id` and
  `Fulfilment.correlation_id` are the UUID identity slots;
  `OrderPlaced.order_id` has checked type `UUID` as an ordinary event field;
  and the control retains its declared `Text` bounds and profile. No String or
  UUID member refuses as an unsupported model member.
- Step 2: equal scalar sequences compare equal. The two differently
  normalized String sequences compare unequal. Under NFC Text they compare
  equal. Type display identifies `String` separately from `Text`, and String
  value display preserves the original scalar sequence without normalization,
  case folding or truncation.
- Step 3: equal UUID bits compare equal and produce the same canonical key;
  the changed last bit compares unequal and produces a different key. The
  mixed String/UUID equality is refused as `ill_typed` during static checking
  and yields no Boolean result. UUID type display is `UUID`; UUID value display
  is the exact canonical lowercase text.
- Step 4: both wrong-kind runtime inputs are refused as
  `invalid_runtime_input`/`wrong-value-kind` at their member input paths, with
  no value produced. Neither type is coerced to the other, `Text`, `Integer` or
  `Reference`.
