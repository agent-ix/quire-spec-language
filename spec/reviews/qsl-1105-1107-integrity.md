---
id: SR-20112
title: "Integrity review of QSL-1105 and QSL-1107"
type: SpecReview
analysis: integrity
scope: "FR-401 and FR-485 changes for QSL-1105/QSL-1107"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-401
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-485
    type: reviews
---

## Summary

FR-401 assigns the leaf error vocabulary and reservation-control dependency; FR-485 assigns owner-scoped nominal views and propagation through the checked-closure facade. The ACs are observable and distinguish the failure and ownership branches without changing the existing union contract.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No material completeness, consistency, or atomicity defect found in the changed requirements. | FR-401, FR-485 |
