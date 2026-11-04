---
id: SR-20261104139901
title: "QSL-1416 spec diff code review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; spec/functional/FR-498-carry-a-probabilistic-claim-s-checked-package-identity.md, spec/test-cases/TC-1288-a-probabilistic-claim-carries-its-checked-package-identity.md"
review_set: subset
---

## Summary
The spec-only change closes the producer node-emission dependency and aligns TC-1288 with the new contract. One gap remains: the permutation check compares only an identity set, so it does not prove that an identity remains associated with the same claim rather than being reassigned by source order.

## Verdict
Finding: medium. No production code changed, so implementation gap analysis does not apply. FR-498-AC-1 and TC-1288 steps 1–2 were examined together for requirement/test alignment.

## Scope
- FR-498-AC-1 (`spec/functional/FR-498-carry-a-probabilistic-claim-s-checked-package-identity.md`): examined. “Permuting source order preserves the set of producer-owned node and occurrence identities” does not assert the per-claim association is preserved.
- FR-498-AC-2 (same path): examined. It requires shared identity and disallows derivation from declaration order, but does not add the missing permutation oracle.
- TC-1288 Description, Test Procedure steps 1–2, Expected Results steps 1–2 (`spec/test-cases/TC-1288-a-probabilistic-claim-carries-its-checked-package-identity.md`): examined. The steps match checked claims to nodes by the identity under test, so the stated set comparison can pass if identities are reassigned between claims.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The source-permutation oracle preserves only the set of identities, not each identity's association with its checked claim; declaration-order reassignment can therefore pass the criterion despite being forbidden. Assert the same identity remains attached to each distinguishable claim across permutations. | FR-498-AC-1, FR-498-AC-2, TC-1288 |

## Dispositions

Round 1: the corrected FR-498-AC-1 and TC-1288 now make the vector-to-identity correspondence observable across source-order permutations. Finding FND-001 is fixed.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | after_excerpt: Permuting source order preserves the mapping from each vector identity (`PC-01`, `PC-02`) to its producer-owned `{declaration, occurrence}` and emits rows in the required structural order. |
