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
its provenance, its FR-100-aligned function selection and its exit codes.

Scope: FR-109-AC-1 to FR-109-AC-5.

## Test Procedure

Build `ClauseRunRequest`s from FR-108's fixtures, in memory.

1. healthy-parent; violating-parent.
2. missing-model (no domain package supplied); healthy-parent with an
   expected `package_id` taken from compiling another unit; healthy-parent
   selecting the clause `Absent`; healthy-parent with a `Clause` selection
   naming the function `sameIdentity`.
3. dangling-parent; incomplete-population; exhausted-work (`work_units` 0).
4. `Function { name: sameIdentity, arguments: [{parameter: b, value: child},
   {parameter: a, value: root}], snapshot: distinct-identities }`; the same
   with both `child`; with `b` naming `ghost`; with an extra argument `c`; a
   function `n(a: Config::ConfigVersion): Integer pure { 1 }` selected the
   same way.
5. healthy-parent twice; healthy-parent with the snapshot bytes edited after
   the selection digest was taken; then, for each S6a outcome FR-100-AC-9
   constructs other than `Completed`, pass it through `run_clause`'s mapping
   and through FR-100's, and compare.

Tag the tests `#[trace("TC-468", "FR-109-AC-n")]`.

## Expected Results

- Step 1: `evaluate`, `success`, `truth: true`, exit 0, provenance holding the
  source digest, `package_id`, model selection, selection and the snapshot's
  identity and digest; then `violation`, `truth: false`, exit 10.
- Step 2: `compile`, `refusal`, `missing_import`/`missing-selection`, exit 20,
  no snapshot in provenance; `compile`, `stale_dependency`, naming both
  identities; `select`, `missing_declaration`/`missing-name`, twice.
- Step 3: `admit`, `refusal`, `dangling_reference`, exit 20; `admit`,
  `incomplete`, `incomplete_population`, exit 22; `evaluate`, `incomplete`,
  `{"kind": "incomplete", "limit": "work_units"}`, exit 22. None has `truth`.
- Step 4: `violation`, `truth: false`; `success`, `truth: true`; `admit`,
  `invalid_runtime_input`/`wrong-role-mapping`; `admit`, FR-100's
  unknown-parameter refusal naming `c`; `select`, `ill_typed`/
  `type-mismatch`, with an uncharged meter.
- Step 5: equal reports including usage; `admit`, `stale_dependency`/
  `byte-digest-mismatch`; every compared outcome gives the same `outcome`
  member and exit status in both mappings.

## Status

Planned (QSL-273).
