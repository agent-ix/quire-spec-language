---
id: SR-16370
title: "STD-392 risk and complexity review"
type: SpecReview
analysis: risk-complexity
scope: "FR-187 and FR-189 quantity-measure behavior"
review_set: all
---

## Summary

The remaining risk is low: exact arithmetic and unit checking are established dependencies, and the change specifies how EN-4 uses them. The unit ambiguity is mitigated with a value example and a same-dimension conversion case.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No high-risk or high-volatility driver was found in the changed scope. | FR-189-AC-15 |

## Risk register

| Requirement | Technical risk | Volatility | Driver | Mitigation |
| --- | --- | --- | --- | --- |
| FR-189-AC-15 | Low | Low | Exact quantity magnitudes depend on existing Rational and checked unit contracts. | TC-624 exercises integer and fractional magnitudes, equivalent spellings, threshold normalization and pre-sampling refusals. |

## Top hazards

1. Interpreting a value in a base unit would change the quantile magnitude; FR-189-AC-15 fixes the reward and threshold unit to `ms` in its positive case.

Failure-domain gaps: none found.
