---
id: SR-12703
title: "Scope boundary check of QSL and QSV fairness values"
type: SpecReview
analysis: scope-boundary
scope: "FR-123, QSV FR-121, ADR-011 X-11"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: reviews
---

## Summary

QSL's S3 checker owns the core checking and collection duties in FR-123. It
consumes QSV's public `fairness` values and canonical encoding contract from
FR-121; it does not re-export or redefine them. The QSV contract is a named
external dependency with its own planned tests, and ADR-011 X-11 allocates it
to the lower shared-value layer.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No amendment-specific ownership overlap found. | FR-123 Outputs and Behavior; QSV FR-121 |
