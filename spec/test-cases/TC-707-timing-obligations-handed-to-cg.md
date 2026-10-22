---
id: TC-707
title: "QSL writes monitor-agreement, tick-arithmetic and timestamp-contract obligations, and disposes elapsed-time obligations unsupported"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-252
    type: verifies
---
# TC-707: QSL writes monitor-agreement, tick-arithmetic and timestamp-contract obligations, and disposes elapsed-time obligations unsupported

## Description

Verify the timing obligations QSL writes for CG and the unsupported disposition for obligations outside the code model.

Scope: FR-252-AC-1 to FR-252-AC-4.

## Test Procedure

Fixtures: the plan of TC-706; the contracts of FR-252-AC-2.

1. Write obligations for the plan with the default and with `agreement_horizon` 4.
2. Write obligations for the timestamp contract with `timestamp_parameters` keying that contract to `{now, start}`, for the elapsed-time clause (with `handler` declared nowhere) and for the WCET clause.
3. Write obligations for the timestamp contract with `timestamp_parameters` empty, and for a contract that compares only `now` with a constant while `{now, start}` is named, and classify the clauses of FR-252-AC-3.
4. Check the timing forms against a unit that declares `wcet`, in non-left positions, and with a non-time right operand (FR-252-AC-4).

Tag the tests `#[trace("TC-707", "FR-252-AC-n")]`.

## Expected Results

- Step 1: `MonitorAgreement` with 8 events and `TickArithmetic` with the plan's width, buffers and rate; 4 events.
- Step 2: `TimestampContract`; `unsupported`, `unsupported-requested-capability` naming elapsed real time; the same naming WCET.
- Step 3: no `TimestampContract` for either contract; the `elapsed_wall_time` and `wcet` comparisons are timing clauses and the `u32` comparison is not.
- Step 4: the left-operand `wcet(f) < 2 ms` is a timing clause despite the declaration; the two misplaced forms are `missing-name`; `wcet(f) < 3` is `ill_typed`/`type-mismatch`.
