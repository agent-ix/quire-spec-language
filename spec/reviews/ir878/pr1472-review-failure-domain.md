---
id: SR-17053
title: "IR-878 PR #1472 failure-domain review of FR-486"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language; spec/functional/FR-486-supplied-admission-and-conversion-owner-allocation.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-486
    type: reviews
---

## Summary

Ticket: IR-878. PR: quire-spec-language#1472 (head 5e56adbb7, diff `origin/main...HEAD`). Reviewer pass (failure-domain) over the spec-only re-derivation of closed #995 as FR-486. Review set: base plus integrity, ears-conformance, evidence, scope-boundary, failure-domain (subset chosen by the dispatching brief; no AssuranceProfile in spec/).

Examined units: FR-486 (context_only), FR-486 (examined), FR-486-AC-1 (examined), FR-486-AC-2 (examined), FR-486-AC-3 (examined).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Conversion-phase failures are not enumerated. AC-3 lists membership, nonmembership, budget denial and cancellation for the membership fixture only. No criterion says that conversion budget denial, or cancellation through the original handle during conversion, leaves the evaluator meter and charge sequence unchanged and is attributed to the conversion phase. AC-1 only says such failures are not labelled FunctionCall; it does not say what typed phase or locus they carry. | FR-486-AC-3:42 |

## Verdict

Membership-side failures are covered for Decimal. Conversion-phase failure and cancellation are not.
