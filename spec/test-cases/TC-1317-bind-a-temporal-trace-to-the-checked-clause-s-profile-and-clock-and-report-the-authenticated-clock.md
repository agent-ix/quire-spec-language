---
id: TC-1317
title: "Bind a temporal trace to the checked clause's profile and clock and report the authenticated clock"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-522
    type: verifies
---
# TC-1317: Bind a temporal trace to the checked clause's profile and clock and report the authenticated clock

## Description

Verify FR-522-AC-1 to FR-522-AC-15. **R** is the recipe package of FR-517 with its three clock kinds. Each refusal row varies one member of the asserted binding from the clause's own row. The numeric clock values are the rows of **R**'s lock, read by the test and not copied here.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | fixed-sample clause, `Progressed` request with another profile identity | stage `admit`, `invalid_runtime_input`/`invalid-value`, `mismatched_dimension` `Profile`, asserted and expected identities, no outcome, no `authenticated` |
| 2 | 2 | event-position clause, another `clock_name` | `Clock` |
| 3 | 3 | fixed-sample clause, `EventPosition` parameters | `ClockParameters` |
| 4 | 4 | fixed-sample clause: `epoch` alone, `period` alone, `unit` alone, then `epoch` and `unit` together | `Epoch`, `SamplePeriod`, `ClockUnit`, `Epoch` |
| 5 | 5 | event-position `sequence_authority` alone; timestamped-event `timestamp_unit` alone | `SequenceAuthority`; `TimestampUnit` |
| 6 | 6 | wrong profile and wrong clock name; wrong clock name and wrong parameter | `Profile`; `Clock` |
| 7 | 7 | a request equal to the row | evaluated; `authenticated` holds package identity, declaration, definition identity and revision, clock name and `ClockConfiguration` equal to the lock and row |
| 8 | 8 | a refused binding; a `Timestamped` request with stamps 0, 5, 2 and a wrong clock name | empty charge log; `Clock`, not the stamp-order refusal |
| 9 | 9 | a clause with no `temporal_bindings` row, request with a `TraceBinding` | `Clock`; no substituted configuration |
| 10 | 10 | rows 1 to 5 through `evaluate_temporal_evidence` | `Err(CallFailure::Admission)` with the same dimension, empty charge log, no result |
| 11 | 11 | `Finite`, `Lasso` and `Model` traces | no `TraceBinding`; reports hold no `authenticated` |
| 12 | 12 | each of **R**'s three requests evaluated twice | one identity per request; the three pairwise distinct |
| 13 | 13 | fixed-sample request varied alone in row `epoch`, `period`, `unit`, profile, clock name, one stamp, one valuation, `W`, `through`, lower boundary, one of the thirteen ceilings, the meter ceiling | each gives an identity distinct from row 12's |
| 14 | 14 | row 13's request, then the row 12 request again | the first result's bytes equal those returned before |
| 15 | 15 | event-position `Progressed`: `sequence_authority`, `W`, scope varied alone; in-process vs read-back package | distinct identities; in-process identity equals read-back identity |

Tag the test `#[trace("FR-522-AC-1", ..., "FR-522-AC-15", "TC-1317")]`.

## Expected Results

Each row's Expected column.
