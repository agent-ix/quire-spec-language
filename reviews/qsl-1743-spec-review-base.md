---
id: SR-20261105101027
title: "Spec review base for QSL-1743 PR #2415"
type: SpecReview
analysis: base
scope: "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md, FR-390-AC-68, TC-1071 row 26"
review_set: subset
---
## Summary

The changed FR-390 rule is consistent with the existing static dependency-closure and introduced-binder contract; the new wording states the unread-dependency edge explicitly. No defect found in the changed requirement or its trace target.

## Scope

| Unit | Role | Excerpt |
| --- | --- | --- |
| `FR-390 rule 3` | examined | A binder introduced by a let, collection query, or case arm is not a source; the new sentence clarifies unread binders even when listed as dependencies. |
| `FR-390-AC-68` | examined | Sources exclude introduced binders; unread binders are not sources even when application dependencies list them. |
| `TC-1071 row 26` | examined | Specifies unread binder and case-arm walk forms and expected source set/refusal. |
| `QSpec FR-464` | context_only | Dependency closure definition referenced by FR-390 rule 3. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
