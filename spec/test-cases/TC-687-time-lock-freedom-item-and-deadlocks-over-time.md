---
id: TC-687
title: "The request carries one time-lock-freedom item per timed subject, and deadlocks, fairness and vacuity read over time"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-232
    type: verifies
---
# TC-687: The request carries one time-lock-freedom item per timed subject, and deadlocks, fairness and vacuity read over time

## Description

Verify the derived time-lock-freedom item, local and non-local time-locks, deadlocks over time, weak fairness over time and the vacuity rule.

Scope: FR-232-AC-1 to FR-232-AC-4.

## Test Procedure

Fixtures: ADR-026 §11's `Rpc` unit with `T = 3 ms` and `T = 4 ms`, universe `{c}`; the strict-guard `Rpc` variant; the untimed `Counter` unit; the `Stall` and `Serve` models of FR-232-AC-3 and AC-4.

1. Write requests for `NoLateReply` and `Settles` over one `Rpc` subject; for one claim over `Rpc` and one over `Counter`; for `Rpc` with `terminal any`.
2. Classify `(Waiting, inflight, x = 3)` in the strict-guard variant and `(Replied, x = 2)` in `Rpc`.
3. Request `always holds(true)` under the timed profile over `Stall`, and classify `(s0, x = 1)`.
4. Check fairness of the `idle` lasso over `Serve` with guard `x >= 1 ms` and with guard `x <= 1 ms`.

Tag the tests `#[trace("TC-687", "FR-232-AC-n")]`.

## Expected Results

- Step 1: one deadlock-freedom and one time-lock-freedom item; one time-lock-freedom item, for `Rpc`; still present; its identity differs from the deadlock-freedom item's.
- Step 2: local time-lock, `deadlocked` false; quiescent, not terminal.
- Step 3: `inconclusive`, `NoAdmittedBehaviour`; a non-local time-lock.
- Step 4: unfair; fair.
