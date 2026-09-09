---
id: TC-064
title: "Retain mixed runtime diagnostics"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-007
    type: verifies
---
# TC-064: Retain mixed runtime diagnostics

## Description

Property, priority P1. Verifies FR-007-AC-5, FR-007-AC-13. Planned until actual Rust
execution. Setup uses qualified public model/checker APIs; unrelated setup
failure cannot stand in for the intended phase's outcome.

## Test Procedure

Generate inventory/population/field permutations of mixed stale, invalid-value, incomplete-population and frame defects using ample budgets. Inspect native clause/foreign-selection spans, related exact model loci and runtime paths. Then lower work and diagnostic capacity, including zero, to stop during the known-defect traversal.

## Expected Results

Complete diagnostic enumeration has identical sorted phase/location/span/code results under permutations. Stopped runs retain only actually observed diagnostics and a separate terminal reason, with no full-enumeration claim. A known invalid defect keeps overall Refused even if detail capacity is zero; otherwise an incomplete defect keeps Incomplete. No Boolean or partial validated context escapes.
