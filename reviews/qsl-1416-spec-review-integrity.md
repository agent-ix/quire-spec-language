---
id: SR-20261104139903
title: "QSL-1416 integrity analysis"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; FR-498 and TC-1288"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-498
    type: reviews
---

## Summary
The expanded behavior decomposes node creation, occurrence declaration, sidecar association, checked-claim exposure, no-emission behavior, and sidecar invariance into observable obligations. The source-order identity condition remains under-specified because it constrains an identity set rather than the claim-to-identity mapping.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The permutation oracle allows the producer to swap the identities associated with the two claims while preserving the set; it therefore does not verify the prohibition on declaration-order-derived identity. Specify and test per-claim mapping preservation across the source permutation. | FR-498-AC-1, FR-498-AC-2, TC-1288 |

## Scope
- FR-498-AC-1..4: examined for atomicity, observability, consistency, and verifiability.
- TC-1288 procedure and expected results: examined against all four criteria.

## Analysis
The FR-498 behavior clauses separate the key producer obligations. The new requirement makes node emission an explicit producer responsibility and cross-links it to QSpec FR-322. No contradictory sidecar versus graph behavior was found. The finding concerns the adequacy of one verification oracle, not a missing producer responsibility.

## Dispositions

Round 1: the corrected FR-498-AC-1 and TC-1288 now make the vector-to-identity correspondence observable across source-order permutations. Finding FND-001 is fixed.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | after_excerpt: Permuting source order preserves the mapping from each vector identity (`PC-01`, `PC-02`) to its producer-owned `{declaration, occurrence}` and emits rows in the required structural order. |
