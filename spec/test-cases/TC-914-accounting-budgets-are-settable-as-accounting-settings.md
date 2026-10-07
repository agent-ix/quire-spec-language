---
id: TC-914
title: "Accounting budgets are settable as accounting.<counter> and a reached budget names its setting"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: verifies
---
# TC-914: Accounting budgets are settable as accounting.<counter> and a reached budget names its setting

## Description

Verify that the settings operation sets each of the ten
`quire.value.accounting/v1` counters by its `accounting.<counter>` name
under the same operand grammar as the stage settings, that a reached budget
settles `Incomplete` naming its setting, that a replay request's
`stage_limits` refuses an accounting setting, and that a native-run/1
request's `call.accounting` object reaches the call as its accounting
limits.

Scope: FR-255-AC-7, FR-255-AC-8, FR-255-AC-9, FR-255-AC-10, FR-100-AC-12.

## Test Procedure

1. Call the settings operation with `accounting.work_units=0` and call
   `seven` from `tests/fixtures/spine-compile.native` under the accounting
   limits it returns; serialize the outcome as `quire-outcome/1`. Repeat
   with `accounting.work_units=1000000`.
2. For each row of FR-255's accounting table, call the settings operation
   with `<setting>=123456789`. Call it with no operand, and with
   `accounting.work_units=18446744073709551615`.
3. Call the settings operation with `accounting.depth=1`, with
   `accounting=1`, with `accounting.work_units=ten`, with
   `accounting.work_units=-1`, with
   `accounting.work_units=18446744073709551616`, and with
   `accounting.work_units=5` and `accounting.work_units=6` together.
4. Decode a replay request whose `stage_limits` holds the entry
   `accounting.work_units`.
5. Run the `1-draft` fixture request for `seven` with `call.accounting`
   naming all ten counters at the values 1 to 10 in accounting-table order,
   with `{"work_units": 7}`, with `{}`, and with no `accounting`, and read
   the accounting limits of the `Call` handed to `qsl_replay::spine::run`.

Tag the tests `#[trace("TC-914", "FR-255-AC-7")]`, `#[trace("TC-914", "FR-255-AC-8")]`, `#[trace("TC-914", "FR-255-AC-9")]`, `#[trace("TC-914", "FR-255-AC-10")]`, `#[trace("TC-914", "FR-100-AC-12")]`.

## Expected Results

- Step 1: with `accounting.work_units=0` the call settles `Incomplete`
  naming counter `work_units`, bound 0, count 1 and setting
  `accounting.work_units`, and the `result` is `{"kind": "incomplete",
  "limit": {"kind": "work_units", "bound": "0", "counter": "1", "field":
  "work_units", "setting": "accounting.work_units"}}`; with
  `accounting.work_units=1000000` it completes with 7.
- Step 2: each operand sets its counter to 123456789 and leaves the other
  nine at their defaults (`work_units` 1000000, the others `u64::MAX`); no
  operand gives the defaults; the `u64::MAX` operand sets `work_units` to
  `u64::MAX`.
- Step 3: each returns a `UsageRefusal` naming its operand: unknown setting
  for `accounting.depth=1` and `accounting=1`; not an integer for `ten`,
  `-1` and `18446744073709551616`; repeated for `accounting.work_units=6`.
  No stage runs.
- Step 4: request decode refuses, naming `accounting.work_units`.
- Step 5: the ten-counter object gives `ScalarLimits` holding exactly 1 to
  10; `{"work_units": 7}` gives `work_units` 7 and the others `u64::MAX`;
  `{}` and no `accounting` give `work_units` 1000000 and the others
  `u64::MAX`.
