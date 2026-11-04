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

1. Compile one unit with three probabilistic claims: two sharing a declared
   name and differing in written content (the QSL counterpart of QSpec
   TC-233 `PC-01` and `PC-02`, referenced by ID; one source cannot give two
   declarations equal spans) and a third with another name. Emit the package
   and record each claim's `{declaration, occurrence}`, telling the claims
   apart by their written content, never by the identity under test.
   Re-emit every tested permutation of the three declarations and record the
   identities again.
2. For each claim, match the checked claim, its local `claim` node with
   `semantic_form` `probabilistic_claim`, its declared `claim` occurrence and
   its sidecar row by `{declaration, occurrence}`. Rename the third claim and
   re-emit; parse a claim moved, reformatted and commented, and one with a
   changed token, and compare their written content. Compile two claims
   written identically.
3. Check an elapsed-time claim with an unsupported form and a punctual interval
   in a non-admitted form over those claims and read the settled causes.
4. Read the emitted package with the strict reader and check every row names
   a node of the emitted graph with that node's own `claim` occurrence.

Tag the test `#[trace("FR-498-AC-1", "FR-498-AC-2", "FR-498-AC-3", "FR-498-AC-4", "TC-1288")]`.
Vectors are referenced by QSpec case ID, never copied.

## Expected Results

- Step 1: three local claim nodes, each with one declared `claim`
  occurrence, and three matching rows with distinct declaration identities in
  structural order; every permutation keeps each claim's identity and emits
  byte-identical rows.
- Step 2: each checked claim, its graph node and its row expose the same
  `{declaration, occurrence}`; renaming changes only the renamed claim's
  identity; moving, reformatting and commenting leave written content equal
  and a changed token makes it differ; the identical twins have distinct
  identities.
- Step 3: each cause carries the selected claim's identity and member path.
- Step 4: the strict reader admits the package and every row joins its node
  and occurrence.
