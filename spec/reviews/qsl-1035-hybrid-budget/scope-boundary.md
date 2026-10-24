---
id: SR-1316
title: "Scope-boundary analysis of hybrid budget coverage"
type: SpecReview
analysis: scope-boundary
scope: "FR-250-AC-3/5, TC-705"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-250
    type: reviews
---

## Summary

QSL owns writing and validating hybrid request limits, canonical key identity, result settlement and record storage. A process provider consumes the FR-331 v2 bytes; the external solver's internal clock and algorithm remain outside this requirement's boundary.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No responsibility allocation gap found in the changed scope. | FR-250; FR-447; TC-705 |
