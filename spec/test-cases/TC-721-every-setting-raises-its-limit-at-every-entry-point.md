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

Scope: FR-255-AC-4, FR-255-AC-5, FR-255-AC-6 and FR-255-AC-15 to FR-255-AC-20.

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
4. Read the native identity-join fixture with explicit limits and no outer
   I2 configuration (FR-255-AC-15), then with the outer configuration supplying
   the table defaults.
5. Set `i2.work_units` and `i2.native_join_storage` independently through the
   library limits, replay `stage_limits`, the settings operation and the FR-517
   request's `native_join` (FR-255-AC-16).
6. Set each once to `0` and once to `u64::MAX` (FR-255-AC-17).
7. Measure the fixture's native join charges W (work) and S (storage), then read
   with (W, S), (W - 1, S) and (W, S - 1) (FR-255-AC-18).
8. Retain each refused read's `stage_limit_exceeded`-class refusal, the IR error
   and the shared budget's first unpaid charge (FR-255-AC-19).
9. Inspect the public declaration of `ExactTypeProjectionError` or compile a
   public API probe against it; enumerate its public fields and check whether a
   field named `cause` exists.

10. Set `emit.output_bytes` through the emit limits' builder, a replay
   request's `stage_limits` and the settings operation; read the outer
   configuration's default; set `0` and `u64::MAX` (FR-255-AC-20).

Tag the tests `#[trace("TC-721", "FR-255-AC-20")]`, `#[trace("TC-721", "FR-255-AC-4")]`, `#[trace("TC-721", "FR-255-AC-5")]`, `#[trace("TC-721", "FR-255-AC-6")]`, `#[trace("TC-721", "FR-255-AC-15")]`, `#[trace("TC-721", "FR-255-AC-16")]`, `#[trace("TC-721", "FR-255-AC-17")]`, `#[trace("TC-721", "FR-255-AC-18")]`, `#[trace("TC-721", "FR-255-AC-19")]`.

## Expected Results

- Step 1: every rerun passes the limit it reached at the default.
- Step 2: each returns a usage refusal naming the offending operand, and no
  stage runs.
- Step 3: each stage ran at FR-255's defaults, and the recorded effective
  limits equal them.
- Step 4: with explicit limits the fixture reads under them; with the outer
  configuration the defaults are `i2.work_units=1000000` and
  `i2.native_join_storage=16777216`; a limits value with no storage value is
  not constructible.
- Step 5: every override changes only its matching field.
- Step 6: zero and `u64::MAX` are accepted; zero storage refuses the nonempty
  join.
- Step 7: `(W, S)` admits the join; `(W - 1, S)` refuses naming
  `i2.work_units`; `(W, S - 1)` refuses naming `i2.native_join_storage`.
- Step 8: each refusal is `stage_limit_exceeded`-class with `kind`, `bound`,
  `actual` and `setting`; the IR error has code `exhausted_work` or
  `exhausted_storage` and no separate public `cause` field, and
  `NativeJoinBudget::first_unpaid()` gives the first denied charge kind and
  amount.
- Step 9: `ExactTypeProjectionError` exposes `code` and has no public field
  named `cause`.
- Step 10: each entry point changes only the output-byte bound; the outer
  configuration's default is 16777216; zero and `u64::MAX` are accepted;
  `EmitLimits` has no `Default`.
