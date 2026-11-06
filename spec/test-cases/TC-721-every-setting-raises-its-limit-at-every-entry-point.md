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

Scope: FR-255-AC-4, FR-255-AC-5, FR-255-AC-6 and FR-255-AC-15.

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
4. Read the native identity-join fixture under no configured I2 limits, then
   set `i2.work_units` and `i2.native_join_storage` independently through the
   library limits, replay `stage_limits`, and the settings operation. Set each
   once to `0` and once to `u64::MAX`; use the settings operation and typed
   limits to confirm that each value changes only its matching field. Measure
   the fixture's native join charges W (work) and S (storage), then read with
   (W, S), (W − 1, S), (W, S − 1), and (W, 0). Retain the IR error and inspect
   the shared budget's first unpaid charge for each refused read.

Tag the tests `#[trace("TC-721", "FR-255-AC-4")]`, `#[trace("TC-721", "FR-255-AC-5")]`, `#[trace("TC-721", "FR-255-AC-6")]`, `#[trace("TC-721", "FR-255-AC-15")]`.

## Expected Results

- Step 1: every rerun passes the limit it reached at the default.
- Step 2: each returns a usage refusal naming the offending operand, and no
  stage runs.
- Step 3: each stage ran at FR-255's defaults, and the recorded effective
  limits equal them.
- Step 4: defaults are `i2.work_units=1000000` and
  `i2.native_join_storage=16777216`; every override changes only its matching
  field, and zero and `u64::MAX` are accepted. `(W, S)` admits the join;
  `(W − 1, S)` returns the native typed work refusal with code
  `exhausted_work`; `(W, S − 1)` and `(W, 0)` return the native typed storage
  refusal with code `exhausted_storage`. The errors have no separate public
  `cause` field, and `NativeJoinBudget::first_unpaid()` gives the first denied
  charge kind and amount.
