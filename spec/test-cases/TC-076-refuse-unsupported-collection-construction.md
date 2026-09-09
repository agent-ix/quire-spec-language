---
id: TC-076
title: "Refuse unsupported collection construction"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-008
    type: verifies
---
# TC-076: Refuse unsupported collection construction

## Description

Integration, priority P1. Verifies FR-008-AC-20. Planned until actual Rust
execution. Setup uses qualified public model/checker APIs; unrelated setup
failure cannot stand in for the intended phase's outcome.

## Test Procedure

Submit an otherwise valid native source containing collect whose hypothetical output duplicates values through the actual frontend, alongside a supported duplicate-preserving Seq/forall control. Keep helper, recursion and temporal refusal controls in the existing frontend suite.

## Expected Results

Collect receives unsupported_construct at the actual frontend and cannot reach runtime evaluation. The supported control preserves duplicate occurrences. No deduplication, skipped assertion or unrelated model-setup failure substitutes for the required refusal.
