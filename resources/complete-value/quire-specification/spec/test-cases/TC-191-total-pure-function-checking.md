---
id: TC-191
title: "Total pure function checking"
type: TC
relationships:
  - target: ix://agent-ix/quire-specification/FR-146
    type: verifies
---

# TC-191: Total pure function checking

## Description

Check typed pure calls, proved recursion, failed termination and prohibited effects.

## Test Procedure

Use the exact version-locked subject and inputs named by FR-146. Execute its positive case, each declared boundary, and each explicit refusal/non-conclusive mutation through the real local Rust boundary; compare the complete typed result and artifact accounting with the criterion.

## Expected Results

Every positive and boundary result matches FR-146; each invalid, unsupported or exhausted mutation returns its exact typed disposition with source/provenance, emits no approximation, and preserves independent sibling results.
