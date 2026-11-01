---
id: TC-1023
title: "Model identity uses owner and original while facts retain ancestry"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: verifies
---
# TC-1023: Model identity uses owner and original while facts retain ancestry

## Description

Verify FR-081-AC-10: an effective identity is the structural pair of its
owner effective type and original declaration key. Parent and phase-4
provenance stays in derivation facts and the effective view, and distinct
effective declarations never share an identity.

## Test Procedure

Implement as the Rust integration test
`tests/it/model_identity_links.rs::identity_uses_owner_and_original_while_facts_retain_ancestry`.

1. Normalize a five-type chain with a field on the root and a redefining field
   on the leaf. Record each declaration's `{owner_effective_type, original}`
   identity, derivation facts and the effective view.
2. Rebuild the fixture with a source declaration key changed. Verify that the
   identity changes for each declaration whose `owner_effective_type` or
   `original` component changed.
3. Rebuild it with one middle supertype edge changed, then with one phase-4
   subset or redefine fact changed. Verify that the affected inherit or
   phase-4 facts and the effective view change. For declarations whose owner
   and original key are unchanged, verify that their structural identities
   remain unchanged.
4. In every view, verify that distinct effective declarations have distinct
   structural identities.

## Expected Results

An effective identity is exactly the owning effective type's identity (or
`null` for an effective type) paired with the original declaration key. A
source-key change changes the corresponding identity component. Parent-edge
and phase-4 changes are preserved in derivation facts and affect the effective
view without being added to an unchanged declaration's identity. No two
distinct effective declarations in a view share an identity. Equal-shaped
declarations with distinct original keys have distinct identities.
