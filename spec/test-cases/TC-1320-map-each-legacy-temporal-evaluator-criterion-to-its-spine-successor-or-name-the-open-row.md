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

Verify FR-525-AC-1 to FR-525-AC-5. The check reads the FR-525 tables and the spec tree and the test tags; it is a spec-and-tag audit, not a behavior test.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | the criterion tables of FR-525 against the lists of FR-043, FR-044, FR-045, FR-052, FR-053 and NFR-008 | every listed criterion has a row with a successor criterion or an OPEN row |
| 2 | 2 | every successor criterion id in the tables, resolved with `quire trace` | each id exists in the spec |
| 3 | 3 | the Open rows table against the criterion tables | each OPEN id appears in a criterion row; no row reads RETIRE |
| 4 | 4 | the `#[trace]` tags of the spine tests at the deleting change | each PORT row's successor id is tagged; no legacy TC-122, TC-123, TC-125, TC-144, TC-149, TC-1277 to TC-1279 is cited as evidence |
| 5 | 5 | the deleting change's file list against the PORT and OPEN rows | no `src/temporal` file is deleted while a PORT row is untagged or an OPEN row has no ruling |

Run the check with the other FR-499 gates; no `#[trace]` tag names TC-1320 because the check is not a Rust test.

## Expected Results

Each row's Expected column.
