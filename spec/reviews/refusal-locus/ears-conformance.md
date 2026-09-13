---
id: SR-409
title: "EARS conformance review of protocol-role refusal loci"
type: SpecReview
analysis: ears-conformance
scope: "FR-042 refusal-locus statements; FR-042-AC-8"
review_set: subset
---

## Summary

Quire reports the targeted FR-042 and TC-121 documents grammar-clean with zero EARS findings. The
semantic pass replaced a non-canonical `Before` trigger with explicit `When` event clauses naming
the compiler and retained the separate invariant prohibition.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No remaining EARS grammar or semantic ambiguity was found in the changed refusal-locus requirements. | FR-042; FR-042-AC-8 |

The first event establishes the owning locus when role admission begins. The second event defines
the exact source/span response when admission refuses in a multi-source package. The final
`SHALL NOT` clause states the unwanted stale-source condition without changing the refusal verdict.
