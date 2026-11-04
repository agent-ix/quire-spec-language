---
id: SR-20261104139906
title: "QSL-1416 failure-domain analysis"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language; FR-498 and TC-1288"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-498
    type: reviews
---

## Summary
The change explicitly identifies each emitted claim node by its structural `NodeRef`, a declared occurrence, and a matching sidecar row, while rejecting name/span/order inference. A source permutation can still reassign the producer-owned identity between claims and pass the current set-equality check.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Identity reassignment between claims across source permutations remains undetected because the criterion preserves only the identity set, not each claim's identity; assert the per-claim mapping to rule out declaration-order identity. | FR-498-AC-1, FR-498-AC-2, TC-1288 |

## Scope
- FR-498-AC-1..4: examined for entity identity, duplicate/unmatched associations, no-emission behavior, and graph/sidecar topology.
- TC-1288 steps 1–4: examined for identity confusion and edge-case coverage.

## Analysis
Uniqueness and one-to-one association are explicit. No callback failure, user-supplied evaluation purity, or graph traversal algorithm is introduced by this specification slice. The unresolved risk is identity reassignment under the stated permutation case.

## Dispositions

Round 1: the corrected FR-498-AC-1 and TC-1288 now make the vector-to-identity correspondence observable across source-order permutations. Finding FND-001 is fixed.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | after_excerpt: Permuting source order preserves the mapping from each vector identity (`PC-01`, `PC-02`) to its producer-owned `{declaration, occurrence}` and emits rows in the required structural order. |
