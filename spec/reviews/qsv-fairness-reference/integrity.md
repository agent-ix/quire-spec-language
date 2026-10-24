---
id: SR-12701
title: "Integrity check of QSL and QSV fairness ownership"
type: SpecReview
analysis: integrity
scope: "FR-123, TC-518, QSV FR-121, QSL FR-129, ADR-011 X-11"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: reviews
---

## Summary

Cross-checked FR-123 against QSV PR #44's FR-121. QSL still resolves model
operations, supplies their declaring-type `EffectiveId` and name, resolves
omitted kind and granularity, rejects unsupported profiles, deduplicates by
kind/operation/granularity in first-occurrence source order, includes the list
in checked identity, classifies property forms and checks intervals. QSV owns
the shared value definitions, equality, ordering and canonical encoding. FR-123
no longer defines a competing `DeclarationKey`-based value. This is an author
check pending independent review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
