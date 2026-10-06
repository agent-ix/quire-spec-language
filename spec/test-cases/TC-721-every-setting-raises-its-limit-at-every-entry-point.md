---
id: TC-721
title: "Every setting raises its limit through the library, the replay request and the CLI"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: verifies
---
# TC-721: Every setting raises its limit through the library, the replay request and the CLI

## Description

Verify that each setting name raises its limit at each entry point, that
the settings operation refuses a malformed `--limit` operand before any
stage runs, and that an
unconfigured limit takes its default.

Scope: FR-255-AC-4, FR-255-AC-5, FR-255-AC-6.

## Test Procedure

1. For each row of FR-255's setting table, take TC-720 step 1's input at
   the row's default bound and run it again with the row's setting raised
   to fit it: through the limits type's builder method; through a replay
   request whose `stage_limits` carries that entry, for each row whose stage
   a replay runs; and through the settings operation given the operand
   `<name>=<value>`.
2. Call the settings operation with `s9.nodes=1`, with `s3.nodes=ten`, with
   `s3.nodes=-1`, and with `s3.nodes=5` and `s3.nodes=6` together.
3. Compile a unit with no limit configured and read the checked package's
   effective limits.

Tag the tests `#[trace("TC-721", "FR-255-AC-4")]`, `#[trace("TC-721", "FR-255-AC-5")]`, `#[trace("TC-721", "FR-255-AC-6")]`.

## Expected Results

- Step 1: every rerun passes the limit it reached at the default.
- Step 2: each returns a usage refusal naming the offending operand, and no
  stage runs.
- Step 3: each stage ran at FR-255's defaults, and the recorded effective
  limits equal them.

## Status

Implemented. Step 1 is stage-driven for `s1.tokens`, `s3.nodes`, `s3.work_units`, `identity.input_bytes` and the `library.*`, `admission.*` and dispatch limits named in FR-255's Status, and checked at the entry points for the other rows; steps 2 and 3 are implemented.
