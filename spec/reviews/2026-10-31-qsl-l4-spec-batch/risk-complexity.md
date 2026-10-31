---
id: SR-1940018
title: "Risk review of the QSL Layer-4 specification batch"
type: SpecReview
analysis: risk-complexity
scope: "QSL-1248/498/1363/1365: FR-041, FR-042, FR-047, FR-098, FR-100, FR-104, FR-218, FR-383; TC-1056, TC-121, TC-136, TC-137"
review_set: subset
---
# Risk review of the QSL Layer-4 specification batch

## Summary

The principal migration risk is retained temporal/choreography behavior whose old artifact path is retired before complete spine successors exist. The disposition table preserves these behaviors and assigns their mapping/test work to QSL-1380.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-043/044/045/048/050/052/053 retain native behaviors without complete named spine successors; QSL-1380 tracks the Layer-4 migration and prevents silent deletion. | FR-041, QSL-1380 |
