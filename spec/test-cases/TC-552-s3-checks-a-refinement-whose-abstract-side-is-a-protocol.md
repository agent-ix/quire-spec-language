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
the profile's internal list (its shape, resolution and refusal), the
`refinement` requirement record and the unsupported liveness half.

Scope: FR-147-AC-1 to FR-147-AC-7. Steps 1, 3 and 4 wait on STD-199 and
STD-182 (FR-147 Remaining work); steps 2 and 5 to 7 run now.

## Test Procedure

Fixtures: `Twice` and `CasTwice` (FR-147); `CasRefinesCounter` (ADR-020 §8).

1. Check `CasTwice`, and with `commitA -> node Spec::Twice::a1(self)`.
2. Check each refusal variant of FR-147-AC-2.
3. Check and settle `CasTwice` with `ensure fair weak Spec::Counter::inc`.
4. Check `CasTwice` with `finish` removed from the profile's internal list,
   with the list naming `finish`, and with the list naming `finish` beside
   a step row `-> node Spec::Twice::finish(self)`.

5. Resolve `CasTwice`'s list from the FR-147 value, with the value absent,
   `[]`, holding only `{"refinement": "CasRefinesCounter", "internal": []}`,
   and holding the two entries of FR-147-AC-5 in both orders and in two arrays
   swapped.
6. Resolve each defect variant of FR-147-AC-6, one value per variant.
7. Give the refinement check a non-empty resolved list for `CasTwice` and for
   `CasRefinesCounter`, and an empty one.

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
- Step 5: the fixture value gives `abstract_internal = [finish]`; the other
  three give it empty; the two orders and the swapped arrays give the same
  list, ordered by the bytes of the canonical spellings.
- Step 6: each variant refuses with FR-147-AC-6's code and subcode at its
  JSON Pointer (both pointers for a conflict) and gives no list.
- Step 7: the two non-empty lists refuse
  `unsupported_construct`/`not-yet-implemented` at the `<protocol>` name span
  with no `CheckedRefinement`, classified `unsupported`; the empty list is not
  refused for the list.
