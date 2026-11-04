---
id: TC-1311
title: "Carry the temporal evidence request and result on the checked-package route"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-516
    type: verifies
---
# TC-1311: Carry the temporal evidence request and result on the checked-package route

## Description

Verify the typed request and result of the temporal evidence route. Scope:
FR-516-AC-1 to FR-516-AC-46. The numeric vectors are the QSpec FR-370 and
FR-043 cases and TC-144; they are referenced, never copied.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | compile-fail: set `truth`, `settlement`, `support`, `instance` on `EvidenceRequest` | does not compile; no document or QSL-computed digest member |
| 2 | 2 | **R** event-position clause: closed complete scope and no delivery; open scope; one admitted delivery; a delivery with a missing capture; anchor withdrawn; a clause the package does not hold | `Inactive`/`Inactive`/no truth; `Unknown`/`ActivationUnknown`; `Active` with a truth; `Unavailable` with a `Missing` non-value naming the capture; `Unavailable`; `Err(CallFailure::Admission)`, empty charge log |
| 3 | 3 | the Active request with the decision closure withdrawn; with the completeness record withdrawn; with one support fact withdrawn | FR-043 values of the remaining evidence and a `Missing` non-value for each withdrawn fact; no default value in any field |
| 4 | 4 | trigger id `t1` vs `t1` with one byte changed; empty id | different `result_identity`; refused at construction |
| 5 | 5 | change the surrounding closure, then the completeness state, then the trigger scope | each echoed exactly; only the field and the FR-043-derived truth change |
| 6 | 6 | superseding result of `r1` with the ledger of `r1`'s call | `lineage + 1` and the identity of `r1`; `r1` unchanged |
| 7 | 7 | each of `formula_nodes`, `instances`, `positions`, `valuations`, `captures`, `support`, `history_span`, `evaluation_steps`, `lineage`, `visited`, `request_bytes`, `result_bytes`, `member_bytes` at measured usage and one below | completed; `Incomplete` naming the counter, no truth |
| 8 | 8 | the timestamped-event clause under FR-501's vectors (TC-1295) through `evaluate_temporal_evidence` | the outcomes those cases state; `temporal.visit` charges FR-501-AC-12's plus one per delivery or origin |
| 9 | 9 | evaluate with `contract`, `schema_digest`, `digest`, `authority_digest` set to arbitrary and to empty bytes, then change each by one byte | each echoed byte for byte; only the echoed member differs between results |
| 10 | 10 | `deliveries` holding `t1` and `t2` with equal timestamp/receipt/payload/captures | two instances in `instances` |
| 11 | 11 | `deliveries` holding `t1` twice with an equal payload | one instance holding both receipts |
| 12 | 12 | `deliveries` holding `t1` twice with different payloads | `Err(CallFailure::Admission)`, empty charge log |
| 13 | 13 | the ledger of row 6: an unchanged request; a predecessor that is not the latest; a request for another obligation | three `Err(CallFailure::Admission)` with empty charge logs |
| 14 | 14 | a sibling clause evaluated after row 7's stop | keeps its own result |
| 15 | 15 | `control` naming the protocol clause whose FR-449 row names the call's clause | evaluated |
| 16 | 16 | `control` naming a protocol clause with no FR-449 row; one whose row names another clause | `activation-mapping-missing`; `activation-mapping-mismatch`, each `Err(CallFailure::Admission)` |
| 17 | 17 | a `progress_ledger` holding W 100: a request at W 60; a request with `Complete { through: 60 }` after `Complete { through: 100 }` | each refused as FR-502-AC-1 and AC-2, empty charge log, progress ledger unchanged |
| 18 | 18 | **R**'s four clauses over its three packages: the result over the package from the `Emission` vs over the read-back package; then one clock row, one `CheckedNodeId` or one instance-bound atom replaced on one side | equal fields, canonical bytes, charge log and `result_identity`; the replaced side differs |
| 19 | 19 | `deliveries` present and a `trigger` that no delivery or ledger entry holds | `trigger-unknown`, `Err(CallFailure::Admission)` |
| 20 | 20 | `deliveries` present and `trigger` `None` | `trigger-required` |
| 21 | 21 | `trigger` naming a delivery with a false guard | no truth; FR-524's disposition |
| 22 | 22 | `on each` clause with `anchor` naming the observation at index 2; `on origin` clause with `anchor` naming index 2 | position 2; position 0 |
| 23 | 23 | `anchor` naming no observed position | `Unavailable`, `Missing` non-value naming the anchor |
| 24 | 24 | closed complete scope, no delivery, `trigger` `None`, `anchor` naming no observed position | `Unavailable`, not `Inactive` |
| 25 | 25 | a counter stop, a meter stop, a `lineage` stop, an evicted required valuation, a `formula_nodes` stop, a `history_span` stop, a non-fitting tick sum, a `progress.covered_positions` capacity stop; a withdrawn anchor | each stop `Ok` with `Unavailable`, no truth and `incomplete` naming the stop; the anchor `Ok` with `Unavailable`, a `Missing` non-value and `incomplete` `None`; no `Err`; no unset activation |
| 26 | 26 | a clause whose verdict is `PendingNextEvent`; one whose verdict is `Pending` | `Truth::Pending` each |
| 27 | 27 | the clause of FR-523-AC-21 through the evidence call | `evaluation_steps` 4, `positions` 1, `valuations` 4, meter usage at `temporal.visit` 13, thirteen counters |
| 28 | 28 | a guard-`None` delivery with a healthy one; a request with a surplus capture fact | both listed, the first `Unavailable`; `Err(CallFailure::Admission)`, empty charge log |
| 29 | 29 | the package from the `Emission` vs the read-back package | equal profile selection and `TemporalBinding` row; the clause's own `binding` authenticates |
| 30 | 30 | a package with two clauses of different formulas and clock rows; a `binding` asserting the other's row | `binding-mismatch` with the FR-522 dimension, in the package from the `Emission` and the read-back package |
| 31 | 31 | each stop kind of row 25 with instances created or a receipt added before the stop | `ledger` and `progress_ledger` equal the request's; the retry equals the call that never stopped |
| 32 | 32 | the constructed V2 twin package (node `occurrences` 7 then 3, lock rows ascending 3 then 7, different clock rows) read back and evaluated through the read-back route only: `occurrence` 3 with each row's `binding`; `occurrence` 7 likewise | the matching row authenticates, the other is refused `binding-mismatch` with the FR-522 dimension; position pairing fails; no in-process comparison |
| 33 | 33 | the twins with no `occurrence`; with ordinal 5; a one-occurrence node with no `occurrence` | `occurrence-required`; `occurrence-unknown`, each empty charge log; evaluated |
| 34 | 34 | `execution` `Failed` vs `Completed`, open scope; the same over an admitted delivery | `Unknown` carrying each; same activation and truth |
| 35 | 35 | closed admitted scope, no delivery: record `Complete`; record `Incomplete` | `Inactive`; `Unknown(open)`; `execution` echoed |
| 36 | 36 | `on each` clause with `origins`; `on origin` clause with `deliveries`; the matching lists | `activation-list-mismatch` naming `origins`; naming `deliveries`, empty charge log; evaluated |
| 37 | 37 | fixed-sample and event-position: `Complete { through }` below the last coordinate; fixed-sample watermark below it; both equal to it | `progress-inconsistent`, empty charge log; evaluated |
| 38 | 38 | `progress.covered_positions` at the total minus one; at the total | `Ok` `Incomplete(LimitReached)` naming `progress.covered_positions`, ledgers unchanged; evaluated |
| 39 | 39 | formula with an unbounded operator; a timed interval; a cyclic or out-of-range `operands`; a `predicate_leaves()`/`holds` count mismatch; a readable twin of each; a tick sum over `u64` | `clause-unreadable` each, empty charge log; the readable ones return a result; the overflow is the `history_span` `Incomplete` |
| 40 | 40 | `control` naming a clause with no FR-449 row; a row naming another clause; the call's own row | `activation-mapping-missing`; `activation-mapping-mismatch`; evaluated, each refusal with an empty charge log |
| 41 | 41 | every cause of the table, each with its condition met and then unmet | `Err` with the cause, empty charge log, ledgers unchanged; `Ok`; no cause outside the table and the cross-referenced ones |
| 42 | 42 | timestamped-event `p until[0,5] q`, two positions of one stamp without order keys; the same with distinct keys of one authority; `eventually[0,5] holds(p)` over the same two positions | `Ok`, `Active`, no truth, `Refused`/`temporal-refusal`/`unordered-tie`; a truth; a truth |
| 43 | 43 | a support valuation absent at position 1 of atom 0; present; a read but non-support valuation absent (atom 1, position 2); with the decision closure withdrawn too | `Missing`/`fact-withdrawn`/`valuation:0@1`; no non-value; `valuation:1@2`; `decision_closure` |
| 44 | 44 | fixed-sample clause with a support pair at a silent sample | support holds the observed pairs' entries only |
| 45 | 45 | a `CheckedNodeId` absent from the package; the id of a `protocol_clause` node | `Err(CallFailure::Admission)`, cause `clause-root-unresolved`, empty charge log, ledgers unchanged |
| 46 | 46 | the `from_emission` package of TC-1310 row 24 and the `from_read` package of TC-1310 row 29, each with its paired clause differing | each run's first call reaching the clause: `clause-unreadable`, empty charge log, ledgers unchanged |

Tag the test `#[trace("FR-516-AC-1", ..., "FR-516-AC-46", "TC-1311")]`; rows 6, 8 and 13 also carry FR-052-AC-12 to AC-14. Every request states the clause's own `binding` (FR-522) unless a row varies it.

## Expected Results

Each row's Expected column.
