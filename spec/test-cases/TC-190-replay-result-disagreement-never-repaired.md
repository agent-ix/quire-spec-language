---
id: TC-190
title: "A replay disagreement settles inconclusive with a typed cause and is never repairable"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-072
    type: verifies
---
# TC-190: A replay disagreement settles inconclusive with a typed cause and is never repairable

## Description

Verify that a disagreement between the proved verdict and the replayed
verdict (as taken from the fixed ADR-013 O-16-category-to-verdict map)
settles `inconclusive` with a typed cause, and that the replay result
type's public API exposes no constructor, setter, or `From`/`TryFrom`
conversion capable of turning a disagreement into an agreement result. This
is the specific defect #231's acceptance criteria call out by name: "tests
fail if ... identities are re-keyed without correspondence, or if outcome
categories collapse." A wrong implementation this test would catch: a
result type with a public `force_agreement()` or a `Default`/`From<bool>`
impl that lets a caller downstream of the executor override a disagreement,
silently repairing what AD-016 arrow 7 requires to stay disagreeing. Scope:
FR-072-AC-2.

## Test Procedure

1. Construct a proved-verdict value and a replayed-verdict value for the
   same item that differ under the O-16-category-to-verdict map (for
   example, proved `success`, replayed `refusal`).
2. Build the replay result from these two verdicts.
3. Enumerate every public constructor, setter, and conversion trait
   implementation (`From`, `TryFrom`, `Default`) on the replay result type.
4. Attempt, for each item enumerated in step 3, to produce an agreement
   result (`reproduced-with-evaluated-witness` or
   `reproduced-without-witness`) from the disagreeing verdicts of step 1.

## Expected Results

- The result built in step 2 settles `inconclusive` with a typed cause
  naming the disagreement.
- No item enumerated in step 3 can produce an agreement result from
  disagreeing verdicts; every attempt in step 4 either fails to compile or
  is rejected at runtime with a refusal, never silently returning an
  agreement value.
