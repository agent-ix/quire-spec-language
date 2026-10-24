---
id: SR-16366
title: "Failure-domain review of the QSL-932/933/934 RA weak-memory spec batch"
type: SpecReview
analysis: failure-domain
scope: "FR-215, FR-224, FR-225, FR-227, FR-228, FR-338; TC-669, TC-670, TC-672, TC-673, TC-1230"
review_set: subset
---

## Summary

The RA lasso criterion names a repeated stale load and the per-thread/location strong visibility constraint. Replay is required to recompute the memory component and check all memory constraints, so no additional failure-domain rule is needed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found in the changed failure-domain scope. | FR-227-AC-4, FR-228-AC-1 |
