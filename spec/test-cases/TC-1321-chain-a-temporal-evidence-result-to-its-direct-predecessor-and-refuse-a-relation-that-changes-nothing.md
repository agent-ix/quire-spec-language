---
id: TC-1321
title: "Chain a temporal evidence result to its direct predecessor and refuse a relation that names another obligation or changes nothing"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-526
    type: verifies
---
# TC-1321: Chain a temporal evidence result to its direct predecessor and refuse a relation that names another obligation or changes nothing

## Description

Verify FR-526-AC-1 to FR-526-AC-10. Results `r1`, `r2`, `r3` are evaluated from **R**'s event-position clause with one delivery `t1`.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | an `Original` request | `lineage` 0, relation `Original` |
| 2 | 2 | `Superseding(r1)` with changed inputs, same package, clause, trigger, correspondence | `lineage` 1, relation `Superseding` naming `r1.result_identity`; `r1` byte-equal |
| 3 | 3 | `Invalidating(r1)` under row 2's conditions | relation `Invalidating`, same `lineage`, truth, settlement and support equal to row 2's |
| 4 | 4 | a request with `r1`'s `input_identity`, under both relations | `Err(CallFailure::Admission)`, empty charge log |
| 5 | 5 | a predecessor of another clause; of the same clause in another package | `Err(CallFailure::Admission)`, empty charge log |
| 6 | 6 | a predecessor whose trigger differs in one byte; whose correspondence differs | `Err(CallFailure::Admission)` |
| 7 | 7 | a predecessor at the `lineage` ceiling; one below; at `u64::MAX` | `Incomplete` naming `lineage`; completes; `Incomplete` naming `lineage`, no wrap |
| 8 | 8 | three corrections `r1`, `r2`, `r3` | `lineage` 1, 2, 3 naming direct predecessors; `r1`, `r2` unchanged |
| 9 | 9 | one request and relation twice | equal results, charge log included |
| 10 | 10 | an empty trigger identity; a non-empty identity through a chain | refused at construction; byte-for-byte echoed |

Tag the test `#[trace("FR-526-AC-1", ..., "FR-526-AC-10", "TC-1321")]`; rows 4 to 6 also carry FR-052-AC-12 and FR-516-AC-6 where they are the same vectors.

## Expected Results

Each row's Expected column.
