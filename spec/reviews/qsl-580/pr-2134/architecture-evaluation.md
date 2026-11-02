---
id: SR-2026110215405
title: "QSL-580 PR 2134 architecture evaluation"
type: SpecReview
analysis: architecture-evaluation
scope: "S2 source form and S3 timed-model carrier scope"
review_set: subset
---
# SR-2026110215405: QSL-580 PR 2134 architecture evaluation

## Summary

The refusal follows the checked-package boundary: `time_invariant` has a single optional ordered binder aggregate that scopes its predicate and bounds, so one node cannot preserve both quantified and unquantified bound scopes. Keeping S2 lossless and refusing the unrepresentable mixed S3 form is a sound boundary decision. The positive quantified-only invariant remains governed by FR-230-AC-56 and QSpec FR-322-AC-295 / FR-415-AC-24.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No defect found in the scoped analysis. | Changed FR-230/TC-685 scope |
