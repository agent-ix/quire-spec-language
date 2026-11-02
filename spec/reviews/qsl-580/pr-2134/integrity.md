---
id: SR-2026110215404
title: "QSL-580 PR 2134 integrity review"
type: SpecReview
analysis: integrity
scope: "FR-230 quantified and mixed invariant bounds; TC-685 steps 37-38"
review_set: subset
---
# SR-2026110215404: QSL-580 PR 2134 integrity review

## Summary

The new rule and AC-57 are consistent with FR-393-AC-9: S2 preserves a mixed ordinary/quantified bound list, while S3 rejects that combination because one invariant-level binder list cannot express separate scopes. TC-685 step 38 asserts both phases and the no-partial-output result. Existing quantified-only AC-56 remains the positive path.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No defect found in the scoped analysis. | Changed FR-230/TC-685 scope |
