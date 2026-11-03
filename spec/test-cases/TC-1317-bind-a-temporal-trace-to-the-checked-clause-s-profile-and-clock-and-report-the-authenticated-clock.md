---
id: TC-1317
title: "Bind a temporal trace to the checked clause's profile and clock and report the authenticated clock"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-522
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-330
    type: verifies
---
# TC-1317: Bind a temporal trace to the checked clause's profile and clock and report the authenticated clock

## Description

Verify FR-522-AC-1 to FR-522-AC-23 and FR-330-AC-11. **R** is the recipe package of FR-517 with its three clock kinds. Each refusal row varies one member of the asserted binding from the clause's own row. The numeric clock values are the rows of **R**'s lock, read by the test and not copied here. Every request states the clause's own `binding` unless the row varies it.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | fixed-sample clause, `Progressed` request with another profile identity | stage `admit`, `invalid_runtime_input`/`invalid-value`, `mismatched_dimension` `Profile`, asserted and expected identities, no outcome, no `authenticated` |
| 2 | 2 | event-position clause, another `clock_name` | `Clock` |
| 3 | 3 | fixed-sample clause, `EventPosition` parameters | `ClockParameters` |
| 4 | 4 | fixed-sample clause: `epoch` alone, `period` alone, `unit` alone | `Epoch`, `SamplePeriod`, `ClockUnit` |
| 5 | 5 | fixed-sample clause: `epoch` and `unit` together | `Epoch` |
| 6 | 6 | event-position `sequence_authority` alone; timestamped-event `timestamp_unit` alone | `SequenceAuthority`; `TimestampUnit` |
| 7 | 7 | wrong profile and wrong clock name | `Profile` |
| 8 | 8 | wrong clock name and wrong parameter | `Clock` |
| 9 | 9 | a request equal to the row | evaluated; `authenticated` holds package identity, declaration, definition identity and revision, clock name and `ClockConfiguration` equal to the lock and row |
| 10 | 10 | a refused binding | empty charge log |
| 11 | 11 | a `Timestamped` request with a wrong clock name and no positions | `Clock`, not the empty-trace refusal |
| 12 | 12 | a clause with no `temporal_bindings` row, request with a `TraceBinding` | `Clock`; no substituted configuration |
| 13 | 13 | rows 1 to 6 through `evaluate_temporal_evidence` | `Err(CallFailure::Admission)` with the same dimension, empty charge log, no result |
| 14 | 14 | `Finite`, `Lasso` and `Model` traces | no `TraceBinding`; reports hold no `authenticated` |
| 15 | 15 | each of **R**'s three requests evaluated twice | one identity per request |
| 16 | 16 | **R**'s three requests | pairwise distinct identities |
| 17 | 17a | fixed-sample request varied alone in the clock row: `epoch`, `period`, `unit` | each gives an identity distinct from row 15's |
| 18 | 17b | varied alone: profile, clock name, cover | each distinct |
| 19 | 17c | varied alone: one stamp, one valuation, one order key | each distinct |
| 20 | 17d | varied alone: watermark, `through`, scope closure, surrounding-execution closure, lower boundary | each distinct |
| 21 | 17e | varied alone: one retained-instance entry; one progress-ledger entry; each of the thirteen ceilings of FR-523; the meter's work-unit ceiling | each distinct |
| 22 | 18 | row 17's changed request, then row 15's request again | the first result's bytes equal those returned before |
| 23 | 19 | event-position `Progressed`: `sequence_authority`, `W`, scope varied alone | distinct identities |
| 24 | 20 | in-process package vs read-back package | equal `result_identity` |
| 25 | FR-330-AC-11 | `run_clause`: `Timestamped` with another clock name; `Progressed` with another profile; `Timestamped` with no `binding` (FR-330-AC-11) | stage `admit`, `invalid_runtime_input`/`invalid-value` with `Clock`; with `Profile`; refused |
| 26 | 21 | two requests differing only in the relation, the `EvidenceLedger` or the progress ledger | equal `input_identity` |
| 27 | 22 | two requests differing only in a retained-instance entry | distinct `result_identity` |
| 28 | 23 | two requests differing only in the progress-ledger entry | distinct `result_identity` |

Tag the test `#[trace("FR-522-AC-1", ..., "FR-522-AC-23", "FR-330-AC-11", "TC-1317")]`; the identity rows 17a to 17e carry FR-522-AC-17.

## Expected Results

Each row's Expected column.
