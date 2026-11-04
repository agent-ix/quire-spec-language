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

Verify FR-526-AC-1 to FR-526-AC-19. Results `r1`, `r2`, `r3` are evaluated from **R**'s event-position clause with trigger `t1`, passing the ledger from call to call.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | an `Original` request | `lineage` 0, relation `Original`; the ledger holds its entry |
| 2 | 2 | `Superseding(r1)` with changed inputs, `r1`'s ledger, same package, clause, trigger, correspondence | `lineage` 1, relation `Superseding` naming `r1.result_identity` |
| 3 | 3 | `r1` after row 2; the returned ledger | `r1` byte-equal; the new entry is the latest |
| 4 | 4 | `Invalidating(r1)` under row 2's conditions | relation `Invalidating`, same `lineage` |
| 5 | 5 | row 4's inputs under `Superseding(r1)` | equal truth, settlement and support |
| 6 | 6 | a request with `r1`'s `input_identity`, under both relations | `Err(CallFailure::Admission)`, cause `unchanged-input`, empty charge log |
| 7 | 7 | `Superseding` with no `ledger` | `Err(CallFailure::Admission)`, cause `no-ledger` |
| 8 | 8 | a ledger holding `r1` and `r2`; `Superseding(r1)`; `Superseding(r2)` | `predecessor-not-latest`; accepted |
| 9 | 9 | another clause naming `r1`; another package naming `r1` | `predecessor-unknown` each |
| 10 | 10 | trigger differing in one byte, naming `r1` | `predecessor-unknown` |
| 11 | 11 | `correspondence` differing, naming `r1` | `correspondence-changed` |
| 12 | 12 | a three-correction chain with the `lineage` ceiling 2, then 3 | the third correction an `Ok` `Incomplete` result naming `lineage`, `Unavailable`, no truth, no ledger entry for it; completes |
| 13 | 13 | inspect the lineage sum | checked `u64` arithmetic, no wrapping or saturating form |
| 14 | 14 | three corrections `r1`, `r2`, `r3` | `lineage` 1, 2, 3 naming direct predecessors; `r1`, `r2` unchanged |
| 15 | 15 | an `Original` request over a ledger holding its key | `lineage` 0; entry replaced |
| 16 | 16 | one request, relation and ledger twice | equal results and ledgers, charge log included |
| 17 | 17 | constructing an `EvidenceLedger` entry outside the evaluator (compile-fail) | does not compile |
| 18 | 18 | an empty trigger identity | refused at construction |
| 19 | 19 | a non-empty identity through a three-correction chain | echoed byte for byte by every result |

Tag the test `#[trace("FR-526-AC-1", ..., "FR-526-AC-19", "TC-1321")]`; rows 6 to 11 also carry FR-052-AC-12 and FR-516-AC-13; AC-13 is Inspection and carries no tag.

## Expected Results

Each row's Expected column.
