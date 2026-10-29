---
id: SR-117434
title: "QSL-1232 failure-domain analysis"
type: SpecReview
analysis: failure-domain
scope: "Symmetry/POR state-graph exploration, evidence and settlement"
review_set: all
---

## Summary

The requirements cover reduction admission, canonical state coalescing, binding-aware labels, incomplete runs, and the distinction between exhaustive and uncertified reduced proofs. Edge permutations and concrete replay are explicitly retained through the existing ADR-021/FR-170 contract.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No unresolved failure-domain gap found in the changed behavior. | FR-167; FR-168; FR-169; ADR-021 EI-1 to EI-7; ADR-022 GR-1 to GR-4 |

## Failure-domain checks

- A selected reduction that does not preserve a claim is refused before expansion.
- Open nodes and stopped runs cannot produce `HoldsReduced`; possible witnesses on such runs remain unchecked.
- Predicate undefinedness continues to take precedence over a reduced proof.
- Symmetry uses each claim instance's binding stabilizer and retains the concrete-to-canonical permutation needed to recover transitions.
- POR proofs are marked reduced and uncertified; concrete counterexamples retain the existing replay path.
