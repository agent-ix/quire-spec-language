---
id: SR-1933002
title: "EARS review for FR-493 and NFR-009-AC-3"
type: SpecReview
analysis: ears-conformance
scope: "Normative statements added or changed in FR-493 and NFR-009-AC-3"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-493
    type: reviews
  - target: ix://agent-ix/quire-spec-language/NFR-009
    type: reviews
---

## Summary

The Behavior bullets in FR-493 each name QSL or the root package as subject and state one absence, one retention or one refusal. The new bullet on keeping `formal_source`, `native_model` and `composed` modules states one obligation and one order.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The bullet "QSL SHALL keep ... this requirement neither deletes nor forbids them" mixes an obligation with a disclaimer; acceptable, but the disclaimer carries no testable content. | FR-493 Behavior |
| FND-002 | low | NFR-009-AC-3 is two conditions (overflow, non-limit denial) in one criterion; both are tested by separate TC-137 controls C1 and C2. | NFR-009-AC-3 |
