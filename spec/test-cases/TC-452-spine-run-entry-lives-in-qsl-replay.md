---
id: TC-452
title: "The spine run entry is qsl_replay::spine::run, agrees with the CLI, and maps every outcome"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: verifies
---
# TC-452: The spine run entry is qsl_replay::spine::run, agrees with the CLI, and maps every outcome

## Description

Verify that the library entry `qsl_replay::spine::run` returns the same
result the CLI renders, that neither the root crate nor `qsl_replay`'s
public spine surface reaches `qsl-eval`, and that the one outcome mapping
converts every S6a outcome into its `outcome` member and exit status. This
catches CLI-only run behavior (ADR-011 §5: no behavior is reachable only
through the CLI), a layer-5 type leaking through `qsl_replay`, and an
outcome kind rendered with the wrong spelling or exit status.

Scope: FR-100-AC-7 to FR-100-AC-9.

## Test Procedure

1. In `qsl-replay`, call `qsl_replay::spine::run` directly over the inputs
   of TC-450 step 1 and TC-451 steps 1, 2, 3, 5, 6 and 7, with the default
   spine stage limits, an empty package input and an empty dependency input.
2. Run TC-390's dependency check
   (`tests/it/family_outcome_layering.rs`) after the CLI change.
3. Run the `tools/arch-lint` API-surface check over `qsl_replay`: every
   public item of `qsl_replay::spine`, and every `qsl_replay` re-export, is
   scanned for a `qsl_eval` path. Then run it over a probe that adds
   `pub use qsl_eval::value::Evaluation;` to `qsl_replay`.
4. Construct each S6a outcome below and convert it with the outcome mapping
   (the `qsl_replay` conversion `spine::run` applies, then the root crate's
   renderer and exit mapping): `Outcome::Completed` of `true`, `false`, `0`,
   `-17` and `2^70`; `Outcome::Refused` of each of the nine kernel
   refusals; `Outcome::Undefined` of each of the four kernel reasons;
   `Outcome::Incomplete` at `work_units`; `FamilyResult::Refused` of an
   `invalid_runtime_input` cause and an `unsupported_construct` cause; and
   `FamilyResult::Undefined` with reason `precondition-false` and
   `absent-key`.

Tag the tests `#[trace("TC-452", "FR-100-AC-7")]` (steps 1 and 2),
`#[trace("TC-452", "FR-100-AC-8")]` (step 3) and
`#[trace("TC-452", "FR-100-AC-9")]` (step 4).

## Expected Results

- Step 1: each call returns the `package_id`, outcome category, value,
  catalog code, reason, counter, and parameter name or position that the
  CLI renders for the same input in TC-450 and TC-451.
- Step 2: the root crate names `qsl-eval` in no normal, dev or build
  dependency table.
- Step 3: the check passes over `qsl_replay`, and fails over the probe,
  naming the re-export.
- Step 4: `completed` with `{"kind": "boolean", "value": true}`,
  `{"kind": "boolean", "value": false}`, and `{"kind": "integer",
  "decimal": ...}` of `"0"`, `"-17"` and `"1180591620717411303424"`, exit 0;
  `refused` with `code` `invalid_runtime_input` and each kernel refusal's
  tabled `cause`, exit 20; `undefined` with each kernel reason's tabled
  spelling, exit 20; `incomplete` with `limit` `work_units`, exit 22;
  `refused` with `code` `invalid_runtime_input` and no `cause`, exit 20, and
  with `code` `unsupported_construct`, exit 21; `undefined` with `reason`
  `precondition-false` and `absent-key`, exit 20.

## Status

Specified under QSL-271. Not run.
