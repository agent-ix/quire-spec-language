---
id: SR-277
title: "Failure-domain review of the runtime input schema"
type: SpecReview
analysis: failure-domain
scope: "FR-024-AC-5; native-state-input/1 schema at f4679ef"
review_set: all
---
## Summary

Reviewed schema/reader trust boundaries, identity, purity and topology. Schema
success grants structural parsed-data validity, with no artifact admission.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings | - |

## Checks

All references are local schema definitions; validation needs no resolver or
foreign runtime. Exact selected bytes remain the reader's authority. Duplicate
keys and fractional/exponent integer tokens demonstrate information lost before
schema validation; stale digest and cyclic arena controls fail at the reader's
own stages. Artifact limits, model lookup, closure and frames remain outside the
schema. Flat ValueIds do not introduce recursive record schemas. No callback,
shared state, I/O or evaluation side effect is added.
