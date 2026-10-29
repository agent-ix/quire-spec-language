---
id: SR-117439
title: "Integrity review of QSL-834 FR-049 retirement"
type: SpecReview
analysis: integrity
scope: "FR-049; TC-134; TC-136; TC-137; TC-142"
review_set: subset
---

## Summary

Checked completeness, consistency, atomicity and testability for the FR-049 replacement map. Each surviving capability has an owning checked-term, family, observation or shared-meter contract; behavior tied only to the retired artifact-graph inputs is identified explicitly as retired.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The mapping distinguishes checked expression selection, model/runtime input, authority, result categories and accounting; the detailed family rules remain with FR-046/047/107/390 instead of being copied into FR-049. | FR-049; TC-136; TC-137; TC-142 |

The replacement tests have separate scopes: input and identity admission (TC-136), work boundaries and retry (TC-137), and V2 emission/read-back integration (TC-142). TC-134 tests static compensation emission only. The old v1/v2 state evaluator compatibility claim is removed under the explicit M-6c decision.
