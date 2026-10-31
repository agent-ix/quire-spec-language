---
id: SR-1940014
title: "Failure-domain review of the QSL Layer-4 specification batch"
type: SpecReview
analysis: failure-domain
scope: "QSL-1248/498/1363/1365: FR-041, FR-042, FR-047, FR-098, FR-100, FR-104, FR-218, FR-383; TC-1056, TC-121, TC-136, TC-137"
review_set: subset
---
# Failure-domain review of the QSL Layer-4 specification batch

## Summary

The changed refusal paths and identity boundaries were checked for typed causes, exact loci, short-circuit behavior, purity and graph termination. The S3 refusal occurs before emission.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No new failure-domain gap found in the changed contract; bare-entry refusal is typed and pre-emission. | FR-218, FR-383, TC-1056 |
