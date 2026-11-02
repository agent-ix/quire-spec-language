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
FR-516-AC-1 to FR-516-AC-9. The numeric vectors are the QSpec FR-370 and
FR-043 cases and TC-144; they are referenced, never copied.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | compile-fail: set `truth` on `EvidenceRequest`; list its fields | does not compile; the fields are those of the Inputs table |
| 2 | 2 | **R** event-position clause: closed complete scope and no delivery; open scope; one admitted delivery; a refused request | `Inactive`/`Inactive`/no truth; `Unknown`/`ActivationUnknown`; `Active` with a truth; `Unavailable` |
| 3 | 3 | the Active request with the decision closure withdrawn; with the completeness record withdrawn; with one support fact withdrawn | `Missing` non-value or `settlement = Unavailable` each; no default value in any field |
| 4 | 4 | trigger id `t1` vs `t1` with one byte changed; equal timestamp/receipt/payload/captures under `t1` and `t2`; redelivery of `t1` with equal then different payload; empty id | different `result_identity` or refusal; two instances; existing instance; FR-044 contradiction; refused at construction |
| 5 | 5 | change the surrounding closure, then the completeness state, then the trigger scope | each echoed exactly; only the field and the FR-043-derived truth change |
| 6 | 6 | superseding result of `r1`; self-predecessor; predecessor of another obligation; wrong identity | `lineage + 1` and the identity of `r1`; three refusals; `r1` unchanged |
| 7 | 7 | each of `formula_nodes`, `formula_depth`, `positions`, `valuations`, `captures`, `support`, `history_span`, `evaluation_steps`, `lineage`, `visited` at measured usage and one below; sibling clause | completed; `Incomplete` naming the counter, no truth; the sibling keeps its result |
| 8 | 8 | the timestamped-event clause under FR-501's vectors (TC-1295) and FR-502's (TC-1296) through `evaluate_temporal_evidence`; in-process vs read-back | the outcomes those cases state; equal results and equal charge logs |
| 9 | 9 | reflect over the public fields of both types | each has a row in FR-516; the test fails on a field with none |

Tag the test `#[trace("FR-516-AC-1", ..., "FR-516-AC-9", "TC-1311")]`; rows 6 and 8 also carry FR-052-AC-12 to AC-14.

## Expected Results

Each row's Expected column.
