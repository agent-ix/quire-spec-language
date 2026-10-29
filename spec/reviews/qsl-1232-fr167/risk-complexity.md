---
id: SR-117436
title: "QSL-1232 risk and complexity analysis"
type: SpecReview
analysis: risk-complexity
scope: "State graph symmetry quotient and POR outcome costs"
review_set: all
---

## Summary

The change selects existing ADR-021 reduction algorithms and adds no structural cutoff. The main implementation risks are preserving deterministic canonical edges and ensuring claim labels remain correct for an instance stabilizer.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No new risk requiring a spec change found; implementation must retain ADR-021's canonicalization and POR proviso guarantees. | FR-167; ADR-021 SYM-6/SYM-7; ADR-021 POR-6/POR-8 |

## Risk disposition

- Symmetry canonicalization inherits the sort-based canonicalizer's per-class `O(n log n)` behavior and binding-stabilizer rules; the quotient-size acceptance example guards the intended reduction.
- POR inherits the breadth-first revisit proviso and its enabled-transition footprint contract.
- The spec adds no artificial state/group limit; existing request limits remain the only search limits.
- Determinism must include selected reductions and their parameters in the result identity and canonical reduction order.
