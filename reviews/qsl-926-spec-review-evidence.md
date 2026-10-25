---
id: SR-17194
title: "QSL-926 PR #1524 evidence review of FR-337-AC-6, FR-337-AC-7 and FR-125-AC-18"
type: SpecReview
analysis: evidence
scope: "agent-ix/quire-spec-language; spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md, spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: reviews
---

## Summary

Ticket: QSL-926. PR: quire-spec-language#1524 (head a8b073ce6). The three new criteria declare `Test`: FR-337-AC-6 and FR-337-AC-7 via TC-1265, and FR-125-AC-18 via TC-520. Each names a concrete compiled input and an observable output: node members, keys, refusal codes and loci, outcomes, prefixes and replay verdicts. So Test is the fitting method, and no Analysis or Inspection obligation is hidden in them. The four amended ACs (FR-126-AC-16, FR-338-AC-10, FR-339-AC-5, TC-521/525/1201/1215) keep their method. Only their fixture prose changed.

Examined units: FR-337-AC-6, FR-337-AC-7, FR-125-AC-18, FR-126-AC-16, FR-338-AC-10, FR-339-AC-5.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean: every method fits its criterion.
