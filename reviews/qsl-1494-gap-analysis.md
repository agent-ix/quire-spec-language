---
id: SR-2026110216375
title: "Gap analysis — QSL-1494 progress ledger route coverage"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-502, FR-052 AC-13, FR-330 AC-9; qsl-eval/src/temporal/ledger.rs, qsl-replay/src/spine/clause/temporal.rs, qsl-replay/src/spine/clause/tests/temporal.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-502
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-330
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-052
    type: reviews
---

## Summary

Manual acceptance review found that the new ledger unit tests exercise several contradiction rules, but the replay entry point has no progress-carrying trace path and never calls `admit_progress`. The required behavior therefore remains absent from `ClauseRunReport` and cannot satisfy the route-level acceptance criteria.

## Verdict

**FAIL** — the missing route implementation is a high-severity implementation gap.

## Coverage

Quoin/Quire matrix commands were attempted (`quoin matrix --repo . --json` and `quire matrix --strict --scope .`) but produced no matrix before their 90-second and 60-second execution windows ended; automated traceability status is unavailable. Manual review compared the requirement text, trace tags in the new ledger tests, and the changed replay code. Semantic intent review was not run. Plan completion: not assessed.

- FR-502 AC-1: ledger unit test checks watermark refusal and retained watermark; it does not compare prior temporal result bytes.
- FR-502 AC-2: ledger unit test checks completeness retreat, `Incomplete`, and uncovered position; retained ledger is immutable.
- FR-502 AC-3: ledger unit test checks closed-scope/execution reopen and closure/completeness independence.
- FR-502 AC-4: unit test varies four string fields but does not execute a temporal binding or verify `Pending` remains unchanged.
- FR-502 AC-5: non-progress finite, lasso, and infinite-profile refusals are tested; accepted Timestamped and Progressed paths are absent from `TraceInput`.
- FR-502 AC-6: ledger-level sequences are deterministic; replay reports do not return an updated ledger because no request calls `admit_progress`.
- FR-502 AC-7: a new uncovered position at 80 is rejected and a new position at 120 is retained in the ledger unit test.
- FR-502 AC-8: ledger capacity and omission behavior are tested at the ledger API; replay capacity reporting is not reached from a progress-carrying trace.
- FR-052 AC-13: no end-to-end temporal progress contradiction test.
- FR-330 AC-9: no finite-window Timestamped route; existing evaluator refusal remains `ProfileNotEvaluated`.

Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The replay implementation does not connect progress-carrying traces to the ledger, leaving FR-502 AC-5/6, FR-052 AC-13, and FR-330 AC-9 unimplemented at the route boundary. | qsl-replay/src/spine/clause/temporal.rs:40; qsl-replay/src/spine/clause/temporal.rs:250; qsl-replay/src/spine/clause/temporal.rs:349 | correct-requirement-no-evidence |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `TraceInput` now has progress-carrying inputs and `run_temporal` calls `admit_progress`; the replay test verifies the report's updated ledger and evaluate-stage watermark contradiction. |

## New findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | The test suite does not cover a new position whose observation identity matches a retained position but whose stamp differs; the ledger currently admits it as covered. | qsl-eval/src/temporal/ledger.rs:93; qsl-eval/src/temporal/ledger.rs:306 |


**Round 2 disposition.**

| FND | outcome | reason |
| --- | --- | --- |
| FND-002 | still-open | The covered set and contradiction check still ignore stamp, so the missing-position case remains possible. |
