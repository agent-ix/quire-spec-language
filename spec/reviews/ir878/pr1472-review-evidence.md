---
id: SR-17051
title: "IR-878 PR #1472 evidence review of FR-486"
type: SpecReview
analysis: evidence
scope: "agent-ix/quire-spec-language; spec/functional/FR-486-supplied-admission-and-conversion-owner-allocation.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-486
    type: reviews
---

## Summary

Ticket: IR-878. PR: quire-spec-language#1472 (head 5e56adbb7, diff `origin/main...HEAD`). Reviewer pass (evidence) over the spec-only re-derivation of closed #995 as FR-486. Review set: base plus integrity, ears-conformance, evidence, scope-boundary, failure-domain (subset chosen by the dispatching brief; no AssuranceProfile in spec/).

Examined units: FR-486-AC-1 (examined), FR-486-AC-2 (examined), FR-486-AC-3 (examined).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-1 claims something about every reachable supplied-membership and conversion event ('has one initiating phase and budget owner') but gives only Test as the method and does not enumerate the events, so a test can only sample them. It also carries a clause no test can observe ('keeps the ... allocation from QSV FR-109-AC-6'). Split it: Test for the observable no-double-charge and no-FunctionCall properties, and Inspection or Analysis for the event-to-owner enumeration. | FR-486-AC-1:40 |

## Verdict

AC-2 and AC-3 fit Test. AC-1 needs splitting. Scoped matrix rows untagged as expected for PLANNED status.
