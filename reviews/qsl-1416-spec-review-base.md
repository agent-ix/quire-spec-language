---
id: SR-20261104139902
title: "QSL-1416 base spec review"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; FR-498 and TC-1288"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-498
    type: reviews
---

## Summary
FR-498 now specifies producer emission, row/node/checked-claim correspondence, and the no-emission case, with TC-1288 covering those outputs. One acceptance oracle does not establish per-claim identity stability under source permutation, leaving the explicit ban on declaration-order identity insufficiently tested.

## Verdict
Base checklist verdict: conditional on FND-001. IDs, references, inputs, outputs, and test links are structurally coherent. The test case is linked and criterion verification names a method only. No applicable AssuranceProfile was found under `spec/assurance/`; direct review uses the requested subset. The hash/digest/pin antipattern search found no requirement to create or verify such records. The full-repository computed matrix did not finish within 60 seconds; the edited artifacts validate with 4/4 criteria property-extractable and TC-1288 is explicitly linked from all four criteria.

## Scope
- FR-498-AC-1: examined. “Permuting source order preserves the set of producer-owned node and occurrence identities.”
- FR-498-AC-2: examined. “the producer does not infer identity from name, span, normal form, display text or declaration order.”
- FR-498-AC-3: examined. Cause carries selected claim identity and member path.
- FR-498-AC-4: examined. One-to-one correspondence, no-emission, and sidecar invariance.
- TC-1288 Description, Test Procedure steps 1–4, Expected Results steps 1–4: examined. The case inspects emitted graph nodes, occurrences, sidecar rows, checked claims, and causes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The permutation criterion compares only the set of identities, so identities can be reassigned between claims by declaration order and still pass; require equality of each distinguishable claim's identity across permutations. | FR-498-AC-1, FR-498-AC-2, TC-1288 |

## Dispositions

Round 1: the corrected FR-498-AC-1 and TC-1288 now make the vector-to-identity correspondence observable across source-order permutations. Finding FND-001 is fixed.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | after_excerpt: Permuting source order preserves the mapping from each vector identity (`PC-01`, `PC-02`) to its producer-owned `{declaration, occurrence}` and emits rows in the required structural order. |
