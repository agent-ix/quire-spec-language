---
id: SR-1305
title: "Risk and complexity analysis of hybrid replay requirements"
type: SpecReview
analysis: risk-complexity
scope: "FR-450, FR-290-AC-19, ADR-013-O-16/O-24"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-450
    type: reviews
---

## Summary

The shared result type and wire codec change has high technical risk because all `DisagreementCause` consumers must remain exhaustive and encoding must round-trip the new hybrid payload. The wire shape is a pre-release extension, and the two variants are explicitly specified.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Code review must check every `DisagreementCause` match and typed cause codec, and round-trip the full counterexample in both variants. | ADR-013; FR-450; FR-290 |
