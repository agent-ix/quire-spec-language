---
id: TC-1083
title: "The refinement mapping lifts nested references, unwraps optional fields and reports what it cannot map"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-140
    type: verifies
---
# TC-1083: The refinement mapping lifts nested references, unwraps optional fields and reports what it cannot map

## Description

Verify the rules of `map_state` that TC-545 does not reach: reference lifting
through option and collection values, an unmapped concrete type, the two
`ModelSystem` arguments and the `over_model` refusal, the `EvalFailure` causes
of an undetermined row, a mapped state with no relationship link, and the
unwrap rule for optional abstract fields.

Scope: FR-140-AC-6 to FR-140-AC-11.

## Test Procedure

Fixtures: `RingIsQueue` (FR-139) and the variants FR-140-AC-6, AC-10 and
AC-11 describe; `CasRefinesCounter` (ADR-020 §8) for AC-7 to AC-9.

1. Map the ring state of FR-140-AC-2 under the AC-6 variant; repeat with a
   reference whose type no object row names.
2. Map a state whose source population holds an object of a type no object
   row names.
3. Build a `ModelSystem` over a model the unit does not select; map with the
   two systems of FR-140-AC-1.
4. Map with a meter budget of zero, with an evaluator limit exceeded, and with
   a row whose `only` finds two objects.
5. Map the ring state under the AC-10 variant and compare its key with the
   abstract state built directly.
6. Map `head = 1`, `size = 2` and `size = 0` under the AC-11 variant.

Tag the tests `#[trace("TC-1083", "FR-140-AC-n")]`.

## Expected Results

- Step 1: `owners = [r, r]` and `parent` set, each a `Queue` reference; then
  `Undetermined` with `UnmappedType`.
- Step 2: `Undetermined` naming the `Object` row for that type, `UnmappedType`;
  no object is dropped.
- Step 3: `invalid_runtime_input`/`wrong-role-mapping`; then a state whose key
  is the abstract system's.
- Step 4: `Incomplete`; `Refused`; `Undefined` with an `UndefinedRecord` of
  reason `only-not-one`.
- Step 5: no relationship extent; the keys are equal.
- Step 6: `head` 1; then `head` unset and not hidden.
