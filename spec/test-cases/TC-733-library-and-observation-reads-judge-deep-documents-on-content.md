---
id: TC-733
title: "Observation reads judge deep documents on their content"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-261
    type: verifies
---
# TC-733: Observation reads judge deep documents on their content

## Description

Verify that the observation document reader, FR-261's one D-4.10 site,
reads through the shared reader and keeps malformed input apart from limits
and from wrong shapes.

Scope: FR-261-AC-2, FR-261-AC-3.

## Test Procedure

Run every step on a thread spawned with a 512 KiB stack unless the step says otherwise.

1. **RETIRED (the library package identity read is deleted, FR-261-AC-1).**
2. Run observation admission on a snapshot whose field value is
   nested 100,000 deep, within `observation.input_bytes`.
3. Read a snapshot whose population field holds a value nested 100,000
   levels deep that the field's declared type does not admit. First with
   `observation.input_bytes` and `observation.values` raised to fit; then
   with `observation.values` at 50,000, which the nested value alone
   crosses.

Tag the tests `#[trace("TC-733", "FR-261-AC-2")]`, `#[trace("TC-733", "FR-261-AC-3")]`.

## Expected Results

- Step 1: **RETIRED.**
- Step 2: the document is read and admission proceeds to FR-106's later
  checks.
- Step 3: refused `invalid_runtime_input`/`wrong-value-kind` at the field;
  then refused `stage_limit_exceeded`/`node-count-exceeded` naming
  `observation.values`, bound 50,000 and the count reached, 50,001.
