---
id: SR-60307
title: "Scope-boundary review of QSL conformance input adapters"
type: SpecReview
analysis: scope-boundary
scope: "QSpec FR-336 and QSL FR-350 input adapters"
review_set: all
---

## Summary

QSpec owns the harness files and tagged values; QSL owns mapping those inputs to its existing APIs. The spec states that these representations do not change native-run/1 or the typed FR-071 product request.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No ownership ambiguity found; FR-336 defines harness shapes and FR-350 binds QSL-specific conversion and admission. | FR-336, FR-350 |
