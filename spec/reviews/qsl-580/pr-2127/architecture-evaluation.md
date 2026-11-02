---
id: SR-2026110215396
title: "QSL-580 PR 2127 architecture evaluation"
type: SpecReview
analysis: architecture-evaluation
scope: "S2 time-invariant forms and S3 timed-model checking/emission in FR-393 and FR-230"
review_set: subset
---

## Summary

Evaluated the parser-to-form and form-to-checked-package boundaries for ordinary and quantified invariant bounds, including an `exists` refusal path. The change keeps source preservation in S2 and leaves admission/emission semantics to S3, with typed binders governed by the QSpec carrier.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
