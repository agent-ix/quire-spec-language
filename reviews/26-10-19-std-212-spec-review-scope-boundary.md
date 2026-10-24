---
id: SR-13327
title: "Spec review of PR #1050: scope-boundary analysis"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quire-spec-language@c5407fabd; git diff origin/main...HEAD (PR #1050)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-146
    type: reviews
---

## Summary

Ticket: STD-212. Ownership of the types and identities the new text defines. QSL owning the hypotheses order matches QSV FR-123 ('QSV SHALL NOT sort or deduplicate hypotheses; QSL owns their order').

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-146 Outputs StepObligation comment | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146 Obligation identity bullet | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| QSV FR-123 StepRowRef description | quire-semantic-value:spec/functional/FR-123-step-simulation-record-and-obligation.md | context_only |
| ADR-013 O-09 | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-146 redefines what a `StepRowRef` means (position in the digest-sorted `steps` order) and what `hypotheses` holds, but the owner of both types, quire-semantic-value FR-123, still says 'position ... in declaration order' and 'the concrete package's invariant clauses'. FR-146 itself says the types live in quire-semantic-value (ADR-018 LA-5), so the two owners disagree until QSV FR-123 changes. Land or ticket the QSV FR-123 edit together with this PR (see the dependency SR) so the meaning has one owner. | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:75-79 |

## Verdict

Findings: 1 medium. The obligation-identity ownership problem (CG computes O-09's digest) is the base SR's FND-002 and is not repeated here.

## Dispositions

Round 1, head 7cfb5530b (fix commit 7cfb5530b over merge 2b96e20e4).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | FR-146 now names the disagreement and says QSV FR-123 is to change ('which still says "declaration order" and "the concrete package's invariant clauses"; it is to change to this requirement's wording'); the QSV FR-123 edit itself is STD-217 item 1, a cross-repo change outside this PR. |
