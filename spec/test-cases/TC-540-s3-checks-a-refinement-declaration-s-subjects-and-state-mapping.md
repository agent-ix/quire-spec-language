---
id: TC-540
title: "S3 checks a refinement declaration's subjects, population map and object map"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: verifies
---
# TC-540: S3 checks a refinement declaration's subjects, population map and object map

## Description

Verify that S3 checks a refinement declaration's header, population rows and
object rows into a `CheckedRefinement`, refuses each missing, duplicated,
misplaced or ill-typed row at its span, lifts references, records hidden
fields and puts the declaration in the package identity.

Scope: FR-135-AC-1 to FR-135-AC-8.

## Test Procedure

Fixtures: ADR-020 §8's `CasRefinesCounter` unit, with `Spec` local and with
`Spec` supplied as a dependency package; ADR-020 §8's `RingIsQueue` unit.

1. Check `CasRefinesCounter` with `Spec` local, then with `Spec` as a
   dependency.
2. Check each refusal variant of FR-135-AC-2.
3. Check `value = self.busyA`; check `RingIsQueue` with a
   `Reference<Q::Queue>` field mapped by `self.ring` and by an expression of
   type `Reference<R::Slot>`.
4. Check `CasRefinesCounter` with the `value` row removed; check it with
   `value = self.tmpA` and compare node identities and `package_id`s with
   step 1.
5. Check `RegisterHistory` with one `history` row and with two, `CasTwice`, and
   `CasRefinesCounter`, each in the build as it stands, that is, for each
   feature the build has not built. The step does not choose a support set; it
   runs only while a feature is unbuilt, and is rewritten when the feature
   lands.
6. Check `RingIsQueue` with a nested `Sequence<Reference<Ring>>` row, and
   with the same field as a plain reference.
7. Check `RingIsQueue` with a relationship declared on the abstract `Queue`.
8. Check `RingIsQueue` in a unit that selects no layer of its own, then with
   a Boolean row naming a protocol of the unit.

Tag the tests `#[trace("TC-540", "FR-135-AC-n")]`.

## Expected Results

- Step 1: a `CheckedRefinement` with one population row, one object row with
  `value = self.value` and no hidden field; then `AbstractSide::Dependency`
  with the dependency's `package_id`.
- Step 2: each variant refuses with FR-135-AC-2's code and subcode at its
  span, and no `CheckedRefinement`.
- Step 3: `ill_typed`/`type-mismatch` at the expression; the `self.ring` row
  checks; the `Reference<R::Slot>` row refuses `ill_typed`/`type-mismatch`.
- Step 4: `hidden = [value]`; different node identity and different
  `package_id`.
- Step 5: each `history` row refuses `unsupported_construct`/`not-yet-implemented`
  at its `history` keyword naming `history-row`, the refinement and the profile
  (two rows, two refusals); `CasTwice` refuses the same pair at `Twice` naming
  `protocol-abstract-side`; each classifies `unsupported`; no
  `CheckedRefinement`; `CasRefinesCounter` checks.
- Step 6: `unsupported_construct`/`not-yet-implemented` at the row's
  expression; then it checks.
- Step 7: `unsupported_construct`/`declaration-form` at the alias `Q`; no
  `CheckedRefinement`.
- Step 8: the rows check; then `unsupported_construct`/`expression-form` at
  the `protocol` token naming `quire.protocol.complete/v1`.
