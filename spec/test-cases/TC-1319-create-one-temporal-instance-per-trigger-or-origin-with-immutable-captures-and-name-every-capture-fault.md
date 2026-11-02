---
id: TC-1319
title: "Create one temporal instance per semantic trigger or execution origin with immutable captures, and name every activation fault"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-524
    type: verifies
---
# TC-1319: Create one temporal instance per semantic trigger or execution origin with immutable captures, and name every activation fault

## Description

Verify FR-524-AC-1 to FR-524-AC-16. The vectors are the FR-044 cases and TC-123 and TC-1278, re-stated on the spine activation function. Captures are `Int` values at the delivery anchor.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | deliveries `t1`, `t2` with equal payloads and captured amounts | two instances; a proposition over `t1`'s capture is not satisfied by `t2`'s; the mismatch names `t1` and `t2` |
| 2 | 2 | `t1` again under another receipt, equal payload | one instance; the new receipt after the first; environment byte-equal |
| 3 | 3 | `t1` again with a different payload; the same through `evaluate_temporal_evidence` | contradiction refusal naming `t1`, environment unchanged; `Err(CallFailure::Admission)`, empty charge log |
| 4 | 4 | equal timestamp, receipt, payload and captures, identities differing in one byte | two instances |
| 5 | 5 | `on origin`: origins `o1`, `o2`; `o1` re-evaluated; `o1` with two payloads | two instances; no split; contradiction naming `o1` |
| 6 | 6 | guard false; closed scope whose every delivery has a false guard | no instance, no refusal; `Inactive` |
| 7 | 7 | guarded clause, delivery with `guard` `None` | `ActivationFault` naming the guard, `Missing`, no instance |
| 8 | 8 | `Missing` capture; `Null` capture; evicted capture; one capture too few | `ActivationFault` naming the capture, `Missing`; no position evaluated |
| 9 | 9 | `WrongType`; `Stale`; delivery anchor differing; capture anchor differing | `ActivationFault`, `Refused`, naming the capture or anchor; no position evaluated |
| 10 | 10 | one healthy and one faulted instance in one call | the healthy instance keeps its activation and environment |
| 11 | 11 | three captures; incremental re-evaluation; admitted restoration | authored order, once per instance; count stays one |
| 12 | 12 | a later delivery with a changed amount or reference; the type's API | environment byte-identical; no mutator |
| 13 | 13 | an initializer reading a later capture | refused at S3; no clause node |
| 14 | 14 | an evicted required valuation; the same trace with it present | `Incomplete` naming the subject, no truth; whole interval evaluated |
| 15 | 15 | closed with admitted evidence and no delivery; open; `Missing`; `Refused`; closed with incomplete record | `Inactive`; `Unknown(open)`; `Unknown(missing)`; `Unknown(refused)`; `Unknown(open)` incomplete; no truth, no position |
| 16 | 16 | rows 8, 9 and 15 through `evaluate_temporal_evidence` | `Unavailable` for faults; the dispositions of row 15 |

Tag the test `#[trace("FR-524-AC-1", ..., "FR-524-AC-16", "TC-1319")]`.

## Expected Results

Each row's Expected column.
