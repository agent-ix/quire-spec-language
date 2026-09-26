---
id: TC-468
title: "The spine clause run entry reports typed dispositions with provenance and exit codes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: verifies
---
# TC-468: The spine clause run entry reports typed dispositions with provenance and exit codes

## Description

Verify `qsl_replay::spine::run_clause` over one case per stage and category,
its provenance, its function selection and its exit-code map.

Scope: FR-109-AC-1 to FR-109-AC-5.

## Test Procedure

Build `ClauseRunRequest`s from FR-108's fixtures, in memory.

1. healthy-parent; violating-parent.
2. missing-model (no domain package supplied); healthy-parent with an
   expected `package_id` taken from compiling another unit; healthy-parent
   selecting `Absent`.
3. dangling-parent; incomplete-population; exhausted-work (meter budget 0).
4. `Function { name: sameIdentity, arguments: [child, root], snapshot:
   distinct-identities }`; the same with `[child, child]`; a function
   `n(a: Config::ConfigVersion): Integer pure { 1 }` selected the same way.
5. healthy-parent twice; healthy-parent with the snapshot bytes edited after
   the selection digest was taken; then call `exit_code()` on a report of
   each category and on an `unsupported_construct` refusal.

Tag the tests `#[trace("TC-468", "FR-109-AC-n")]`.

## Expected Results

- Step 1: `evaluate`, `success`, `truth: true`, exit 0, provenance holding the
  source digest, `package_id`, model selection, selection and the snapshot's
  identity and digest; then `violation`, `truth: false`, exit 10.
- Step 2: `compile`, `refusal`, `missing_import`/`missing-selection`, exit 20,
  no snapshot in provenance; `compile`, `stale_dependency`, naming both
  identities; `admit`, `missing_declaration`/`missing-name`.
- Step 3: `admit`, `refusal`, `dangling_reference`, exit 20; `admit`,
  `incomplete`, `incomplete_population`, exit 22; `evaluate`, `incomplete`,
  `resource_exhausted`, exit 22. None has `truth`.
- Step 4: `violation`, `truth: false`; `success`, `truth: true`; `admit`,
  `ill_typed`/`type-mismatch`, with an uncharged meter.
- Step 5: equal reports including usage; `admit`, `stale_dependency`/
  `byte-digest-mismatch`; exit codes 0, 10 and 20 for `success`, `violation`
  and `undefined`, 20 for a `dangling_reference` refusal, 22 for a
  `resource_exhausted` incomplete, and 21 for the `unsupported_construct`
  refusal.

## Status

Planned (QSL-273).
