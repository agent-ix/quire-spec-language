---
id: SR-20113
title: "Dependency review of QSL-1105 and QSL-1107"
type: SpecReview
analysis: dependency
scope: "FR-401 and FR-485 changes for QSL-1105/QSL-1107"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-401
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-485
    type: reviews
---

## Summary

The existing FR-485 dependency on FR-401 orders the shared leaf behavior before the QSL facade propagation. QSL-1105 supplies owner-authored composite member data for nominal environment construction; QSL-1107 consumes QSV's `EnvironmentFailure` vocabulary and reservation behavior.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The newly required QSV public reservation-control seam is not implemented by the QSL-locked QSV revision: `TypeEnvironment` construction keeps reservation policy private, and `admit_supplied_with_reservation` is a different operation. The QSL code cannot meet the real Allocation/Capacity acceptance path until the QSV owner supplies this seam or an equivalent executable public route. | FR-401-AC-13, FR-485-AC-15 |
