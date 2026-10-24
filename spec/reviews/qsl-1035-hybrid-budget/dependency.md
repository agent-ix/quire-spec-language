---
id: SR-1313
title: "Dependency analysis of hybrid budget criteria"
type: SpecReview
analysis: dependency
scope: "FR-250-AC-3/5, TC-705"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-250
    type: reviews
---

## Summary

FR-250 budget behavior depends on FR-331's `hybrid.limits` contract and FR-447's checked package/request emission. TC-705 consumes both through the in-process and process provider fixtures. The dependency direction remains acyclic.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No dependency-cycle or ordering issue found. | FR-250; FR-447; TC-705 |
