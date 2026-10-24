---
id: TC-665
title: "S3 checks orderings and fences and records access orderings; EN-1 classifies accesses"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-220
    type: verifies
---
# TC-665: S3 checks orderings and fences and records access orderings; EN-1 classifies accesses

## Description

Verify access classification, admitted orderings with their refusal, the access-shape rule at S3 and in the pre-check, and that orderings change nothing under `sc`.

Scope: FR-220-AC-1 to FR-220-AC-15. Steps 1 to 9 are S3 and pre-check checks; steps 10 to 14 are EN-1 checks, owned by the EN-1 pre-check and run against the EN-1 pre-check.

## Test Procedure

1. Check `SB` and read each attempt's recorded class and ordering; check the
   non-atomic and `fetch_add` variants and an attempt that touches no data place.
2. Check each ordering of FR-220-AC-2 that no possible kind takes, the RMW with
   no ordering and the `release` load that also writes a register, and each
   admitted fence.
   Parse `fence F ordering relaxed;` at S1 (FR-220-AC-6).
3. Check the two-field attempt under `memory tso` and under `sc`, check the
   any-object field write under `memory tso`, then select `tso` from the request.
4. Check `SB` under `sc` with all `relaxed` and with all `seq_cst`.
5. Run the pre-check on AC-3's two-field attempt and on FR-219-AC-4's
   `any`-join `parallel`, each with a request selecting `tso`.
6. Check two branches each storing the same cell `relaxed` with no reader, under `tso` and `ra` (FR-220-AC-7).
7. Check a branch that writes a core register and reads it again (FR-220-AC-8).
8. Check `SB` under `memory tso` and under `memory ra` (FR-220-AC-9).
9. Check an attempt with `ordering seq_cst` outside every `parallel` (FR-220-AC-10).

EN-1 steps (not S3 results):

10. Instantiate `SB` under `tso` and under `ra` and read EN-1's classes and
    sharing (FR-220-AC-11); include applications whose selected precondition
    is false and confirm they contribute no footprint.
11. Instantiate AC-7's two stores and read EN-1's classes (FR-220-AC-12).
12. Instantiate AC-8's branch and read EN-1's sharing (FR-220-AC-13).
13. Run the pre-check on an instantiated `release` load, an `acq_rel` load, an
    `acquire` store, an `acq_rel` store and an RMW with no ordering, under
    `tso`, under `ra` and under `sc`, on a `release` attempt that writes a
    branch-local register and touches no shared location on instantiation, and on an `acquire` attempt that reads a branch-local register
    and writes two shared fields, under `tso` (FR-220-AC-14).
14. Run the pre-check on FR-220-AC-15's `event` reader beside the storing
    branch under `tso` and under `sc`, on the `z.v` reader of the variant whose
    universe also holds an unaccessed `z`, and on the `finish` reader. Include
    the same item with the storing attempt given `acquire` and verify that the
    attempt's ordering-mismatch and reader's multi-location entries are
    returned together, in typed protocol transition identity order, with the
    reader's `transition` set to `None`. In the unaccessed-`z` variant, verify
    that the attempts' selected preconditions and constraints reject `z`.

Tag the tests `#[trace("TC-665", "FR-220-AC-n")]`.

## Expected Results

- Step 1: no refusal for `SB` and the variants, each attempt atomic or non-atomic by its ordering, no class recorded; class `Local` for the attempt that touches no data place.
- Step 2: `ill_typed`/`operator-ineligible` at S3 naming each access no possible kind takes; the RMW with no ordering and the `release` load that also writes a register check; the four fences check; `invalid_syntax` at S1 at the `relaxed` token of the
  fence, with no S3 result.
- Step 3: the two-field attempt checks at S3 under `tso` and under `sc`; the any-object field write refuses `unsupported_construct`/`declaration-form` naming the location; the selection produces a `WeakAccessShape.failures` entry with `MultiLocation` under the selected weak model.
- Step 4: 16 states and `proved` both times.
- Step 5: `WeakAccessShape.failures` contains an entry with
  `MultiLocation{locations}` naming both locations and an entry with
  `JoinPolicy`; each shape serializes as QSpec FR-439 specifies, and both use
  the common entry form and canonical ordering.
- Step 6: no refusal; both attempts atomic with ordering `relaxed`; no class recorded.
- Step 7: no refusal; no class recorded.
- Step 8: checks under both models; every attempt atomic, ordering recorded, no class recorded.
- Step 9: `ill_typed`/`operator-ineligible` naming the attempt.
- Step 10: only applications with true selected preconditions and matching attempt constraints contribute footprints; `Sx` and `Sy` are stores, `Ly` and `Lx` are loads, and (`c1`, `r`), (`c2`, `r`) are branch-local.
- Step 11: two stores to one shared location.
- Step 12: the register is branch-local.
- Step 13: under `tso` and `ra`, the pre-check at the initial state returns V-8 `WeakAccessShape.failures` with the `ordering-mismatch` entry FR-220-AC-14 specifies for each case, before generating outgoing transitions; under `sc`, each is admitted; the local access has `access` `local`; the two-field attempt has only `multi-location` for its own shape, while the pre-check continues collecting other failures.
- Step 14: under `tso`, the base example has a `MultiLocation` entry for the `event` reader and not for the ordinary store. With the store's ordering changed to `acquire`, one V-8 `WeakAccessShape.failures` list contains the reader's `MultiLocation` and the store's `OrderingMismatch`; both share ascending JCS-byte order of typed protocol transition identities, and the reader entry's `transition` is `None`. For the unaccessed-`z` variant, the store precondition and load constraints exclude `z` in every reachable state, so the `z.v` reader is not shared and the item is admitted. Under `sc`, the item is admitted for the shared reader and the `finish` reader.
