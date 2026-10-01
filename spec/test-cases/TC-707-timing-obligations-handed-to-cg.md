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

Scope: FR-252-AC-1 to FR-252-AC-2.

## Test Procedure

Fixtures: the plan of TC-706; the contracts of FR-252-AC-2.

1. Write obligations for the plan with the default and with `kani_events` 4.
2. Write obligations for the timestamp contract, the elapsed-time clause and the WCET clause.

Tag the tests `#[trace("TC-707", "FR-252-AC-n")]`.

## Expected Results

- Step 1: `MonitorAgreement` with 8 events and `TickArithmetic` with the plan's width, buffers and rate; 4 events.
- Step 2: `TimestampContract`; `unsupported`, `unsupported-requested-capability` naming elapsed real time; the same naming WCET.
