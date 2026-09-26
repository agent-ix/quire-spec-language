---
id: TC-452
title: "The spine run entry is qsl_replay::spine::run and agrees with the CLI"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: verifies
---
# TC-452: The spine run entry is qsl_replay::spine::run and agrees with the CLI

## Description

Verify that the library entry `qsl_replay::spine::run` returns the same
`package_id`, outcome category, value, code and parameter the CLI renders,
and that the root crate reaches it without a `qsl-eval` dependency. This
catches CLI-only run behavior (ADR-011 §5: no behavior is reachable only
through the CLI) and a root-crate edge to layer 5.

Scope: FR-100-AC-7.

## Test Procedure

1. In `qsl-replay`, call `qsl_replay::spine::run` directly over the inputs
   of TC-450 step 1 and TC-451 steps 1, 2, 3, 5 and 7, with the default
   spine stage limits, an empty package input and an empty dependency input.
2. Run TC-390's dependency check
   (`tests/it/family_outcome_layering.rs`) after the CLI change.

Tag the tests `#[trace("TC-452", "FR-100-AC-7")]`.

## Expected Results

- Step 1: each call returns the `package_id`, outcome category, value,
  catalog code and parameter position or name that the CLI renders for the
  same input in TC-450 and TC-451.
- Step 2: the root crate names `qsl-eval` in no normal, dev or build
  dependency table.

## Status

Specified under QSL-271. Not run.
