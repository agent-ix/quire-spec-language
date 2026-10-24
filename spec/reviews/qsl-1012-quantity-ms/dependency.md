---
id: SR-16626
title: "QSL-1012 dependency review"
type: SpecReview
analysis: dependency
scope: "QSL-1012 Quantity<ms> code path: FR-120, FR-185, FR-186 and FR-189"
review_set: all
---

## Summary

The dependency chain is acyclic. FR-185 supplies typed random parameter domains, FR-186 checks and normalizes quantity thresholds, FR-120 owns exact simulation value encoding, and FR-189 consumes the checked reward type and reports the quantile.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No missing or cyclic dependency was found for the implemented path. | FR-120; FR-185; FR-186; FR-189 |

## Dependency graph

```mermaid
graph LR
  FR185[FR-185 typed supports] --> FR189[FR-189 measure]
  FR186[FR-186 threshold checking] --> FR189
  FR120[FR-120 simulation values] --> FR189
  FR187[FR-187 rewards] --> FR189
```

FR-185 and FR-186 provide the type and threshold contracts; this change implements the FR-120 and FR-189 consumer path.
