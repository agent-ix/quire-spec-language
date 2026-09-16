---
id: TC-190
title: "Collection query and conversion algebra"
type: TC
relationships:
  - target: ix://agent-ix/quire-specification/FR-145
    type: verifies
---

# TC-190: Collection query and conversion algebra

## Description

Run map/filter/flatten/fold and explicit lossy conversions across every collection kind.

## Test Procedure

Use the exact version-locked subject and inputs named by FR-145. Execute its positive case, each declared boundary, and each explicit refusal/non-conclusive mutation through the real local Rust boundary; compare the complete typed result and artifact accounting with the criterion.

## Expected Results

Every positive and boundary result matches FR-145; each invalid, unsupported or exhausted mutation returns its exact typed disposition with source/provenance, emits no approximation, and preserves independent sibling results.
