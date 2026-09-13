---
id: SR-408
title: "Scope-boundary review of protocol-role refusal loci"
type: SpecReview
analysis: scope-boundary
scope: "FR-042-AC-8; TC-121; issue #68"
review_set: subset
---

## Summary

The native compiler owns the refusal report and its source-coordinate integrity. The formal source
layout is an internal guaranteed mapping dependency; downstream readers consume the resulting locus
but do not repair, guess, or search for its source.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The amendment stays within compiler diagnostic ownership and does not widen package admission, wire format, runtime evaluation, or downstream consumer responsibilities. | FR-042-AC-8; TC-121 |

Responsibility class is core compiler error handling in `protocol_artifact::native`. The existing
layout/source inventory is guaranteed through the compiler's own integration test. Quire extraction,
generated backends, the compiled-protocol schema, and embedded `resources/native-v1/` are outside
this slice and remain unchanged.
