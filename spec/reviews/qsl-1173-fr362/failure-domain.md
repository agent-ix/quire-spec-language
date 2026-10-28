---
id: SR-60318
title: "Failure-domain review of QSL-1173 FR-362 source reference change"
type: SpecReview
analysis: failure-domain
scope: "QSL-1173; FR-362, FR-480, ADR-033, TC-1026, TC-1027, TC-1210"
review_set: subset
---

## Summary

Reviewed source identity, malformed input, and multi-unit boundaries. One `RawSourceRef` identifies each checked unit; source intake still validates the input, and a graph/report mismatch is not exposed as an artificial runtime failure.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No new failure-domain gap found. Parse and type-stage failures remain governed by their existing typed outcomes, and each unit in a multi-unit package keeps its own source reference. | FR-362, FR-480 |
