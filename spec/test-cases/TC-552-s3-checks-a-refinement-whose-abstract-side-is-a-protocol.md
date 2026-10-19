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
the refinement's internal list (its shape, spellings, resolution, locus and refusal), the
`refinement` requirement record and the unsupported liveness half.

Scope: FR-147-AC-1 to FR-147-AC-11.

## Test Procedure

Fixtures: `Twice`, `CasTwice` and `Full` (FR-147); `CasRefinesCounter`
(ADR-020 §8).

1. Check `CasTwice`, and with `commitA -> node Spec::Twice::a1(self)`.
2. Check each refusal variant of FR-147-AC-2.
3. Check and settle `CasTwice` with `ensure fair weak Spec::Counter::inc`.
4. Check `CasTwice` with `finish` removed from the refinement's internal list,
   with the list naming `finish`, and with the list naming `finish` beside
   a step row `-> node Spec::Twice::finish(self)`.

5. Resolve `CasTwice`'s list from the FR-147 value, with the value absent,
   `[]`, holding only `{"refinement": "CasRefinesCounter", "internal": []}`,
   and holding the two entries of FR-147-AC-5 in both orders and in two arrays
   swapped.
6. Resolve each defect variant of FR-147-AC-6, one value per variant.
7. Give the refinement check a non-empty resolved list for `CasTwice` and for
   `CasRefinesCounter`, and an empty one.
8. Resolve a value naming one entry of every class over `Full`.
9. Resolve `CasTwice`'s list, rename the abstract alias to `S` with the value
   renamed to match, and resolve again; compare with the empty list.
10. Place one defect in the unit's value and the same defect in a supplied
    library's value.
11. Give the refinement check, in a build omitting `refinement-internal-list`,
    a list with an unresolved entry, a value attributable to no refinement, a
    well-formed non-empty list and a malformed entry on `CasRefinesCounter`.

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
  (input, JSON Pointer) pair (both pairs for a conflict) and gives no list.
- Step 7: the non-empty list for `CasRefinesCounter` refuses exactly once,
  `unsupported_construct`/`not-yet-implemented` at its model alias, naming
  `refinement-internal-list`, the refinement and the profile; the non-empty
  list for `CasTwice` refuses exactly twice, `protocol-abstract-side` at
  `Twice` and `refinement-internal-list` at `Spec`, each once; all classified
  `unsupported`, with no `CheckedRefinement`; the empty list is not refused
  for the list.
- Step 8: each entry makes exactly its own class internal; the resolved list
  is in the byte order of the alias-free spellings.
- Step 9: `abstract_internal = [Twice::finish]` before and after the rename,
  with the same node identity; the empty list gives a different identity.
- Step 10: the unit's defect is located at (the unit's value, pointer), the
  library's at (that library's value, pointer) and the compilation is not
  admitted; no compile-request member is named.
- Step 11: the unresolved entry gives `missing-name` and no
  `not-yet-implemented`; the value attributable to no refinement gives its
  defect alone; the well-formed non-empty list gives only the
  `not-yet-implemented` refusal at the model alias; the malformed entry gives
  `malformed-declaration`.
