---
id: TC-1288
title: "A probabilistic claim carries its checked-package identity"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-498
    type: verifies
---
# TC-1288: A probabilistic claim carries its checked-package identity

## Description

Verify that the checked-package producer emits one local
`claim`/`probabilistic_claim` node and one declared `claim` occurrence per
included `CheckedProbabilisticClaim`, writes one matching
`probabilistic_claims` row, that the checked claim exposes the same
producer-owned identity, and that the FR-421 causes carry it. Scope:
FR-498-AC-1 to FR-498-AC-4.

## Test Procedure

1. Compile the QSpec TC-233 claims `PC-01` and `PC-02` (equal display names and
   spans) by reference. Emit the package and inspect the checked semantic graph
   nodes, their declared occurrences and the sidecar rows. Permute source
   order and re-emit.
2. For each claim, match the checked claim, local `claim` node with
   `semantic_form` `probabilistic_claim`, its declared `claim` occurrence and
   sidecar row by producer-owned identity; compare the identity exposed by the
   `CheckedProbabilisticClaim` with the row. Verify no name, span, normal form,
   display text or declaration-order inference supplies that identity.
3. Check an elapsed-time claim with an unsupported form and a punctual interval
   in a non-admitted form over those claims and read the settled causes.
4. Check a claim without package emission and compare its identity and cause.
   Enable or omit sidecar row emission for an equivalent package and verify
   the semantic graph node set and all node identities remain equal.

Tag the test `#[trace("FR-498-AC-1", "FR-498-AC-2", "FR-498-AC-3", "FR-498-AC-4", "TC-1288")]`.
Vectors are referenced by QSpec case ID, never copied.

## Expected Results

- Step 1: two local claim nodes, each with one declared `claim` occurrence,
  and two matching rows with distinct declaration identities; source
  permutation preserves the set of producer-owned identities and the rows
  remain in structural order.
- Step 2: each checked claim, its graph node and its row expose the same
  `{declaration, occurrence}` identity; no name, span, normal form, display
  text or order-derived key participates.
- Step 3: each cause carries the selected claim's identity and member path.
- Step 4: the unemitted claim exposes no package identity and keeps the
  span-locus refusal with no synthesized locus; enabling or omitting the
  sidecar leaves the semantic graph nodes and identities equal.
