---
id: TC-701
title: "S3 checks task sets and routes schedulability claims to EN-7"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-246
    type: verifies
---
# TC-701: S3 checks task sets and routes schedulability claims to EN-7

## Description

Verify task-set checking, refusals, premises in the obligation identity, and routing under the `schedulability` capability kind.

Scope: FR-246-AC-1 to FR-246-AC-3.

## Test Procedure

Fixtures: ADR-026 §12's `Ctl`; its variant with third WCET 6; the refusal fixtures of FR-246-AC-2.

1. Check `Ctl` and route `CtlFp` (`schedulable CtlFp on Ctl under fixed_priority;`) and `CtlEdf` (`under edf`); compare obligation identities with the WCET-6 variant.
2. Check each refusal fixture of FR-246-AC-2: the empty set, the non-positive period, deadline and WCET, the negative jitter and blocking, the zero minimum inter-arrival time, the budget-order violation, the `LO` task with dual budgets, the missing priority and the priority clash under `fixed-priority`, blocking and jitter under `edf`, the missing priority and the priority clash under `amc` and under a `response` claim, a deadline above the period under `amc`, the duplicate task name, the unknown `response` task and the deadline of `3 m`; then the priority clash and the missing priority under `edf`.
3. Check `Ctl` in `ms`, then check it with task `a`'s period written `4000 us` and `R`'s bound `12 ms`; settle `CtlFp` for each. Check the same `us`-based task set with tasks listed in reverse name order and verify its unit, converted values and identity are unchanged. Check the two several-defect sets of FR-246-AC-3 and `Empty` with its `response` claim, and read the full ordered list of refusals.

Tag the tests `#[trace("TC-701", "FR-246-AC-n")]`.

## Expected Results

- Step 1: checks; both route to EN-7 under `schedulability`; identities differ.
- Step 2: each refuses at its span with its stated code, naming the task (the task set for the empty one, both tasks for a clash); the `edf` claims with a priority clash or a missing priority check.
- Step 3: units `ms` and `us`; the bound `12000` in `us`; fixpoints `(1, 3, 12)` and `(1000, 3000, 12000)`; the reverse-source-order `us` task set has the same unit, checked values and claim identity; the four `invalid-task-set` refusals in the stated order, `duplicate-declaration` at the second `a` then `invalid-task-set` at `z`; for `Empty`, no unit, `invalid-task-set` naming `Empty` then `missing-name` at `c`, the bound kept as `12 ms`.
