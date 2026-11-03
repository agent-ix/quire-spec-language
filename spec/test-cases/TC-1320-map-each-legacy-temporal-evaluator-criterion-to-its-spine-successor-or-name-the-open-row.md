---
id: TC-1320
title: "Map each legacy temporal evaluator criterion to its spine successor or name the open row"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-525
    type: verifies
---
# TC-1320: Map each legacy temporal evaluator criterion to its spine successor or name the open row

## Description

Verify FR-525-AC-1 to FR-525-AC-3. The check reads the FR-525 tables and the spec tree; it is a spec audit, not a behavior test.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | the criterion tables of FR-525 against the lists of FR-043, FR-044, FR-045, FR-052, FR-053 and NFR-008 | every listed criterion has a row with a successor criterion or a RETIRE row |
| 2 | 2 | every successor criterion id in the tables, resolved with `quire trace` | each id exists in the spec |
| 3 | 3 | every RETIRE row | cites a structural reason or a ruling |

Run the check with the other FR-499 gates; no `#[trace]` tag names TC-1320 because the check is not a Rust test.

## Expected Results

Each row's Expected column.
