---
id: TC-049
title: "Retain exact checked source and authored clause bindings"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-016
    type: verifies
---
# TC-049: Retain exact checked source and authored clause bindings

## Description

Integration, priority P1. Verifies FR-016-AC-6. Planned until real execution; setup
must use the qualified source-derived Rust producer and actual public IR APIs.

## Test Procedure

Supply complete explicit clause-name/RequirementRef/ClauseId/ExecutionPoint mappings to check. Independently remove, duplicate or add a binding, change native source labels/path/bytes, collide with a model source identity, or supply a mismatched invariant/operation anchor. Include multiple requirements reusing a local ClauseId and an actual duplicate qualified clause identity.

## Expected Results

Only the complete exact mapping succeeds. The result owns the original parsed/linked AST, all ExprIds/source spans, requirement and clause IDs, selected execution points and model identities. Foreign/duplicate/missing mappings refuse without a checked package; a qualified duplicate is distinguished from the same local ClauseId under another RequirementRef. No IDs are minted from native names.

