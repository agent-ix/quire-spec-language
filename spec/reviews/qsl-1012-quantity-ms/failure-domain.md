---
id: SR-16629
title: "QSL-1012 failure-domain review"
type: SpecReview
analysis: failure-domain
scope: "Quantity typeRef intake, model assembly, exact encoding and sampling refusal paths"
review_set: all
---

## Summary

The checked boundary retains unresolved unit references through intake and resolves them against the compiling unit. An unknown unit refusal carries both the member declaration URI and a typed member path. Mismatched quantity support is rejected before the sampler runs. Quantity encoding keeps exact rational numerator/denominator and the unit identity digest; it does not coerce values to floating point or silently change units.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No unhandled quantity-identity or pre-sampling failure mode was found in the selected path. | FR-056-AC-42; FR-185-AC-8; FR-189-AC-15 |

## Review note

DomainPackage records do not carry a source span for `typeRef`; the refusal therefore locates the declaration by its identity and typed path while retaining the model-selection span.
