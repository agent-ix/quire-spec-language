---
id: SR-16631
title: "QSL-1012 risk and complexity review"
type: SpecReview
analysis: risk-complexity
scope: "Exact Quantity<ms> value encoding and integration in probabilistic models"
review_set: all
---

## Summary

The main implementation risks are preserving rational exactness, keeping the unit attached to identity, and rejecting a unit mismatch before sampling. The encoder uses the existing exact rational representation and unit identity; focused and crate-level tests exercise those boundaries. The additional model unit typeRef work is limited to admitting and resolving the existing unit namespace.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The selected change introduces no unresolved high-volatility design choice or unbounded new mechanism. | FR-120-AC-16; FR-189-AC-15 |
