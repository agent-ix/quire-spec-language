---
id: SR-1957008
title: "QSL-1379 PR #1960 base spec review of FR-281-AC-12 and TC-763"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-281-analyze-claims-with-in-process-engines.md (AC-12), spec/test-cases/TC-763-analyze-settles-rejected-certificates-unreproduced-counterexamples.md (step 8)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-763
    type: reviews
---

## Summary

Ticket QSL-1379; PR quire-spec-language#1960, head `spec/qsl-1379-fr281-deadline`, base `main`. Read the PR diff, FR-281's request and item settlement behavior, TC-763, repository instructions, and QSpec FR-331's deadline/cancellation vocabulary on main. The new AC is behavior-level and correctly identifies a finite `AnalyzeLimits.deadline_ms` as request-wide, measured from `analyze` entry; deadline expiry yields `incomplete` with `cancelled{source: deadline}`, preserving settled records, while caller cancellation and engine/accounting limits retain their typed outcomes. Binding refusal remains before item execution. No contradiction with the inspected FR-331 vocabulary was found. `quire validate` on the changed requirement and TC passed; full `quire validate --scope .` also passed (1921/1931 documents clean, pre-existing grammar findings elsewhere). The requested `quoin advise --repo . --json` and `quire matrix --scope . --format json` commands produced no output and were interrupted after running for over two minutes each. The new AC is currently untagged in main; its test trace is sequenced for the following implementation PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-763 step 8 exercises one deadline-ended open item, so it does not verify that a request-wide expiry settles every open item. An implementation with per-item deadlines, or one that leaves a second open item unsettled, could pass the stated case despite violating AC-12. Include at least two open items at expiry and assert that each is `incomplete` with `cancelled{source: deadline}`, while the previously settled record is unchanged. | FR-281-AC-12; TC-763 step 8, Expected Results step 8 |

## Verdict

Changes requested: update TC-763 step 8 and its expected result to exercise multiple open items and assert that all are settled by the one request-wide deadline. The requirement's typed outcomes and composition are otherwise coherent.

## Dispositions

No prior findings.

Round 1, head `9abab1f90` (fix commit after `cf84574dd`): rechecked FND-001 against the updated TC-763 step 8 and expected result. The single request now contains one earlier settled item and two open items when the request-wide deadline expires, and expected results require both open items to settle incomplete with the deadline cause. The request-wide all-open-items gap is fixed; no new finding from this repair.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | after_excerpt: `8. Using a deterministic clock and cancellation fake, run one three-item request whose first item settles before the deadline and whose other two items remain open when the deadline expires. Separately run a request where caller cancellation happens first and one where an engine or accounting limit happens first. Do not use wall-clock sleeps.` |
