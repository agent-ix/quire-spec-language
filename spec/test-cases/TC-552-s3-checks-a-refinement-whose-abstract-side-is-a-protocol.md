---
id: TC-552
title: "S3 checks a refinement whose abstract side is a protocol and writes a refinement record"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-147
    type: verifies
---
# TC-552: S3 checks a refinement whose abstract side is a protocol and writes a refinement record

## Description

Verify the abstract protocol side: resolution of the protocol, node targets,
the unit's `refinement_internals` member (the profile's internal list), the `refinement` requirement record and the unsupported
liveness half.

Scope: FR-147-AC-1 to FR-147-AC-6.

## Test Procedure

Fixtures: `Twice` and `CasTwice` (FR-147); `CasRefinesCounter` (ADR-020 §8).

1. Check `CasTwice`, and with `commitA -> node Spec::Twice::a1(self)`.
2. Check each refusal variant of FR-147-AC-2.
3. Check and settle `CasTwice` with `ensure fair weak Spec::Counter::inc`.
4. Check `CasTwice` with `finish` removed from the profile's internal list,
   with the list naming `finish`, and with the list naming `finish` beside
   a step row `-> node Spec::Twice::finish(self)`.

5. Check `CasTwice` with `refinement_internals` set to the FR-147 fixture
   member, absent, `[]`, and holding only an object for another refinement;
   compute each `package_id`.
6. Check each member defect of FR-147-AC-6, one unit per variant.

Tag the tests `#[trace("TC-552", "FR-147-AC-n")]`.

## Expected Results

- Step 1: `AbstractSide::Protocol` naming `Twice`, `abstract_internal =
  [finish]`, one `refinement` record and no `temporal-satisfaction` record;
  the node row checks naming `a1`.
- Step 2: each variant refuses with FR-147-AC-2's code and subcode.
- Step 3: liveness half `unsupported`, `unsupported-requested-capability`,
  naming hidden abstract fields.
- Step 4: checks with `finish` visible and `abstract_internal` empty; checks
  with `finish` internal; `invalid_model_binding`/`conflicting-binding`.
- Step 5: the fixture member gives `abstract_internal = [finish]`; the other
  three give it empty with `finish` visible; the fixture unit's `package_id`
  differs from the other three, which are equal to one another.
- Step 6: each variant refuses with FR-147-AC-6's code and subcode, naming the
  member path or both objects, and no `CheckedRefinement`.
