---
id: TC-1022
title: "Model identity chain is unique and binds ancestry"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: verifies
---
# TC-1022: Model identity chain is unique and binds ancestry

## Description

Verify FR-081-AC-10: an identity binds its whole ancestry through the parent
link, and distinct effective declarations never share a preimage.

## Test Procedure

Implement as the Rust integration test
`tests/it/model_identity_links.rs::identity_binds_ancestry_and_is_unique`.

1. Normalize a five-type chain with a field on the root and a redefining field
   on the leaf. Record every effective identity.
2. Rebuild the fixture four times, each with one change: the root's original
   key, one middle supertype edge replaced by an edge to a sibling, the
   redefinition target, and one subset target.
3. For each rebuild, compare every effective identity with the baseline.
4. Across the baseline and all rebuilds, collect every effective declaration's
   preimage bytes and identity.

## Expected Results

Each change alters the identity of every descendant that reaches the changed
item and leaves identities of unrelated declarations unchanged. No two distinct
effective declarations share a preimage or an identity. Two declarations with
equal declared shape and distinct original keys differ in their identities.
