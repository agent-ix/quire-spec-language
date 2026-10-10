---
id: TC-951
title: "Accounting budgets are settable by counter name and a reached budget names its setting"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: verifies
---
# TC-951: Accounting budgets are settable by counter name and a reached budget names its setting

## Description

Verify that the settings operation sets each of the ten
`quire.value.accounting/v1` counters by its bare counter name under the
same operand grammar as the stage settings, that a reached budget settles
`Incomplete` naming its setting, that a replay request's `stage_limits`
entry of a counter name sets that budget, and that a native-run/1
request's `call.accounting` object reaches the call as its accounting
limits.

Scope: FR-255-AC-7, FR-255-AC-8, FR-255-AC-9, FR-255-AC-10, FR-255-AC-11, FR-100-AC-13.

## Test Procedure

1. Call the settings operation with `work_units=0` and call `seven` from
   `tests/fixtures/spine-compile.native` under the accounting limits it
   returns; serialize the outcome as `quire-outcome/1`. Repeat with
   `work_units=1000000`.
2. For each row of FR-255's accounting table, call the settings operation
   with `<setting>=123456789`. Call it with no operand, and with
   `work_units=18446744073709551615`.
3. Call the settings operation with `depth=1`, with
   `accounting.work_units=1`, with `work_units=ten`, with `work_units=-1`,
   with `work_units=18446744073709551616`, and with `work_units=5` and
   `work_units=6` together.
4. Decode a replay request whose `stage_limits` holds the entry
   `work_units` at a bound `B` that differs from its accounting limits'
   `work_units`, and read the accounting limits its call runs under. Replay
   FR-098-AC-9's counterexample with the entry `work_units` one below the
   argument's node count, then with it raised to fit.
5. Build two function-level value-parity requests for the same claim with
   the same effective accounting limits: one with every counter in
   `accounting_limits` and no accounting entry in `stage_limits`; the other
   with `work_units` and `value_occurrences` given only as `stage_limits`
   entries, over `accounting_limits` holding other values for those two.
   Replay each once with limits that fit and once with `work_units` too
   small.
6. Run the `1-draft` fixture request for `seven` with `call.accounting`
   naming all ten counters at the values 1 to 10 in accounting-table order,
   with `{"work_units": 7}`, with `{}`, and with no `accounting`, and read
   the accounting limits of the `Call` handed to `qsl_replay::spine::run`.

Tag the tests `#[trace("TC-951", "FR-255-AC-7")]`, `#[trace("TC-951", "FR-255-AC-8")]`, `#[trace("TC-951", "FR-255-AC-9")]`, `#[trace("TC-951", "FR-255-AC-10")]`, `#[trace("TC-951", "FR-255-AC-11")]`, `#[trace("TC-951", "FR-100-AC-13")]`.

## Expected Results

- Step 1: with `work_units=0` the call settles `Incomplete` naming counter
  `work_units`, bound 0, count 1 and setting `work_units`, and the `result`
  is `{"kind": "incomplete", "limit": {"kind": "work_units", "bound": "0",
  "counter": "1", "field": "work_units", "setting": "work_units"}}`; with
  `work_units=1000000` it completes with 7.
- Step 2: each operand sets its counter to 123456789 and leaves the other
  nine at their defaults (`work_units` 1000000, the others `u64::MAX`); no
  operand gives the defaults; the `u64::MAX` operand sets `work_units` to
  `u64::MAX`.
- Step 3: each returns a `UsageRefusal` naming its operand: unknown setting
  for `depth=1` and `accounting.work_units=1`; not an integer for `ten`,
  `-1` and `18446744073709551616`; repeated for `work_units=6`. No stage
  runs.
- Step 4: the request decodes; its call runs with `work_units` at `B` and
  every other counter at the request's accounting-limits value. The
  counterexample settles `inconclusive`, `NoValue`, naming `work_units`,
  and replays once the entry fits.
- Step 5: both requests give equal `claim()` identities, the same
  settlement and the same charges, in each run.
- Step 6: the ten-counter object gives `ScalarLimits` holding exactly 1 to
  10; `{"work_units": 7}` gives `work_units` 7 and the others `u64::MAX`;
  `{}` and no `accounting` give `work_units` 1000000 and the others
  `u64::MAX`.
