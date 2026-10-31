---
id: SR-1957011
title: "QSL-1379 PR #1960 evidence review of FR-281-AC-12"
type: SpecReview
analysis: evidence
scope: "agent-ix/quire-spec-language; spec/functional/FR-281-analyze-claims-with-in-process-engines.md (AC-12), spec/test-cases/TC-763-analyze-settles-rejected-certificates-unreproduced-counterexamples.md (step 8)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-763
    type: reviews
---

## Summary

Ticket QSL-1379; PR quire-spec-language#1960. AC-12 is classified `Test (TC-763)`, appropriate evidence for observable settlements and ordering. Step 8 specifies a deterministic clock/cancellation fake and forbids wall-clock sleeps; its expected result states the deadline, caller-cancel, limit, preservation, and binding-refusal outcomes. The step needs multiple open items to demonstrate the request-wide scope and `each open item` obligation (FND-001). `quoin advise --repo . --json` was requested, but produced no output after more than two minutes and was interrupted (exit 130); recommendation was therefore checked manually against the declared Test method and concrete TC. `quire matrix --scope . --format json` likewise produced no output after more than two minutes and was interrupted (exit 130). The AC is not yet trace-tagged on main because the planned implementation PR will add the code test; this sequencing is noted, not treated as a defect of this spec-only PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The selected Test evidence does not yet demonstrate request-wide settlement because TC-763 checks only one deadline-ended open item. Add at least one further open item and assert all open items settle with the deadline cause. | FR-281-AC-12; TC-763 step 8 |

## Verdict

Changes requested for test coverage: expand step 8 to prove the request-wide, all-open-items behavior. The authored evidence method itself is suitable.

## Dispositions

No prior findings.

Round 1, head `9abab1f90` (fix commit after `cf84574dd`): rechecked FND-001 against the updated TC-763 step 8 and expected result. The single request now contains one earlier settled item and two open items when the request-wide deadline expires, and expected results require both open items to settle incomplete with the deadline cause. The request-wide all-open-items gap is fixed; no new finding from this repair.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | after_excerpt: `8. Using a deterministic clock and cancellation fake, run one three-item request whose first item settles before the deadline and whose other two items remain open when the deadline expires. Separately run a request where caller cancellation happens first and one where an engine or accounting limit happens first. Do not use wall-clock sleeps.` |
