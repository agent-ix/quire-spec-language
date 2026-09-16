---
id: TC-188
title: "Record tuple and recursive values"
type: TC
relationships:
  - target: ix://agent-ix/quire-specification/FR-143
    type: verifies
---

# TC-188: Record tuple and recursive values

## Description

Compare same/different declared structures and construct finite versus cyclic recursive values.

## Test Procedure

Use the exact version-locked subject and inputs named by FR-143. Execute its positive case, each declared boundary, and each explicit refusal/non-conclusive mutation through the real local Rust boundary; compare the complete typed result and artifact accounting with the criterion.

## Expected Results

Every positive and boundary result matches FR-143; each invalid, unsupported or exhausted mutation returns its exact typed disposition with source/provenance, emits no approximation, and preserves independent sibling results.
