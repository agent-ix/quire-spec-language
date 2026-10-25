---
id: SR-17050
title: "IR-878 PR #1472 EARS conformance review of FR-486"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language; spec/functional/FR-486-supplied-admission-and-conversion-owner-allocation.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-486
    type: reviews
---

## Summary

Ticket: IR-878. PR: quire-spec-language#1472 (head 5e56adbb7, diff `origin/main...HEAD`). Reviewer pass (ears-conformance) over the spec-only re-derivation of closed #995 as FR-486. Review set: base plus integrity, ears-conformance, evidence, scope-boundary, failure-domain (subset chosen by the dispatching brief; no AssuranceProfile in spec/).

Examined units: FR-486 (examined), FR-486 (examined), FR-486-AC-1 (context_only), FR-486-AC-2 (context_only), FR-486-AC-3 (context_only).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | 'The initiating phase shall debit a shared helper exactly once.' The phase debits a budget, not a helper, and 'exactly once' can be read per helper invocation or per logical event (QSV FR-109-AC-6 says each reachable logical event is charged once). Two implementers could charge differently. State the budget being debited and the counted unit. | FR-486:34 |
| FND-002 | low | The Behavior section mixes subjects: 'Supplied membership shall not create...' makes a data path the subject instead of the QSL evaluator or caller, and the closing sentence ('A direct QSV caller that omits the required bound follows QSV FR-122-AC-3...') is informational, not a QSL requirement. Name the QSL system as the subject and move the QSV note to Dependencies. | FR-486:34 |

## Verdict

Description and ACs name a subject and response. The Behavior section has one ambiguous and one mis-subjected statement.
