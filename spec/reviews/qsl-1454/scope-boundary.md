---
id: SR-202611010124
title: "Scope-boundary review of QSL-1454 evaluator value reach"
type: SpecReview
analysis: scope-boundary
scope: "FR-390 evaluator projection; TC-1289 through TC-1292"
review_set: all
---
## Summary

QSL owns which admitted values its evaluator reaches and its reader-to-kernel projection. IR owns typed reads and identity witnesses; QSpec and the shared value kernel retain their existing semantic and canonical-value contracts. The change adds no operator or conversion semantics.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No responsibility allocation or external-boundary gap found in the changed scope. | FR-390 |
