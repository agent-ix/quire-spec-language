---
id: SR-17212
title: "QSL-1084/1085/1086/1087 integrity review"
type: SpecReview
analysis: integrity
scope: "FR-331-AC-9; TC-847 step 8"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-331
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-847
    type: reviews
---

## Summary

Checked the new replay criterion for consistency, atomicity and testability. AC-9 states a single settlement rule, names the concrete clause and trace, and has a corresponding procedure and expected result in TC-847.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No remaining integrity issue found. AC-9 distinguishes the packet's atom locus from the actual activation locus and checks the inconclusive settlement at the same position and cause. | FR-331-AC-9, TC-847 step 8 |
