---
id: SR-13323
title: "Spec review of PR #1050: dependency analysis"
type: SpecReview
analysis: dependency
scope: "agent-ix/quire-spec-language@c5407fabd; git diff origin/main...HEAD (PR #1050)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-146
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---

## Summary

Ticket: STD-212. Dependency edges and the cross-repo follow-ups. FR-146 gains `depends_on` FR-095 and FR-314, matching its new text. A Linear search (titles 'FR-123', descriptions 'StepRowRef') finds only STD-212 and QSL-899 (Done).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-146 Obligation identity bullet | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 Occurrence key bullet | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 Outputs StepObligation comment | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-136 Order of the checked rows bullet | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| QSV FR-123 StepRowRef description | quire-semantic-value:spec/functional/FR-123-step-simulation-record-and-obligation.md | context_only |
| ADR-013 O-09 | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The cross-repo and ADR follow-ups this text depends on have no tickets: quire-semantic-value FR-123 (declaration order; hypotheses comment), ADR-013 O-07/O-09 and the CG preimage, QSpec FR-322 for the occurrence role, and QSpec FR-375/FR-432 for protocol-row order. The PR body lists the QSV FR-123 change as a follow-up but files nothing, and #1040's (QSL-548) occurrence-key remainder is now built on text the ADR and the QSV owner contradict. File one ticket for each and link it to STD-212 and QSL-548: QSV FR-123, ADR-013 amendment plus a CG ask, QSpec FR-322 role, and QSpec FR-375 protocol-row order. Mark as blocking the ones the record half cannot be built without (ADR-013/CG). | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:140-148 |

## Verdict

Findings: 1 medium. Edges FR-095 and FR-314 are correct and needed.
