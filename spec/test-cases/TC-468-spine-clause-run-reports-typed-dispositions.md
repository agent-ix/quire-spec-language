---
id: TC-468
title: "The spine clause run entry reports typed dispositions and exit codes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: verifies
---
# TC-468: The spine clause run entry reports typed dispositions and exit codes

## Description

Verify `qsl_replay::spine::run_clause` over one case per stage and category,
its compiled `package_id`, its FR-100-aligned function selection and its
exit codes.

Scope: FR-109-AC-1 to FR-109-AC-7.

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
5. healthy-parent twice; then, for each S6a outcome FR-100-AC-9
   constructs other than `Completed`, pass it through `run_clause`'s mapping
   and through FR-100's, and compare; FR-100's internal failures (the kernel
   `CheckedInvariant` and a `CallFailure::Fault`) among them.
6. With feature `quire-extraction`: extract a Markdown document whose one
   selected fence holds the step 1 unit through `qsl_source::extract`, and
   run healthy-parent, violating-parent and missing-model with that
   extracted source as the unit; then healthy-parent with the same unit
   extracted from an `ix:formal` fence.
7. The step 1 unit with FR-109-AC-7's `Ratio` clause, over healthy-parent
   selecting `Ratio`.

Tag the tests `#[trace("TC-468", "FR-109-AC-n")]`.

## Expected Results

- Step 1: `evaluate`, `success`, `truth: true`, exit 0, with the compiled
  `package_id`; then `violation`, `truth: false`, exit 10.
- Step 2: `compile`, `refusal`, `missing_import`/`missing-selection`, exit 20,
  no `package_id`; `compile`, `stale_dependency`, naming both
  identities; `select`, `missing_declaration`/`missing-name`, twice.
- Step 3: `admit`, `refusal`, `dangling_reference`, exit 20; `admit`,
  `incomplete`, `incomplete_population`, exit 22; `evaluate`, `incomplete`,
  `{"kind": "incomplete", "limit": "work_units"}`, exit 22. None has `truth`.
- Step 4: `violation`, `truth: false`; `success`, `truth: true`; `admit`,
  `invalid_runtime_input`/`wrong-role-mapping`; `admit`, FR-100's
  unknown-parameter refusal naming `c`; `select`, `ill_typed`/
  `type-mismatch`, with an uncharged meter.
- Step 5: equal reports including usage; every compared outcome gives the same `outcome`
  member and exit status in both mappings; `CheckedInvariant` and the
  `CallFailure::Fault` each report stage `evaluate`, category
  `internal-failure`, the fault's stage and invariant (for
  `CheckedInvariant`, `S6a` and `checked-program-invariant`), no `outcome`
  member, and FR-100's internal-failure exit status.
- Step 6: `success`, exit 0, the `package_id` of compiling the extracted
  body alone; `violation`, exit 10; `compile`, `missing_import`/
  `missing-selection`; `compile`, `refusal`, `unknown_language`, exit 20,
  no `package_id`.
- Step 7: `evaluate`, `violation`, `UndefinedEvaluation{cause:
  division-by-zero}`, no `truth`, exit 10.

## Status

Planned.
