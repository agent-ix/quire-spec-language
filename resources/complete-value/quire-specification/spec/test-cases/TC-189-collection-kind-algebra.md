---
id: TC-189
title: "Collection kind algebra"
type: TC
relationships:
  - target: ix://agent-ix/quire-specification/FR-144
    type: verifies
---

# TC-189: Collection kind algebra

## Description

Construct, permute, compare and overflow sequence, set, bag and ordered-set values.

## Test Procedure

Use the exact version-locked subject and inputs named by FR-144. Execute its positive case, each declared boundary, and each explicit refusal/non-conclusive mutation through the real local Rust boundary; compare the complete typed result and artifact accounting with the criterion.

## Expected Results

Every positive and boundary result matches FR-144; each invalid, unsupported or exhausted mutation returns its exact typed disposition with source/provenance, emits no approximation, and preserves independent sibling results.
