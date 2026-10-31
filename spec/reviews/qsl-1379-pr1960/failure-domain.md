---
id: SR-1957010
title: "QSL-1379 PR #1960 failure-domain review of FR-281-AC-12"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language; spec/functional/FR-281-analyze-claims-with-in-process-engines.md (AC-12), spec/test-cases/TC-763-analyze-settles-rejected-certificates-unreproduced-counterexamples.md (step 8)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-763
    type: reviews
---

## Summary

Ticket QSL-1379; PR quire-spec-language#1960. Reviewed deadline/cancellation/limit ordering, request/item scope, settled versus open records, and binding admission refusal. The AC names typed causes for deadline and caller cancellation, preserves prior settled records, retains a typed engine/accounting-limit result when that stops an item first, and preserves binding refusal before any item runs. The failure-domain gap is test discrimination: step 8 has only one deadline-ended open item, although the requirement's request-wide behavior applies to each open item. A one-item case cannot distinguish request-wide cancellation from per-item timeout or prove complete settlement of all outstanding work. FR-331's vocabulary supports the named cancellation source values. `quire validate` passed on the changed files.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-763 step 8 does not exercise multiple open items when the request-wide deadline expires. Assert that every open item is settled `incomplete` with `cancelled{source: deadline}`, while the prior settled record is preserved. | FR-281-AC-12; TC-763 step 8 and expected result |

## Verdict

Changes requested: add a multi-open-item deadline case to TC-763 so request-wide cancellation and full settlement are observable.

## Dispositions

No prior findings.

Round 1, head `9abab1f90` (fix commit after `cf84574dd`): rechecked FND-001 against the updated TC-763 step 8 and expected result. The single request now contains one earlier settled item and two open items when the request-wide deadline expires, and expected results require both open items to settle incomplete with the deadline cause. The request-wide all-open-items gap is fixed; no new finding from this repair.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | after_excerpt: `8. Using a deterministic clock and cancellation fake, run one three-item request whose first item settles before the deadline and whose other two items remain open when the deadline expires. Separately run a request where caller cancellation happens first and one where an engine or accounting limit happens first. Do not use wall-clock sleeps.` |
