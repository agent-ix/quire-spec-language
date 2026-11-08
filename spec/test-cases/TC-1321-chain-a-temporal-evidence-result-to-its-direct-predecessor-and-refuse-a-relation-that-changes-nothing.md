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

Verify FR-526-AC-1 to FR-526-AC-30. Results `r1`, `r2`, `r3` are evaluated from **R**'s event-position clause with trigger `t1`, passing the ledger from call to call. Row 29 verifies FR-516's `valuation-atom-unknown`, the successor of FR-043-AC-16's `OperandNotInFormula`. Rows 20 to 28 verify the subject, support, predecessor-authority and lineage-limit criteria that succeed FR-052-AC-15, AC-16, AC-19 and AC-20. Expected values come from the package, the request and the FR-501 decision-support rule, never from the evaluator's own output.

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
| 20 | 20 | an `Original` call over each of **R**'s three packages; one call stopped one below a counter's usage | `subject` equals the package identity, package version, clause, occurrence and occurrence source-map entry read from the package; `correspondence` equal member for member; the stopped result carries the same `subject` |
| 21 | 21 | the FR-516-AC-32 twins, equal positions, valuations and correspondence, each occurrence's own binding, occurrence 3 then 7 | subjects agree in package identity, version and clause and differ in `occurrence` |
| 22 | 22 | a test-authored event-position unit with `on origin eventually[0,1] (holds(a) and holds(b))`, `a` and `b` true at 0 and false at 1, `Complete { through: 1 }`, `Closed` | `True`; `support` exactly {(0, `a`), (0, `b`)}, each naming position 0's `observation` |
| 23 | 23 | row 22 with `b` withdrawn at position 0; a fixed-sample clause with a silent-sample support pair; every result of this test case | no entry for a pair with no admitted valuation; every entry names a request position and an atom with an admitted valuation there |
| 24 | 24 | `r1` and `r1'` (one valuation differs, truths differ); `Superseding(r1)` with `r1'`'s ledger; `Superseding(r1')` with changed inputs and that ledger | `predecessor-not-latest`; accepted |
| 25 | 25 | the twins, `r3` for occurrence 3; `Superseding(r3)` selecting occurrence 7 with `r3`'s ledger and changed inputs | `predecessor-unknown`; `r3` and its ledger unchanged |
| 26 | 26 | `Superseding(r1)` with a clone of `r1`'s ledger, then with that ledger moved out of `r1`; compile-fail setting an entry's identity, input identity, correspondence or lineage | results equal to the request with the ledger itself; does not compile (the `compile_fail` doc tests on `ResultEntry`, which carry no trace tag) |
| 27 | 27 | `Superseding(r1)` with changed inputs, `lineage` ceiling 1, then 0 | completes; then `Ok` `Incomplete` naming `lineage` with `value` 0 and setting `temporal_evidence.lineage`, result `lineage` 1, relation naming `r1`, `Unavailable`, no truth, `usage.instances` and `usage.positions` 0, ledger `==` the request's |
| 28 | 28 | a meter charged `U` by one call, then row 27's stopped call, then a third call | meter total `U` plus the stopped call's charge log after the stop, never below `U`; the third call continues from it |
| 29 | 29 | row 1's request with a `valuations` entry for an atom index the formula does not have (such as 99) at position 1; the same at position 0; the same in an `instance_valuations` map; row 1's request unchanged | `Err(CallFailure::Admission)`, cause `valuation-atom-unknown`, `invalid_runtime_input`/`invalid-value` at member `valuations` with `atom` and `position` `1`; `position` `0`; member `instance_valuations`; each an empty charge log; the unchanged request evaluates |

| 30 | 30 | a real emitted/read checked-package original and changed-input correction; repeated carrier borrows; mutation of a cloned snapshot; compile-fail construction, mutation and snapshot substitution | original truth, support, settlement, subject and ledger remain unchanged; correction changes actual outputs and latest ledger, names original identity and advances lineage; borrows make no meter charges; all three compile-fail examples on `EvaluatedEvidence` fail |

Tag the test `#[trace("FR-526-AC-1", ..., "FR-526-AC-29", "TC-1321")]`; rows 6 to 11 and 25 also carry FR-052-AC-12 and FR-516-AC-13; AC-13 is Inspection and carries no tag.

## Expected Results

Each row's Expected column.
