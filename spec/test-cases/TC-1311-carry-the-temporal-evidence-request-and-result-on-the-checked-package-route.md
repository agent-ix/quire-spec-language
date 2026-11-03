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
FR-516-AC-1 to FR-516-AC-16. The numeric vectors are the QSpec FR-370 and
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
| 8 | 8 | the timestamped-event clause under FR-501's vectors (TC-1295) and FR-502's (TC-1296) through `evaluate_temporal_evidence`; in-process vs read-back | the outcomes those cases state; equal results and equal charge logs |
| 9 | 9 | evaluate with `contract`, `schema_digest`, `digest`, `authority_digest` set to arbitrary and to empty bytes, then change each by one byte | each echoed byte for byte; only the echoed member differs between results |
| 10 | 10 | `deliveries` holding `t1` and `t2` with equal timestamp/receipt/payload/captures | two instances in `instances` |
| 11 | 11 | `deliveries` holding `t1` twice with an equal payload | one instance holding both receipts |
| 12 | 12 | `deliveries` holding `t1` twice with different payloads | `Err(CallFailure::Admission)`, empty charge log |
| 13 | 13 | the ledger of row 6: an unchanged request; a predecessor that is not the latest; a request for another obligation | three `Err(CallFailure::Admission)` with empty charge logs |
| 14 | 14 | a sibling clause evaluated after row 7's stop | keeps its own result |
| 15 | 15 | `control` naming the protocol clause whose FR-449 row names the call's clause | evaluated |
| 16 | 16 | `control` naming a protocol clause with no FR-449 row; one whose row names another clause | `activation-mapping-missing`; `activation-mapping-mismatch`, each `Err(CallFailure::Admission)` |

Tag the test `#[trace("FR-516-AC-1", ..., "FR-516-AC-16", "TC-1311")]`; rows 6, 8 and 13 also carry FR-052-AC-12 to AC-14. Every request states the clause's own `binding` (FR-522) unless a row varies it.

## Expected Results

Each row's Expected column.
