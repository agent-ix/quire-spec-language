---
id: TC-227
title: "Reusable semantic library resolution"
type: TC
relationships:
  - target: ix://agent-ix/quire-specification/FR-307
    type: verifies
---

# TC-227: Reusable semantic library resolution

## Description

Resolve a compatible diamond and reject conflict, cycle, ambiguity and invalid migration.

## Test Procedure

Use the exact version-locked subject and inputs named by FR-307. Execute its positive case, each declared boundary, and each explicit refusal/non-conclusive mutation through the real local Rust boundary; compare the complete typed result and artifact accounting with the criterion.

## Expected Results

Every positive and boundary result matches FR-307; each invalid, unsupported or exhausted mutation returns its exact typed disposition with source/provenance, emits no approximation, and preserves independent sibling results.
