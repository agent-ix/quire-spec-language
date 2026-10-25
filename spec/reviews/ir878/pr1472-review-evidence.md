---
id: SR-17031
title: "IR-878 PR #1472 evidence review of FR-485"
type: SpecReview
analysis: evidence
scope: "agent-ix/quire-spec-language; spec/functional/FR-485-supplied-admission-and-conversion-owner-allocation.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-485
    type: reviews
---

## Summary

Ticket: IR-878. PR: quire-spec-language#1472 (head 5e56adbb7, diff `origin/main...HEAD`). Reviewer pass (evidence) over the spec-only re-derivation of closed #995 as FR-485. Review set: base plus integrity, ears-conformance, evidence, scope-boundary, failure-domain (subset chosen by the dispatching brief; no AssuranceProfile in spec/).

Examined units: FR-485-AC-1 (examined), FR-485-AC-2 (examined), FR-485-AC-3 (examined).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-1 claims something about every reachable supplied-membership and conversion event ('has one initiating phase and budget owner') but gives only Test as the method and does not enumerate the events, so a test can only sample them. It also carries a clause no test can observe ('keeps the ... allocation from QSV FR-109-AC-6'). Split it: Test for the observable no-double-charge and no-FunctionCall properties, and Inspection or Analysis for the event-to-owner enumeration. | FR-485-AC-1:40 |

## Verdict

AC-2 and AC-3 fit Test. AC-1 needs splitting. Scoped matrix rows untagged as expected for PLANNED status.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | The inspection target of FR-486-AC-2 is a document, not the code: 'A table in the review shows that each maps to exactly one initiating phase and budget owner'. Writing the table satisfies the criterion, and nothing ties it to the evaluator or supplied-caller source, so the criterion cannot fail against the implementation. Name the source paths and call sites being inspected (the qsl-eval evaluator and supplied-caller entry points) and what the inspection must find. | spec/functional/FR-486-supplied-admission-and-conversion-owner-allocation.md:45 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Verified in dbb95abc5 against origin/main. |
