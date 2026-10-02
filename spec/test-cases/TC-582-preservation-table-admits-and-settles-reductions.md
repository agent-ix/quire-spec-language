---
id: TC-582
title: "The preservation table admits or settles each selected reduction before expansion"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-159
    type: verifies
---
# TC-582: The preservation table admits or settles each selected reduction before expansion

## Description

Verify `ReductionNotPreserving` for position-counting forms under partial-order reduction, admission under symmetry, every table row, and which reduction a combined failure names.

Scope: FR-159-AC-1 to FR-159-AC-2.

## Test Procedure

Fixtures: ADR-021 §7.2's subject with `counters` annotated; the forms of FR-159-AC-2.

1. Request the TP-2 claim and the TP-3 claim with an interval, each with partial-order reduction and then with symmetry.
2. Call `preserves` for every form and fairness row of FR-159-AC-2; request TP-2 with symmetry and partial-order reduction.

Tag the tests `#[trace("TC-582", "FR-159-AC-n")]`.

## Expected Results

- Step 1: `ReductionNotPreserving{PartialOrder, TP-2}` and `{PartialOrder, TP-3}` with no state expanded; both admitted under symmetry.
- Step 2: each value as FR-159-AC-2 states; `ReductionNotPreserving{PartialOrder, TP-2}`.
