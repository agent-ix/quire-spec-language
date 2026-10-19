---
id: TC-1086
title: "The refinement mapping reports an unmapped type, a wrong system and the first failure, and never drops an object"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-140
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: verifies
---
# TC-1086: The refinement mapping reports an unmapped type, a wrong system and the first failure, and never drops an object

## Description

Verify the rules of `map_state` that TC-545 and TC-555 do not reach: an
unmapped concrete type reached through an object and through a reference,
the two `ModelSystem` arguments and the `over_model` refusal, the order in
which failures are returned, the `only-not-one` reason, and a mapped state
with no relationship extent. Optional-field mapping has no vector (FR-140, remaining work STD-211).

Scope: FR-140-AC-6 to FR-140-AC-10, FR-120-AC-14.

## Test Procedure

Fixtures: `RingIsQueue` (FR-139) with the variants FR-140-AC-6 and AC-9
describe; `CasRefinesCounter` (ADR-020 §8) with the `SubCounter` subtype of
FR-140-AC-7.

1. Map the FR-140-AC-6 variant with `q` of type `SubRing`, then with `q` of
   type `Ring`.
2. Map the FR-140-AC-7 state, in which `c` has type `SubCounter`.
3. Map with the two systems of FR-140-AC-1, then with them swapped and with a
   system over a third model.
4. Map the FR-140-AC-9 variants.
5. Map the ring state of FR-140-AC-2 and read its relationship extents.
6. Call `ModelSystem::over_model` over `example/not-selected`, and over each
   of the unit's selected models.

Tag the tests `#[trace("TC-1086", "<AC id>")]`.

## Expected Results

- Step 1: `Undetermined` naming the `next` row and object `a`, with
  `UnmappedType(SubRing)`; then `a.next` maps to the `Queue` reference `q`.
- Step 2: `Undetermined` naming `Object` for `SubCounter` and `c`, with
  `UnmappedType(SubCounter)`; no object dropped.
- Step 3: a state whose key is the abstract system's; then `WrongSystem` with
  `invalid_runtime_input`/`wrong-role-mapping` for each, with no row read.
- Step 4: `Undefined` whose `UndefinedRecord` reason is `only-not-one`;
  `Undefined` also when a `SubRing` sorts first; `Undetermined` for `SubRing`
  with the original `items` row.
- Step 5: no relationship extent.
- Step 6: `invalid_runtime_input`/`wrong-role-mapping`; then a system that
  reports its model.
