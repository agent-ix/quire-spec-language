---
id: SR-20115
title: "Risk and complexity review of QSL-1105 and QSL-1107"
type: SpecReview
analysis: risk-complexity
scope: "FR-401 and FR-485 changes for QSL-1105/QSL-1107"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-401
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-485
    type: reviews
---

## Summary

The main risk is the new shared-leaf/QSL public error surface, particularly preserving the exact EnvironmentFailure and cancellation distinction across a body transition. It is high risk and externally dependent; the seam and typed propagation need focused public-consumer tests before the aggregate gate.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The QSV reservation-control seam is an external contract and is absent from the QSL-locked revision; avoid implementing a synthetic QSL-only failure route. Resolve the QSV owner dependency before claiming allocation/capacity acceptance. | FR-401-AC-13, FR-485-AC-15 |
