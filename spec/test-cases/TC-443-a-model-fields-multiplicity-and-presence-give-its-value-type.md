---
id: TC-443
title: "A model field's multiplicity and presence give its assembled value type"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: verifies
---
# TC-443: A model field's multiplicity and presence give its assembled value type

## Description

Verify `model_field`'s own mapping from a domain package field's
`multiplicity` and `presence` to its assembled `FieldDeclaration`, per
QSpec's `model-complete.md` Presence row and FR-322's "Model-owned members"
step 4: a multiplicity lower bound of `0` never makes a field optional, only
`presence` does, and `[1, 1]` gives the declared value type outright while
any other multiplicity gives the `ordered`/`unique`-selected collection.
Also verify intake's own `presence` wire-value check reports
`invalid_model_binding`/`malformed-declaration`, not a panic, for a value
outside `required`/`optional`. Scope: FR-056-AC-10.

## Test Procedure

1. Call `model_field` directly over a native-`Integer` field at `[1, 1]`,
   `[0, 1]`, `[0, 3]`, `[0, 5]`, `[0, unbounded]` and `[1, unbounded]`, at
   `Required` and `Optional` presence, and over every `ordered`/`unique`
   flag combination at a finite multiplicity distinct from `[1, 1]`/`[0,
   1]`.
2. Call `read_field_member` over a field whose `presence` is an
   unrecognized string, missing entirely, and a non-string value.

## Expected Results

- Step 1: `[1, 1]` gives the element type `E` regardless of the
  `ordered`/`unique` flags; any other multiplicity gives the bounded
  collection its flags name (`[0, unbounded]` unbounded, with no
  `collection_bounds`); `[1, unbounded]` refuses -- QSpec's merged
  `model-member-type-vectors.json` MA-07 pins the same refusal. `presence`
  alone decides `FieldDeclaration::presence()`: a required `[0, 1]`/`[0,
  3]` stays the bounded collection, never `Option`, and an optional `[1,
  1]`/`[0, 1]`/`[0, 3]` leaves the multiplicity's own value type unwrapped,
  with only `presence()` carrying `Optional`.
- Step 2: every case refuses `invalid_model_binding` with
  `IntakeMalformedDeclaration`, naming the offending value or that
  `presence` is missing or not a string.

## Status

Backed: `qsl-semantics/src/check/assemble/tests.rs` (step 1, each test
tagged `#[trace("TC-443", "FR-056-AC-10")]`) and
`qsl-semantics/src/model/intake.rs` (step 2, same tag).
