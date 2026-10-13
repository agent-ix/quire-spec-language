---
id: TC-1021
title: "Model identity links expand to exact provenance"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: verifies
---
# TC-1021: Model identity links expand to exact provenance

## Description

Verify FR-081-AC-9: the retained one-edge facts and parent identities expand
to the exact path-form facts, and a mis-wired link is refused.

## Test Procedure

Implement as the Rust integration test
`tests/it/model_identity_links.rs::links_expand_to_independent_enumeration`.

1. Construct a four-type chain, a diamond (`D` with parents `B` and `C`, both
   with parent `A`, `A` declaring a field `f`), and a hidden-redefinition
   fixture (a descendant field redefining an inherited field, plus a second
   redefiner on another branch). Admit each through the real model
   normalization boundary.
2. Enumerate the expected path-form facts independently from the original
   declaration graph, in phase, input and rule order: every distinct supertype
   path, every member inherit fact `[path, member]`, and the subset and
   redefine facts with FR-081's rule: one fact per redefiner or subsets target
   carrying the least path to the owner (so the diamond's phase-4 facts carry
   one path though its inherit facts list both), and the equal-input redefine
   and subset overlap, with exact zero-based decimal ordinals and multiplicity.
3. Expand each declaration's retained facts through the parent identities in
   the test oracle, without calling the production expansion routine, and
   compare every fact.
4. Check retained counts: the diamond type `D` holds one qualify fact and two
   inherit facts, not three; member `(D, f)` holds one qualify fact and two
   inherit facts.
5. Submit mutations to the strict reader: an inherit fact whose parent names
   a member where a type is required, a parent whose original differs from the
   fact's input key, a parent whose owner is not the named supertype, a
   repeated parent on one declaration, and a missing parent preimage.

## Expected Results

Expansion equals the independent enumeration for every effective declaration,
hidden ones included. The diamond's two paths stay distinct in the expansion.
Every mutation refuses with no partial effective view.
