---
id: SR-117428
title: "Dependency review of FR-361 QSpec conformance statements"
type: SpecReview
analysis: dependency
scope: "FR-361-AC-10, FR-361-AC-15, TC-1017 steps 6 and 10"
review_set: subset
---

## Summary

The conformance checks depend on the QSpec lock and layer-selection contracts in FR-001, FR-201-AC-9, and FR-453. FR-361 still depends on QSL FR-110 for E3 resolution and on its own model-gating behavior.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No new dependency edge or cycle was introduced; QSpec remains the source of the compared lock entries. | FR-361, FR-110, FR-001, FR-201-AC-9, FR-453 |
