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

Verify FR-524-AC-1 to FR-524-AC-31. The vectors are the FR-044 cases and TC-123 and TC-1278, re-stated on the spine activation function. Captures are `Int` values at the delivery anchor.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | deliveries `t1`, `t2` with equal payloads and captured amounts | two instances, two environments keyed by `t1` and `t2` |
| 2 | 2 | an instance-bound atom with an entry only for `t2` at a position | `t1` assessed with a missing valuation naming `t1` and `t2`; `t2` assessed with its entry |
| 3 | 3 | `t1` again under another receipt, equal payload | one instance; the new receipt after the first |
| 4 | 4 | the environment of `t1` before and after row 3 | byte-equal |
| 5 | 5 | two deliveries of `t1` with different payloads, passed to `activate` | contradiction refusal naming `t1`; environment unchanged |
| 6 | 6 | the same through `evaluate_temporal_evidence` | `Err(CallFailure::Admission)`, empty charge log |
| 7 | 7 | equal timestamp, receipt, payload and captures, identities differing in one byte | two instances |
| 8 | 8 | `on origin`: origins `o1`, `o2`; `o1` re-evaluated | two instances; no split |
| 9 | 9 | `on origin`: `o1` twice with two payloads | contradiction naming `o1` |
| 10 | 10 | guard `Some(false)`; closed scope with every guard false | no instance, no refusal; `Inactive` |
| 11 | 11 | guarded clause, delivery with `guard` `None` | `ActivationFault` naming the guard, `Missing`, no instance |
| 12 | 12 | `Missing` capture; `Null` capture; evicted capture | `ActivationFault` naming the capture, `Missing`; no position evaluated |
| 13 | 13 | one capture too few | `ActivationFault` naming the first capture not supplied, `Missing` |
| 14 | 14 | `WrongType`; `Stale` | `ActivationFault`, `Refused`, naming the capture |
| 15 | 15 | delivery anchor differing; capture anchor differing | `ActivationFault`, `Refused` |
| 16 | 16 | wrong delivery anchor with a missing capture; evicted capture with a later wrong-type capture | the anchor fault; the evicted capture |
| 17 | 17 | one healthy and one faulted instance | the healthy instance keeps its activation and environment |
| 18 | 18 | three captures | authored order, once per instance |
| 19 | 19 | a ledger holding `t1`; `t1` delivered again, equal payload | receipt added; no initializer evaluated; evaluation count one |
| 20 | 20 | a ledger holding `t1`; `t1` delivered with a different payload | contradiction; ledger unchanged |
| 21 | 21 | a ledger holding `t1`; a retained capture of `t1` in `evicted` | `ActivationFault` naming the capture, `Missing`; nothing recomputed |
| 22 | 22 | a later delivery or observation with a changed amount or reference | environment byte-identical; no mutator |
| 23 | 23 | an initializer reading a later capture | refused at S3; no clause node |
| 24 | 24 | an evicted required valuation | `Incomplete` naming the subject, no truth |
| 25 | 25 | the same trace with the valuation present | whole interval evaluated |
| 26 | 26 | two deliveries, one capture each | `evaluation_steps` 2 for deliveries and 2 for captures; `instances` 2; `captures` 2 |
| 27 | 27 | closed with admitted evidence and no delivery; open | `Inactive`; `Unknown(open)` |
| 28 | 28 | `Missing` evidence; `Refused` evidence | `Unknown(missing)`; `Unknown(refused)` |
| 29 | 29 | closed scope with an incomplete trigger record | `Unknown(open)` with incomplete completeness |
| 30 | 30 | rows 27 to 29 | no truth value; no position evaluated |
| 31 | 31 | rows 12, 14 and 27 to 28 through `evaluate_temporal_evidence` | `Unavailable` for the faulted assessed instance; the dispositions of rows 27 and 28 |

Tag the test `#[trace("FR-524-AC-1", ..., "FR-524-AC-31", "TC-1319")]`.

## Expected Results

Each row's Expected column.
