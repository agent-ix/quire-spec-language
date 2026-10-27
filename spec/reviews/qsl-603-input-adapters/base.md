---
id: SR-60301
title: "Base review of QSL conformance input adapters"
type: SpecReview
analysis: base
scope: "spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md; spec/test-cases/TC-875-the-runner-executes-and-compares-vectors-through-public-entry-points.md; paired QSpec FR-336 input-file contract"
review_set: all
---

## Summary

Reviewed the QSL adapter bindings and paired QSpec contract. Field mappings are closed and tied to the existing typed Call, FR-106 population admission and FR-071/072 replay contracts; the new adapter formats are explicitly harness-owned.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new call, population and replay criteria are specified but their source trace bindings will only exist after the implementation tests are added; TC-875 names each obligation. | FR-350-AC-8, FR-350-AC-9, FR-350-AC-10, FR-336-AC-7 |
