---
id: TC-1315
title: "Evaluate state and compensation expressions through the checked-package evaluation entry points"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-520
    type: verifies
---
# TC-1315: Evaluate state and compensation expressions through the checked-package evaluation entry points

## Description

Verify the slot evaluation quire-protocol's compensation engine relies on.
Scope: FR-520-AC-1 to FR-520-AC-13. **R** is the package of FR-517.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | **R**: `trigger.guard` true then false; `retry.relation` true then false; `recovery.predicate` true then false | `Completed(true)` / `Completed(false)` each |
| 2 | 1 | a binder supplied `Unavailable`; a value of the wrong type; an unsupported construct; a meter one below measured usage | `Unavailable` with cause and location; `CallFailure::Admission`, nothing charged; the unsupported result (exit 21); `Evaluated(Incomplete)` naming the counter |
| 3 | 2 | `Full` registration capture from a payment record of total 10, twice; then retry and recovery with the captured 10 while the record says 12 | value 10 and equal charge logs; the later calls see 10 |
| 4 | 3 | target 10; receipts `[3,4]`, `[4,6]`, `[]`, `[5,6]` | `Partial` T and `Full` F; `Full` T and `Partial` F; both F; both F |
| 5 | 4 | `retry.relation` over attempts 1 and 2; read `attempts maximum` | a Boolean from the relation; the authored count |
| 6 | 5 | target 11; receipts `[3,5]` with target 10; the other relation's slot; a record with only a successful operation and an outcome `successful` | each changes the true result to false or to no result; no true from the record alone |
| 7 | 6 | recovery supply with an incomplete population reference; missing target; missing achieved state; `Unavailable` binder | `Admission`, `Unavailable` or `Incomplete`, never `Completed(true)` |
| 8 | 7 | three slot calls on one meter vs three on fresh meters; `retry.relation` at exact usage and one below | per counter, the sum for `work_units` and `result_units` and the maximum for the other eight; completed / `Incomplete` |
| 9 | 8 | `cargo tree` of qsl-eval and qsl-package evaluation path; run under a denied-syscall harness | no I/O dependency; completes |
| 10 | 9 | each slot of both compensations, in-process and read back | equal results and charge logs |
| 11 | 10 | search the root crate for a `state` module and for the four retired names in their `state::` form | none; the `evaluate_v2` of `src/temporal.rs` and the `ACCOUNTING_VERSION` of `src/linking/composed/binding_work.rs` remain |
| 12 | 11 | a `Meter` with per-binder attribution and `accounting_version` (IR-1280); a slot call over three binders, one `Unavailable`; the same call twice | per-binder entries for the binders read, none for the unread one; entries combined with unattributed equal the total; equal logs; `accounting_version` present; gated on IR-1280 |
| 13 | 12 | the `compile_fail` doctests of `ProtocolSlotEvaluation` and `Evaluation` for `as_bool` and `Into<bool>`, read by the tagged doctest-binding test | each block keeps its pinned error code and compiling control; neither compiles |
| 14 | 13 | search the evaluation crates' sources for `unwrap_or(false)` over an evaluation result | none |

Tag the test `#[trace("FR-520-AC-1", ..., "FR-520-AC-12", "TC-1315")]`; rows 1, 2, 8 also carry FR-390-AC-9 and AC-10.

## Expected Results

Each row's Expected column.
