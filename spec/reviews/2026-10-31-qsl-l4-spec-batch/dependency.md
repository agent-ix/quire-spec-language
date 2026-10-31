---
id: SR-1940016
title: "Dependency review of the QSL Layer-4 specification batch"
type: SpecReview
analysis: dependency
scope: "QSL-1248/498/1363/1365: FR-041, FR-042, FR-047, FR-098, FR-100, FR-104, FR-218, FR-383; TC-1056, TC-121, TC-136, TC-137"
review_set: subset
---
# Dependency review of the QSL Layer-4 specification batch

## Summary

The changed requirements were checked for dependency cycles and retired-owner references. FR-100, FR-104, FR-098, FR-046, FR-047 and FR-049 now point to current spine owners where their behaviors have successors.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No new dependency cycle found; retained temporal/choreography gaps are explicitly assigned to QSL-1380. | FR-041, FR-046, FR-047, FR-049 |
