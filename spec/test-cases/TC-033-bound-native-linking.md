---
id: TC-033
title: "Enforce native linking resource ceilings"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-013
    type: verifies
---

## Description

Enforce native linking resource ceilings. Integration/property controls, priority P1. Traces: FR-013-AC-5.
Planned before implementation; use the public link API and actual IR constructors.

## Test Procedure

Exercise model/import/clause/node/depth and per-model/aggregate canonical byte budgets with real IR environments. For each active dimension use its exact required count, one lower, zero and a caller value above the hard ceiling. Include a deep flat syntax chain that parsing accepts but linking traversal cannot admit. Run the same healthy request after each refused request.

## Expected Results

An exact permitted boundary succeeds where all other prerequisites hold. A needed operation above a lowered or hard ceiling returns resource_exhausted with is_incomplete true and no package; zero never disables a limit. Later healthy requests are unchanged. Emitted-byte bounds are not described as allocator-capacity bounds.

