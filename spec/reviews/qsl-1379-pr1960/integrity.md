---
id: SR-1957012
title: "QSL-1379 PR #1960 integrity review of FR-281-AC-12 and TC-763"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; spec/functional/FR-281-analyze-claims-with-in-process-engines.md (AC-12), spec/test-cases/TC-763-analyze-settles-rejected-certificates-unreproduced-counterexamples.md (scope, step 8, expected result)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-763
    type: reviews
---

## Summary

Ticket QSL-1379; PR quire-spec-language#1960. Reviewed identifier and relationship validity, AC atomicity, requirement/test alignment, cross-reference consistency, and test expected results. The new AC ID follows FR-281-AC-11, is listed in the TC scope, and uses the existing QSpec cancellation-source vocabulary. AC-12 makes one observable claim that an operation-level test can verify, with the caller cancellation, limit, preservation, and binding-refusal branches stated explicitly. Validation passes on the two changed artifacts. Integrity gap: the AC says expiry applies to the whole request and settles each open item, but the single-item deadline case in TC-763 cannot demonstrate this cardinality or distinguish it from a per-item deadline (FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-763 step 8 must include multiple open items and assert all are settled by one request-wide expiry to cover the AC's “each open item” and whole-request obligations. | FR-281-AC-12; TC-763 step 8, expected result |

## Verdict

Changes requested: strengthen the TC case as described. No broken links, duplicate IDs, or conflicting result vocabulary found in the changed artifacts.

## Dispositions

No prior findings.

Round 1, head `9abab1f90` (fix commit after `cf84574dd`): rechecked FND-001 against the updated TC-763 step 8 and expected result. The single request now contains one earlier settled item and two open items when the request-wide deadline expires, and expected results require both open items to settle incomplete with the deadline cause. The request-wide all-open-items gap is fixed; no new finding from this repair.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | after_excerpt: `8. Using a deterministic clock and cancellation fake, run one three-item request whose first item settles before the deadline and whose other two items remain open when the deadline expires. Separately run a request where caller cancellation happens first and one where an engine or accounting limit happens first. Do not use wall-clock sleeps.` |
