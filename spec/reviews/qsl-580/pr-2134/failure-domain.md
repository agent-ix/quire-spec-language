---
id: SR-2026110215408
title: "QSL-580 PR 2134 failure-domain review"
type: SpecReview
analysis: failure-domain
scope: "Mixed quantified and ordinary invariant bounds"
review_set: subset
---
# SR-2026110215408: QSL-580 PR 2134 failure-domain review

## Summary

The criterion closes the empty-collection hazard identified in PR #2133: the implementation cannot silently attach the quantifier binder to the ordinary bound or drop that condition. It refuses the mixed invariant at its owner with no partial timed graph. The separately specified `forall` empty behavior, `exists` refusal, and element-presence refusal remain covered.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No defect found in the scoped analysis. | Changed FR-230/TC-685 scope |
