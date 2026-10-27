---
id: SR-60307
title: "Scope-boundary review of QSL conformance input adapters"
type: SpecReview
analysis: scope-boundary
scope: "QSpec FR-336 and QSL FR-350 input adapters"
review_set: all
---

## Summary

QSpec owns the replay harness representation; QSL owns conversion to ReplayRequestWire. FR-323 is reused only for source, target and result members, and bytes_path remains harness metadata.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The corpus adapter does not alter FR-071/098 product request semantics. | FR-336; FR-323; FR-350; FR-071 |
